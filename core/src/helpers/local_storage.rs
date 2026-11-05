use crate::errors::storage_error::StorageError;
use crate::helpers::load_outcome::LoadOutcome;
use crate::managers::list_manager::ListManager;
use dirs::data_dir;
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

/// On-disk layout of `lists.json`: the format version followed by the list data.
///
/// The fields of [`ListManager`] are flattened into the same JSON object.
#[derive(Serialize)]
struct ListsFile<'manager> {
    format_version: u16,
    #[serde(flatten)]
    list_manager: &'manager ListManager,
}

/// Stores the lists file in the application's data directory.
///
/// Files are stored in a `tuido` subdirectory of the platform data directory.
/// Each write goes to a temporary file first and then replaces the target, so an
/// interrupted write does not leave a partially written file behind.
///
/// Lists are only serialized through this struct. When reading, the JSON is first
/// parsed as a `serde_json::Value` so that `format_version` can be checked before the
/// list data is deserialized.
pub struct LocalStorage {
    app_directory: PathBuf,
    lists_path: PathBuf,
    temp_lists_path: PathBuf,
    presave_lists_path: PathBuf,
    temp_presave_lists_path: PathBuf,
    startup_lists_path: PathBuf,
}

impl LocalStorage {
    /// Format version written to `lists.json`. Only this version can be loaded.
    pub const FORMAT_VERSION: u16 = 1;

    /// Creates storage in the platform data directory.
    ///
    /// Creates the `tuido` directory if it does not exist.
    ///
    /// Returns [`StorageError::DataDirectoryNotFound`] if the data directory cannot be
    /// determined, or [`StorageError::Io`] if the application directory cannot be created.
    pub fn new() -> Result<Self, StorageError> {
        let data_directory = data_dir().ok_or(StorageError::DataDirectoryNotFound)?;
        Self::with_data_directory(data_directory)
    }

    /// Creates `path` and any missing parent directories.
    ///
    /// Returns [`StorageError::Io`] with `path` if the directory cannot be created.
    fn create_directory(path: &Path) -> Result<(), StorageError> {
        match std::fs::create_dir_all(path) {
            Ok(()) => Ok(()),
            Err(source) => Err(StorageError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// Creates storage inside the `tuido` subdirectory of `data_directory`.
    ///
    /// Creates the `tuido` directory if it does not exist. This is used by [`Self::new`]
    /// and lets tests run against a temporary directory.
    fn with_data_directory(data_directory: PathBuf) -> Result<Self, StorageError> {
        let app_directory = data_directory.join("tuido");
        let lists_path = app_directory.join("lists.json");
        let temp_lists_path = app_directory.join("temp_lists.json");
        let presave_lists_path = app_directory.join("presave_lists.json");
        let temp_presave_lists_path = app_directory.join("temp_presave_lists.json");
        let startup_lists_path = app_directory.join("startup_lists.json");

        Self::create_directory(&app_directory)?;

        Ok(Self {
            app_directory,
            lists_path,
            temp_lists_path,
            presave_lists_path,
            temp_presave_lists_path,
            startup_lists_path,
        })
    }

    /// Syncs the application directory so that renamed files survive a crash.
    ///
    /// Errors are ignored.
    // todo(storage): A failed application directory sync should be logged as a warning (waiting for TUI implementation)
    #[cfg(unix)]
    fn sync_app_directory(path: &Path) {
        let Ok(directory) = File::open(path) else {
            return;
        };

        let _ = directory.sync_all();
    }

    /// Does nothing on platforms where directories cannot be synced.
    #[cfg(not(unix))]
    fn sync_app_directory(_path: &Path) {}

    /// Writes `data` to `temp`, syncs it to disk, and renames it to `target`.
    ///
    /// `target` is replaced only after the data has been synced. If any step fails,
    /// `temp` is removed and `target` keeps its previous content.
    ///
    /// Returns [`StorageError::Io`] with the path of the failed step.
    fn replace_atomically(
        &self,
        data: &[u8],
        temp: &Path,
        target: &Path,
    ) -> Result<(), StorageError> {
        let result = (|| -> Result<(), StorageError> {
            {
                let mut temp_lists = File::create(temp).map_err(|source| StorageError::Io {
                    path: temp.to_path_buf(),
                    source,
                })?;
                temp_lists
                    .write_all(data)
                    .map_err(|source| StorageError::Io {
                        path: temp.to_path_buf(),
                        source,
                    })?;
                temp_lists.sync_all().map_err(|source| StorageError::Io {
                    path: temp.to_path_buf(),
                    source,
                })?;
            }

            std::fs::rename(temp, target).map_err(|source| StorageError::Io {
                path: target.to_path_buf(),
                source,
            })?;

            Ok(())
        })();

        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = std::fs::remove_file(temp);
                Err(error)
            }
        }
    }

    /// Serializes `manager` with the current format version and replaces `lists.json`.
    ///
    /// Returns [`StorageError::Serialization`] if the data cannot be serialized, or
    /// [`StorageError::Io`] if the file cannot be written.
    fn write_lists(&self, manager: &ListManager) -> Result<(), StorageError> {
        let lists_file = ListsFile {
            format_version: Self::FORMAT_VERSION,
            list_manager: manager,
        };

        let data = serde_json::to_string(&lists_file)
            .map_err(|source| StorageError::Serialization { source })?;

        self.replace_atomically(data.as_bytes(), &self.temp_lists_path, &self.lists_path)?;

        Self::sync_app_directory(&self.app_directory);

        Ok(())
    }

    /// Copies the current `lists.json` to `presave_lists.json`.
    ///
    /// Nothing is copied if `lists.json` is missing, or if it cannot be parsed or uses an
    /// unsupported format version. This keeps the last good presave file intact.
    ///
    /// Returns [`StorageError::Io`] if the file cannot be read or copied.
    fn copy_lists_to_presave(&self) -> Result<(), StorageError> {
        match Self::read_lists_bytes(&self.lists_path) {
            Ok(None) => Ok(()),
            Ok(Some(data)) => match Self::parse_lists_bytes(&self.lists_path, &data) {
                Ok(_) => self.replace_atomically(
                    &data,
                    &self.temp_presave_lists_path,
                    &self.presave_lists_path,
                ),
                Err(
                    StorageError::Deserialization { .. }
                    | StorageError::UnsupportedFormatVersion { .. },
                ) => Ok(()),
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        }
    }

    /// Saves `manager` to `lists.json`.
    ///
    /// The previous `lists.json` is first copied to `presave_lists.json` when it is valid.
    ///
    /// Returns an error if the backup or the write fails. In that case `lists.json` keeps
    /// its previous content.
    pub fn save_lists(&self, manager: &ListManager) -> Result<(), StorageError> {
        self.copy_lists_to_presave()?;
        self.write_lists(manager)
    }

    /// Parses lists data and checks its format version.
    ///
    /// Data without a `format_version` field is treated as version 1.
    ///
    /// Returns [`StorageError::Deserialization`] if the data is not valid JSON or does not
    /// match the list format, or [`StorageError::UnsupportedFormatVersion`] if the version
    /// differs from [`Self::FORMAT_VERSION`]. `path` is used only for the error.
    fn parse_lists_bytes(path: &Path, data: &[u8]) -> Result<ListManager, StorageError> {
        let mut document: serde_json::Value = serde_json::from_slice(data)
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

        Ok(manager)
    }

    /// Reads the raw contents of the file at `path`.
    ///
    /// Returns `Ok(None)` if the file does not exist.
    ///
    /// Returns [`StorageError::Io`] for any other read error.
    fn read_lists_bytes(path: &Path) -> Result<Option<Vec<u8>>, StorageError> {
        match std::fs::read(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Ok(data) => Ok(Some(data)),
            Err(source) => Err(StorageError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// Reads and parses the lists file at `path`.
    ///
    /// Returns `Ok(None)` if the file does not exist. Parsing errors are the same as
    /// [`Self::parse_lists_bytes`].
    fn read_lists_file(path: &Path) -> Result<Option<ListManager>, StorageError> {
        match Self::read_lists_bytes(path) {
            Ok(None) => Ok(None),
            Ok(Some(data)) => {
                let manager = Self::parse_lists_bytes(path, &data)?;
                Ok(Some(manager))
            }
            Err(error) => Err(error),
        }
    }

    /// Loads the lists, falling back to backup copies if `lists.json` cannot be used.
    ///
    /// Candidates are tried in this order: `lists.json`, `presave_lists.json`, and
    /// `startup_lists.json`.
    ///
    /// - If `lists.json` loads, returns [`LoadOutcome::Loaded`].
    /// - If a backup loads, it is written back to `lists.json` and
    ///   [`LoadOutcome::Recovered`] is returned with the path of the backup.
    /// - If no candidate exists, returns [`LoadOutcome::Missing`].
    /// - If candidates exist but cannot be parsed, returns the first
    ///   [`StorageError::Deserialization`] error.
    ///
    /// Other errors, such as I/O errors or unsupported format versions, are returned
    /// immediately without trying the remaining candidates.
    pub fn load_lists(&self) -> Result<LoadOutcome, StorageError> {
        let mut first_error: Option<StorageError> = None;

        // todo(storage): nothing writes startup_lists.json yet (waiting for TUI implementation)
        let candidates = [
            &self.lists_path,
            &self.presave_lists_path,
            &self.startup_lists_path,
        ];

        for path in candidates {
            match Self::read_lists_file(path) {
                Ok(Some(manager)) => {
                    if path == &self.lists_path {
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
    fn load_lists_defaults_missing_format_version_to_one() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_file(
            &storage.lists_path,
            r#"{"lists":{"0":{"id":0,"name":"A","column_manager":{"columns":{},"next_id":0}}},"next_id":1}"#,
        );

        let manager = loaded_manager(storage.load_lists().unwrap());

        assert_eq!(manager.get_by_id(0).unwrap().get_name(), "A");
    }

    #[test]
    fn load_lists_returns_unsupported_version_without_fallback() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_file(
            &storage.lists_path,
            r#"{"format_version":2,"lists":{},"next_id":0}"#,
        );
        write_manager(
            &storage.presave_lists_path,
            &manager_with_lists(&["Presaved"]),
        );

        match storage.load_lists() {
            Err(StorageError::UnsupportedFormatVersion {
                path,
                version_found,
            }) => {
                assert_eq!(path, storage.lists_path);
                assert_eq!(version_found, 2);
            }
            other => panic!("Expected UnsupportedFormatVersion error, got {other:?}"),
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

    #[test]
    fn save_lists_keeps_last_good_presave_when_lists_json_is_corrupt() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_file(&storage.lists_path, "Invalid JSON");
        write_manager(
            &storage.presave_lists_path,
            &manager_with_lists(&["Presaved"]),
        );
        let presave_before = read_file(&storage.presave_lists_path);

        storage.save_lists(&manager_with_lists(&["New"])).unwrap();

        assert_eq!(read_file(&storage.presave_lists_path), presave_before);
        let written = read_manager(&storage.lists_path);
        assert_eq!(written.get_by_id(0).unwrap().get_name(), "New");
        assert!(!storage.temp_presave_lists_path.exists());
    }

    #[test]
    fn save_lists_skips_presave_backup_for_future_format_version() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_file(
            &storage.lists_path,
            r#"{"format_version":2,"lists":{},"next_id":0}"#,
        );
        write_manager(
            &storage.presave_lists_path,
            &manager_with_lists(&["Presaved"]),
        );
        let presave_before = read_file(&storage.presave_lists_path);

        storage.save_lists(&manager_with_lists(&["New"])).unwrap();

        assert_eq!(read_file(&storage.presave_lists_path), presave_before);
        let written = read_manager(&storage.lists_path);
        assert_eq!(written.get_by_id(0).unwrap().get_name(), "New");
        assert!(!storage.temp_presave_lists_path.exists());
    }

    #[test]
    fn save_lists_returns_io_error_and_keeps_files_when_temp_presave_is_blocked() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_manager(&storage.lists_path, &manager_with_lists(&["Old"]));
        write_manager(
            &storage.presave_lists_path,
            &manager_with_lists(&["Presaved"]),
        );
        let lists_before = read_file(&storage.lists_path);
        let presave_before = read_file(&storage.presave_lists_path);
        std::fs::create_dir(&storage.temp_presave_lists_path).unwrap();

        match storage.save_lists(&manager_with_lists(&["New"])) {
            Err(StorageError::Io { path, .. }) => {
                assert_eq!(path, storage.temp_presave_lists_path);
            }
            other => panic!("Expected Io error, got {other:?}"),
        }

        assert_eq!(read_file(&storage.lists_path), lists_before);
        assert_eq!(read_file(&storage.presave_lists_path), presave_before);
    }

    #[test]
    fn save_lists_leaves_no_temp_presave_after_successful_backup() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        write_manager(&storage.lists_path, &manager_with_lists(&["Old"]));

        storage.save_lists(&manager_with_lists(&["New"])).unwrap();

        assert!(storage.presave_lists_path.exists());
        assert!(!storage.temp_presave_lists_path.exists());
    }

    #[test]
    fn save_lists_produces_identical_text_across_two_saves() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);
        let manager = manager_with_lists(&["A", "B", "C"]);

        storage.save_lists(&manager).unwrap();
        let first = read_file(&storage.lists_path);

        storage.save_lists(&manager).unwrap();
        let second = read_file(&storage.lists_path);

        assert_eq!(first, second);
    }

    #[test]
    fn save_lists_writes_format_version_and_lists_in_id_order() {
        let temp = TempDir::new().unwrap();
        let storage = storage(&temp);

        storage
            .save_lists(&manager_with_lists(&["A", "B"]))
            .unwrap();

        let expected = concat!(
            r#"{"format_version":1,"lists":{"#,
            r#""0":{"id":0,"name":"A","column_manager":{"columns":{},"next_id":0}},"#,
            r#""1":{"id":1,"name":"B","column_manager":{"columns":{},"next_id":0}}}"#,
            r#","next_id":2}"#,
        );
        assert_eq!(read_file(&storage.lists_path), expected);
    }
}
