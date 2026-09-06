//! VRAM auto-detection and training preset selection.
//!
//! Uses the HardwareRegistry SSOT to identify available video memory.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VramInfo {
    pub total_gb: f32,
    pub used_gb: f32,
    pub free_gb: f32,
}

fn parse_nvidia_smi_output(stdout: &str) -> Option<VramInfo> {
    let first_line = stdout.lines().next()?.trim();
    let parts: Vec<&str> = first_line.split(',').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return None;
    }
    let total_mib: f32 = parts[0].parse().ok()?;
    let used_mib: f32 = parts[1].parse().ok()?;
    let free_mib: f32 = parts[2].parse().ok()?;
    Some(VramInfo {
        total_gb: total_mib / 1024.0,
        used_gb: used_mib / 1024.0,
        free_gb: free_mib / 1024.0,
    })
}

/// Query total, used, and free VRAM info from `nvidia-smi`.
pub fn query_nvidia_smi_vram() -> Option<VramInfo> {
    let out = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.total,memory.used,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_nvidia_smi_output(&String::from_utf8_lossy(&out.stdout))
}

/// Memory reserved for macOS, WindowServer, the compositor and the Vox GUI on a
/// unified-memory host.
///
/// This is a fixed floor, not a fraction, because the OS/GUI footprint does not
/// scale with model size — unlike CUDA context overhead, which the fractional
/// `VOX_MENS_VRAM_SAFETY` knob in `memory_budget.rs` covers.
pub const DEFAULT_UNIFIED_MEM_RESERVE_GIB: f32 = 12.0;

/// Model-usable memory on a unified-memory host: total minus the OS/GUI reserve.
/// Clamped at zero so a small machine yields "nothing fits", never a negative budget.
#[must_use]
pub fn usable_unified_memory_gb(total_gb: f32, reserve_gb: f32) -> f32 {
    (total_gb - reserve_gb).max(0.0)
}

/// Reserve override, in GiB. Plain env var (a memory budget is not a secret),
/// matching the `VOX_MENS_VRAM_SAFETY` idiom in `memory_budget.rs`.
fn unified_mem_reserve_gib() -> f32 {
    std::env::var("VOX_MENS_UNIFIED_MEM_RESERVE_GIB")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| v.is_finite() && *v >= 0.0)
        .unwrap_or(DEFAULT_UNIFIED_MEM_RESERVE_GIB)
}

/// Parse `sysctl -n hw.memsize` output (bytes) into GiB. `None` on garbage or zero.
fn parse_hw_memsize(stdout: &str) -> Option<f32> {
    let bytes: f64 = stdout.trim().parse().ok()?;
    if bytes <= 0.0 {
        return None;
    }
    Some((bytes / (1024.0 * 1024.0 * 1024.0)) as f32)
}

/// Apple Silicon unified memory, reported as the budget available to training.
///
/// `total_gb` is deliberately the **usable** figure (physical minus
/// [`DEFAULT_UNIFIED_MEM_RESERVE_GIB`]): downstream consumers — `pick_base`,
/// `auto_preset`, `memory_budget` — all read `total_gb` as "what training may
/// use", and on unified memory that is not the physical total.
#[cfg(target_os = "macos")]
pub fn query_apple_unified_memory() -> Option<VramInfo> {
    let out = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let physical_gb = parse_hw_memsize(&String::from_utf8_lossy(&out.stdout))?;
    let usable_gb = usable_unified_memory_gb(physical_gb, unified_mem_reserve_gib());
    Some(VramInfo {
        total_gb: usable_gb,
        used_gb: physical_gb - usable_gb,
        free_gb: usable_gb,
    })
}

/// Non-macOS hosts have no unified-memory pool to report.
#[cfg(not(target_os = "macos"))]
pub fn query_apple_unified_memory() -> Option<VramInfo> {
    None
}

/// Query available GPU VRAM info.
pub fn get_system_vram_info() -> Option<VramInfo> {
    // Priority 1: env override
    if let Some(v) = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxVramOverrideGb).expose()
        && let Ok(gb) = v.parse::<f32>()
        && gb > 0.0
    {
        return Some(VramInfo {
            total_gb: gb,
            used_gb: 0.0,
            free_gb: gb,
        });
    }

    // Priority 2: nvidia-smi query
    if let Some(info) = query_nvidia_smi_vram() {
        return Some(info);
    }

    // Priority 3: Apple Silicon unified memory, minus the OS/GUI reserve.
    if let Some(info) = query_apple_unified_memory() {
        return Some(info);
    }

    // Priority 4: hardware SSOT
    let hardware = futures::executor::block_on(crate::mens::hardware::probe());
    if hardware.vram_mb > 0 {
        let gb = hardware.vram_mb as f32 / 1024.0;
        return Some(VramInfo {
            total_gb: gb,
            used_gb: 0.0,
            free_gb: gb,
        });
    }

    None
}

/// Query available GPU VRAM in GiB.
pub fn get_system_vram_gb() -> Option<f32> {
    get_system_vram_info().map(|i| i.total_gb)
}

/// Select the best training preset for the detected hardware.
///
/// Returns a preset name matching `preset_schema.rs` known aliases.
/// Returns `None` when CUDA is not in use or VRAM is too low.
pub fn auto_preset(device_is_cuda: bool, vram_gb: Option<f32>) -> Option<&'static str> {
    if !device_is_cuda {
        return None;
    }
    match vram_gb {
        Some(v) if v < 6.0 => None, // Too small for QLoRA
        Some(v) if v < 10.0 => Some("safe"),
        Some(v) if v <= 16.0 => Some("qwen_4080_16g"),
        Some(v) if v <= 24.0 => Some("4080"),
        Some(_) => Some("a100"),
        None => None,
    }
}

/// Which accelerator the training run will actually use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceleratorKind {
    Cuda,
    Metal,
    Cpu,
}

/// Select a training preset for an accelerator kind.
///
/// The Metal arm maps unified-memory budgets onto the existing `qwen3_*` ladder
/// in `preset_schema::KNOWN_PRESETS`. The CUDA arm delegates to [`auto_preset`]
/// so the two paths cannot drift.
#[must_use]
pub fn auto_preset_for(kind: AcceleratorKind, vram_gb: Option<f32>) -> Option<&'static str> {
    match kind {
        AcceleratorKind::Cpu => None,
        AcceleratorKind::Cuda => auto_preset(true, vram_gb),
        AcceleratorKind::Metal => match vram_gb {
            Some(v) if v < 6.0 => None,
            Some(v) if v < 16.0 => Some("qwen3_dev_cpu"),
            Some(v) if v < 24.0 => Some("qwen3_16g"),
            Some(v) if v < 48.0 => Some("qwen3_24g"),
            Some(v) if v < 96.0 => Some("qwen3_48g"),
            Some(_) => Some("qwen3_96g"),
            None => None,
        },
    }
}

/// Human-readable summary of detected VRAM + auto-selected preset.
pub fn vram_summary(device_is_cuda: bool) -> String {
    let info = get_system_vram_info();
    let preset = auto_preset(device_is_cuda, info.map(|i| i.total_gb));
    match (info, preset) {
        (Some(i), Some(p)) => format!(
            "VRAM: {:.1} GiB total, {:.1} GiB used, {:.1} GiB free → preset '{p}'",
            i.total_gb, i.used_gb, i.free_gb
        ),
        (Some(i), None) => format!(
            "VRAM: {:.1} GiB total, {:.1} GiB used, {:.1} GiB free (no matching preset; specify --preset manually)",
            i.total_gb, i.used_gb, i.free_gb
        ),
        (None, _) => {
            "Could not detect VRAM (set VOX_VRAM_OVERRIDE_GB or pass --preset manually)".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usable_unified_memory_subtracts_the_gui_reserve() {
        // A 128 GiB Mac with the default 12 GiB reserve must budget 116 GiB,
        // not 128 — the GUI, WindowServer and the compositor share this pool.
        assert_eq!(usable_unified_memory_gb(128.0, 12.0), 116.0);
    }

    #[test]
    fn usable_unified_memory_never_returns_negative() {
        // Catches: an 8 GiB Mac with a 12 GiB reserve producing -4.0, which
        // would flow into pick_base as a nonsense budget.
        assert_eq!(usable_unified_memory_gb(8.0, 12.0), 0.0);
    }

    #[test]
    fn parse_hw_memsize_converts_bytes_to_gib() {
        // `sysctl -n hw.memsize` on a 128 GiB machine prints exactly this.
        assert_eq!(parse_hw_memsize("137438953472\n"), Some(128.0));
    }

    #[test]
    fn parse_hw_memsize_rejects_garbage_and_zero() {
        assert_eq!(parse_hw_memsize(""), None);
        assert_eq!(parse_hw_memsize("not-a-number"), None);
        assert_eq!(parse_hw_memsize("0"), None);
    }

    #[test]
    fn auto_preset_maps_correctly() {
        assert_eq!(auto_preset(true, Some(16.0)), Some("qwen_4080_16g"));
        assert_eq!(auto_preset(true, Some(8.0)), Some("safe"));
        assert_eq!(auto_preset(true, Some(80.0)), Some("a100"));
        assert_eq!(auto_preset(false, Some(16.0)), None);
        assert_eq!(auto_preset(true, Some(4.0)), None);
    }

    #[test]
    fn metal_128g_mac_selects_the_32b_preset() {
        // 128 GiB physical - 12 GiB reserve = 116 GiB usable -> the top rung.
        assert_eq!(
            auto_preset_for(AcceleratorKind::Metal, Some(116.0)),
            Some("qwen3_96g")
        );
    }

    #[test]
    fn metal_tiers_walk_the_qwen3_ladder() {
        assert_eq!(
            auto_preset_for(AcceleratorKind::Metal, Some(20.0)),
            Some("qwen3_16g")
        );
        assert_eq!(
            auto_preset_for(AcceleratorKind::Metal, Some(32.0)),
            Some("qwen3_24g")
        );
        assert_eq!(
            auto_preset_for(AcceleratorKind::Metal, Some(64.0)),
            Some("qwen3_48g")
        );
    }

    #[test]
    fn metal_below_floor_and_cpu_kind_yield_no_preset() {
        assert_eq!(auto_preset_for(AcceleratorKind::Metal, Some(4.0)), None);
        assert_eq!(auto_preset_for(AcceleratorKind::Metal, None), None);
        assert_eq!(auto_preset_for(AcceleratorKind::Cpu, Some(128.0)), None);
    }

    #[test]
    fn cuda_kind_matches_the_legacy_boolean_api() {
        // Catches divergence between the new enum path and the old boolean one.
        for gb in [8.0_f32, 16.0, 24.0, 80.0] {
            assert_eq!(
                auto_preset_for(AcceleratorKind::Cuda, Some(gb)),
                auto_preset(true, Some(gb)),
                "enum and boolean CUDA paths disagree at {gb} GiB"
            );
        }
    }

    #[test]
    #[allow(unsafe_code)]
    fn vram_override_env_is_respected() {
        // Set a fake value and confirm it returns correctly.
        unsafe {
            std::env::set_var("VOX_VRAM_OVERRIDE_GB", "20.0");
        }
        assert_eq!(get_system_vram_gb(), Some(20.0));
        unsafe {
            std::env::remove_var("VOX_VRAM_OVERRIDE_GB");
        }
    }
}

#[cfg(test)]
mod semcov_wave26_tests {
    use super::*;

    // ── auto_preset boundary probes ──────────────────────────────────────────

    #[test]
    fn auto_preset_none_when_no_cuda() {
        // Catches: auto_preset() ignoring device_is_cuda=false and returning a
        // preset for any VRAM value, triggering CUDA ops on a CPU-only host.
        assert_eq!(auto_preset(false, Some(24.0)), None);
        assert_eq!(auto_preset(false, Some(100.0)), None);
        assert_eq!(auto_preset(false, None), None);
    }

    #[test]
    fn auto_preset_none_when_vram_none() {
        // Catches: auto_preset returning Some("safe") when VRAM is unknown,
        // which would silently run training with the wrong budget preset.
        assert_eq!(auto_preset(true, None), None);
    }

    #[test]
    fn auto_preset_exact_boundary_6gb() {
        // Catches: off-by-one at the 6.0 GiB threshold: < 6 → None, >= 6 → Some.
        // A "<= 6.0" comparison would incorrectly make 6.0 GiB → None (too small).
        assert_eq!(
            auto_preset(true, Some(6.0)),
            Some("safe"),
            "exactly 6.0 GiB should select 'safe', not be rejected as too small"
        );
        assert_eq!(
            auto_preset(true, Some(5.99)),
            None,
            "5.99 GiB is below the 6 GiB floor"
        );
    }

    #[test]
    fn auto_preset_exact_boundary_10gb() {
        // Catches: off-by-one at 10 GiB: < 10 → "safe", >= 10 → next tier.
        // A "<= 10.0" would wrongly assign 10.0 GiB → "safe" not "qwen_4080_16g".
        assert_eq!(
            auto_preset(true, Some(9.99)),
            Some("safe"),
            "9.99 GiB should be 'safe'"
        );
        // 10 GiB is between 10 and 16 → should land in qwen_4080_16g tier
        assert_eq!(
            auto_preset(true, Some(10.0)),
            Some("qwen_4080_16g"),
            "10.0 GiB should select 'qwen_4080_16g', not 'safe'"
        );
    }

    #[test]
    fn auto_preset_exact_boundary_16gb() {
        // Catches: strict `< 16` instead of `<= 16` that would push a 16 GiB card
        // into the "4080" (24 GiB) preset, over-allocating and causing OOM.
        assert_eq!(
            auto_preset(true, Some(16.0)),
            Some("qwen_4080_16g"),
            "16.0 GiB should be 'qwen_4080_16g'"
        );
        assert_ne!(
            auto_preset(true, Some(16.0)),
            Some("4080"),
            "16 GiB must not fall through to the 24 GiB '4080' preset"
        );
    }

    #[test]
    fn auto_preset_exact_boundary_24gb() {
        // Catches: `< 24` vs `<= 24` confusion: a 24 GiB card should use "4080",
        // not "a100"; using strict-less would silently bump it to a100.
        assert_eq!(
            auto_preset(true, Some(24.0)),
            Some("4080"),
            "24.0 GiB should be '4080'"
        );
        assert_ne!(
            auto_preset(true, Some(24.0)),
            Some("a100"),
            "24 GiB must not be assigned the a100 preset"
        );
    }

    #[test]
    fn auto_preset_very_large_vram_is_a100() {
        // Catches: missing wildcard arm that panics or returns None for very
        // large VRAM values (e.g. 80 GiB A100, 96 GiB H100).
        assert_eq!(auto_preset(true, Some(80.0)), Some("a100"));
        assert_eq!(auto_preset(true, Some(96.0)), Some("a100"));
    }

    #[test]
    fn auto_preset_zero_vram_returns_none() {
        // Catches: zero VRAM (e.g., hardware probe returning 0 and caller
        // converting to Some(0.0)) passing the 6-GiB floor and selecting "safe".
        assert_eq!(
            auto_preset(true, Some(0.0)),
            None,
            "0 GiB VRAM should be too small for any preset"
        );
    }

    // ── vram_summary format ───────────────────────────────────────────────────

    #[test]
    fn vram_summary_no_cuda_reports_no_preset_when_vram_known() {
        // Catches: vram_summary() ignoring device_is_cuda=false and printing a
        // CUDA preset name in the summary for a CPU-only machine.
        // We can only test the CPU-only path safely without env side-effects.
        let summary = vram_summary(false);
        // Either VRAM was detected or not; in neither case should a preset appear.
        assert!(
            !summary.contains("qwen_4080_16g")
                && !summary.contains("4080")
                && !summary.contains("a100")
                && !summary.contains("safe"),
            "CPU summary must not mention a GPU preset; got: {summary}"
        );
    }

    #[test]
    fn test_parse_nvidia_smi_output() {
        let sample = "16376, 1401, 14645\n";
        let info = parse_nvidia_smi_output(sample).unwrap();
        assert_eq!(info.total_gb, 16376.0 / 1024.0);
        assert_eq!(info.used_gb, 1401.0 / 1024.0);
        assert_eq!(info.free_gb, 14645.0 / 1024.0);
    }

    #[test]
    fn test_parse_nvidia_smi_output_invalid() {
        assert!(parse_nvidia_smi_output("invalid").is_none());
        assert!(parse_nvidia_smi_output("16376, 1401").is_none());
    }
}
