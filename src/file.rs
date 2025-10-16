//! OneFile wrapper for reading and writing ONE files

use crate::error::{OneError, Result};
use crate::ffi;
use crate::schema::OneSchema;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::ptr;

// Note: The C library's errorString is now _Thread_local (patched in ONEcode/ONElib.c)
// so no mutex is needed for error handling

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

    /// Get the length of the list field in the current line
    ///
    /// This corresponds to the `oneLen()` macro in C.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> i64 {
        unsafe {
            let line_type = (*self.ptr).lineType;
            let info = (*self.ptr).info[line_type as usize];
            if info.is_null() {
                return 0;
            }
            let list_field = (*info).listField as usize;
            let fields = (*self.ptr).field;
            (*fields.add(list_field)).len as i64 & 0xffffffffffffffi64
        }
    }

    /// Check if the list field is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a string from the current line
    ///
    /// This corresponds to the `oneString()` macro in C.
    /// Returns a reference to the string data.
    pub fn string(&self) -> Option<&str> {
        unsafe {
            let ptr = ffi::_oneList(self.ptr) as *const i8;
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_str().unwrap_or(""))
            }
        }
    }

    /// Get DNA sequence as characters from the current line
    ///
    /// This corresponds to the `oneDNAchar()` macro in C.
    pub fn dna_char(&self) -> Option<&[u8]> {
        unsafe {
            let ptr = ffi::_oneList(self.ptr) as *const u8;
            if ptr.is_null() {
                None
            } else {
                let len = self.len() as usize;
                Some(std::slice::from_raw_parts(ptr, len))
            }
        }
    }

    /// Get DNA sequence as 2-bit compressed data from the current line
    ///
    /// This corresponds to the `oneDNA2bit()` macro in C.
    pub fn dna_2bit(&self) -> Option<&[u8]> {
        unsafe {
            let ptr = ffi::_oneCompressedList(self.ptr) as *const u8;
            if ptr.is_null() {
                None
            } else {
                let len = (self.len() + 3) / 4; // 4 bases per byte in 2-bit encoding
                Some(std::slice::from_raw_parts(ptr, len as usize))
            }
        }
    }

    /// Get an integer list from the current line
    ///
    /// This corresponds to the `oneIntList()` macro in C.
    pub fn int_list(&self) -> Option<&[i64]> {
        unsafe {
            let ptr = ffi::_oneList(self.ptr) as *const i64;
            if ptr.is_null() {
                None
            } else {
                let len = self.len() as usize;
                Some(std::slice::from_raw_parts(ptr, len))
            }
        }
    }

    /// Get a real/double list from the current line
    ///
    /// This corresponds to the `oneRealList()` macro in C.
    pub fn real_list(&self) -> Option<&[f64]> {
        unsafe {
            let ptr = ffi::_oneList(self.ptr) as *const f64;
            if ptr.is_null() {
                None
            } else {
                let len = self.len() as usize;
                Some(std::slice::from_raw_parts(ptr, len))
            }
        }
    }

    /// Get the next string in a string list
    ///
    /// This corresponds to the `oneNextString()` macro in C.
    /// Pass the current string pointer to get the next one.
    pub fn next_string<'a>(&self, current: &'a str) -> Option<&'a str> {
        unsafe {
            let next_ptr = current.as_ptr().add(current.len() + 1) as *const i8;
            if *next_ptr == 0 {
                None
            } else {
                Some(CStr::from_ptr(next_ptr).to_str().unwrap_or(""))
            }
        }
    }

    /// Get the object count for a given line type
    ///
    /// This corresponds to the `oneObject()` macro in C.
    /// Returns the count, or -1 if the line type doesn't exist.
    pub fn object(&self, line_type: char) -> i64 {
        unsafe {
            let info = (*self.ptr).info[line_type as usize];
            if info.is_null() {
                -1
            } else {
                (*info).accum.count
            }
        }
    }

    /// Get the reference count
    ///
    /// This corresponds to the `oneReferenceCount()` macro in C.
    pub fn reference_count(&self) -> i64 {
        unsafe {
            let info = (*self.ptr).info['<' as usize];
            if info.is_null() {
                0
            } else {
                (*info).accum.count
            }
        }
    }

    /// Get the internal pointer (for advanced use with FFI)
    pub fn as_ptr(&self) -> *mut ffi::OneFile {
        self.ptr
    }

    /// Get sequence name by ID from embedded GDB
    ///
    /// This method searches for 'S' (scaffold/sequence) line types in the file
    /// and returns the name for the given sequence ID. The sequence IDs are
    /// 0-indexed and correspond to the order of S lines in the file.
    ///
    /// # Arguments
    /// * `seq_id` - Sequence/scaffold ID from alignment record (0-indexed)
    ///
    /// # Returns
    /// The sequence name, or None if not found
    ///
    /// # Note
    /// This method requires scanning through the file to find S lines.
    /// For repeated lookups, consider using `get_all_sequence_names()` to
    /// build a complete mapping once.
    pub fn get_sequence_name(&mut self, seq_id: i64) -> Option<String> {
        // Save current position
        let saved_line = self.line_number();

        // Go to the beginning of the file to scan for S lines
        // Note: 'S' is a group member inside 'g' objects, not a top-level object
        // Schema: ~ O g 0 (groups scaffolds into a GDB skeleton)
        //         ~ G S 0 (collection of scaffolds constituting a GDB)
        //         ~ O S 1 6 STRING (id for a scaffold)
        unsafe {
            // Navigate to the FIRST 'g' group object (objects are numbered starting at 1)
            if ffi::oneGoto(self.ptr, 'g' as i8, 1) {
                let mut current_id = 0i64;
                let mut is_first_line = true;
                loop {
                    let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                    if line_type == '\0' {
                        break; // EOF
                    }
                    if line_type == 'S' {
                        if current_id == seq_id {
                            let name = self.string();
                            // Restore position (best effort - goto may not work on all files)
                            let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
                            return name.map(|s| s.to_string());
                        }
                        current_id += 1;
                    }
                    // Skip the first 'g' line (we're positioned at it after oneGoto)
                    // Stop when we hit the NEXT 'g' or reach 'A' (alignments)
                    if !is_first_line && (line_type == 'g' || line_type == 'A') {
                        break;
                    }
                    is_first_line = false;
                }
            }
        }
        None
    }

    /// Get all sequence names from the embedded GDB
    ///
    /// Returns a map of sequence ID → name. This is more efficient than
    /// calling `get_sequence_name()` repeatedly.
    ///
    /// # Returns
    /// A HashMap mapping sequence IDs (0-indexed) to their names
    pub fn get_all_sequence_names(&mut self) -> HashMap<i64, String> {
        let mut names = HashMap::new();

        // Save current position
        let saved_line = self.line_number();

        // Go to the beginning to scan for S lines
        // Note: 'S' is a group member inside 'g' objects, not a top-level object
        unsafe {
            // Navigate to the FIRST 'g' group object (objects are numbered starting at 1)
            if ffi::oneGoto(self.ptr, 'g' as i8, 1) {
                let mut current_id = 0i64;
                let mut is_first_line = true;
                loop {
                    let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                    if line_type == '\0' {
                        break; // EOF
                    }
                    if line_type == 'S' {
                        if let Some(name) = self.string() {
                            names.insert(current_id, name.to_string());
                            current_id += 1;
                        }
                    }
                    // Skip the first 'g' line (we're positioned at it after oneGoto)
                    // Stop when we hit the NEXT 'g' or reach 'A' (alignments)
                    if !is_first_line && (line_type == 'g' || line_type == 'A') {
                        break;
                    }
                    is_first_line = false;
                }
                // Restore position (best effort)
                let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
            }
        }
        names
    }

    /// Get all sequence lengths from the embedded GDB
    ///
    /// Returns a map of sequence ID → total sequence length.
    /// The length is computed by summing all contig (C) and gap (G) lengths
    /// for each scaffold (S).
    ///
    /// # Returns
    /// A HashMap mapping sequence IDs (0-indexed) to their total lengths
    pub fn get_all_sequence_lengths(&mut self) -> HashMap<i64, i64> {
        let mut lengths = HashMap::new();

        // Save current position
        let saved_line = self.line_number();

        // Navigate to the FIRST 'g' group object (GDB skeleton)
        unsafe {
            if ffi::oneGoto(self.ptr, 'g' as i8, 1) {
                let mut current_seq_id = -1i64;
                let mut current_length = 0i64;
                let mut is_first_line = true;

                loop {
                    let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                    if line_type == '\0' {
                        break; // EOF
                    }

                    match line_type {
                        'S' => {
                            // Save previous scaffold's length if any
                            if current_seq_id >= 0 {
                                lengths.insert(current_seq_id, current_length);
                            }
                            // Start new scaffold
                            current_seq_id += 1;
                            current_length = 0;
                        }
                        'G' => {
                            // Gap record - add to current scaffold length
                            let gap_len = self.int(0);
                            current_length += gap_len;
                        }
                        'C' => {
                            // Contig record - add to current scaffold length
                            let contig_len = self.int(0);
                            current_length += contig_len;
                        }
                        'g' | 'A' | 'a' => {
                            // Hit next GDB group or alignments - stop
                            if !is_first_line {
                                // Save last scaffold's length
                                if current_seq_id >= 0 {
                                    lengths.insert(current_seq_id, current_length);
                                }
                                break;
                            }
                        }
                        _ => {
                            // Skip other records (M, etc.)
                        }
                    }
                    is_first_line = false;
                }

                // Restore position (best effort)
                let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
            }
        }
        lengths
    }

    /// Get all contig offset information from the embedded GDB
    ///
    /// Returns a map of contig ID → (sbeg, clen) where:
    /// - sbeg: scaffold begin offset (position where this contig starts in the scaffold)
    /// - clen: contig length
    ///
    /// This information is needed to convert contig coordinates to scaffold/chromosome
    /// coordinates, matching ALNtoPAF's behavior.
    ///
    /// # Returns
    /// A HashMap mapping contig IDs (0-indexed) to (scaffold_offset, contig_length)
    pub fn get_all_contig_offsets(&mut self) -> HashMap<i64, (i64, i64)> {
        let mut contigs = HashMap::new();

        // Save current position
        let saved_line = self.line_number();

        // Navigate to the FIRST 'g' group object (GDB skeleton)
        unsafe {
            if ffi::oneGoto(self.ptr, 'g' as i8, 1) {
                let mut contig_id = 0i64;
                let mut spos = 0i64; // scaffold position accumulator
                let mut is_first_line = true;

                loop {
                    let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                    if line_type == '\0' {
                        break; // EOF
                    }

                    match line_type {
                        'S' => {
                            // Scaffold record - reset scaffold position for new scaffold
                            spos = 0;
                        }
                        'G' => {
                            // Gap record - advance scaffold position by gap length
                            let gap_len = self.int(0);
                            spos += gap_len;
                        }
                        'C' => {
                            // Contig record - record (sbeg, clen) and advance position
                            let clen = self.int(0);
                            contigs.insert(contig_id, (spos, clen));
                            contig_id += 1;
                            spos += clen;
                        }
                        'g' | 'A' => {
                            // Hit next GDB group or alignments - stop
                            if !is_first_line {
                                break;
                            }
                        }
                        _ => {
                            // Skip other records (M, etc.)
                        }
                    }
                    is_first_line = false;
                }

                // Restore position (best effort)
                let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
            }
        }
        contigs
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
