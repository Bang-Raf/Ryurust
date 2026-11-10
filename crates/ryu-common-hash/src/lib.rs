//! SIMD-accelerated xxHash3 128-bit hashing crate for Ryubing.

pub mod ffi;
pub mod xxh3;

pub use ffi::*;
pub use xxh3::*;
