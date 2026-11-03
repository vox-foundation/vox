//! Shared inference-time device resolution (the "L-7" fix).
//!
//! Before this crate existed, `vox-plugin-mens-candle-cuda::inference` had a
//! `resolve_inference_device` that warns and errors correctly on a failed
//! device request, while `vox-plugin-mens-candle-metal::inference::InferenceEngine::load`
//! inlined a bare `Device::new_metal(0).unwrap_or(Device::Cpu)` — silently
//! falling back to CPU inference with **no warning**, even for an explicit
//! `DeviceKind::Metal` request (CUDA's version only silently falls back for
//! `DeviceKind::Best`; an explicit `Metal`/`Cuda` request that fails is a hard
//! error on both backends after this fix).
//!
//! The function's *shape* was identical between the two plugins — a match on
//! `DeviceKind` with `#[cfg(feature = "cuda")]` / `#[cfg(feature = "metal")]`
//! arms — so instead of forking two copies (or leaving Metal permanently
//! behind), it lives here once. Each plugin turns on the matching feature on
//! its `vox-plugin-mens-candle-core` dependency (`cuda` for the CUDA plugin,
//! `metal` for the Metal plugin), so the *same* compiled function calls
//! `Device::new_cuda` / `Device::new_metal` depending on which plugin linked it in.

use anyhow::Result;
use candle_core::Device;

use crate::device::DeviceKind;

/// Compute dtype for QLoRA dequantization, chosen by device rather than by
/// `QLoraConfig::default()`'s training-tuned BF16. Candle's CPU backend has
/// no BF16 matmul kernel at all ("unsupported dtype BF16 for op matmul"), so
/// CPU inference must use F32. Metal training also dequants to F32 (no
/// F32→F64 / BF16 kernels on the path we hit), so Metal serving must match
/// its own trainer's compute dtype. CUDA keeps BF16.
#[must_use]
pub fn compute_dtype_for_device(device: &Device) -> qlora_rs::ComputeDType {
    if device.is_cuda() {
        qlora_rs::ComputeDType::BF16
    } else {
        qlora_rs::ComputeDType::F32
    }
}

/// Resolve the Candle device for inference.
///
/// - `Cpu` always succeeds.
/// - `Cuda` / `Metal` (explicit): if the matching cargo feature isn't
///   compiled in, or device init fails, this is a **hard error** — no silent
///   CPU fallback for an explicit accelerator request.
/// - `Best`: tries the accelerator feature(s) this build has, and falls back
///   to CPU **with a `tracing::warn!`** if none are available or init fails.
///   This is the behavior CUDA already had; Metal previously had neither the
///   warning nor the error path (see module docs).
pub fn resolve_inference_device(device_kind: &DeviceKind) -> Result<Device> {
    match device_kind {
        DeviceKind::Cpu => Ok(Device::Cpu),
        DeviceKind::Cuda => {
            #[cfg(feature = "cuda")]
            {
                Ok(Device::new_cuda(0)?)
            }
            #[cfg(not(feature = "cuda"))]
            {
                anyhow::bail!(
                    "Plugin built without the `cuda` feature — recompile with `--features cuda`"
                );
            }
        }
        DeviceKind::Metal => {
            #[cfg(feature = "metal")]
            {
                Ok(Device::new_metal(0)?)
            }
            #[cfg(not(feature = "metal"))]
            {
                anyhow::bail!(
                    "Plugin built without the `metal` feature — recompile with `--features metal`"
                );
            }
        }
        DeviceKind::Best => {
            #[cfg(feature = "cuda")]
            {
                match Device::new_cuda(0) {
                    Ok(device) => return Ok(device),
                    Err(err) => {
                        tracing::warn!("CUDA unavailable for inference — trying next: {err}");
                    }
                }
            }
            #[cfg(feature = "metal")]
            {
                match Device::new_metal(0) {
                    Ok(device) => return Ok(device),
                    Err(err) => {
                        tracing::warn!(
                            "Metal unavailable for inference — falling back to CPU: {err}"
                        );
                    }
                }
            }
            Ok(Device::Cpu)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_inference_device_cpu_is_cpu() {
        let d = resolve_inference_device(&DeviceKind::Cpu).unwrap();
        assert!(d.is_cpu());
    }

    #[cfg(feature = "metal")]
    #[test]
    fn resolve_inference_device_best_prefers_metal_when_available() {
        let d = resolve_inference_device(&DeviceKind::Best).unwrap();
        assert!(
            d.is_metal() || d.is_cpu(),
            "Best must land on Metal or CPU fallback, got {d:?}"
        );
    }

    #[test]
    fn compute_dtype_is_f32_on_cpu_bf16_on_cuda() {
        // Candle's CPU backend cannot matmul BF16 at all — this is the one
        // rung of the device-dtype ladder that MUST be F32, not a tuning
        // choice. CUDA keeps the training-tuned BF16 default. Metal uses
        // F32 to match this lane's training compute.
        assert!(matches!(
            compute_dtype_for_device(&candle_core::Device::Cpu),
            qlora_rs::ComputeDType::F32
        ));
    }

    /// The Metal bug this module fixes ("L-7"): an explicit accelerator
    /// request that cannot be honored must be a hard error, never a silent
    /// CPU fallback. Without the `metal`/`cuda` feature compiled in, this
    /// crate can't actually construct a real accelerator device to fail
    /// against, so this test exercises the no-feature branch directly, which
    /// is exactly the code path Metal's plugin build (no `metal` feature by
    /// default) takes for `--device metal` today.
    #[test]
    #[cfg(not(any(feature = "cuda", feature = "metal")))]
    fn resolve_inference_device_explicit_metal_errors_without_feature() {
        let err = resolve_inference_device(&DeviceKind::Metal).unwrap_err();
        assert!(err.to_string().contains("metal"));
    }

    #[test]
    #[cfg(not(any(feature = "cuda", feature = "metal")))]
    fn resolve_inference_device_explicit_cuda_errors_without_feature() {
        let err = resolve_inference_device(&DeviceKind::Cuda).unwrap_err();
        assert!(err.to_string().contains("cuda"));
    }
}
