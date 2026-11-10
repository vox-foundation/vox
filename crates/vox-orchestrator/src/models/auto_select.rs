//! Local inference auto-selection through the live size selector (SIZE-02).
//!
//! Picks the largest Qwen3 ladder size whose QLoRA floor fits live free
//! accelerator memory, with the same selector training uses
//! (`vox_populi::mens::tensor::size_select`). The fixed quantization tiers were
//! removed in Phase 318: with no reading, or when nothing fits, there is no pick.

use serde::{Deserialize, Serialize};

#[cfg(feature = "runtime")]
use vox_populi::mens::tensor::size_select::{
    self, FreeMemory, Selection, load_ladder_overlay, select_size,
};

#[cfg(not(feature = "runtime"))]
/// Fallback representation of free accelerator memory when runtime/mens is disabled.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeMemory {
    pub mb: u64,
    pub source: &'static str,
}

/// The live pick for local inference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutoModelSelection {
    /// The pinned model id (`owner/name@<40-hex revision>`).
    pub selected_model_id: String,
    /// The ladder size label (`8B`, `0.6B`, ...).
    pub size: String,
    /// The free memory in MB the pick was judged against.
    pub free_mb: u64,
    /// The size's QLoRA floor in MB, used as its footprint.
    pub floor_mb: u32,
    /// The selector's note saying why this size was picked.
    pub reason: String,
    /// The detected total/wired VRAM in gigabytes (compatibility).
    #[serde(default)]
    pub detected_vram_gb: f64,
    /// Rationale explaining the chosen sizing tier (compatibility).
    #[serde(default)]
    pub tier_reason: String,
}

/// Live free memory: the registered NVML probe's CUDA free memory (minimum
/// across the GPUs `CUDA_VISIBLE_DEVICES` exposes, x0.88, read the way training
/// reads nvidia-smi) where a host registered one and it reads above 0, otherwise
/// vox-populi's Metal or nvidia-smi reading. `None` when nothing can be read.
#[cfg(feature = "runtime")]
fn host_free_memory() -> Option<FreeMemory> {
    crate::models::vram::device_free_mb_from_probe()
        .and_then(|d| size_select::cuda_free_memory(&d))
        .or_else(size_select::read_free_memory)
}

/// The largest ladder size that fits `free`, or the selector's refusal (no
/// reading, nothing fits, or a broken ladder). Judged against the workspace's
/// `gpu-specs.yaml`, as training and serve are, else the embedded copy.
#[cfg(feature = "runtime")]
pub fn select_for_free_memory(free: Option<&FreeMemory>) -> Result<AutoModelSelection, String> {
    select_with_ladder_from(vox_repository::find_cargo_workspace_root().as_deref(), free)
}

#[cfg(not(feature = "runtime"))]
pub fn select_for_free_memory(_free: Option<&FreeMemory>) -> Result<AutoModelSelection, String> {
    Err("local model auto-selection requires the runtime feature (vox-populi/mens)".to_string())
}

#[cfg(feature = "runtime")]
fn select_with_ladder_from(
    workspace_root: Option<&std::path::Path>,
    free: Option<&FreeMemory>,
) -> Result<AutoModelSelection, String> {
    let overlay = load_ladder_overlay(workspace_root).map_err(|e| format!("{e:#}"))?;
    match select_size(&overlay, free, None, false).map_err(|e| format!("{e:#}"))? {
        Selection::Size(pick) => Ok(AutoModelSelection {
            selected_model_id: pick.rung.hf_id,
            size: pick.rung.size.clone(),
            free_mb: free.map_or(0, |f| f.mb),
            floor_mb: pick.rung.floor_mb,
            detected_vram_gb: free.map_or(0.0, |f| f.mb as f64 / 1024.0),
            tier_reason: pick.rung.size,
            reason: pick.note,
        }),
        Selection::PassThrough(id) => Err(format!("selector passed through {id} without a size")),
    }
}

/// The live pick for this host, or the reason there is none.
#[cfg(feature = "runtime")]
pub fn select_optimal_local_model_for_host() -> Result<AutoModelSelection, String> {
    select_for_free_memory(host_free_memory().as_ref())
}

#[cfg(not(feature = "runtime"))]
pub fn select_optimal_local_model_for_host() -> Result<AutoModelSelection, String> {
    Err("local model auto-selection requires the runtime feature (vox-populi/mens)".to_string())
}

#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;

    #[test]
    fn registered_probe_free_memory_wins_and_keeps_the_margin() {
        use crate::models::vram::{
            VRAM_PROBE_TEST_LOCK, VramProbe, clear_vram_probe_for_test, register_vram_probe,
        };
        fn summary() -> Result<String, String> {
            Err("unused".into())
        }
        fn metrics() -> Result<String, String> {
            Ok(r#"{"metrics":[{"memory_free_mb":20000},{"memory_free_mb":30000}]}"#.into())
        }
        let _g = VRAM_PROBE_TEST_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        register_vram_probe(VramProbe {
            probe_summary_json: summary,
            device_metrics_json: metrics,
        });
        let free = host_free_memory();
        clear_vram_probe_for_test();
        assert_eq!(free.map(|f| f.mb), Some(17_600), "minimum device, x0.88");
    }

    /// WR-02: NVML reports 0 for a device it cannot read; that is no reading, so
    /// the next source (Metal or nvidia-smi) is tried instead of refusing on 0 MB.
    #[test]
    fn a_zero_probe_reading_falls_through_to_the_next_source() {
        use crate::models::vram::{
            VRAM_PROBE_TEST_LOCK, VramProbe, clear_vram_probe_for_test, register_vram_probe,
        };
        fn summary() -> Result<String, String> {
            Err("unused".into())
        }
        fn metrics() -> Result<String, String> {
            Ok(r#"{"metrics":[{"index":0,"memory_free_mb":0},{"index":1,"memory_free_mb":30000}]}"#.into())
        }
        let _g = VRAM_PROBE_TEST_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        register_vram_probe(VramProbe {
            probe_summary_json: summary,
            device_metrics_json: metrics,
        });
        let free = host_free_memory();
        clear_vram_probe_for_test();
        // Not the probe's 0 MB: the next source's reading, or none.
        assert!(free.is_none_or(|f| f.mb > 0), "{free:?}");
    }

    #[test]
    fn auto_selection_picks_the_largest_fitting_pin() {
        let free = FreeMemory {
            mb: 130_000,
            source: "test",
        };
        let sel = select_for_free_memory(Some(&free)).expect("8B fits 130000 MB");
        assert_eq!(
            sel.selected_model_id,
            "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218"
        );
        assert_eq!(sel.size, "8B");
        assert_eq!(sel.floor_mb, 122_876);
        assert_eq!(sel.free_mb, 130_000);
        assert!(!sel.reason.is_empty());
    }

    /// WR-06: an edited workspace `gpu-specs.yaml` (Phase 320 measured floors)
    /// changes chat's pick the same way it changes training's, without a rebuild.
    #[test]
    fn auto_selection_reads_the_workspace_ladder() {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let yaml = std::fs::read_to_string(repo.join("mens/config/gpu-specs.yaml")).unwrap();
        let pin_8b = "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218\",   floor_mb: 122876";
        assert!(yaml.contains(pin_8b), "8B rung moved; update this test");
        let raised = yaml.replace(pin_8b, &pin_8b.replace("122876", "140000"));
        let root =
            std::env::temp_dir().join(format!("vox-auto-select-ladder-{}", std::process::id()));
        std::fs::create_dir_all(root.join("mens/config")).unwrap();
        std::fs::write(root.join("mens/config/gpu-specs.yaml"), raised).unwrap();

        let free = FreeMemory {
            mb: 130_000,
            source: "test",
        };
        let sel = select_with_ladder_from(Some(&root), Some(&free));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(sel.expect("4B fits").size, "4B", "8B now needs 140000 MB");
        let missing = root.join("no-such-dir");
        assert_eq!(
            select_with_ladder_from(Some(&missing), Some(&free))
                .expect("embedded")
                .size,
            "8B",
            "no workspace file falls back to the embedded ladder"
        );
    }

    #[test]
    fn auto_selection_refuses_without_a_reading() {
        let err = select_for_free_memory(None).expect_err("no reading, no pick");
        assert!(err.contains("no live free-memory reading"), "{err}");
    }

    #[test]
    fn auto_selection_refuses_when_nothing_fits() {
        let free = FreeMemory {
            mb: 1_000,
            source: "test",
        };
        let err = select_for_free_memory(Some(&free)).expect_err("nothing fits");
        assert!(err.contains("0.6B"), "{err}");
    }
}
