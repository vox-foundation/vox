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

/// Minimum safety reserve subtracted from live-available memory, in GiB, even
/// when the proportional margin (`VOX_MENS_LIVE_MEM_MARGIN_PCT`) would compute
/// less — protects a nearly-idle small Mac from a near-zero margin.
pub const MIN_LIVE_MEM_RESERVE_GIB: f32 = 2.0;

/// Parse `vm_stat` output into `(page_size_bytes, free_pages, inactive_pages, speculative_pages)`.
/// `None` if any required field is missing or unparseable.
///
/// English prefixes only (same class as the `nvidia-smi` CSV parser). Purgeable
/// pages are omitted from the reclaimable sum by design.
fn parse_vm_stat(stdout: &str) -> Option<(u64, u64, u64, u64)> {
    fn trailing_count(rest: &str) -> Option<u64> {
        rest.trim().trim_end_matches('.').parse::<u64>().ok()
    }

    let mut page_size = None;
    let mut free = None;
    let mut inactive = None;
    let mut speculative = None;

    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Mach Virtual Memory Statistics: (page size of ") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            page_size = digits.parse::<u64>().ok();
        } else if let Some(rest) = line.strip_prefix("Pages free:") {
            free = trailing_count(rest);
        } else if let Some(rest) = line.strip_prefix("Pages inactive:") {
            inactive = trailing_count(rest);
        } else if let Some(rest) = line.strip_prefix("Pages speculative:") {
            speculative = trailing_count(rest);
        }
    }

    Some((page_size?, free?, inactive?, speculative?))
}

/// Reduce a reclaimable-memory pool by a proportional margin, floored at a
/// minimum absolute reserve so a small pool isn't left with almost no margin.
#[must_use]
pub(crate) fn apply_live_margin(reclaimable_gb: f32, margin_pct: f32, min_reserve_gib: f32) -> f32 {
    let margin = (reclaimable_gb * margin_pct).max(min_reserve_gib);
    (reclaimable_gb - margin).max(0.0)
}

/// `VOX_MENS_LIVE_MEM_MARGIN_PCT` override, as a fraction in `[0, 1]`. Defaults to 0.15.
fn live_mem_margin_pct() -> f32 {
    std::env::var("VOX_MENS_LIVE_MEM_MARGIN_PCT")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
        .unwrap_or(0.15)
}

/// `VOX_MENS_DISABLE_LIVE_MEM=1` forces the static nameplate-minus-reserve
/// heuristic instead of the live `vm_stat` query.
fn live_mem_disabled() -> bool {
    std::env::var("VOX_MENS_DISABLE_LIVE_MEM")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
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

/// Apple Silicon unified memory sized to what's **actually available right
/// now** (free + inactive + speculative pages, per `vm_stat` — the standard
/// macOS reclaimable-memory definition), not nameplate total minus a flat
/// reserve. Falls back to [`query_apple_unified_memory`] on any shell/parse
/// failure, or when `VOX_MENS_DISABLE_LIVE_MEM=1` is set.
///
/// Leaf function: must not call [`get_system_vram_info`] / [`get_system_vram_gb`]
/// (those Priority-4-recurse into `hardware::probe()` → `probe_metal()`).
#[cfg(target_os = "macos")]
pub fn query_apple_available_memory() -> Option<VramInfo> {
    if live_mem_disabled() {
        return query_apple_unified_memory();
    }

    let vm_out = std::process::Command::new("vm_stat").output().ok();
    let parsed = vm_out
        .filter(|o| o.status.success())
        .and_then(|o| parse_vm_stat(&String::from_utf8_lossy(&o.stdout)));

    let Some((page_size, free, inactive, speculative)) = parsed else {
        return query_apple_unified_memory();
    };

    let reclaimable_bytes = (free + inactive + speculative).saturating_mul(page_size);
    let reclaimable_gb = reclaimable_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
    let available_gb = apply_live_margin(
        reclaimable_gb,
        live_mem_margin_pct(),
        MIN_LIVE_MEM_RESERVE_GIB,
    );

    let physical_gb = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| parse_hw_memsize(&String::from_utf8_lossy(&o.stdout)));

    Some(VramInfo {
        total_gb: available_gb,
        used_gb: physical_gb
            .map(|p| (p - available_gb).max(0.0))
            .unwrap_or(0.0),
        free_gb: available_gb,
    })
}

/// Non-macOS hosts have no unified-memory pool to report.
#[cfg(not(target_os = "macos"))]
pub fn query_apple_available_memory() -> Option<VramInfo> {
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

    // Priority 3: Apple Silicon unified memory, sized to live-available (not
    // nameplate) memory -- falls back internally to the static reserve.
    if let Some(info) = query_apple_available_memory() {
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
    #[serial_test::serial(vox_vram_override_env)]
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

    #[test]
    fn parse_vm_stat_reads_page_size_and_reclaimable_pages() {
        // Real `vm_stat` output captured on a 128 GiB Mac (2026-09-05).
        let sample = "Mach Virtual Memory Statistics: (page size of 16384 bytes)\n\
Pages free:                                    53189.\n\
Pages active:                                3598773.\n\
Pages inactive:                              2935813.\n\
Pages speculative:                            712573.\n\
Pages throttled:                                   0.\n\
Pages wired down:                             413750.\n\
Pages purgeable:                               17793.\n";
        let (page_size, free, inactive, speculative) = parse_vm_stat(sample).expect("parses");
        assert_eq!(page_size, 16384);
        assert_eq!(free, 53189);
        assert_eq!(inactive, 2935813);
        assert_eq!(speculative, 712573);
    }

    #[test]
    fn parse_vm_stat_reads_4k_page_size() {
        let sample = "Mach Virtual Memory Statistics: (page size of 4096 bytes)\n\
Pages free:                               100.\n\
Pages inactive:                           200.\n\
Pages speculative:                         50.\n";
        let (page_size, free, inactive, speculative) = parse_vm_stat(sample).expect("parses");
        assert_eq!(page_size, 4096);
        assert_eq!(free, 100);
        assert_eq!(inactive, 200);
        assert_eq!(speculative, 50);
    }

    #[test]
    fn parse_vm_stat_rejects_missing_fields() {
        assert!(parse_vm_stat("").is_none());
        assert!(
            parse_vm_stat("Mach Virtual Memory Statistics: (page size of 16384 bytes)\n").is_none()
        );
    }

    #[test]
    fn apply_live_margin_uses_proportional_reserve_above_the_floor() {
        // 40 GiB reclaimable, 15% margin -> 6 GiB margin (above the 2 GiB floor) -> 34 GiB.
        assert_eq!(
            apply_live_margin(40.0, 0.15, MIN_LIVE_MEM_RESERVE_GIB),
            34.0
        );
    }

    #[test]
    fn apply_live_margin_floors_the_reserve_on_small_pools() {
        // 5 GiB reclaimable, 15% would be 0.75 GiB -- the 2 GiB floor wins.
        assert_eq!(apply_live_margin(5.0, 0.15, MIN_LIVE_MEM_RESERVE_GIB), 3.0);
    }

    #[test]
    fn apply_live_margin_never_returns_negative() {
        assert_eq!(apply_live_margin(1.0, 0.15, MIN_LIVE_MEM_RESERVE_GIB), 0.0);
    }

    #[test]
    #[serial_test::serial(vox_mens_live_mem_env)]
    fn live_mem_margin_pct_defaults_and_clamps() {
        assert_eq!(live_mem_margin_pct(), 0.15);
    }

    #[test]
    #[serial_test::serial(vox_mens_live_mem_env)]
    #[allow(unsafe_code)]
    fn live_mem_margin_pct_override_and_clamp() {
        unsafe {
            std::env::set_var("VOX_MENS_LIVE_MEM_MARGIN_PCT", "0.25");
        }
        assert_eq!(live_mem_margin_pct(), 0.25);
        unsafe {
            std::env::set_var("VOX_MENS_LIVE_MEM_MARGIN_PCT", "1.5");
        }
        assert_eq!(live_mem_margin_pct(), 0.15);
        unsafe {
            std::env::set_var("VOX_MENS_LIVE_MEM_MARGIN_PCT", "-0.1");
        }
        assert_eq!(live_mem_margin_pct(), 0.15);
        unsafe {
            std::env::remove_var("VOX_MENS_LIVE_MEM_MARGIN_PCT");
        }
    }

    #[test]
    #[serial_test::serial(vox_mens_live_mem_env)]
    #[allow(unsafe_code)]
    fn disable_live_mem_env_falls_back_to_the_static_heuristic() {
        unsafe {
            std::env::set_var("VOX_MENS_DISABLE_LIVE_MEM", "1");
        }
        let live = query_apple_available_memory();
        let static_fallback = query_apple_unified_memory();
        unsafe {
            std::env::remove_var("VOX_MENS_DISABLE_LIVE_MEM");
        }
        #[cfg(target_os = "macos")]
        assert_eq!(live, static_fallback);
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(live, None);
            assert_eq!(static_fallback, None);
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
