//! Schema management for ONE files

use crate::error::{OneError, Result};
use crate::ffi;
use std::ffi::CString;

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
    /// This function is now thread-safe. The C library uses `_Thread_local` for
    /// all global state (errorString, isBootStrap) and mkstemp() for temp files.
    pub fn from_text(text: &str) -> Result<Self> {
        let c_text = CString::new(text)?;

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
