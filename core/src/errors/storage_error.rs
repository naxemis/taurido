use std::io;
use std::path::PathBuf;

// todo(storage): Add a variant for a lock held by another instance, so the GUI can tell it apart from I/O errors

/// Errors that can occur while reading or writing local storage.
#[derive(Debug)]
pub enum StorageError {
    /// A file or directory could not be read, written, created, or renamed.
    Io { path: PathBuf, source: io::Error },
    /// Lists could not be serialized to JSON.
    Serialization { source: serde_json::Error },
    /// Stored data is not valid JSON or does not match the expected list format.
    Deserialization { source: serde_json::Error },
    /// The platform data directory could not be determined.
    DataDirectoryNotFound,
    /// The stored file uses a format version that this build cannot read.
    UnsupportedFormatVersion { path: PathBuf, version_found: u64 },
}
