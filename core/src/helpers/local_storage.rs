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
        Self::with_data_directory(data_directory)
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

    fn with_data_directory(data_directory: PathBuf) -> Result<Self, StorageError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn storage(temp: &TempDir) -> LocalStorage {
        LocalStorage::with_data_directory(temp.path().to_path_buf()).unwrap()
    }

    fn write_file(path: &Path, contents: &str) {
        std::fs::write(path, contents).unwrap();
    }

    fn write_manager(path: &Path, manager: &ListManager) {
        write_file(path, &serde_json::to_string(manager).unwrap());
    }

    fn manager_with_lists(names: &[&str]) -> ListManager {
        let mut manager = ListManager::new();

        for name in names {
            manager.add(name).unwrap();
        }

        manager
    }

    fn loaded_manager(outcome: LoadOutcome) -> ListManager {
        match outcome {
            LoadOutcome::Loaded(manager) => manager,
            other => panic!("Expected Loaded, got {other:?}"),
        }
    }

    fn recovered_manager(outcome: LoadOutcome) -> (ListManager, PathBuf) {
        match outcome {
            LoadOutcome::Recovered { manager, source } => (manager, source),
            other => panic!("Expected Recovered, got {other:?}"),
        }
    }

    #[test]
    fn load_lists_returns_missing_when_no_candidate_files_exist() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);

        let outcome = storage.load_lists().unwrap();

        assert!(matches!(outcome, LoadOutcome::Missing));
    }

    #[test]
    fn load_lists_returns_loaded_from_lists_json() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_manager(&storage.lists_path, &manager_with_lists(&["A"]));

        let manager = loaded_manager(storage.load_lists().unwrap());

        let list = manager.get_by_id(0).expect("List should be loaded");
        assert_eq!(list.get_name(), "A");
    }

    #[test]
    fn load_lists_preserves_next_id() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);

        let mut saved = manager_with_lists(&["A", "B"]);
        saved.remove(0);
        write_manager(&storage.lists_path, &saved);

        let mut manager = loaded_manager(storage.load_lists().unwrap());

        assert_eq!(manager.add("C"), Ok(2));
        assert_eq!(manager.get_by_id(2).unwrap().get_name(), "C");
    }

    #[test]
    fn load_lists_recovers_from_presave_when_lists_json_is_missing() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_manager(
            &storage.presave_lists_path,
            &manager_with_lists(&["Presaved"]),
        );

        let (manager, source) = recovered_manager(storage.load_lists().unwrap());

        assert_eq!(source, storage.presave_lists_path);
        assert_eq!(manager.get_by_id(0).unwrap().get_name(), "Presaved");
    }

    #[test]
    fn load_lists_recovers_from_presave_when_lists_json_is_corrupt() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_file(&storage.lists_path, "Invalid json");
        write_manager(
            &storage.presave_lists_path,
            &manager_with_lists(&["Presaved"]),
        );

        let (manager, source) = recovered_manager(storage.load_lists().unwrap());

        assert_eq!(source, storage.presave_lists_path);
        assert_eq!(manager.get_by_id(0).unwrap().get_name(), "Presaved");
    }

    #[test]
    fn load_lists_recovers_from_startup_when_earlier_candidates_are_unusable() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_file(&storage.lists_path, "Invalid json");
        write_file(&storage.presave_lists_path, "Invalid json");
        write_manager(
            &storage.startup_lists_path,
            &manager_with_lists(&["From startup"]),
        );

        let (manager, source) = recovered_manager(storage.load_lists().unwrap());

        assert_eq!(source, storage.startup_lists_path);
        assert_eq!(manager.get_by_id(0).unwrap().get_name(), "From startup");
    }

    #[test]
    fn load_lists_returns_first_deserialization_error_when_all_candidates_are_corrupt() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        let corrupt_lists = "Invalid json";
        write_file(&storage.lists_path, corrupt_lists);
        write_file(&storage.presave_lists_path, "[]");
        write_file(&storage.startup_lists_path, "{}");

        let expected = serde_json::from_str::<ListManager>(corrupt_lists)
            .err()
            .expect("Corrupt input should fail to deserialize")
            .to_string();

        match storage.load_lists() {
            Err(StorageError::Deserialization { source }) => {
                assert_eq!(source.to_string(), expected);
            }
            other => panic!("Expected Deserialization error, got {other:?}"),
        }
    }

    #[test]
    fn load_lists_returns_io_error_without_trying_fallback_candidates() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        // Reading a directory produces a non-NotFound I/O error.
        std::fs::create_dir(&storage.lists_path).unwrap();
        write_manager(
            &storage.presave_lists_path,
            &manager_with_lists(&["Presaved"]),
        );

        match storage.load_lists() {
            Err(StorageError::Io { path, .. }) => {
                assert_eq!(path, storage.lists_path);
            }
            other => panic!("Expected Io error, got {other:?}"),
        }
    }
}

// todo(tests): Add tests for LocalStorage methods
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
