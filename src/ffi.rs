//! Raw FFI bindings to the ONEcode C library
//!
//! This module contains the bindings to ONEcode C library.
//! By default, uses pre-generated bindings. Enable `regenerate-bindings` feature
//! to regenerate them at build time (requires libclang).
//!
//! Use the safe wrappers in the parent modules instead of these raw bindings.

#![allow(dead_code)]

// When regenerating bindings, use the ones from OUT_DIR
#[cfg(feature = "regenerate-bindings")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// By default, use pre-generated bindings (no libclang required)
#[cfg(not(feature = "regenerate-bindings"))]
include!("bindings.rs");
