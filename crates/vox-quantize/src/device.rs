//! Device selection — GPU-first with CPU fallback.

use crate::error::QuantizeError;
use candle_core::Device;

/// User device preference. `Auto` picks the best available accelerator,
/// falling back to CPU. Mirrors vox-plugin-mens-candle-cuda::device_select.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DevicePref {
    #[default]
    Auto,
    Cuda(usize),
    Metal,
    Cpu,
}

pub fn select(pref: DevicePref) -> Result<Device, QuantizeError> {
    let to_err = |e: candle_core::Error| QuantizeError::Quantize(e);
    match pref {
        DevicePref::Cpu => Ok(Device::Cpu),
        DevicePref::Cuda(i) => Device::new_cuda(i).map_err(to_err),
        DevicePref::Metal => Device::new_metal(0).map_err(to_err),
        DevicePref::Auto => {
            if let Ok(d) = Device::new_cuda(0) {
                tracing::info!("vox-quantize: using CUDA:0");
                return Ok(d);
            }
            // Note: Candle performs GGML quantization on CPU via NEON/AVX kernels.
            // Placing QTensor on Metal triggers uncollected Metal buffer cache allocations.
            // Auto defaults to CPU for fast, leak-free quantization.
            tracing::info!("vox-quantize: using CPU (NEON/AVX vectorized quantization)");
            Ok(Device::Cpu)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cpu_pref_always_resolves_to_cpu() {
        let d = select(DevicePref::Cpu).unwrap();
        assert!(d.is_cpu());
    }
    #[test]
    fn auto_resolves_without_error() {
        // On CI (no GPU feature) Auto must resolve to CPU, never error.
        let d = select(DevicePref::Auto).unwrap();
        let _ = d;
    }
}
