//! # Timestamp Utilities
//!
//! Provides high-precision timestamps for ordering operations.

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current timestamp in milliseconds since Unix epoch.
///
/// Used for ordering batches, key rotations, and other time-sensitive operations.
/// The `u128` type ensures no overflow for millions of years.
///
/// # Panics
///
/// Panics if the system clock is set before the Unix epoch (January 1, 1970).
pub fn timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
