#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

//! A mobile-optimized Rust library for the Noise Protocol Framework.
//! 
//! This library provides FFI-safe bindings for iOS and Android applications,
//! specifically designed for P2P messaging apps.

pub mod core;
pub mod ffi;
pub mod mobile;

// Re-export common types
pub use crate::core::error::{NoiseError, Result};
pub use crate::core::session::NoiseSession;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_loads() {
        // Basic smoke test
        assert_eq!(crate::core::crypto::NOISE_MAX_MESSAGE_LEN, 65535);
    }
}