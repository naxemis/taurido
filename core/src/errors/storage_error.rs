use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum StorageError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Serialization {
        source: serde_json::Error,
    },
    Deserialization {
        source: serde_json::Error,
    },
    DataDirectoryNotFound,
}
