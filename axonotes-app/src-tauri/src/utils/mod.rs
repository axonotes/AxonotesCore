//! # Utility Module
//!
//! Common utility functions used throughout the application.
//!
//! ## Submodules
//!
//! - **`timestamp`**: High-precision timestamp generation for ordering operations
//! - **`varint`**: LEB128 varint encoding for key_index
//! - **`vec_array`**: Conversions between `Vec<u8>` and fixed-size arrays

pub(crate) mod timestamp;
pub(crate) mod varint;
pub(crate) mod vec_array;
