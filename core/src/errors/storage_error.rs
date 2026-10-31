use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum StorageError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Serialization {
        path: PathBuf,
        source: serde_json::Error,
    },
    Deserialization {
        path: PathBuf,
        source: serde_json::Error,
    },
    DataDirectoryNotFound,
}
