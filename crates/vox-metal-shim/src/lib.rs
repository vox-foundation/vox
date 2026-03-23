//! A shim crate that conditionally depends on candle's `metal` features only on macOS.
//! On Windows and Linux, this crate compiles completely empty, preventing Cargo from
//! trying to build `objc2` and failing the `--all-features` workspace build.

/// A dummy struct to ensure the crate has at least one symbol.
pub struct MetalShim;
