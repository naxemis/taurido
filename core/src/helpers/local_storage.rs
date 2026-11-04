use crate::errors::storage_error::StorageError;
use crate::helpers::load_outcome::LoadOutcome;
use crate::managers::list_manager::ListManager;
use dirs::data_dir;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct LocalStorage {
    _app_directory: PathBuf,
    lists_path: PathBuf,
    temp_lists_path: PathBuf,
    presave_lists_path: PathBuf,
    startup_lists_path: PathBuf,
}

impl LocalStorage {
    pub fn new() -> Result<Self, StorageError> {
        let data_directory = data_dir().ok_or(StorageError::DataDirectoryNotFound)?;

        let app_directory = data_directory.join("tuido");
        let lists_path = app_directory.join("lists.json");
        let temp_lists_path = app_directory.join("temp_lists.json");
        let presave_lists_path = app_directory.join("presave_lists.json");
        let startup_lists_path = app_directory.join("startup_lists.json");

        Self::create_directory(&app_directory)?;

        Ok(Self {
            _app_directory: app_directory,
            lists_path,
            temp_lists_path,
            presave_lists_path,
            startup_lists_path,
        })
    }

    fn create_directory(path: &Path) -> Result<(), StorageError> {
        match std::fs::create_dir_all(path) {
            Ok(()) => Ok(()),
            Err(source) => Err(StorageError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    fn write_lists(&self, manager: &ListManager) -> Result<(), StorageError> {
        let data = serde_json::to_string(manager)
            .map_err(|source| StorageError::Serialization { source })?;

        {
            let mut temp_lists =
                File::create(&self.temp_lists_path).map_err(|source| StorageError::Io {
                    path: self.temp_lists_path.clone(),
                    source,
                })?;
            temp_lists
                .write_all(data.as_bytes())
                .map_err(|source| StorageError::Io {
                    path: self.temp_lists_path.clone(),
                    source,
                })?;
            temp_lists.sync_all().map_err(|source| StorageError::Io {
                path: self.temp_lists_path.clone(),
                source,
            })?;
        }

        std::fs::rename(&self.temp_lists_path, &self.lists_path).map_err(|source| {
            StorageError::Io {
                path: self.lists_path.clone(),
                source,
            }
        })?;

        Ok(())
    }

    fn copy_if_exists(from: &Path, to: &Path) -> Result<(), StorageError> {
        match std::fs::copy(from, to) {
            Ok(_) => Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(StorageError::Io {
                path: to.to_path_buf(),
                source,
            }),
        }
    }

    pub fn save_lists(&self, manager: &ListManager) -> Result<(), StorageError> {
        Self::copy_if_exists(&self.lists_path, &self.presave_lists_path)?;
        self.write_lists(manager)
    }

    fn read_lists_file(path: &Path) -> Result<Option<ListManager>, StorageError> {
        match std::fs::read_to_string(path) {
            Ok(data) => {
                let manager = serde_json::from_str::<ListManager>(&data)
                    .map_err(|source| StorageError::Deserialization { source })?;

                Ok(Some(manager))
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(StorageError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    pub fn load_lists(&self) -> Result<LoadOutcome, StorageError> {
        let mut first_error: Option<StorageError> = None;
        let candidates = [
            &self.lists_path,
            &self.presave_lists_path,
            &self.startup_lists_path,
        ];

        for (index, path) in candidates.into_iter().enumerate() {
            match Self::read_lists_file(path) {
                Ok(Some(manager)) => {
                    if index == 0 {
                        return Ok(LoadOutcome::Loaded(manager));
                    }

                    self.write_lists(&manager)?;

                    return Ok(LoadOutcome::Recovered {
                        manager,
                        source: path.to_path_buf(),
                    });
                }
                Ok(None) => {}
                Err(error @ StorageError::Deserialization { .. }) => {
                    first_error.get_or_insert(error);
                }
                Err(other) => {
                    return Err(other);
                }
            }
        }

        match first_error {
            None => Ok(LoadOutcome::Missing),
            Some(first_error) => Err(first_error),
        }
    }
}

// todo(tests): Add tests for LocalStorage methods
//   - Put tests in #[cfg(test)] mod tests at the bottom of this file
//   - Add a constructor that accepts a data directory; use a tempfile directory in tests
//   - load_lists: no candidate files exist -> LoadOutcome::Missing
//   - load_lists: valid lists.json -> LoadOutcome::Loaded
//   - load_lists: valid lists.json preserves next_id (the next added list gets the expected ID)
//   - load_lists: missing lists.json and valid presave_lists.json -> Recovered from presave_lists.json
//   - load_lists: corrupt lists.json and valid presave_lists.json -> Recovered from presave_lists.json
//   - load_lists: corrupt lists.json and presave_lists.json, valid startup_lists.json -> Recovered from startup_lists.json
//   - load_lists: all candidates are corrupt -> Deserialization error from the first corrupt candidate
//   - load_lists: a non-NotFound read error -> Io error; do not continue to fallback candidates
//   - recovery: recovered data is written to lists.json
//   - recovery: existing presave_lists.json is not changed by recovery
//   - recovery: missing lists.json is created from a valid fallback
//   - recovery: successful write-back leaves no temp_lists.json
//   - recovery: failed write-back returns Io and does not replace lists.json
//   - save_lists: saved data can be loaded back, with next_id preserved
//   - save_lists: existing lists.json is copied to presave_lists.json before saving
//   - save_lists: missing lists.json does not create presave_lists.json
//   - save_lists: successful save leaves no temp_lists.json
//   - save_lists: failed write leaves the existing lists.json unchanged
//   - Use the tempfile crate as a dev-dependency
//   - Add startup snapshot and lock tests after those features are implemented
