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
    /// Trim sequence name at first whitespace character
    /// This removes FASTA header descriptions, keeping only the sequence ID
    fn trim_sequence_name(name: &str) -> String {
        name.split_whitespace()
            .next()
            .unwrap_or(name)
            .to_string()
    }
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
        let c_type = file_type.map(CString::new).transpose()?;
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
                let msg = if err_msg.trim().is_empty() {
                    path.to_string()
                } else {
                    format!("{}: {}", path, err_msg)
                };
                return Err(OneError::OpenFailed(msg));
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
                let msg = if err_msg.trim().is_empty() {
                    path.to_string()
                } else {
                    format!("{}: {}", path, err_msg)
                };
                return Err(OneError::OpenFailed(msg));
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
                let msg = if err_msg.trim().is_empty() {
                    path.to_string()
                } else {
                    format!("{}: {}", path, err_msg)
                };
                return Err(OneError::OpenFailed(msg));
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
            (*fields.add(list_field)).len & 0xffffffffffffffi64
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

    /// Get all references from the file header
    ///
    /// Returns a vector of (filename, count) tuples
    pub fn get_references(&self) -> Vec<(String, i64)> {
        let mut references = Vec::new();
        let count = self.reference_count();

        if count == 0 {
            return references;
        }

        unsafe {
            let ref_array = (*self.ptr).reference;
            if ref_array.is_null() {
                return references;
            }

            for i in 0..count {
                let ref_ptr = ref_array.add(i as usize);
                let filename = if (*ref_ptr).filename.is_null() {
                    String::new()
                } else {
                    std::ffi::CStr::from_ptr((*ref_ptr).filename)
                        .to_string_lossy()
                        .into_owned()
                };
                let count = (*ref_ptr).count;
                references.push((filename, count));
            }
        }

        references
    }

    /// Get the internal pointer (for advanced use with FFI)
    pub fn as_ptr(&self) -> *mut ffi::OneFile {
        self.ptr
    }

    /// Get sequence name by contig ID from embedded GDB
    ///
    /// This method maps a contig ID (as used in alignment records) to the name
    /// of the scaffold containing that contig.
    ///
    /// # Arguments
    /// * `seq_id` - Contig ID from alignment record (0-indexed)
    ///
    /// # Returns
    /// The scaffold name containing this contig, or None if not found
    pub fn get_sequence_name(&mut self, seq_id: i64) -> Option<String> {
        // Save current position
        let saved_line = self.line_number();

        unsafe {
            // Navigate to the FIRST 'g' group object (objects are numbered starting at 1)
            if ffi::oneGoto(self.ptr, 'g' as i8, 1) {
                let mut contig_id = 0i64;
                let mut current_scaffold_name = String::new();
                let mut is_first_line = true;
                
                loop {
                    let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                    if line_type == '\0' {
                        break; // EOF
                    }
                    
                    match line_type {
                        'S' => {
                            // New scaffold - store its name
                            if let Some(name) = self.string() {
                                current_scaffold_name = name.to_string();
                            }
                        }
                        'C' => {
                            // Contig record - check if this is the one we're looking for
                            if contig_id == seq_id {
                                // Restore position and return the scaffold name
                                let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
                                return Some(current_scaffold_name);
                            }
                            contig_id += 1;
                        }
                        'g' | 'A' | 'a' => {
                            // Hit next GDB group or alignments - stop
                            if !is_first_line {
                                break;
                            }
                        }
                        _ => {
                            // Skip other records (G for gaps, M for masks, etc.)
                        }
                    }
                    is_first_line = false;
                }
                // Restore position (best effort - goto may not work on all files)
                let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
            }
        }
        None
    }

    /// Read all embedded GDB group metadata in a single pass
    ///
    /// Returns a vector of tuples, one per 'g' group, each containing:
    /// (sequence_names, sequence_lengths, contig_offsets)
    /// where each HashMap maps global contig IDs to their values.
    ///
    /// # Returns
    /// A Vec of (names, lengths, offsets) tuples, one per 'g' group in order
    pub fn get_all_groups_metadata(&mut self) -> Vec<(HashMap<i64, String>, HashMap<i64, i64>, HashMap<i64, (i64, i64)>)> {
        let mut groups = Vec::new();
        let saved_line = self.line_number();

        unsafe {
            if ffi::oneGoto(self.ptr, 'g' as i8, 1) {
                let mut group_contig_id = 0i64;  // Per-group contig ID (resets for each 'g')
                let mut current_group_names = HashMap::new();
                let mut current_group_lengths = HashMap::new();
                let mut current_group_offsets = HashMap::new();

                let mut current_scaffold_name = String::new();
                let mut current_scaffold_length = 0i64;
                let mut scaffold_contigs = Vec::new();
                let mut scaffold_pos = 0i64;
                let mut is_first_line = true;

                loop {
                    let line_type = ffi::oneReadLine(self.ptr) as u8 as char;

                    if line_type == '\0' {
                        // EOF - save final scaffold and final group
                        for cid in scaffold_contigs.iter() {
                            current_group_lengths.insert(*cid, current_scaffold_length);
                        }
                        if !current_group_names.is_empty() {
                            groups.push((current_group_names, current_group_lengths, current_group_offsets));
                        }
                        break;
                    }

                    match line_type {
                        'g' => {
                            if !is_first_line {
                                // Save current scaffold to current group
                                for cid in scaffold_contigs.iter() {
                                    current_group_lengths.insert(*cid, current_scaffold_length);
                                }
                                // Save current group and start new one
                                groups.push((current_group_names, current_group_lengths, current_group_offsets));
                                current_group_names = HashMap::new();
                                current_group_lengths = HashMap::new();
                                current_group_offsets = HashMap::new();
                                scaffold_contigs.clear();
                                current_scaffold_length = 0;
                                scaffold_pos = 0;
                                group_contig_id = 0;  // Reset contig ID for new group
                            }
                        }
                        'S' => {
                            // Process previous scaffold
                            for cid in scaffold_contigs.iter() {
                                current_group_lengths.insert(*cid, current_scaffold_length);
                            }
                            // Start new scaffold
                            if let Some(name) = self.string() {
                                current_scaffold_name = Self::trim_sequence_name(name);
                            }
                            scaffold_contigs.clear();
                            current_scaffold_length = 0;
                            scaffold_pos = 0;
                        }
                        'G' => {
                            // Gap
                            let gap_len = self.int(0);
                            current_scaffold_length += gap_len;
                            scaffold_pos += gap_len;
                        }
                        'C' => {
                            // Contig - use per-group contig ID
                            let clen = self.int(0);
                            current_group_names.insert(group_contig_id, current_scaffold_name.clone());
                            current_group_offsets.insert(group_contig_id, (scaffold_pos, clen));
                            current_scaffold_length += clen;
                            scaffold_contigs.push(group_contig_id);
                            scaffold_pos += clen;
                            group_contig_id += 1;
                        }
                        'A' | 'a' => {
                            // Hit alignments - save final scaffold and final group
                            if !is_first_line {
                                for cid in scaffold_contigs.iter() {
                                    current_group_lengths.insert(*cid, current_scaffold_length);
                                }
                                if !current_group_names.is_empty() {
                                    groups.push((current_group_names, current_group_lengths, current_group_offsets));
                                }
                                break;
                            }
                        }
                        _ => {}
                    }
                    is_first_line = false;
                }
                let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
            }
        }
        groups
    }

    /// Get sequence names from a specific 'g' group with correct global contig IDs
    ///
    /// Contig IDs are global across all 'g' groups. This function correctly calculates
    /// the starting contig ID for the requested group by counting contigs in previous groups.
    ///
    /// # Arguments
    /// * `group_num` - Which 'g' group to read (1-indexed)
    ///
    /// # Returns
    /// A HashMap mapping global contig IDs to their scaffold names
    pub fn get_group_sequence_names(&mut self, group_num: i64) -> HashMap<i64, String> {
        let mut names = HashMap::new();
        let saved_line = self.line_number();

        unsafe {
            // First, count contigs in all previous groups to get the starting contig_id
            let mut starting_contig_id = 0i64;
            for prev_group in 1..group_num {
                if ffi::oneGoto(self.ptr, 'g' as i8, prev_group) {
                    loop {
                        let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                        if line_type == '\0' || line_type == 'g' || line_type == 'A' || line_type == 'a' {
                            break;
                        }
                        if line_type == 'C' {
                            starting_contig_id += 1;
                        }
                    }
                }
            }

            // Now read the target group with correct starting ID
            if ffi::oneGoto(self.ptr, 'g' as i8, group_num) {
                let mut contig_id = starting_contig_id;
                let mut current_scaffold_name = String::new();
                let mut is_first_line = true;

                loop {
                    let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                    if line_type == '\0' {
                        break;
                    }

                    match line_type {
                        'S' => {
                            if let Some(name) = self.string() {
                                current_scaffold_name = Self::trim_sequence_name(name);
                            }
                        }
                        'C' => {
                            names.insert(contig_id, current_scaffold_name.clone());
                            contig_id += 1;
                        }
                        'g' | 'A' | 'a' => {
                            if !is_first_line {
                                break;
                            }
                        }
                        _ => {}
                    }
                    is_first_line = false;
                }
                let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
            }
        }
        names
    }

    /// Get sequence names mapped by contig ID for alignment files (all groups)
    ///
    /// In alignment files with embedded GDB skeletons, alignments reference
    /// contigs by their global ID. This method returns a mapping from contig ID
    /// to the name of the scaffold containing that contig.
    ///
    /// # Returns
    /// A HashMap mapping contig IDs (0-indexed) to their scaffold names
    pub fn get_all_sequence_names(&mut self) -> HashMap<i64, String> {
        let mut names = HashMap::new();
        let saved_line = self.line_number();

        unsafe {
            // Navigate to the first 'g' group object (GDB skeleton)
            if ffi::oneGoto(self.ptr, 'g' as i8, 1) {
                let mut contig_id = 0i64;
                let mut current_scaffold_name = String::new();
                let mut is_first_line = true;

                loop {
                    let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                    if line_type == '\0' {
                        break; // EOF
                    }

                    match line_type {
                        'S' => {
                            // New scaffold - store its name (trim at first whitespace)
                            if let Some(name) = self.string() {
                                current_scaffold_name = Self::trim_sequence_name(name);
                            }
                        }
                        'C' => {
                            // Contig record - map this contig ID to current scaffold name
                            names.insert(contig_id, current_scaffold_name.clone());
                            contig_id += 1;
                        }
                        'g' => {
                            // Hit next 'g' group - continue reading to get all genomes
                            if !is_first_line {
                                // Continue to next group instead of breaking
                                is_first_line = true;
                            }
                        }
                        'A' | 'a' => {
                            // Hit alignments - stop reading groups
                            if !is_first_line {
                                break;
                            }
                        }
                        _ => {
                            // Skip other records (G for gaps, M for masks, etc.)
                        }
                    }
                    is_first_line = false;
                }
                // Restore position (best effort)
                let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
            }
        }
        names
    }

    /// Get sequence lengths from a specific 'g' group with correct global contig IDs
    ///
    /// # Arguments
    /// * `group_num` - Which 'g' group to read (1-indexed)
    ///
    /// # Returns
    /// A HashMap mapping global contig IDs to their scaffold lengths
    pub fn get_group_sequence_lengths(&mut self, group_num: i64) -> HashMap<i64, i64> {
        let mut lengths = HashMap::new();
        let saved_line = self.line_number();

        unsafe {
            // Count contigs in previous groups
            let mut starting_contig_id = 0i64;
            for prev_group in 1..group_num {
                if ffi::oneGoto(self.ptr, 'g' as i8, prev_group) {
                    loop {
                        let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                        if line_type == '\0' || line_type == 'g' || line_type == 'A' || line_type == 'a' {
                            break;
                        }
                        if line_type == 'C' {
                            starting_contig_id += 1;
                        }
                    }
                }
            }

            // Read target group
            if ffi::oneGoto(self.ptr, 'g' as i8, group_num) {
                let mut contig_id = starting_contig_id;
                let mut current_scaffold_length = 0i64;
                let mut scaffold_contigs = Vec::new();
                let mut is_first_line = true;

                loop {
                    let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                    if line_type == '\0' {
                        // Process final scaffold
                        for cid in scaffold_contigs.iter() {
                            lengths.insert(*cid, current_scaffold_length);
                        }
                        break;
                    }

                    match line_type {
                        'S' => {
                            // Process previous scaffold
                            for cid in scaffold_contigs.iter() {
                                lengths.insert(*cid, current_scaffold_length);
                            }
                            scaffold_contigs.clear();
                            current_scaffold_length = 0;
                        }
                        'G' => {
                            current_scaffold_length += self.int(0);
                        }
                        'C' => {
                            let contig_len = self.int(0);
                            current_scaffold_length += contig_len;
                            scaffold_contigs.push(contig_id);
                            contig_id += 1;
                        }
                        'g' | 'A' | 'a' => {
                            if !is_first_line {
                                // Process final scaffold
                                for cid in scaffold_contigs.iter() {
                                    lengths.insert(*cid, current_scaffold_length);
                                }
                                break;
                            }
                        }
                        _ => {}
                    }
                    is_first_line = false;
                }
                let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
            }
        }
        lengths
    }

    /// Get contig offsets from a specific 'g' group with correct global contig IDs
    ///
    /// # Arguments
    /// * `group_num` - Which 'g' group to read (1-indexed)
    ///
    /// # Returns
    /// A HashMap mapping global contig IDs to (scaffold_offset, contig_length)
    pub fn get_group_contig_offsets(&mut self, group_num: i64) -> HashMap<i64, (i64, i64)> {
        let mut contigs = HashMap::new();
        let saved_line = self.line_number();

        unsafe {
            // Count contigs in previous groups
            let mut starting_contig_id = 0i64;
            for prev_group in 1..group_num {
                if ffi::oneGoto(self.ptr, 'g' as i8, prev_group) {
                    loop {
                        let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                        if line_type == '\0' || line_type == 'g' || line_type == 'A' || line_type == 'a' {
                            break;
                        }
                        if line_type == 'C' {
                            starting_contig_id += 1;
                        }
                    }
                }
            }

            // Read target group
            if ffi::oneGoto(self.ptr, 'g' as i8, group_num) {
                let mut contig_id = starting_contig_id;
                let mut spos = 0i64;
                let mut is_first_line = true;

                loop {
                    let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                    if line_type == '\0' {
                        break;
                    }

                    match line_type {
                        'S' => {
                            spos = 0;
                        }
                        'G' => {
                            spos += self.int(0);
                        }
                        'C' => {
                            let clen = self.int(0);
                            contigs.insert(contig_id, (spos, clen));
                            contig_id += 1;
                            spos += clen;
                        }
                        'g' | 'A' | 'a' => {
                            if !is_first_line {
                                break;
                            }
                        }
                        _ => {}
                    }
                    is_first_line = false;
                }
                let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
            }
        }
        contigs
    }

    /// Get sequence lengths mapped by contig ID for alignment files (all groups)
    ///
    /// In alignment files with embedded GDB skeletons, this returns the total
    /// scaffold length for each contig ID. Each contig maps to the total length
    /// of its containing scaffold.
    ///
    /// # Returns
    /// A HashMap mapping contig IDs (0-indexed) to their scaffold's total length
    pub fn get_all_sequence_lengths(&mut self) -> HashMap<i64, i64> {
        let mut lengths = HashMap::new();
        let saved_line = self.line_number();

        unsafe {
            if ffi::oneGoto(self.ptr, 'g' as i8, 1) {
                let mut contig_id = 0i64;
                let mut current_scaffold_length = 0i64;
                let mut scaffold_contigs = Vec::new(); // Track contigs in current scaffold
                let mut is_first_line = true;

                loop {
                    let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                    if line_type == '\0' {
                        // EOF - process final scaffold
                        for cid in scaffold_contigs.iter() {
                            lengths.insert(*cid, current_scaffold_length);
                        }
                        break;
                    }

                    match line_type {
                        'S' => {
                            // Process previous scaffold's contigs
                            for cid in scaffold_contigs.iter() {
                                lengths.insert(*cid, current_scaffold_length);
                            }
                            // Start new scaffold
                            scaffold_contigs.clear();
                            current_scaffold_length = 0;
                        }
                        'G' => {
                            // Gap record - add to scaffold length
                            let gap_len = self.int(0);
                            current_scaffold_length += gap_len;
                        }
                        'C' => {
                            // Contig record - add to scaffold length and track this contig
                            let contig_len = self.int(0);
                            current_scaffold_length += contig_len;
                            scaffold_contigs.push(contig_id);
                            contig_id += 1;
                        }
                        'g' => {
                            // Hit next 'g' group - process current scaffold and continue to next group
                            if !is_first_line {
                                // Process current scaffold's contigs
                                for cid in scaffold_contigs.iter() {
                                    lengths.insert(*cid, current_scaffold_length);
                                }
                                // Reset for next group
                                scaffold_contigs.clear();
                                current_scaffold_length = 0;
                                is_first_line = true;
                            }
                        }
                        'A' | 'a' => {
                            // Hit alignments - process final scaffold and stop
                            if !is_first_line {
                                for cid in scaffold_contigs.iter() {
                                    lengths.insert(*cid, current_scaffold_length);
                                }
                                break;
                            }
                        }
                        _ => {
                            // Skip other records (M for masks, etc.)
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
                        'g' => {
                            // Hit next 'g' group - reset scaffold position and continue
                            if !is_first_line {
                                spos = 0;
                                is_first_line = true;
                            }
                        }
                        'A' => {
                            // Hit alignments - stop reading groups
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

    /// Load metadata from a GDB file (.gdb or .1gdb)
    ///
    /// This reads contig-to-scaffold mappings from a standalone GDB file (not an embedded skeleton).
    /// Standalone GDB files have S and C records at the top level, not in a 'g' group.
    ///
    /// # Arguments
    /// * `path` - Path to the GDB file (.gdb or .1gdb)
    ///
    /// # Returns
    /// A tuple of (seq_names, seq_lengths, contig_offsets) HashMaps
    pub fn read_gdb_metadata(path: &str) -> Result<(HashMap<i64, String>, HashMap<i64, i64>, HashMap<i64, (i64, i64)>)> {
        let mut file = Self::open_read(path, None, Some("gdb"), 1)?;

        let mut seq_names = HashMap::new();
        let mut seq_lengths = HashMap::new();
        let mut contig_offsets = HashMap::new();

        let mut contig_id = 0i64;
        let mut spos = 0i64; // scaffold position accumulator
        let mut current_scaffold_name = String::new();
        let mut current_scaffold_length = 0i64;
        let mut scaffold_contigs = Vec::new(); // Track contigs in current scaffold

        // Read S (scaffold) and C (contig) records from top level
        loop {
            let line_type = file.read_line();
            if line_type == '\0' {
                // EOF - process final scaffold
                for cid in scaffold_contigs.iter() {
                    seq_lengths.insert(*cid, current_scaffold_length);
                }
                break;
            }

            match line_type {
                'S' => {
                    // Process previous scaffold's contigs
                    for cid in scaffold_contigs.iter() {
                        seq_lengths.insert(*cid, current_scaffold_length);
                    }
                    // Start new scaffold
                    scaffold_contigs.clear();
                    spos = 0;
                    current_scaffold_length = 0;

                    if let Some(name) = file.string() {
                        current_scaffold_name = Self::trim_sequence_name(name);
                    }
                }
                'G' => {
                    // Gap record - advance scaffold position and add to scaffold length
                    let gap_len = file.int(0);
                    spos += gap_len;
                    current_scaffold_length += gap_len;
                }
                'C' => {
                    // Contig record - record metadata and advance position
                    let clen = file.int(0);
                    seq_names.insert(contig_id, current_scaffold_name.clone());
                    contig_offsets.insert(contig_id, (spos, clen));
                    scaffold_contigs.push(contig_id);
                    contig_id += 1;
                    spos += clen;
                    current_scaffold_length += clen;
                }
                _ => {
                    // Skip other records (M for masks, f for frequency, u for uppercase, etc.)
                }
            }
        }

        Ok((seq_names, seq_lengths, contig_offsets))
    }

    /// Get byte offset for a specific alignment object (0-indexed)
    /// Returns None if index unavailable or out of bounds
    pub fn get_alignment_byte_offset(&self, alignment_index: i64) -> Option<i64> {
        unsafe {
            let li = (*self.ptr).info['A' as usize];
            if li.is_null() { return None; }
            let index_ptr = (*li).index;
            if index_ptr.is_null() { return None; }
            if alignment_index < 0 || alignment_index >= (*li).indexSize { return None; }
            Some(*index_ptr.offset(alignment_index as isize))
        }
    }

    /// Get all byte offsets for alignment objects
    /// Returns empty vector if index unavailable
    pub fn get_all_alignment_byte_offsets(&self) -> Vec<i64> {
        unsafe {
            let li = (*self.ptr).info['A' as usize];
            if li.is_null() { return Vec::new(); }
            let index_ptr = (*li).index;
            if index_ptr.is_null() { return Vec::new(); }
            let count = (*li).given.count;
            std::slice::from_raw_parts(index_ptr, (count + 1) as usize).to_vec()
        }
    }

    /// Seek to a specific byte offset in the file
    pub fn seek_to_byte_offset(&mut self, byte_offset: i64) -> Result<()> {
        unsafe {
            let file_ptr = (*self.ptr).f as *mut libc::FILE;
            // First seek clears input buffer
            if libc::fseek(file_ptr, byte_offset, libc::SEEK_SET) != 0 {
                return Err(OneError::Other(format!("Failed to seek to byte {}", byte_offset)));
            }
            // Second seek to same position ensures buffer is properly reset
            if libc::fseek(file_ptr, byte_offset, libc::SEEK_SET) != 0 {
                return Err(OneError::Other(format!("Failed to seek to byte {}", byte_offset)));
            }
        }
        Ok(())
    }

    /// Seek and read line - optimized for batching multiple reads from same file
    pub fn seek_and_read_line(&mut self, byte_offset: i64) -> Result<char> {
        self.seek_to_byte_offset(byte_offset)?;
        Ok(self.read_line())
    }

    /// Get current byte position in the file using ftell
    /// Returns the byte offset of the current position in the file
    pub fn get_current_byte_position(&self) -> i64 {
        unsafe {
            let file_ptr = (*self.ptr).f as *mut libc::FILE;
            libc::ftell(file_ptr)
        }
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
