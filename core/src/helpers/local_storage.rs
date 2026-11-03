use crate::errors::storage_error::StorageError;
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
}

// todo(storage): load_lists() -> Result<Option<ListManager>, StorageError>
//   - Ok(None) when lists.json does not exist (first start)
//   - Err(Io) on access errors, Err(Deserialization) on corrupt JSON

// todo(storage): Atomic save of the whole ListManager
//   1. Write temp_lists.json and sync_all
//   2. Copy lists.json -> presave_lists.json
//   3. Rename temp_lists.json -> lists.json

// todo(recovery): On Deserialization error try presave_lists.json, then startup_lists.json (to confirm)
impl LocalStorage {
    pub fn new() -> Result<Self, StorageError> {
        let data_directory = data_dir().ok_or(StorageError::DataDirectoryNotFound)?;

        let app_directory = data_directory.join("tuido");
        let lists_path = app_directory.join("lists.json");
        let temp_lists_path = app_directory.join("temp_lists.json");
        let presave_lists_path = app_directory.join("presave_lists.json");

        Self::create_directory(&app_directory)?;

        Ok(Self {
            _app_directory: app_directory,
            lists_path,
            temp_lists_path,
            presave_lists_path,
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
        let current_data = serde_json::to_string(manager)
            .map_err(|source| StorageError::Serialization { source })?;

        {
            let mut temp_lists =
                File::create(&self.temp_lists_path).map_err(|source| StorageError::Io {
                    path: self.temp_lists_path.clone(),
                    source,
                })?;
            temp_lists
                .write_all(current_data.as_bytes())
                .map_err(|source| StorageError::Io {
                    path: self.temp_lists_path.clone(),
                    source,
                })?;
            temp_lists.sync_all().map_err(|source| StorageError::Io {
                path: self.temp_lists_path.clone(),
                source,
            })?;
        }

        Self::copy_if_exists(&self.lists_path, &self.presave_lists_path)?;

        std::fs::rename(&self.temp_lists_path, &self.lists_path).map_err(|source| {
            StorageError::Io {
                path: self.lists_path.clone(),
                source,
            }
        })?;

        Ok(())
    }
}

// todo(tests): Write tests for methods
//   - Tests go in #[cfg(test)] mod tests at the bottom of this file
//   - Make the base directory injectable: new() calls new_in(data_dir()), tests call new_in(temp_dir)
//   - load_lists: missing file -> Ok(None)
//   - load_lists: valid JSON -> Ok(Some) with next_id preserved
//   - load_lists: corrupt JSON -> Err(Deserialization)
//   - load_lists: lists.json is a directory -> Err(Io)
//   - save round-trip: loaded state equals saved state, and next_id > 0 is kept
//   - save with existing lists.json -> presave_lists.json holds the previous content
//   - save with no lists.json -> no presave_lists.json is created
//   - after successful save -> no temp_lists.json left behind
//   - failed save -> lists.json stays intact
//   - startup copy -> startup_lists.json holds the content from process start
//   - lock: second attempt to take lists.lock fails
//   - Use tempfile crate as dev-dependency for temporary directories
