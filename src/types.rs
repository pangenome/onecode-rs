//! Type definitions for ONEcode

use crate::ffi;
use std::ffi::CStr;

/// Re-export the OneType enum from FFI
pub use ffi::OneType;

/// Provenance information (program, version, command, date)
#[derive(Debug, Clone, PartialEq)]
pub struct OneProvenance {
    pub program: String,
    pub version: String,
    pub command: String,
    pub date: String,
}

impl From<ffi::OneProvenance> for OneProvenance {
    fn from(prov: ffi::OneProvenance) -> Self {
        unsafe {
            OneProvenance {
                program: if prov.program.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(prov.program).to_string_lossy().into_owned()
                },
                version: if prov.version.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(prov.version).to_string_lossy().into_owned()
                },
                command: if prov.command.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(prov.command).to_string_lossy().into_owned()
                },
                date: if prov.date.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(prov.date).to_string_lossy().into_owned()
                },
            }
        }
    }
}

/// Reference information (filename and count)
#[derive(Debug, Clone, PartialEq)]
pub struct OneReference {
    pub filename: String,
    pub count: i64,
}

impl From<ffi::OneReference> for OneReference {
    fn from(ref_: ffi::OneReference) -> Self {
        unsafe {
            OneReference {
                filename: if ref_.filename.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(ref_.filename).to_string_lossy().into_owned()
                },
                count: ref_.count,
            }
        }
    }
}

/// Count information
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OneCounts {
    pub count: i64,
    pub max: i64,
    pub total: i64,
}

impl From<ffi::OneCounts> for OneCounts {
    fn from(counts: ffi::OneCounts) -> Self {
        OneCounts {
            count: counts.count,
            max: counts.max,
            total: counts.total,
        }
    }
}

/// Statistics for a line type
#[derive(Debug, Clone, PartialEq)]
pub struct OneStat {
    pub count: i64,
    pub count0: i64,
    pub max_count: i64,
    pub total: i64,
    pub total0: i64,
    pub max_total: i64,
    pub line_type: char,
    pub is_list: bool,
}

impl From<ffi::OneStat> for OneStat {
    fn from(stat: ffi::OneStat) -> Self {
        OneStat {
            count: stat.count,
            count0: stat.count0,
            max_count: stat.maxCount,
            total: stat.total,
            total0: stat.total0,
            max_total: stat.maxTotal,
            line_type: stat.type_ as u8 as char,
            is_list: stat.isList,
        }
    }
}
