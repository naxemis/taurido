use crate::errors::storage_error::StorageError;
use dirs::data_dir;
use std::path::PathBuf;
use serde::Serialize;
use crate::managers::list_manager::ListManager;

pub struct LocalStorage {
    _app_directory: PathBuf,
    lists_path: PathBuf,
    temp_lists_path: PathBuf,
    presave_lists_path: PathBuf,
}

// todo(storage): save_lists(&ListManager) -> Result<(), StorageError>
//   1. Write the string to temp_lists.json and sync_all - file failure -> Err(Io)
//   2. If lists.json exists, copy it to presave_lists.json - copy failure -> Err(Io)
//   3. Rename temp_lists.json -> lists.json - rename failure -> Err(Io)

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

    fn create_directory(path: &PathBuf) -> Result<(), StorageError> {
        match std::fs::create_dir_all(path) {
            Ok(()) => Ok(()),
            Err(source) => Err(StorageError::Io {
                path: path.clone(),
                source,
            }),
        }
    }

    pub fn save_lists(&self, manager: &ListManager) -> Result<(), StorageError> {
        let _data = serde_json::to_string(manager).map_err(|source| StorageError::Serialization { source })?;

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