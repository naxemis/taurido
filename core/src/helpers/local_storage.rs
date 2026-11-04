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

// todo(tests): Write tests for methods
//   - Tests go in #[cfg(test)] mod tests at the bottom of this file
//   - Make the base directory injectable: new() calls new_in(data_dir()), tests call new_in(temp_dir)
//   - load_lists: no candidate file exists -> Ok(None)
//   - load_lists: valid JSON -> Ok(Some) with next_id preserved
//   - load_lists: corrupt lists.json, no fallbacks -> Err(Deserialization)
//   - load_lists: corrupt lists.json + valid presave_lists.json -> Ok(Some) from presave
//   - load_lists: corrupt lists.json + corrupt presave + valid startup_lists.json -> Ok(Some) from startup
//   - load_lists: every existing candidate corrupt -> Err(Deserialization) carries the lists.json error, not the last one
//   - load_lists: lists.json is a directory -> Err(Io), even when presave_lists.json is valid
//   - load_lists: presave_lists.json is a directory while lists.json is corrupt -> Err(Io)
//   - load_lists: lists.json missing but presave_lists.json valid -> Ok(Some) from presave (confirm intended)
//   - load_lists: lists.json missing and no fallbacks -> Ok(None)
//   - recovery: lists.json is rewritten with the recovered content
//   - recovery: lists.json rewrite does not touch presave_lists.json (last good snapshot is kept)
//   - recovery: lists.json missing -> recovery recreates it from the fallback
//   - recovery: no temp_lists.json left behind after a successful write-back
//   - recovery: all candidates corrupt -> lists.json stays byte-for-byte unchanged
//   - recovery: failed write-back -> Err(Io) and the recovered state is not silently dropped
//   - save round-trip: loaded state equals saved state, and next_id > 0 is kept
//   - save with existing lists.json -> presave_lists.json holds the previous content
//   - save with no lists.json -> no presave_lists.json is created
//   - save after recovery -> presave_lists.json holds the recovered content, not the corrupt one
//   - after successful save -> no temp_lists.json left behind
//   - failed save -> lists.json stays intact
//   - startup copy -> startup_lists.json holds the content from process start (after TUI implementation)
//   - lock: second attempt to take lists.lock fails (after TUI implementation)
//   - Use tempfile crate as dev-dependency for temporary directories
