//! Schema management for ONE files

use crate::error::{OneError, Result};
use crate::ffi;
use std::ffi::CString;
use std::sync::Mutex;

// Global mutex to protect schema creation
// Even though temp files are now thread-safe (mkstemp patch), the C library's schema
// parsing code has global state (static bool isBootStrap at ONElib.c:282) that makes
// concurrent schema creation unsafe
static SCHEMA_CREATION_LOCK: Mutex<()> = Mutex::new(());

/// A ONE file schema
pub struct OneSchema {
    pub(crate) ptr: *mut ffi::OneSchema,
}

impl OneSchema {
    /// Create a schema from a file
    pub fn from_file(path: &str) -> Result<Self> {
        let c_path = CString::new(path)?;

        unsafe {
            let ptr = ffi::oneSchemaCreateFromFile(c_path.as_ptr());
            if ptr.is_null() {
                return Err(OneError::SchemaError(format!(
                    "Failed to create schema from file: {}",
                    path
                )));
            }
            Ok(OneSchema { ptr })
        }
    }

    /// Create a schema from text
    ///
    /// # Thread Safety
    ///
    /// This function uses a mutex to serialize schema creation. Even though temp file
    /// handling is thread-safe (mkstemp), the C library has global state (isBootStrap)
    /// that makes concurrent schema creation unsafe.
    pub fn from_text(text: &str) -> Result<Self> {
        let c_text = CString::new(text)?;

        // Lock to prevent race condition in C library's schema parsing global state
        let _guard = SCHEMA_CREATION_LOCK.lock().unwrap();

        unsafe {
            let ptr = ffi::oneSchemaCreateFromText(c_text.as_ptr());
            if ptr.is_null() {
                return Err(OneError::SchemaError(
                    "Failed to create schema from text".to_string()
                ));
            }
            Ok(OneSchema { ptr })
        }
    }

    /// Get the internal pointer (for use with FFI functions)
    pub(crate) fn as_ptr(&self) -> *mut ffi::OneSchema {
        self.ptr
    }
}

impl Drop for OneSchema {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                ffi::oneSchemaDestroy(self.ptr);
            }
        }
    }
}

// OneSchema is not thread-safe by default
// Send and Sync would need to be carefully considered
