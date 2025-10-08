//! Error handling for ONEcode operations

use std::fmt;

/// Result type for ONEcode operations
pub type Result<T> = std::result::Result<T, OneError>;

/// Errors that can occur when working with ONE files
#[derive(Debug, Clone, PartialEq)]
pub enum OneError {
    /// Failed to open file
    OpenFailed(String),

    /// Failed to close file
    CloseFailed,

    /// Failed to read from file
    ReadFailed,

    /// Failed to write to file
    WriteFailed,

    /// Invalid file format
    InvalidFormat(String),

    /// Schema error
    SchemaError(String),

    /// Null pointer encountered
    NullPointer,

    /// Invalid UTF-8 string
    InvalidUtf8(std::str::Utf8Error),

    /// Invalid CString (contains internal null byte)
    InvalidCString(std::ffi::NulError),

    /// Generic error with message
    Other(String),
}

impl fmt::Display for OneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OneError::OpenFailed(path) => write!(f, "Failed to open file: {}", path),
            OneError::CloseFailed => write!(f, "Failed to close file"),
            OneError::ReadFailed => write!(f, "Failed to read from file"),
            OneError::WriteFailed => write!(f, "Failed to write to file"),
            OneError::InvalidFormat(msg) => write!(f, "Invalid file format: {}", msg),
            OneError::SchemaError(msg) => write!(f, "Schema error: {}", msg),
            OneError::NullPointer => write!(f, "Unexpected null pointer"),
            OneError::InvalidUtf8(e) => write!(f, "Invalid UTF-8: {}", e),
            OneError::InvalidCString(e) => write!(f, "Invalid C string: {}", e),
            OneError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for OneError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OneError::InvalidUtf8(e) => Some(e),
            OneError::InvalidCString(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::str::Utf8Error> for OneError {
    fn from(err: std::str::Utf8Error) -> Self {
        OneError::InvalidUtf8(err)
    }
}

impl From<std::ffi::NulError> for OneError {
    fn from(err: std::ffi::NulError) -> Self {
        OneError::InvalidCString(err)
    }
}
