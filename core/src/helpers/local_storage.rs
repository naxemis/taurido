use std::path::PathBuf;
use dirs::data_dir;
use crate::errors::storage_error::StorageError;

pub struct LocalStorage {
    app_directory: PathBuf,
    lists_directory: PathBuf,
    metadata_path: PathBuf,
}

impl LocalStorage {

    pub fn new() -> Result<Self, StorageError> {
        let data_directory = data_dir().ok_or(StorageError::DataDirectoryNotFound)?;

        let app_directory = data_directory.join("tuido");
        let lists_directory = app_directory.join("lists");
        let metadata_path = app_directory.join("metadata.json");

        Self::create_directory(&lists_directory)?;

        Ok(Self {
            app_directory,
            lists_directory,
            metadata_path,
        })

    }

    fn create_directory(path: &PathBuf) -> Result<(), StorageError> {
        match std::fs::create_dir_all(path) {
            Ok(()) => Ok(()),
            Err(source) => {
                Err(StorageError::Io {
                    path: path.clone(),
                    source,
                })
            }
        }
    }
}