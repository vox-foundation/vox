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
    // Host-registered GPU probe (vox-orchestrator-mcp / vox-cli / vox-gui load the
    // nvml-probe plugin); none registered -> the free-VRAM hint / RAM fallback below.
    let summary = crate::models::vram::registered_vram_probe().map(|p| (p.probe_summary_json)());
    if let Some(Ok(json)) = summary {
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

/// Largest measured `qwen3_code` QLoRA size that fits `free_mb`.
///
/// The reading is injected. This function does not probe hardware.
pub fn select_from_free_mb(free_mb: u32) -> Result<AutoModelSelection, String> {
    #[cfg(feature = "runtime")]
    {
        match vox_populi::mens::tensor::spoke_base_resolver::select_qwen3_code_qlora_for_free_mb(
            free_mb,
        ) {
            Ok(pick) => Ok(AutoModelSelection {
                selected_model_id: pick.hf_id,
                detected_vram_gb: f64::from(free_mb) / 1024.0,
                tier_reason: format!(
                    "{size} QLoRA floor_mb {floor} fits free_mb {free_mb}",
                    size = pick.size_token,
                    floor = pick.floor_mb,
                ),
            }),
            Err(err) => Err(err.to_string()),
        }
    }
    #[cfg(not(feature = "runtime"))]
    {
        let _ = free_mb;
        Err("qwen3 live selector requires the runtime feature".to_string())
    }
}

/// Live free memory, then the measured QLoRA pick. `is_apple_silicon` is unused.
pub fn select_optimal_local_model(is_apple_silicon: bool) -> Result<AutoModelSelection, String> {
    let _ = is_apple_silicon;
    #[cfg(feature = "runtime")]
    {
        let free_mb = vox_populi::mens::tensor::spoke_base_resolver::read_accelerator_free_mb()
            .map_err(|err| err.to_string())?;
        select_from_free_mb(free_mb)
    }
    #[cfg(not(feature = "runtime"))]
    {
        Err("qwen3 live selector requires the runtime feature".into())
    }
}

/// Host live free memory, then the measured QLoRA pick.
pub fn select_optimal_local_model_for_host() -> Result<AutoModelSelection, String> {
    select_optimal_local_model(is_host_apple_silicon())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_from_free_mb_25000_is_the_14b_qlora_pin() {
        let sel = select_from_free_mb(25_000).expect("14B fits 25000");
        assert_eq!(
            sel.selected_model_id,
            "Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18"
        );
        assert!(!sel.selected_model_id.contains("mens/runs"));
        assert_eq!(sel.detected_vram_gb, 25_000_f64 / 1024.0);
        assert!(sel.tier_reason.contains("14B"), "{}", sel.tier_reason);
        assert!(sel.tier_reason.contains("25000"), "{}", sel.tier_reason);
        assert!(select_from_free_mb(1_500).is_err());
        let mid = select_from_free_mb(50_000).expect("14B still fits 50000");
        assert_eq!(mid.selected_model_id, sel.selected_model_id);
        assert!(!mid.selected_model_id.contains("32B"));
        let big = select_from_free_mb(70_000).expect("32B fits 70000");
        assert_eq!(
            big.selected_model_id,
            "Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137"
        );
    }

    #[test]
    fn discrete_vram_comes_from_the_registered_probe() {
        use crate::models::vram::{
            VRAM_PROBE_TEST_LOCK, VramProbe, clear_vram_probe_for_test, register_vram_probe,
        };
        fn summary() -> Result<String, String> {
            Ok(r#"{"devices":[{"vram_total_mb":8192},{"vram_total_mb":24576}]}"#.into())
        }
        fn metrics() -> Result<String, String> {
            Err("unused".into())
        }
        let _g = VRAM_PROBE_TEST_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        register_vram_probe(VramProbe {
            probe_summary_json: summary,
            device_metrics_json: metrics,
        });
        assert_eq!(probe_discrete_vram_gb(), Some(24.0), "largest device wins");
        clear_vram_probe_for_test();
    }

    #[test]
    fn test_select_optimal_local_model_structure() {
        let src = include_str!("auto_select.rs");
        let start = src
            .find("pub fn select_optimal_local_model(")
            .expect("select_optimal_local_model");
        let rest = &src[start..];
        let next = rest.find("\npub fn ").expect("next fn");
        let window = &rest[..next];
        assert!(window.contains("read_accelerator_free_mb"));
        assert!(window.contains("select_from_free_mb"));
        assert!(!window.contains("detect_hardware_vram_gb"));
        let tier_fn = ["select_tier", "_for_vram"].concat();
        assert!(!window.contains(&tier_fn));
        assert!(window.contains("let _ = is_apple_silicon"));

        let mod_rs = include_str!("mod.rs");
        let select_rs = include_str!("select.rs");
        for text in [src, mod_rs, select_rs] {
            assert!(!text.contains(&tier_fn), "{tier_fn} still present");
        }
        let run_dir = ["mens/runs/qwen3_27b", "_metal_check"].concat();
        let retired = ["vox-mens-8b", "-v0.6"].concat();
        assert!(!src.contains(&run_dir), "{run_dir}");
        assert!(!src.contains(&retired), "{retired}");
    }
}
