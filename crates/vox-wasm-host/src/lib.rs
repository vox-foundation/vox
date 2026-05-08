//! # vox-wasm-host
//!
//! Single-source-of-truth Wasmtime engine construction, WASI context wiring,
//! and WASM module execution for the Vox toolchain.
//!
//! Used by:
//! - `vox-cli` (`vox run --backend wasi`) for testing arbitrary Vox programs
//! - `vox-plugin-runtime-wasm` for sandboxed skill execution
//!
//! # Architecture
//!
//! One engine builder, one preopen mapper, one exit-code handler — no duplication
//! of Wasmtime boilerplate across call sites.

mod engine;
mod exec;
mod preopen;

pub use engine::WasmHost;
pub use exec::WasmExecOpts;
pub use preopen::{Preopen, PreopenMode};

/// Outcome of a completed WASM module execution.
#[derive(Debug, Clone)]
pub struct WasmRunOutcome {
    /// Process exit code (0 = success).
    pub exit_code: i32,
    /// Captured stdout bytes.
    pub stdout: Vec<u8>,
    /// Captured stderr bytes.
    pub stderr: Vec<u8>,
    /// Wall-clock execution time in milliseconds.
    pub wall_ms: u64,
}

impl WasmRunOutcome {
    /// Returns `true` if the module exited successfully (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Decode stdout as UTF-8, lossy.
    pub fn stdout_str(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.stdout)
    }

    /// Decode stderr as UTF-8, lossy.
    pub fn stderr_str(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.stderr)
    }
}
