# macOS MENS Training Lane Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `vox mens train` with no flags auto-detect any Mac's real available unified memory and pick a safely-fitting Qwen3 model+preset, without changing behavior on the validated 4080 Super CUDA lane.

**Architecture:** Fix two existing bugs (a hardcoded-zero macOS hardware probe stub, and a hardware-blind CLI preset/model default) by wiring already-correct-but-disconnected code (`vram_autodetect`'s Apple detection, `auto_preset_for`'s Metal ladder) into the real `vox mens train` path, instead of adding a third hardware-detection system. Add a live-available-memory query (macOS `vm_stat` reclaimable pages) to replace the flat nameplate-minus-12GiB heuristic as the primary signal.

**Tech Stack:** Rust (workspace crates `vox-populi`, `vox-ml-cli`), YAML config (`mens/config/gpu-specs.yaml`), macOS `vm_stat`/`sysctl` shell-outs (no new FFI/unsafe).

**Spec:** [docs/superpowers/specs/2026-09-06-macos-mens-training-lane-design.md](../specs/2026-09-06-macos-mens-training-lane-design.md)

## Global Constraints

- Zero behavior change on the CUDA lane: every existing `vram_autodetect`/`preset_schema`/`spoke_base_resolver` test must keep passing unmodified, and every new device-conditional branch must be gated so a CUDA-vendor `DeviceProfile` takes the exact code path it takes today.
- No new `unsafe` code and no new FFI crate dependency — live-memory reads shell out to `vm_stat`/`sysctl`, matching the existing `nvidia-smi`/`sysctl` shell-out idiom in `vram_autodetect.rs`.
- All new env vars are plain (not secrets) via `std::env::var`, matching the existing `VOX_MENS_UNIFIED_MEM_RESERVE_GIB` idiom — do not route them through `vox_secrets`.
- Every new `pub fn` gets a `#[test]` in the same file (repo-wide test-first policy).
- Run `cargo fmt -p vox-populi -p vox-ml-cli` and `cargo clippy -p vox-populi -p vox-ml-cli --all-targets -- -D warnings` before the final commit of each task (Windows-unsafe `cargo fmt --all` must never be run — see root `AGENTS.md` §VoxScript-First Glue Code).

---

### Task 1: Live-available-memory query (`vram_autodetect.rs`)

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/vram_autodetect.rs`

**Interfaces:**
- Produces: `pub fn query_apple_available_memory() -> Option<VramInfo>` (macOS) / `None` stub (other targets) — used by Task 2 and by `get_system_vram_info()`'s Priority 3 in this same task.
- Produces: `pub const MIN_LIVE_MEM_RESERVE_GIB: f32 = 2.0`.

- [ ] **Step 1: Write the failing tests**

Add to the existing `#[cfg(test)] mod tests` block in `vram_autodetect.rs` (after `parse_hw_memsize_rejects_garbage_and_zero`):

```rust
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
    fn parse_vm_stat_rejects_missing_fields() {
        assert!(parse_vm_stat("").is_none());
        assert!(parse_vm_stat("Mach Virtual Memory Statistics: (page size of 16384 bytes)\n").is_none());
    }

    #[test]
    fn apply_live_margin_uses_proportional_reserve_above_the_floor() {
        // 40 GiB reclaimable, 15% margin -> 6 GiB margin (above the 2 GiB floor) -> 34 GiB available.
        assert_eq!(apply_live_margin(40.0, 0.15, MIN_LIVE_MEM_RESERVE_GIB), 34.0);
    }

    #[test]
    fn apply_live_margin_floors_the_reserve_on_small_pools() {
        // 5 GiB reclaimable, 15% margin would be 0.75 GiB -- the 2 GiB floor wins.
        assert_eq!(apply_live_margin(5.0, 0.15, MIN_LIVE_MEM_RESERVE_GIB), 3.0);
    }

    #[test]
    fn apply_live_margin_never_returns_negative() {
        assert_eq!(apply_live_margin(1.0, 0.15, MIN_LIVE_MEM_RESERVE_GIB), 0.0);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vox-populi --features mens vram_autodetect::tests::parse_vm_stat -- --nocapture`
Expected: FAIL with "cannot find function `parse_vm_stat`" (and similarly for `apply_live_margin`) — they don't exist yet.

- [ ] **Step 3: Implement `parse_vm_stat` and `apply_live_margin`**

Insert into `vram_autodetect.rs`, after `parse_hw_memsize` and before the `query_apple_unified_memory` doc comment:

```rust
/// Minimum safety reserve subtracted from live-available memory, in GiB, even
/// when the proportional margin (`VOX_MENS_LIVE_MEM_MARGIN_PCT`) would compute
/// less — protects a nearly-idle small Mac from a near-zero margin.
pub const MIN_LIVE_MEM_RESERVE_GIB: f32 = 2.0;

/// Parse `vm_stat` output into `(page_size_bytes, free_pages, inactive_pages, speculative_pages)`.
/// `None` if any required field is missing or unparseable.
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
fn apply_live_margin(reclaimable_gb: f32, margin_pct: f32, min_reserve_gib: f32) -> f32 {
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
/// heuristic instead of the live `vm_stat` query — used by tests/CI runs that
/// must not depend on the test runner's live memory state.
fn live_mem_disabled() -> bool {
    std::env::var("VOX_MENS_DISABLE_LIVE_MEM")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p vox-populi --features mens vram_autodetect::tests:: -- --nocapture`
Expected: PASS (all 5 new tests, plus existing tests in the module unaffected).

- [ ] **Step 5: Write the failing test for `query_apple_available_memory`**

Add a new test to the same `mod tests` block:

```rust
    #[test]
    #[allow(unsafe_code)]
    fn disable_live_mem_env_falls_back_to_the_static_heuristic() {
        // With live memory disabled, query_apple_available_memory must return
        // exactly what query_apple_unified_memory returns (the static path) --
        // proving the fallback wiring, without depending on this runner's
        // actual live memory state.
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
```

- [ ] **Step 6: Run test to verify it fails**

Run: `cargo test -p vox-populi --features mens disable_live_mem_env_falls_back -- --nocapture`
Expected: FAIL with "cannot find function `query_apple_available_memory`".

- [ ] **Step 7: Implement `query_apple_available_memory` and wire it into `get_system_vram_info`**

Insert after `query_apple_unified_memory`'s macOS/non-macOS pair, before `get_system_vram_info`:

```rust
/// Apple Silicon unified memory sized to what's **actually available right
/// now** (free + inactive + speculative pages, per `vm_stat` — the standard
/// macOS reclaimable-memory definition), not nameplate total minus a flat
/// reserve. Falls back to [`query_apple_unified_memory`] on any shell/parse
/// failure, or when `VOX_MENS_DISABLE_LIVE_MEM=1` is set.
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
    let available_gb = apply_live_margin(reclaimable_gb, live_mem_margin_pct(), MIN_LIVE_MEM_RESERVE_GIB);

    let physical_gb = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| parse_hw_memsize(&String::from_utf8_lossy(&o.stdout)));

    Some(VramInfo {
        total_gb: available_gb,
        used_gb: physical_gb.map(|p| (p - available_gb).max(0.0)).unwrap_or(0.0),
        free_gb: available_gb,
    })
}

/// Non-macOS hosts have no unified-memory pool to report.
#[cfg(not(target_os = "macos"))]
pub fn query_apple_available_memory() -> Option<VramInfo> {
    None
}
```

Then change `get_system_vram_info`'s Priority 3 block from:

```rust
    // Priority 3: Apple Silicon unified memory, minus the OS/GUI reserve.
    if let Some(info) = query_apple_unified_memory() {
        return Some(info);
    }
```

to:

```rust
    // Priority 3: Apple Silicon unified memory, sized to live-available (not
    // nameplate) memory -- falls back internally to the static reserve.
    if let Some(info) = query_apple_available_memory() {
        return Some(info);
    }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p vox-populi --features mens vram_autodetect:: -- --nocapture`
Expected: PASS for every test in the module, including the pre-existing ones (they must be unaffected since `query_apple_unified_memory` itself is untouched).

- [ ] **Step 9: Commit**

```bash
git add crates/vox-populi/src/mens/tensor/vram_autodetect.rs
git commit -m "$(cat <<'EOF'
feat(mens): size macOS training to live-available memory, not nameplate

query_apple_available_memory() reads vm_stat's reclaimable pages
(free + inactive + speculative) instead of nameplate total minus a
flat 12 GiB reserve -- a machine with other apps already using RAM no
longer gets sized as if all of it were free. Falls back to the
existing static heuristic on any shell/parse failure or via
VOX_MENS_DISABLE_LIVE_MEM=1 (tests use this to stay deterministic).

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Fix the `MacosMetalProbe` zero-VRAM stub

**Files:**
- Modify: `crates/vox-populi/src/mens/hardware/macos_metal.rs`

**Interfaces:**
- Consumes: `vram_autodetect::query_apple_available_memory() -> Option<VramInfo>` (Task 1). **Must not** call `get_system_vram_gb()`/`get_system_vram_info()` — those recurse back into `hardware::probe()` (which runs `MacosMetalProbe`) via their own Priority-4 SSOT fallback, which would deadlock/stack-overflow at runtime.
- Produces: `probe_metal()` returning a real, non-zero `vram_mb` on macOS.

- [ ] **Step 1: Write the failing test**

Add a `#[cfg(test)] mod tests` block at the end of `macos_metal.rs` (the file currently has none):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn probe_metal_reports_nonzero_vram_on_macos() {
        // Regression guard for the historical bug: probe_metal() hardcoded
        // vram_mb: 0 unconditionally, so every hardware-registry-driven
        // decision treated every Mac as having no usable memory at all.
        let summary = probe_metal().expect("macOS always returns Some");
        assert!(
            summary.vram_mb > 0,
            "probe_metal() must report real unified memory, not the old hardcoded 0"
        );
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn probe_metal_is_none_off_macos() {
        assert!(probe_metal().is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-populi --features mens macos_metal::tests:: -- --nocapture`
Expected (on macOS): FAIL — `summary.vram_mb > 0` is false (currently hardcoded to `0`).

- [ ] **Step 3: Implement the fix**

Replace the `#[cfg(target_os = "macos")] pub fn probe_metal()` body:

```rust
#[cfg(target_os = "macos")]
pub fn probe_metal() -> Option<HardwareSummary> {
    use crate::mens::hardware::types::{ComputeBackend, GpuVendor};
    let vram_mb = crate::mens::tensor::vram_autodetect::query_apple_available_memory()
        .map(|info| (info.total_gb * 1024.0) as u64)
        .unwrap_or(0);
    Some(HardwareSummary {
        model_name: "Apple Silicon GPU".into(),
        vram_mb,
        gpu_count: 1,
        vendor: GpuVendor::Apple,
        backend: ComputeBackend::Metal,
        driver_version: None,
        pci_bus_id: None,
        probe_failures: None,
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p vox-populi --features mens macos_metal:: -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-populi/src/mens/hardware/macos_metal.rs
git commit -m "$(cat <<'EOF'
fix(mens): wire the real Apple unified-memory probe into the hardware registry

MacosMetalProbe::probe_metal() hardcoded vram_mb: 0 -- a stub that was
never finished. Every hardware-registry-driven decision (DeviceProfile,
resolve_effective_profile) treated every Mac as having zero usable
memory. Wires in the already-correct, already-tested
query_apple_available_memory() instead of inventing a new probe.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Device-conditional preset default (`preset_schema.rs` + `vram_autodetect.rs`)

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/vram_autodetect.rs` (add `AcceleratorKind::from_vendor`)
- Modify: `crates/vox-populi/src/mens/tensor/preset_schema.rs` (`DeviceProfile` gains `vendor`; `resolve_effective_profile`'s default-when-omitted becomes device-conditional)
- Modify: `crates/vox-ml-cli/src/commands/schola/train/run_train.rs` (update the one `DeviceProfile::from_gpu_info` call site)
- Check for other call sites: `grep -rn "DeviceProfile::from_gpu_info\|DeviceProfile {" crates/` and update every match — this is a breaking signature/struct-literal change.

**Interfaces:**
- Consumes: `VramInfo`/`AcceleratorKind` from Task 1/existing code.
- Produces: `AcceleratorKind::from_vendor(vendor: &str) -> AcceleratorKind`; `DeviceProfile { model_name: String, vram_mb: u64, vendor: String }`; `DeviceProfile::from_gpu_info(model_name: &str, vram_mb: u64, vendor: &str) -> Self`.

- [ ] **Step 1: Write the failing test for `AcceleratorKind::from_vendor`**

Add to `vram_autodetect.rs`'s `mod tests`:

```rust
    #[test]
    fn accelerator_kind_from_vendor_maps_known_vendors() {
        assert_eq!(AcceleratorKind::from_vendor("nvidia"), AcceleratorKind::Cuda);
        assert_eq!(AcceleratorKind::from_vendor("NVIDIA"), AcceleratorKind::Cuda);
        assert_eq!(AcceleratorKind::from_vendor("apple"), AcceleratorKind::Metal);
        assert_eq!(AcceleratorKind::from_vendor("Apple"), AcceleratorKind::Metal);
        assert_eq!(AcceleratorKind::from_vendor("amd"), AcceleratorKind::Cpu);
        assert_eq!(AcceleratorKind::from_vendor("unknown"), AcceleratorKind::Cpu);
        assert_eq!(AcceleratorKind::from_vendor(""), AcceleratorKind::Cpu);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-populi --features mens accelerator_kind_from_vendor -- --nocapture`
Expected: FAIL — `from_vendor` doesn't exist.

- [ ] **Step 3: Implement `AcceleratorKind::from_vendor`**

In `vram_autodetect.rs`, right after the `AcceleratorKind` enum definition, add:

```rust
impl AcceleratorKind {
    /// Map a `GpuInfo`/`DeviceProfile` vendor string (as produced by
    /// `crate::mens::tensor::device::probe_gpu`, e.g. `"nvidia"`, `"apple"`,
    /// `"amd"`, `"unknown"`) to the accelerator kind `auto_preset_for` expects.
    /// Anything not recognized as CUDA or Apple maps to `Cpu` (fail-safe: no
    /// GPU-specific preset is ever selected for an unrecognized vendor).
    #[must_use]
    pub fn from_vendor(vendor: &str) -> Self {
        match vendor.to_ascii_lowercase().as_str() {
            "nvidia" => Self::Cuda,
            "apple" => Self::Metal,
            _ => Self::Cpu,
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-populi --features mens accelerator_kind_from_vendor -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Write the failing tests for the device-conditional default**

Add to `preset_schema.rs`'s test module (find it via `grep -n "mod.*tests" crates/vox-populi/src/mens/tensor/preset_schema.rs` — add alongside the existing `resolve_effective_profile` tests):

```rust
    #[test]
    fn cuda_device_with_no_preset_still_uses_default_preset_4080() {
        // Zero behavior change on the CUDA lane: a CUDA DeviceProfile with no
        // --preset must resolve through DEFAULT_PRESET ("4080") exactly as
        // before this change, never through "auto".
        let device = DeviceProfile {
            model_name: "NVIDIA GeForce RTX 4080 Super".into(),
            vram_mb: 16384,
            vendor: "nvidia".into(),
        };
        let via_none = resolve_effective_profile(None, device.clone(), None, CliOverrides::default());
        let via_explicit_4080 =
            resolve_effective_profile(Some("4080"), device, None, CliOverrides::default());
        assert_eq!(via_none.rank, via_explicit_4080.rank);
        assert_eq!(via_none.alpha, via_explicit_4080.alpha);
        assert_eq!(via_none.seq_len, via_explicit_4080.seq_len);
        assert_eq!(via_none.batch_size, via_explicit_4080.batch_size);
        assert_eq!(via_none.grad_accum, via_explicit_4080.grad_accum);
    }

    #[test]
    fn metal_device_with_no_preset_uses_auto_not_4080() {
        // The bug this design fixes: an Apple-vendor DeviceProfile with no
        // --preset must NOT resolve through the CUDA-tuned "4080" default.
        let device = DeviceProfile {
            model_name: "Apple Silicon GPU".into(),
            vram_mb: 116 * 1024, // 116 GiB usable, matches the existing 128 GB Mac test fixtures
            vendor: "apple".into(),
        };
        let via_none = resolve_effective_profile(None, device.clone(), None, CliOverrides::default());
        let via_explicit_4080 =
            resolve_effective_profile(Some("4080"), device, None, CliOverrides::default());
        assert_ne!(
            (via_none.rank, via_none.seq_len, via_none.batch_size),
            (via_explicit_4080.rank, via_explicit_4080.seq_len, via_explicit_4080.batch_size),
            "a Mac with no --preset must not silently get CUDA-16GB-tuned hyperparameters"
        );
    }
```

- [ ] **Step 6: Run tests to verify they fail**

Run: `cargo test -p vox-populi --features mens-train cuda_device_with_no_preset_still_uses_default_preset_4080 metal_device_with_no_preset_uses_auto_not_4080 -- --nocapture`
Expected: FAIL to compile — `DeviceProfile` has no `vendor` field yet.

- [ ] **Step 7: Add `vendor` to `DeviceProfile` and make the default device-conditional**

In `preset_schema.rs`, change:

```rust
#[derive(Debug, Clone)]
pub struct DeviceProfile {
    pub model_name: String,
    pub vram_mb: u64,
}

impl DeviceProfile {
    pub fn from_gpu_info(model_name: &str, vram_mb: u64) -> Self {
        Self {
            model_name: model_name.to_string(),
            vram_mb,
        }
    }
}
```

to:

```rust
#[derive(Debug, Clone)]
pub struct DeviceProfile {
    pub model_name: String,
    pub vram_mb: u64,
    /// Coarse vendor bucket from `GpuInfo::vendor` (`"nvidia"`, `"apple"`,
    /// `"amd"`, `"unknown"`, ...) -- used to keep the CUDA lane's defaults
    /// untouched while giving non-CUDA devices hardware-aware defaults.
    pub vendor: String,
}

impl DeviceProfile {
    pub fn from_gpu_info(model_name: &str, vram_mb: u64, vendor: &str) -> Self {
        Self {
            model_name: model_name.to_string(),
            vram_mb,
            vendor: vendor.to_string(),
        }
    }
}
```

Then change the default-when-omitted line in `resolve_effective_profile` from:

```rust
    let name = normalize_preset_name(preset.or(env_p).unwrap_or(DEFAULT_PRESET));
```

to:

```rust
    // CUDA keeps today's DEFAULT_PRESET exactly as-is (zero behavior change on
    // the validated 4080 Super lane). Every other accelerator kind -- Metal,
    // CPU, unknown -- falls back to "auto" instead of a CUDA-tuned preset.
    let kind = crate::mens::tensor::vram_autodetect::AcceleratorKind::from_vendor(&device.vendor);
    let default_for_device = if kind == crate::mens::tensor::vram_autodetect::AcceleratorKind::Cuda {
        DEFAULT_PRESET
    } else {
        "auto"
    };
    let name = normalize_preset_name(preset.or(env_p).unwrap_or(default_for_device));
```

Then add the fallback helper used by the (unmodified) `"auto"` branch below it — find the existing `} else { base_for_name("4080_safe") }` arms inside the `if name == "auto"` block (both the `load_gpu_specs()` failure arm and the `best_for_vram` no-match arm) and replace **both** occurrences of `base_for_name("4080_safe")` in that block with `auto_fallback_profile(&device)`, then add the helper function near `base_for_name`:

```rust
/// Fallback used when the CUDA-family `presets:` table (gpu-specs.yaml) has
/// no match for this device -- e.g. any non-CUDA accelerator. Consults the
/// Metal-aware `qwen3_*` ladder instead of the hardcoded CUDA "4080_safe"
/// profile, so an Apple Silicon device gets Apple-shaped defaults.
fn auto_fallback_profile(device: &DeviceProfile) -> TrainPresetProfile {
    let kind = crate::mens::tensor::vram_autodetect::AcceleratorKind::from_vendor(&device.vendor);
    let vram_gb = (device.vram_mb as f32) / 1024.0;
    match crate::mens::tensor::vram_autodetect::auto_preset_for(kind, Some(vram_gb)) {
        Some(name) => base_for_name(name),
        None => base_for_name("4080_safe"),
    }
}
```

- [ ] **Step 8: Fix every other call site broken by the signature/struct change**

Run: `grep -rn "DeviceProfile::from_gpu_info\|DeviceProfile {" crates/ --include=*.rs`

For each match outside `preset_schema.rs` itself (expect at least `crates/vox-ml-cli/src/commands/schola/train/run_train.rs:257-258`), update the call:

```rust
    let gpu_info = vox_populi::mens::probe_gpu();
    let device_profile = vox_populi::mens::DeviceProfile::from_gpu_info(
        &gpu_info.model_name,
        gpu_info.vram_mb,
        &gpu_info.vendor,
    );
```

For any test fixture constructing `DeviceProfile { ... }` directly (struct-literal, not `from_gpu_info`), add `vendor: "nvidia".into()` (or the vendor the test's scenario implies) so existing tests keep asserting the CUDA behavior they were written for.

- [ ] **Step 9: Run tests to verify everything passes**

Run: `cargo build -p vox-populi -p vox-ml-cli --features mens-train` (confirms no other broken call site was missed)
Run: `cargo test -p vox-populi --features mens-train tensor::preset_schema:: tensor::vram_autodetect:: -- --nocapture`
Expected: PASS, including every pre-existing boundary test (6/10/16/24 GiB CUDA boundaries, `qwen3_16g`/`24g`/`48g`/`96g` Metal tiers) unmodified.

- [ ] **Step 10: Commit**

```bash
git add crates/vox-populi/src/mens/tensor/vram_autodetect.rs crates/vox-populi/src/mens/tensor/preset_schema.rs crates/vox-ml-cli/src/commands/schola/train/run_train.rs
git commit -m "$(cat <<'EOF'
fix(mens): make the CLI preset default hardware-aware, not CUDA-only

resolve_effective_profile's default-when-omitted was DEFAULT_PRESET
("4080") on every platform, so a bare `vox mens train` on a Mac
silently trained with CUDA-16GB-tuned hyperparameters. Now the default
is chosen by detected accelerator kind: CUDA devices keep
DEFAULT_PRESET exactly as before (regression-tested); every other kind
(Metal, CPU, unknown) uses "auto", which now falls back to the
Metal-aware qwen3_* ladder instead of a hardcoded CUDA "4080_safe"
profile when the GPU-family presets table has no match.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Auto-resolve the base model when `--model` is omitted (Metal only)

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`
- Test: add a unit test near the existing tests for this module, or a new `#[cfg(test)] mod tests` at the bottom of the file if none exists (`grep -n "mod tests" crates/vox-ml-cli/src/commands/schola/train/run_train.rs` to check first).

**Interfaces:**
- Consumes: `vox_populi::mens::tensor::spoke_base_resolver::{load_overlay, pick_base}` (existing), `vox_populi::mens::tensor::vram_autodetect::AcceleratorKind` (Task 3).
- Produces: when `model` is `None` and the detected accelerator is `Metal`, `VOX_BASE_MODEL` is set to the `agentic_default`-tag resolution for the live-detected VRAM before training dispatch, instead of being left unset (which falls through to the flat `DEFAULT_MODEL_ID` = Qwen3-8B regardless of hardware).

- [ ] **Step 1: Write the failing test**

`run_train.rs` doesn't have a test module today (confirm with the grep above). Extract the resolution logic into a small, directly-testable pure function first — add near the top of the file, after the `use` statements:

```rust
/// When `--model` is omitted on a Metal (Apple Silicon) device, resolve a
/// concrete base model id from the live-detected VRAM via the
/// `agentic_default` capability tag, instead of leaving the CUDA-shaped flat
/// default (`DEFAULT_MODEL_ID`, tuned for a 16 GB card) to apply regardless
/// of hardware. Returns `None` when nothing should override the existing
/// flat-default behavior (any explicit `--model`, or any non-Metal device).
fn maybe_auto_resolve_model(
    model: &Option<String>,
    accelerator: vox_populi::mens::tensor::vram_autodetect::AcceleratorKind,
    vram_mb: u64,
    workspace_root: &std::path::Path,
) -> Option<String> {
    if model.is_some() {
        return None;
    }
    if accelerator != vox_populi::mens::tensor::vram_autodetect::AcceleratorKind::Metal {
        return None;
    }
    let overlay = vox_populi::mens::tensor::spoke_base_resolver::load_overlay(workspace_root).ok()?;
    let base = vox_populi::mens::tensor::spoke_base_resolver::pick_base(
        &overlay,
        "agentic_default",
        vram_mb as u32,
    )
    .ok()?;
    Some(base.hf_id.clone())
}

#[cfg(test)]
mod auto_model_tests {
    use super::*;
    use vox_populi::mens::tensor::vram_autodetect::AcceleratorKind;

    fn workspace_root() -> std::path::PathBuf {
        vox_corpus::training::contract::find_workspace_root().expect("workspace root")
    }

    #[test]
    fn explicit_model_is_never_overridden() {
        let explicit = Some("org/My-Model".to_string());
        assert_eq!(
            maybe_auto_resolve_model(&explicit, AcceleratorKind::Metal, 116_000, &workspace_root()),
            None
        );
    }

    #[test]
    fn cuda_device_is_never_auto_resolved() {
        // Zero behavior change on the CUDA lane: even with model=None, a CUDA
        // device must keep falling through to the existing flat default.
        assert_eq!(
            maybe_auto_resolve_model(&None, AcceleratorKind::Cuda, 16_384, &workspace_root()),
            None
        );
    }

    #[test]
    fn metal_device_with_no_model_resolves_from_live_vram() {
        let resolved = maybe_auto_resolve_model(&None, AcceleratorKind::Metal, 116_000, &workspace_root())
            .expect("a base fits 116 GB");
        assert!(
            resolved.contains("Qwen3"),
            "expected a Qwen3 id, got: {resolved}"
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vox-ml-cli --features gpu maybe_auto_resolve_model -- --nocapture`
Expected: FAIL to compile — `maybe_auto_resolve_model` doesn't exist yet (it does, from Step 1 above, but run this before wiring it into `run_train` proper to confirm the pure function alone compiles and its tests pass first; if Step 1's code already makes them pass, proceed directly to Step 3's wiring).

- [ ] **Step 3: Wire it into `run_train`'s existing model/env-var block**

Find this existing block (around line 230):

```rust
    unsafe {
        if let Some(ref m) = model {
            std::env::set_var("VOX_BASE_MODEL", m);
        }
```

Change it to:

```rust
    let auto_resolved_model = {
        let gpu_info = vox_populi::mens::probe_gpu();
        let kind = vox_populi::mens::tensor::vram_autodetect::AcceleratorKind::from_vendor(&gpu_info.vendor);
        workspace_root
            .as_deref()
            .and_then(|root| maybe_auto_resolve_model(&model, kind, gpu_info.vram_mb, root))
    };
    unsafe {
        if let Some(ref m) = model {
            std::env::set_var("VOX_BASE_MODEL", m);
        } else if let Some(ref m) = auto_resolved_model {
            std::env::set_var("VOX_BASE_MODEL", m);
        }
```

Note: `workspace_root` is computed a few lines below this block today (`let workspace_root = vox_corpus::training::contract::find_workspace_root();`) — move that one line above this block so it's available here (it has no dependency on anything computed after it).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p vox-ml-cli --features gpu auto_model_tests:: -- --nocapture`
Expected: PASS.
Run: `cargo build -p vox-ml-cli --features gpu` to confirm the `workspace_root` reorder didn't break anything using it before its new position.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-ml-cli/src/commands/schola/train/run_train.rs
git commit -m "$(cat <<'EOF'
feat(mens): auto-resolve the base model from live VRAM on Metal when --model is omitted

Previously, omitting --model left VOX_BASE_MODEL unset, which falls
through to the flat DEFAULT_MODEL_ID (Qwen3-8B, tuned for a 16 GB
card) regardless of hardware -- the same class of bug as the preset
default. Gated strictly to AcceleratorKind::Metal so CUDA/unknown
devices keep today's exact fallback behavior.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Catalogue gap-coverage tests, CUDA regression proof, and doc update

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` (new tests, no production code changes)
- Modify: `docs/src/architecture/mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md` (Revision 7 entry)

**Interfaces:**
- Consumes: `pick_base`, `load_overlay` (existing, unmodified this task).
- Produces: no new production code — this task is pure verification + documentation.

- [ ] **Step 1: Write the catalogue gap-coverage tests**

Add to `spoke_base_resolver.rs`'s `mod tests` (after `mac_128g_prefers_unquantized_32b_lora`):

```rust
    /// Live-available MB for a given physical-GB Mac tier, using this design's
    /// default margin (15%, floored at 2 GiB) -- mirrors
    /// `vram_autodetect::apply_live_margin` without depending on it directly
    /// (that function is private), so this test is an independent check that
    /// every real 2026 Apple Silicon SKU tier resolves to a safe, non-gapped
    /// rung under the new live-available formula.
    fn live_available_mb_for_physical_gb(physical_gb: f32) -> u32 {
        let margin = (physical_gb * 0.15_f32).max(2.0);
        ((physical_gb - margin).max(0.0) * 1024.0) as u32
    }

    #[test]
    fn every_real_apple_silicon_tier_resolves_without_a_gap() {
        // Real 2026 Apple Silicon SKU memory configurations (M3/M4/M5 lineup:
        // base/Pro/Max/Ultra). Each must resolve to a rung, and the rung must
        // never regress by more than one step versus the next tier down --
        // i.e. no tier is silently stuck at the smallest rung while a much
        // larger tier resolves the same thing for no reason.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let overlay = load_overlay(root).expect("load overlay");
        let tiers_gb: &[f32] = &[8.0, 16.0, 24.0, 32.0, 36.0, 48.0, 64.0, 96.0, 128.0, 192.0, 256.0, 512.0];

        let mut previous_floor = 0u32;
        for &gb in tiers_gb {
            let vram_mb = live_available_mb_for_physical_gb(gb);
            let base = pick_base(&overlay, "agentic_default", vram_mb);
            assert!(
                base.is_ok() || gb < 16.0,
                "tier {gb} GB (live-available {vram_mb} MB) must resolve a base, got: {base:?}"
            );
            if let Ok(base) = base {
                assert!(
                    base.floor_mb >= previous_floor,
                    "tier {gb} GB resolved a smaller-floor rung ({}) than a lower tier already got ({}) -- catalogue regressed",
                    base.floor_mb,
                    previous_floor
                );
                previous_floor = base.floor_mb;
            }
        }
        // The top tiers (192/256/512 GB) all resolve the existing top rung --
        // there is no larger model in the ladder to reach for (out of scope:
        // adding a bigger base model is the hub-model debate, not this design).
        let top = pick_base(&overlay, "agentic_default", live_available_mb_for_physical_gb(512.0))
            .expect("512 GB resolves a base");
        assert!(top.hf_id.contains("Qwen3-32B"), "expected the 32B top rung, got: {}", top.hf_id);
    }

    #[test]
    fn tier_64gb_is_conservative_not_a_bug() {
        // 64 GB physical -> ~53.4 GB live-available at the default margin,
        // which is below the 32B-QLoRA rung's 60 GB floor -- so 64 GB
        // resolves the 14B-LoRA rung (44 GB floor), not 32B. This is
        // deliberate conservatism (the safety-margin goal), not a gap.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let overlay = load_overlay(root).expect("load overlay");
        let vram_mb = live_available_mb_for_physical_gb(64.0);
        let base = pick_base(&overlay, "agentic_default", vram_mb).expect("a base fits at 64GB");
        assert!(
            base.hf_id.contains("Qwen3-14B") && base.methods.iter().any(|m| m == "lora"),
            "64 GB should conservatively resolve 14B-LoRA, got: {} ({:?})",
            base.hf_id,
            base.methods
        );
    }
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p vox-populi --features mens spoke_base_resolver::tests:: -- --nocapture`
Expected: PASS. If `every_real_apple_silicon_tier_resolves_without_a_gap` fails at the 8 GB or 16 GB tier, that confirms a real catalogue gap the spec didn't anticipate — stop and re-open the design conversation rather than loosening the assertion (do not weaken this test to make it pass; a failure here is exactly the class of pre-training crash this design exists to prevent).

- [ ] **Step 3: Full regression sweep**

Run: `cargo test -p vox-populi -p vox-ml-cli --features mens-train,gpu -- --nocapture`
Expected: PASS, full workspace-relevant test suite green, including every test touched or added across Tasks 1-5.

Run: `cargo clippy -p vox-populi -p vox-ml-cli --all-targets --features mens-train,gpu -- -D warnings`
Expected: clean.

- [ ] **Step 4: Record the revision in the architecture decision doc**

Read `docs/src/architecture/mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md` (it already has Revisions 1-6 from this session's earlier work) and append a new `## Revision 7 — ...` section (matching the existing revisions' format) summarizing:
- The three bugs found (zero-VRAM `MacosMetalProbe` stub, hardware-blind `DEFAULT_PRESET`, hardware-blind model default) and their fixes.
- The live-available-memory mechanism and why nameplate-only sizing was rejected (cite the `vm_stat` finding: ~830 MB "free" on a 128 GB idle machine).
- The catalogue gap-coverage test result across all 12 real Apple Silicon SKU tiers.
- A link to the spec: `docs/superpowers/specs/2026-09-06-macos-mens-training-lane-design.md`.
- Confirmation that every pre-existing CUDA/4080-Super test passed unmodified (cite the specific regression tests from Task 3).

- [ ] **Step 5: Commit**

```bash
git add crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs docs/src/architecture/mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md
git commit -m "$(cat <<'EOF'
test(mens): prove no catalogue gap across every real Apple Silicon tier

Adds a table-driven test walking all 12 real 2026 Apple Silicon SKU
memory tiers (8-512 GB) through the live-available formula and
pick_base, asserting monotonic non-regression and that the 64 GB
tier's conservative 14B-LoRA pick is deliberate, not a gap. Records
Revision 7 of the mac-hub decision doc with the full bug trail.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review Notes (completed during plan authoring)

- **Spec coverage:** §1 -> Task 2; §2 -> Task 1; §3 -> Task 5; §4 -> Tasks 3-4; §5 (CUDA lane untouched) -> regression tests in Tasks 3-4 plus the full sweep in Task 5 Step 3. Every spec section has a task.
- **Recursion hazard caught and documented:** Task 2 explicitly calls out why `probe_metal()` must call `query_apple_available_memory()` directly rather than `get_system_vram_gb()`/`get_system_vram_info()`, which would recurse back through `hardware::probe()`.
- **Breaking-change call sites:** Task 3 Step 8 requires grepping for every `DeviceProfile::from_gpu_info`/struct-literal call site, since the signature and struct both change — this is the one change in the plan that can silently fail to compile elsewhere if skipped.
- **Type/signature consistency check:** `AcceleratorKind::from_vendor` (Task 3) is used identically in `preset_schema.rs`'s `auto_fallback_profile` and in Task 4's `maybe_auto_resolve_model` — same signature, same import path, verified against each other.
