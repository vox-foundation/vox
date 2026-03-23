//! Host toolchain bootstrap: probes and optional fixes for building the Vox workspace
//! (especially **Turso → aegis**, which needs **clang-cl** on Windows MSVC).
//!
//! Used by the `vox-bootstrap` binary and `scripts/install.sh` / `scripts/install.ps1`.
//! Interactive project setup (API keys, wasm, Codex) lives in **`vox setup`** in the main CLI.

#![forbid(unsafe_code)]

pub mod engine;
pub mod report;

pub use engine::{BootstrapOptions, evaluate, run_and_print};
pub use report::{BootstrapItem, BootstrapReport};
