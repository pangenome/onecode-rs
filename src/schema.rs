//! Schema management for ONE files

use crate::error::{OneError, Result};
use crate::ffi;
use std::ffi::CString;
use std::sync::Mutex;

// Global mutex to protect oneSchemaCreateFromText() calls
// The C library uses /tmp/OneTextSchema-{pid}.schema which creates race conditions
// when multiple threads in the same process call this function simultaneously
static SCHEMA_FROM_TEXT_LOCK: Mutex<()> = Mutex::new(());

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
    /// This function uses a global mutex to serialize access to the underlying C function
    /// `oneSchemaCreateFromText()`, which has a race condition when called from multiple
    /// threads simultaneously (it uses `/tmp/OneTextSchema-{pid}.schema` as a temporary file).
    pub fn from_text(text: &str) -> Result<Self> {
        let c_text = CString::new(text)?;

        // Lock to prevent race condition in C library's temp file handling
        let _guard = SCHEMA_FROM_TEXT_LOCK.lock().unwrap();

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
