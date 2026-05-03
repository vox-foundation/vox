//! CUDA cdylib spike — proves candle-core's CUDA feature can be linked
//! inside a cdylib for the plugin system redesign.
//!
//! See: docs/src/architecture/plugin-system-redesign-2026.md (SP3).

use std::ffi::c_int;

/// Returns 1 if candle can open CUDA device 0, 0 if not, -1 on unrecoverable error.
#[unsafe(no_mangle)]
pub extern "C" fn vox_spike_cuda_available() -> c_int {
    match candle_core::Device::new_cuda(0) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

/// Static smoke marker so a loader can verify symbol resolution
/// without actually touching CUDA.
#[unsafe(no_mangle)]
pub extern "C" fn vox_spike_smoke() -> *const u8 {
    b"vox-cuda-spike-ok\0".as_ptr()
}
