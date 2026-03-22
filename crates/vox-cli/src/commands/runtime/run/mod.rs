//! Run, script, and test execution.
//!
//! The default `vox` binary uses the `run` submodule (compilerd) and the `test` submodule.
//! Inline script compilation (`script`, `backend`, `sandbox`) is built only with `--features script-execution`.

#[cfg(feature = "script-execution")]
pub mod backend;
#[allow(clippy::module_inception)]
pub mod run;
#[cfg(feature = "script-execution")]
pub mod sandbox;
#[cfg(feature = "script-execution")]
pub mod script;
pub mod test;
