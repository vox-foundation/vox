//! Hardware sizing and dynamic auto model selector for local inference.
//!
//! Automatically selects the optimal local quantized or full weights tier
//! based on detected hardware (Apple Silicon unified memory vs. discrete CUDA/ROCm GPU)
//! and usable VRAM after execution reserve deduction.

use serde::{Deserialize, Serialize};

/// Dynamic recommendation envelope produced by hardware auto selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutoModelSelection {
    /// The canonical model identifier recommended for this hardware.
    pub selected_model_id: String,
    /// The detected total/wired VRAM in gigabytes.
    pub detected_vram_gb: f64,
    /// Rationale explaining the chosen sizing tier.
    pub tier_reason: String,
}

/// Computes the optimal local model tier and reason for a given VRAM size in gigabytes.
///
/// # Sizing Rules
/// - **Apple Silicon (`is_apple_silicon = true`)**:
///   Probed VRAM reflects the dynamic working set wired limit (default 75% of unified RAM).
///   A flat 2.0 GB execution headroom is deducted for OS display and framework buffers.
///   - Usable >= 55.0 GB (e.g. 128GB Mac, 96.0 GB probed) -> `merged_bf16`
///   - Usable >= 32.0 GB (e.g. 48GB Mac, 36.0 GB probed) -> `quant_q8_0`
///   - Usable >= 24.0 GB (e.g. 36GB Mac, 27.0 GB probed) -> `quant_q6_k`
///   - Usable >= 22.0 GB -> `quant_q5_k_m`
///   - Usable >= 18.0 GB -> `quant_q4_k_m`
///   - Usable < 18.0 GB (e.g. 24GB Mac, 18.0 GB probed -> 16.0 GB usable) -> `vox-mens-8b-v0.6`
///
/// - **Discrete / CUDA (`is_apple_silicon = false`)**:
///   Deducts a 10% reserve clamped between 2.8 GB and 6.0 GB:
///   `reserve = (total_vram_gb * 0.10).clamp(2.8, 6.0)`.
///   - 80.0 GB (A100/H100) -> 74.0 GB usable -> `merged_bf16`
///   - 24.0 GB (RTX 3090/4090) -> 21.2 GB usable -> `quant_q4_k_m` (never triggers Q5 OOM)
///   - 16.0 GB (RTX 4070/4080) -> 13.2 GB usable -> `vox-mens-8b-v0.6`
///   - 12.0 GB (RTX 3060) -> 9.2 GB usable -> `vox-mens-8b-v0.6`
///
/// - **Zero/NaN**:
///   Returns `("vox-mens-8b-v0.6", "Default fallback (insufficient or undetected VRAM)")`.
/// Minimum execution reserve in GB on discrete GPUs.
pub const DISCRETE_RESERVE_MIN_GB: f64 = 2.8;

/// Maximum execution reserve in GB on discrete GPUs.
pub const DISCRETE_RESERVE_MAX_GB: f64 = 6.0;

/// Execution headroom in GB on Apple Silicon unified memory (above dynamic wired limit).
pub const APPLE_SILICON_EXECUTION_HEADROOM_GB: f64 = 2.0;

#[must_use]
pub fn select_tier_for_vram(
    total_vram_gb: f64,
    is_apple_silicon: bool,
) -> (&'static str, &'static str) {
    if total_vram_gb.is_nan() || total_vram_gb <= 0.0 {
        return (
            "vox-mens-8b-v0.6",
            "Default fallback (insufficient or undetected VRAM)",
        );
    }

    let usable_gb = if is_apple_silicon {
        total_vram_gb - APPLE_SILICON_EXECUTION_HEADROOM_GB
    } else {
        let reserve =
            (total_vram_gb * 0.10).clamp(DISCRETE_RESERVE_MIN_GB, DISCRETE_RESERVE_MAX_GB);
        total_vram_gb - reserve
    };

    if usable_gb >= 55.0 {
        (
            "mens/runs/qwen3_27b_metal_check/merged_bf16",
            "Optimal unquantized 27B bf16 weights",
        )
    } else if usable_gb >= 32.0 {
        (
            "mens/runs/qwen3_27b_metal_check/quant_q8_0",
            "Near-lossless 8-bit quantized 27B",
        )
    } else if usable_gb >= 24.0 {
        (
            "mens/runs/qwen3_27b_metal_check/quant_q6_k",
            "High-fidelity 6-bit quantized 27B",
        )
    } else if usable_gb >= 22.0 {
        (
            "mens/runs/qwen3_27b_metal_check/quant_q5_k_m",
            "Balanced 5-bit quantized 27B",
        )
    } else if usable_gb >= 18.0 {
        (
            "mens/runs/qwen3_27b_metal_check/quant_q4_k_m",
            "Compact 4-bit quantized 27B",
        )
    } else {
        ("vox-mens-8b-v0.6", "Fallback 8B tier for constrained VRAM")
    }
}

/// Probes detected hardware VRAM in gigabytes.
#[must_use]
pub fn detect_hardware_vram_gb(is_apple_silicon: bool) -> f64 {
    if is_apple_silicon {
        #[cfg(target_os = "macos")]
        {
            if let Some(gb) = probe_macos_vram_gb() {
                return gb;
            }
        }
        0.0
    } else {
        probe_discrete_vram_gb().unwrap_or(0.0)
    }
}

#[cfg(target_os = "macos")]
fn probe_macos_vram_gb() -> Option<f64> {
    #[cfg(feature = "runtime")]
    {
        if let Some(summary) = vox_populi::mens::hardware::macos_metal::probe_metal() {
            if summary.vram_mb > 0 {
                return Some(summary.vram_mb as f64 / 1024.0);
            }
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
        {
            if output.status.success() {
                if let Ok(s) = std::str::from_utf8(&output.stdout) {
                    if let Ok(bytes) = s.trim().parse::<u64>() {
                        let total_ram_gb = (bytes as f64) / (1024.0 * 1024.0 * 1024.0);
                        let pct = if let Ok(po) = std::process::Command::new("sysctl")
                            .args(["-n", "iogpu.wired_limit_percent"])
                            .output()
                        {
                            if po.status.success()
                                && let Ok(ps) = std::str::from_utf8(&po.stdout)
                                && let Ok(p) = ps.trim().parse::<f64>()
                            {
                                p.clamp(65.0, 90.0)
                            } else {
                                75.0
                            }
                        } else {
                            75.0
                        };
                        return Some((total_ram_gb * pct) / 100.0);
                    }
                }
            }
        }
    }
    None
}

fn probe_discrete_vram_gb() -> Option<f64> {
    if let Ok(json) = vox_plugin_nvml_probe::probe::probe_summary() {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(devices) = val.get("devices").and_then(|d| d.as_array()) {
                let max_total_mb = devices
                    .iter()
                    .filter_map(|dev| dev.get("vram_total_mb").and_then(|v| v.as_u64()))
                    .max();
                if let Some(total_mb) = max_total_mb {
                    if total_mb > 0 {
                        return Some(total_mb as f64 / 1024.0);
                    }
                }
            }
        }
    }
    if crate::models::vram::free_vram_mb_hint().is_none() {
        crate::models::vram::refresh_free_vram_hint_from_nvml();
    }
    crate::models::vram::free_vram_mb_hint().map(|mb| mb as f64 / 1024.0)
}

/// Returns true if the host is running Apple Silicon macOS.
#[must_use]
pub fn is_host_apple_silicon() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}

/// Dynamically selects the optimal local model recommendation for the current hardware.
#[must_use]
pub fn select_optimal_local_model(is_apple_silicon: bool) -> AutoModelSelection {
    let detected_vram_gb = detect_hardware_vram_gb(is_apple_silicon);
    let (model_id, reason) = select_tier_for_vram(detected_vram_gb, is_apple_silicon);
    AutoModelSelection {
        selected_model_id: model_id.to_string(),
        detected_vram_gb,
        tier_reason: reason.to_string(),
    }
}

/// Selects the optimal local model recommendation for the host platform.
#[must_use]
pub fn select_optimal_local_model_for_host() -> AutoModelSelection {
    select_optimal_local_model(is_host_apple_silicon())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_selection_tiers() {
        // Apple Silicon tests (probe already reduces RAM to 75%, 2.0GB execution reserve deducted)
        assert_eq!(
            select_tier_for_vram(96.0, true).0,
            "mens/runs/qwen3_27b_metal_check/merged_bf16"
        );
        assert_eq!(
            select_tier_for_vram(36.0, true).0,
            "mens/runs/qwen3_27b_metal_check/quant_q8_0"
        );
        assert_eq!(
            select_tier_for_vram(27.0, true).0,
            "mens/runs/qwen3_27b_metal_check/quant_q6_k"
        );
        assert_eq!(select_tier_for_vram(18.0, true).0, "vox-mens-8b-v0.6");

        // CUDA / Discrete tests (10% clamped reserve between 2.8GB and 6.0GB)
        assert_eq!(
            select_tier_for_vram(80.0, false).0,
            "mens/runs/qwen3_27b_metal_check/merged_bf16"
        );
        assert_eq!(
            select_tier_for_vram(24.0, false).0,
            "mens/runs/qwen3_27b_metal_check/quant_q4_k_m"
        );
        assert_eq!(select_tier_for_vram(16.0, false).0, "vox-mens-8b-v0.6");
        assert_eq!(select_tier_for_vram(12.0, false).0, "vox-mens-8b-v0.6");
    }

    #[test]
    fn test_zero_and_nan_fallback() {
        assert_eq!(select_tier_for_vram(0.0, true).0, "vox-mens-8b-v0.6");
        assert_eq!(select_tier_for_vram(-5.0, false).0, "vox-mens-8b-v0.6");
        assert_eq!(select_tier_for_vram(f64::NAN, false).0, "vox-mens-8b-v0.6");
    }

    #[test]
    fn test_quant_q5_tier() {
        // Apple Silicon: 24.5 GB total - 2.0 reserve = 22.5 GB usable -> quant_q5_k_m
        assert_eq!(
            select_tier_for_vram(24.5, true).0,
            "mens/runs/qwen3_27b_metal_check/quant_q5_k_m"
        );
        // CUDA: 26.0 GB total - 2.8 reserve = 23.2 GB usable -> quant_q5_k_m
        assert_eq!(
            select_tier_for_vram(26.0, false).0,
            "mens/runs/qwen3_27b_metal_check/quant_q5_k_m"
        );
    }

    #[test]
    fn test_select_optimal_local_model_structure() {
        let sel = select_optimal_local_model(true);
        assert!(!sel.selected_model_id.is_empty());
        assert!(!sel.tier_reason.is_empty());
    }
}
