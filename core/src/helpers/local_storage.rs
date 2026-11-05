use crate::errors::storage_error::StorageError;
use crate::helpers::load_outcome::LoadOutcome;
use crate::managers::list_manager::ListManager;
use dirs::data_dir;
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct ListsFile<'manager> {
    format_version: u16,
    #[serde(flatten)]
    list_manager: &'manager ListManager,
}

// Write-only, ready with serde_json::Value
pub struct LocalStorage {
    app_directory: PathBuf,
    lists_path: PathBuf,
    temp_lists_path: PathBuf,
    presave_lists_path: PathBuf,
    startup_lists_path: PathBuf,
}

impl LocalStorage {
    pub const FORMAT_VERSION: u16 = 1;

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
            app_directory,
            lists_path,
            temp_lists_path,
            presave_lists_path,
            startup_lists_path,
        })
    }

    // todo(storage): A failed application directory sync should be logged as a warning (waiting for TUI implementation)
    #[cfg(unix)]
    fn sync_app_directory(path: &Path) {
        let Ok(directory) = File::open(path) else {
            return;
        };

        let _ = directory.sync_all();
    }

    #[cfg(not(unix))]
    fn sync_app_directory(_path: &Path) {
        Ok(())
    }

    fn write_lists(&self, manager: &ListManager) -> Result<(), StorageError> {
        let lists_file = ListsFile {
            format_version: Self::FORMAT_VERSION,
            list_manager: manager,
        };

        let data = serde_json::to_string(&lists_file)
            .map_err(|source| StorageError::Serialization { source })?;

        let result = (|| -> Result<(), StorageError> {
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
        })();

        match result {
            Ok(()) => {}
            Err(error) => {
                let _ = std::fs::remove_file(&self.temp_lists_path);
                return Err(error);
            }
        };

        Self::sync_app_directory(&self.app_directory);

        Ok(())
    }

    // todo(storage): std::fs::copy cannot say which side failed, so errors always blame
    //   the destination. The source is also copied before validation, so a corrupt
    //   lists.json overwrites the last good presave.
    //   - Read the source as raw bytes (NotFound means nothing to copy yet), then write
    //     presave through temp + rename: a crash mid-copy must not truncate the backup
    //   - Validate before copying, or skip the copy when the source does not parse
    fn copy_if_exists(from: &Path, to: &Path) -> Result<(), StorageError> {
        match std::fs::copy(from, to) {
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Ok(_) => Ok(()),
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
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Ok(data) => {
                let mut document: serde_json::Value = serde_json::from_str(&data)
                    .map_err(|source| StorageError::Deserialization { source })?;

                let format_version = document
                    .as_object_mut()
                    .and_then(|map| map.remove("format_version"));
                let format_version = match format_version {
                    None => 1,
                    Some(value) => serde_json::from_value::<u64>(value)
                        .map_err(|source| StorageError::Deserialization { source })?,
                };

                if format_version != u64::from(Self::FORMAT_VERSION) {
                    return Err(StorageError::UnsupportedFormatVersion {
                        path: path.to_path_buf(),
                        version_found: format_version,
                    });
                };

                let manager = serde_json::from_value::<ListManager>(document)
                    .map_err(|source| StorageError::Deserialization { source })?;

                Ok(Some(manager))
            }
            Err(source) => Err(StorageError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    pub fn load_lists(&self) -> Result<LoadOutcome, StorageError> {
        let mut first_error: Option<StorageError> = None;

        // todo(storage): nothing writes startup_lists.json yet (TUI startup snapshot), so
        //   the third candidate is dead weight until then.
        let candidates = [
            &self.lists_path,
            &self.presave_lists_path,
            &self.startup_lists_path,
        ];

        // todo(refactor): index == 0 breaks silently if candidates are reordered; compare
        //   against lists_path instead.
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

// todo(tests): Cover the storage todos above
//   - identical text across two saves (BTreeMap)
//   - load without the version field, and with a future version
//   - corrupt lists.json preserved next to the recovered one
//   - temp file removed after a failed save
//   - read-side failure names the source path
//   - file text with keys in id order, compared against an expected literal
//     (catches HashMap order; a two-save comparison would not)

// todo(tests): Add startup snapshot and lock tests (waiting for TUI implementation)
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

    fn read_file(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap()
    }

    fn read_manager(path: &Path) -> ListManager {
        LocalStorage::read_lists_file(path)
            .unwrap()
            .expect("File should contain a manager")
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
        write_file(&storage.lists_path, "Invalid JSON");
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
        write_file(&storage.lists_path, "Invalid JSON");
        write_file(&storage.presave_lists_path, "Invalid JSON");
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
        let corrupt_lists = "Invalid JSON";
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

    #[test]
    fn recovery_writes_recovered_data_to_lists_json() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);

        let mut saved = manager_with_lists(&["A", "B"]);
        saved.remove(0);
        write_manager(&storage.presave_lists_path, &saved);

        let (manager, source) = recovered_manager(storage.load_lists().unwrap());
        assert_eq!(source, storage.presave_lists_path);
        assert_eq!(manager.get_by_id(1).unwrap().get_name(), "B");

        let mut written = read_manager(&storage.lists_path);
        assert_eq!(written.get_by_id(1).unwrap().get_name(), "B");
        assert_eq!(written.add("C"), Ok(2));
    }

    #[test]
    fn recovery_does_not_change_existing_presave_lists_json() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_file(&storage.lists_path, "Invalid JSON");
        write_manager(&storage.presave_lists_path, &manager_with_lists(&["A"]));
        let presave_before = read_file(&storage.presave_lists_path);

        let (_, source) = recovered_manager(storage.load_lists().unwrap());
        assert_eq!(source, storage.presave_lists_path);

        assert_eq!(read_file(&storage.presave_lists_path), presave_before);
    }

    #[test]
    fn recovery_creates_missing_lists_json_from_valid_fallback() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        assert!(!storage.lists_path.exists());
        write_manager(&storage.startup_lists_path, &manager_with_lists(&["A"]));

        let (_, source) = recovered_manager(storage.load_lists().unwrap());
        assert_eq!(source, storage.startup_lists_path);

        assert!(storage.lists_path.exists());
        let written = read_manager(&storage.lists_path);
        assert_eq!(written.get_by_id(0).unwrap().get_name(), "A");
    }

    #[test]
    fn recovery_leaves_no_temp_lists_json_after_successful_write_back() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_manager(&storage.presave_lists_path, &manager_with_lists(&["A"]));

        recovered_manager(storage.load_lists().unwrap());

        assert!(!storage.temp_lists_path.exists());
    }

    #[test]
    fn recovery_returns_io_error_and_keeps_lists_json_when_write_back_fails() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        let corrupt_lists = "Invalid JSON";
        write_file(&storage.lists_path, corrupt_lists);
        write_manager(&storage.presave_lists_path, &manager_with_lists(&["A"]));
        std::fs::create_dir(&storage.temp_lists_path).unwrap();

        match storage.load_lists() {
            Err(StorageError::Io { path, .. }) => {
                assert_eq!(path, storage.temp_lists_path);
            }
            other => panic!("Expected Io error, got {other:?}"),
        }

        assert_eq!(read_file(&storage.lists_path), corrupt_lists);
    }

    #[test]
    fn save_lists_saves_data_that_can_be_loaded_back_with_next_id() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        let mut saved = manager_with_lists(&["A", "B"]);
        saved.remove(0);

        storage.save_lists(&saved).unwrap();

        let mut loaded = loaded_manager(storage.load_lists().unwrap());
        assert_eq!(loaded.get_by_id(1).unwrap().get_name(), "B");
        assert_eq!(loaded.add("C"), Ok(2));
    }

    #[test]
    fn save_lists_copies_existing_lists_json_to_presave_before_saving() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_manager(&storage.lists_path, &manager_with_lists(&["Old"]));
        let lists_before = read_file(&storage.lists_path);

        storage.save_lists(&manager_with_lists(&["New"])).unwrap();

        assert_eq!(read_file(&storage.presave_lists_path), lists_before);
        let written = read_manager(&storage.lists_path);
        assert_eq!(written.get_by_id(0).unwrap().get_name(), "New");
    }

    #[test]
    fn save_lists_does_not_create_presave_when_lists_json_is_missing() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);

        storage.save_lists(&manager_with_lists(&["A"])).unwrap();

        assert!(!storage.presave_lists_path.exists());
        let written = read_manager(&storage.lists_path);
        assert_eq!(written.get_by_id(0).unwrap().get_name(), "A");
    }

    #[test]
    fn save_lists_leaves_no_temp_lists_json_after_successful_save() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);

        storage.save_lists(&manager_with_lists(&["A"])).unwrap();

        assert!(!storage.temp_lists_path.exists());
    }

    #[test]
    fn save_lists_returns_io_error_and_keeps_lists_json_when_write_fails() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_manager(&storage.lists_path, &manager_with_lists(&["Old"]));
        let lists_before = read_file(&storage.lists_path);
        std::fs::create_dir(&storage.temp_lists_path).unwrap();

        match storage.save_lists(&manager_with_lists(&["New"])) {
            Err(StorageError::Io { path, .. }) => {
                assert_eq!(path, storage.temp_lists_path);
            }
            other => panic!("Expected Io error, got {other:?}"),
        }

        assert_eq!(read_file(&storage.lists_path), lists_before);
    }
}
