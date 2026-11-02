use crate::errors::storage_error::StorageError;
use dirs::data_dir;
use std::path::PathBuf;

pub struct LocalStorage {
    app_directory: PathBuf,
    lists_path: PathBuf,
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

        Self::create_directory(&app_directory)?;

        Ok(Self {
            app_directory,
            lists_path,
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