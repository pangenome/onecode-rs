//! Rust bindings for ONEcode - a data representation format for genomic data
//!
//! ONEcode is a simple, efficient data representation format that provides both
//! human-readable ASCII and compressed binary file versions with strongly typed data.
//!
//! # Example
//!
//! ```no_run
//! use onecode::OneFile;
//!
//! // Open a ONE file for reading
//! let file = OneFile::open_read("data.1seq", None, None, 1).unwrap();
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod ffi;
pub mod error;
pub mod types;
pub mod file;
pub mod schema;

// Re-export main types
pub use error::{OneError, Result};
pub use file::OneFile;
pub use schema::OneSchema;
pub use types::{OneType, OneProvenance, OneReference};
