//! OneFile wrapper for reading and writing ONE files

use crate::error::{OneError, Result};
use crate::ffi;
use crate::schema::OneSchema;
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::Mutex;

// Global mutex to protect oneErrorString() access
// The C library uses a static global buffer for error messages, which is not thread-safe
static ERROR_STRING_LOCK: Mutex<()> = Mutex::new(());

/// A ONE file handle for reading or writing
pub struct OneFile {
    pub(crate) ptr: *mut ffi::OneFile,
    is_owned: bool, // true if we should close this on drop
}

impl OneFile {
    /// Open a ONE file for reading
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to open
    /// * `schema` - Optional schema to validate against
    /// * `file_type` - Optional file type to match (primary or secondary)
    /// * `nthreads` - Number of threads for parallel reading (1 for single-threaded)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use onecode::OneFile;
    ///
    /// let file = OneFile::open_read("data.1seq", None, None, 1).unwrap();
    /// ```
    pub fn open_read(
        path: &str,
        schema: Option<&OneSchema>,
        file_type: Option<&str>,
        nthreads: i32,
    ) -> Result<Self> {
        let c_path = CString::new(path)?;
        let schema_ptr = schema.map_or(ptr::null_mut(), |s| s.as_ptr());
        let c_type = file_type.map(|t| CString::new(t)).transpose()?;
        let type_ptr = c_type.as_ref().map_or(ptr::null(), |t| t.as_ptr());

        unsafe {
            // Lock around both the C function call and error string read
            // The C library writes to a global error buffer, which is not thread-safe
            let _guard = ERROR_STRING_LOCK.lock().unwrap();

            let ptr = ffi::oneFileOpenRead(c_path.as_ptr(), schema_ptr, type_ptr, nthreads);
            if ptr.is_null() {
                let err_str = ffi::oneErrorString();
                let err_msg = if !err_str.is_null() {
                    CStr::from_ptr(err_str).to_string_lossy().into_owned()
                } else {
                    "Unknown error".to_string()
                };
                return Err(OneError::OpenFailed(format!("{}: {}", path, err_msg)));
            }
            Ok(OneFile {
                ptr,
                is_owned: true,
            })
        }
    }

    /// Create a new ONE file for writing
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the file will be written
    /// * `schema` - Schema defining the file format
    /// * `file_type` - Primary file type
    /// * `is_binary` - Whether to write in binary format (true) or ASCII (false)
    /// * `nthreads` - Number of threads for parallel writing (1 for single-threaded)
    pub fn open_write_new(
        path: &str,
        schema: &OneSchema,
        file_type: &str,
        is_binary: bool,
        nthreads: i32,
    ) -> Result<Self> {
        let c_path = CString::new(path)?;
        let c_type = CString::new(file_type)?;

        unsafe {
            // Lock around both the C function call and error string read
            // The C library writes to a global error buffer, which is not thread-safe
            let _guard = ERROR_STRING_LOCK.lock().unwrap();

            let ptr = ffi::oneFileOpenWriteNew(
                c_path.as_ptr(),
                schema.as_ptr(),
                c_type.as_ptr(),
                is_binary,
                nthreads,
            );
            if ptr.is_null() {
                let err_str = ffi::oneErrorString();
                let err_msg = if !err_str.is_null() {
                    CStr::from_ptr(err_str).to_string_lossy().into_owned()
                } else {
                    "Unknown error".to_string()
                };
                return Err(OneError::OpenFailed(format!("{}: {}", path, err_msg)));
            }
            Ok(OneFile {
                ptr,
                is_owned: true,
            })
        }
    }

    /// Create a new ONE file for writing based on an existing file
    ///
    /// Schema and header information is inherited from the source file.
    pub fn open_write_from(
        path: &str,
        source: &OneFile,
        is_binary: bool,
        nthreads: i32,
    ) -> Result<Self> {
        let c_path = CString::new(path)?;

        unsafe {
            // Lock around both the C function call and error string read
            // The C library writes to a global error buffer, which is not thread-safe
            let _guard = ERROR_STRING_LOCK.lock().unwrap();

            let ptr =
                ffi::oneFileOpenWriteFrom(c_path.as_ptr(), source.ptr, is_binary, nthreads);
            if ptr.is_null() {
                let err_str = ffi::oneErrorString();
                let err_msg = if !err_str.is_null() {
                    CStr::from_ptr(err_str).to_string_lossy().into_owned()
                } else {
                    "Unknown error".to_string()
                };
                return Err(OneError::OpenFailed(format!("{}: {}", path, err_msg)));
            }
            Ok(OneFile {
                ptr,
                is_owned: true,
            })
        }
    }

    /// Read the next line from the file
    ///
    /// Returns the line type character, or 0 if at end of file.
    pub fn read_line(&mut self) -> char {
        unsafe { ffi::oneReadLine(self.ptr) as u8 as char }
    }

    /// Read comment text from the current line
    ///
    /// Returns None if there is no comment.
    pub fn read_comment(&mut self) -> Option<String> {
        unsafe {
            let comment_ptr = ffi::oneReadComment(self.ptr);
            if comment_ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(comment_ptr).to_string_lossy().into_owned())
            }
        }
    }

    /// Write a line to the file
    ///
    /// # Arguments
    ///
    /// * `line_type` - The line type character
    /// * `list_len` - Length of the list (0 if no list)
    /// * `list_buf` - Buffer containing list data (None to use internal buffer)
    pub fn write_line(&mut self, line_type: char, list_len: i64, list_buf: Option<*mut std::ffi::c_void>) {
        unsafe {
            ffi::oneWriteLine(
                self.ptr,
                line_type as i8,
                list_len,
                list_buf.unwrap_or(ptr::null_mut()),
            );
        }
    }

    /// Write a comment to the current line
    pub fn write_comment(&mut self, comment: &str) -> Result<()> {
        let c_comment = CString::new(comment)?;
        unsafe {
            // We need to use a format string for the variadic function
            let format = CString::new("%s")?;
            ffi::oneWriteComment(self.ptr, format.as_ptr() as *mut i8, c_comment.as_ptr());
        }
        Ok(())
    }

    /// Add provenance information to the file header
    ///
    /// Must be called before the first write_line().
    pub fn add_provenance(&mut self, prog: &str, version: &str, command: &str) -> Result<bool> {
        let c_prog = CString::new(prog)?;
        let c_version = CString::new(version)?;
        let c_command = CString::new(command)?;

        unsafe {
            let format = CString::new("%s")?;
            let result = ffi::oneAddProvenance(
                self.ptr,
                c_prog.as_ptr(),
                c_version.as_ptr(),
                format.as_ptr() as *mut i8,
                c_command.as_ptr(),
            );
            Ok(result)
        }
    }

    /// Add a reference to the file header
    ///
    /// Must be called before the first write_line().
    pub fn add_reference(&mut self, filename: &str, count: i64) -> Result<bool> {
        let c_filename = CString::new(filename)?;

        unsafe {
            let result = ffi::oneAddReference(self.ptr, c_filename.as_ptr(), count);
            Ok(result)
        }
    }

    /// Inherit provenance from another file
    pub fn inherit_provenance(&mut self, source: &OneFile) -> bool {
        unsafe { ffi::oneInheritProvenance(self.ptr, source.ptr) }
    }

    /// Inherit references from another file
    pub fn inherit_reference(&mut self, source: &OneFile) -> bool {
        unsafe { ffi::oneInheritReference(self.ptr, source.ptr) }
    }

    /// Get statistics for a line type
    ///
    /// Returns (count, max, total) where:
    /// - count: number of lines of this type
    /// - max: maximum list length
    /// - total: total list length
    pub fn stats(&self, line_type: char) -> Result<(i64, i64, i64)> {
        let mut count: i64 = 0;
        let mut max: i64 = 0;
        let mut total: i64 = 0;

        unsafe {
            let success = ffi::oneStats(
                self.ptr,
                line_type as i8,
                &mut count,
                &mut max,
                &mut total,
            );
            if !success {
                return Err(OneError::Other(format!(
                    "Failed to get stats for line type '{}'",
                    line_type
                )));
            }
        }

        Ok((count, max, total))
    }

    /// Navigate to a specific object in the file
    ///
    /// Only works on binary files with an index. The first object is numbered 1.
    /// Setting i == 0 goes to the start of the data.
    pub fn goto(&mut self, line_type: char, index: i64) -> Result<()> {
        unsafe {
            let success = ffi::oneGoto(self.ptr, line_type as i8, index);
            if !success {
                return Err(OneError::Other(format!(
                    "Failed to goto object {} of type '{}'",
                    index, line_type
                )));
            }
        }
        Ok(())
    }

    /// Get the current line type
    pub fn line_type(&self) -> char {
        unsafe { (*self.ptr).lineType as u8 as char }
    }

    /// Get the current line number
    pub fn line_number(&self) -> i64 {
        unsafe { (*self.ptr).line }
    }

    /// Get the file name
    pub fn file_name(&self) -> Option<String> {
        unsafe {
            let name_ptr = (*self.ptr).fileName;
            if name_ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(name_ptr).to_string_lossy().into_owned())
            }
        }
    }

    /// Get the file type (primary)
    pub fn file_type(&self) -> Option<String> {
        unsafe {
            let type_ptr = (*self.ptr).fileType;
            if type_ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(type_ptr).to_string_lossy().into_owned())
            }
        }
    }

    /// Get the sub type (secondary)
    pub fn sub_type(&self) -> Option<String> {
        unsafe {
            let type_ptr = (*self.ptr).subType;
            if type_ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(type_ptr).to_string_lossy().into_owned())
            }
        }
    }

    /// Get an integer field value
    pub fn int(&self, field: usize) -> i64 {
        unsafe {
            let fields = (*self.ptr).field;
            (*fields.add(field)).i
        }
    }

    /// Get a real field value
    pub fn real(&self, field: usize) -> f64 {
        unsafe {
            let fields = (*self.ptr).field;
            (*fields.add(field)).r
        }
    }

    /// Get a character field value
    pub fn char(&self, field: usize) -> char {
        unsafe {
            let fields = (*self.ptr).field;
            (*fields.add(field)).c as u8 as char
        }
    }

    /// Set an integer field value
    pub fn set_int(&mut self, field: usize, value: i64) {
        unsafe {
            let fields = (*self.ptr).field;
            (*fields.add(field)).i = value;
        }
    }

    /// Set a real field value
    pub fn set_real(&mut self, field: usize, value: f64) {
        unsafe {
            let fields = (*self.ptr).field;
            (*fields.add(field)).r = value;
        }
    }

    /// Set a character field value
    pub fn set_char(&mut self, field: usize, value: char) {
        unsafe {
            let fields = (*self.ptr).field;
            (*fields.add(field)).c = value as i8;
        }
    }

    /// Get the internal pointer (for advanced use with FFI)
    pub fn as_ptr(&self) -> *mut ffi::OneFile {
        self.ptr
    }

    /// Close the file explicitly
    ///
    /// This is called automatically on drop, but you can call it manually
    /// to handle any cleanup earlier.
    pub fn close(mut self) {
        if self.is_owned && !self.ptr.is_null() {
            unsafe {
                ffi::oneFileClose(self.ptr);
            }
            self.ptr = ptr::null_mut();
        }
    }
}

impl Drop for OneFile {
    fn drop(&mut self) {
        if self.is_owned && !self.ptr.is_null() {
            unsafe {
                ffi::oneFileClose(self.ptr);
            }
        }
    }
}

// OneFile is not thread-safe by default
// The user needs to manage thread-safety if using nthreads > 1
