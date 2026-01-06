//! # Vector to Array Conversion Utilities
//!
//! Provides ergonomic conversion between `Vec<u8>` and fixed-size byte arrays.
//!
//! ## Use Cases
//!
//! Cryptographic functions often require fixed-size arrays (e.g., `[u8; 32]` for keys),
//! while network and database operations typically use `Vec<u8>`. These utilities
//! bridge that gap with clear error messages.

/// Trait for converting byte vectors to fixed-size arrays.
#[allow(dead_code)]
pub trait ByteArrayConversion {
    /// Converts `Vec<u8>` to a fixed-size array `[u8; N]`, consuming the vector.
    ///
    /// # Errors
    ///
    /// Returns an error if the vector length doesn't match `N`.
    fn to_array<const N: usize>(self) -> Result<[u8; N], String>;

    /// Returns a reference to the vector's contents as a fixed-size array.
    ///
    /// # Errors
    ///
    /// Returns an error if the vector length doesn't match `N`.
    fn as_array<const N: usize>(&self) -> Result<&[u8; N], String>;
}

impl ByteArrayConversion for Vec<u8> {
    fn to_array<const N: usize>(self) -> Result<[u8; N], String> {
        self.try_into()
            .map_err(|v: Vec<u8>| format!("Expected {} bytes, got {}", N, v.len()))
    }

    fn as_array<const N: usize>(&self) -> Result<&[u8; N], String> {
        self.as_slice()
            .try_into()
            .map_err(|_| format!("Expected {} bytes, got {}", N, self.len()))
    }
}
