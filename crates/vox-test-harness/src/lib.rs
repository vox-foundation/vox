//! Shared test infrastructure for Vox tooling and integration tests.
//!
//! Provides shared test fixtures and harness primitives.
//!
//! # Usage
//!
//! ```rust,ignore
//! use vox_test_harness::spans::dummy_span;
//! use vox_test_harness::barriers::TestBarrier;
//! use vox_test_harness::env_scratch::EnvScratch;
//! ```

pub mod barriers;
pub mod diagnosis;
pub mod env_scratch;
pub mod portpicker;
pub mod spans;
pub mod synthetic_workspace;
pub mod temp_root;
pub mod workspace_paths;
