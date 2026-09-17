# MENS Mac Training Enablement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make MENS spoke training resolve and run on a 128 GB Apple Silicon Mac without starving the desktop GUI, and make an unknown `base.preset` in the spoke SSOT fail closed instead of silently defaulting.

**Architecture:** The spoke SSOT (`mens/config/domain-profiles.yaml`) → capability-tag → base-model chain already exists and is CI-gated. It is dead on macOS for exactly one reason: `vram_autodetect.rs` only shells out to `nvidia-smi`, so `get_system_vram_gb()` returns `None` and `spoke_base_resolver::resolve_base_model` fails closed. This plan adds Apple unified-memory detection that reports **usable** memory (total minus an OS/GUI reserve), maps that number onto the existing `qwen3_*` preset ladder for Metal, and closes the one validation hole that makes adding a spoke unsafe.

**Tech Stack:** Rust (edition 2024, let-chains), `serde_yaml`, Candle 0.10, `sysctl hw.memsize`, existing `vox ci spoke-check` gate.

**Spec:** [`docs/src/architecture/mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md`](../../src/architecture/mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md) — see **Revision 2**, findings 2/3 (the blocker and the reserve) and §6r (the corrected action list this plan implements).

## Execution log (updated as tasks land)

Tasks 1-4 executed inline this session on branch `mens/mac-hub-enablement`, commits
`efc26f824`, `4a7b6587c`, `2df7021b5`. Two deviations from the plan as written, both
necessary and both recorded in their commit messages:

- **Task 3:** `KNOWN_PRESETS` could not stay in `preset_schema.rs` as the plan assumed
  (`preset_schema.rs:57` in the "Interfaces" note below is now stale) — that module is
  gated behind `mens-train`/`mens-cloud` (pulls in Candle/QLoRA/tokenizers), while
  `spoke_validate` and the `vox ci spoke-check` gate that runs it deliberately stay on
  the light `mens` feature only. Moved `KNOWN_PRESETS` to `spoke_base_resolver.rs`
  (already `mens`-gated); `preset_schema` re-exports it so no external caller's import
  path changed.
- **Task 3 also fixed a pre-existing bug found while verifying it**: three tests in
  `preset_schema.rs` mutate the global `VOX_BASE_MODEL` env var with no synchronization,
  causing a real, reproducible failure under the default parallel test runner
  (confirmed identical on `main` before this branch). Fixed with `#[serial(vox_base_model_env)]`.

Tasks 1-4 all verified green: `cargo test -p vox-populi --lib --features mens` (180 tests),
`--features mens-train` (291 tests), `vox ci spoke-check` (OK), 4x repeated runs to
confirm the race fix holds.

## Global Constraints

- **Formatting:** never run `cargo fmt --all`. Use `cargo fmt -p vox-populi` for a single crate, or `vox run scripts/fmt.vox` for the workspace.
- **Test-first is mandatory** (AGENTS.md §Test-First Policy): every new `pub fn` lands with a failing test written *first*, in the same file. Enforced by the `tdd-guard` pre-commit hook and `vox-code-audit` detector `skeleton/untested-pub-api`.
- **Local CI before push:** `vox ci pre-push --complete` (not the default fast tier — it omits clippy and tests).
- **Commit message style:** imperative subject under 72 chars; body explains *why*. End every commit message with:
  `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`
- **Branch first.** Do not commit to `main`.
- **Tuning knobs are plain env vars**, matching `memory_budget.rs`'s existing idiom. `vox_secrets::resolve_secret` is for *sensitive* values only (AGENTS.md §Secret Management); a memory reserve is not a secret.
- **No new crate dependencies.** Everything here uses `std`, existing workspace deps, and existing modules.

---

### Task 1: Apple unified-memory detection with a GUI reserve

The blocker. `get_system_vram_info()` has three priority tiers (env override → `nvidia-smi` → hardware SSOT); this inserts Apple unified memory as a new tier. The function deliberately reports **usable** memory in `total_gb`, because every downstream consumer (`pick_base`, `auto_preset`, `memory_budget`) treats that number as "the budget available to training." On a discrete card that equals the card's VRAM; on unified memory it must exclude what macOS and the GUI need, or `pick_base` will happily select a rung that swaps the desktop to death.

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/vram_autodetect.rs` (add functions after `query_nvidia_smi_vram`, ~line 41; extend `get_system_vram_info`, lines 44-74)
- Test: `crates/vox-populi/src/mens/tensor/vram_autodetect.rs` (in the existing `#[cfg(test)] mod tests`, ~line 118)

**Interfaces:**
- Consumes: existing `VramInfo { total_gb, used_gb, free_gb }` (lines 5-10).
- Produces: `pub const DEFAULT_UNIFIED_MEM_RESERVE_GIB: f32`, `pub fn usable_unified_memory_gb(total_gb: f32, reserve_gb: f32) -> f32`, `pub fn query_apple_unified_memory() -> Option<VramInfo>`. Task 2 and Task 5 depend on `query_apple_unified_memory` returning `Some` on macOS.

- [x] **Step 1: Write the failing tests**

Add to the existing `mod tests` in `crates/vox-populi/src/mens/tensor/vram_autodetect.rs`:

```rust
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
```

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vox-populi --lib vram_autodetect 2>&1 | tail -20`
Expected: FAIL — `cannot find function 'usable_unified_memory_gb' in this scope` and `cannot find function 'parse_hw_memsize' in this scope`.

- [x] **Step 3: Write the implementation**

Insert into `crates/vox-populi/src/mens/tensor/vram_autodetect.rs`, immediately after `query_nvidia_smi_vram` (after line 41):

```rust
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
```

Then extend `get_system_vram_info` — insert this block between the `nvidia-smi` tier (ending line 60) and the hardware-SSOT tier:

```rust
    // Priority 3: Apple Silicon unified memory, minus the OS/GUI reserve.
    if let Some(info) = query_apple_unified_memory() {
        return Some(info);
    }
```

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vox-populi --lib vram_autodetect 2>&1 | tail -20`
Expected: PASS — all four new tests, plus the pre-existing `auto_preset_*`, `vram_override_env_is_respected` and `test_parse_nvidia_smi_output*` tests still green.

- [x] **Step 5: Verify it reports real memory on this machine**

Run: `cargo run -q -p vox-cli --features mens -- mens probe 2>&1 | head -20`
Expected: a VRAM line showing ~116 GiB on a 128 GiB Mac (not "Could not detect VRAM"). If the build needs a different feature flag, run instead:
`cargo test -p vox-populi --lib vram_autodetect -- --nocapture`
and confirm no test reports a detection failure.

- [x] **Step 6: Commit**

```bash
git add crates/vox-populi/src/mens/tensor/vram_autodetect.rs
git commit -m "feat(mens): detect Apple unified memory with a GUI reserve

Spoke base resolution failed closed on macOS because vram_autodetect only
queried nvidia-smi, so resolve_base_model could not size any capability tag.
Report usable memory (physical minus a fixed OS/GUI reserve) so the existing
fail-closed ladder picks a rung that leaves the desktop alive.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: Metal-aware preset auto-selection

`auto_preset` returns `None` unless `device_is_cuda` is true, and its existing tests assert exactly that ("must not return a preset for a CPU-only host"). Those assertions are correct and must keep passing, so this adds a sibling that handles accelerator kinds explicitly rather than widening the boolean. The Metal thresholds map onto the `qwen3_*` presets that already exist in `KNOWN_PRESETS`; no new preset is invented here.

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/vram_autodetect.rs` (add after `auto_preset`, ~line 97)
- Test: same file, existing `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: existing `auto_preset(device_is_cuda: bool, vram_gb: Option<f32>) -> Option<&'static str>` (line 85), unchanged.
- Produces: `pub enum AcceleratorKind { Cuda, Metal, Cpu }` and `pub fn auto_preset_for(kind: AcceleratorKind, vram_gb: Option<f32>) -> Option<&'static str>`.

- [x] **Step 1: Write the failing tests**

```rust
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
        assert_eq!(auto_preset_for(AcceleratorKind::Metal, Some(20.0)), Some("qwen3_16g"));
        assert_eq!(auto_preset_for(AcceleratorKind::Metal, Some(32.0)), Some("qwen3_24g"));
        assert_eq!(auto_preset_for(AcceleratorKind::Metal, Some(64.0)), Some("qwen3_48g"));
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
```

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vox-populi --lib vram_autodetect 2>&1 | tail -20`
Expected: FAIL — `cannot find type 'AcceleratorKind'` / `cannot find function 'auto_preset_for'`.

- [x] **Step 3: Write the implementation**

Insert after `auto_preset` (after line 97):

```rust
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
```

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vox-populi --lib vram_autodetect 2>&1 | tail -20`
Expected: PASS, including every pre-existing `auto_preset_*` test (they must be untouched).

- [x] **Step 5: Commit**

```bash
git add crates/vox-populi/src/mens/tensor/vram_autodetect.rs
git commit -m "feat(mens): map Metal unified memory onto the qwen3 preset ladder

auto_preset hard-gates on device_is_cuda, so Apple Silicon got no preset at any
memory size. Add an accelerator-kind sibling rather than widening the boolean,
keeping the existing 'CPU host must not get a CUDA preset' assertions intact.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: Fail closed on an unknown `base.preset`

Today `spoke_validate` checks only that `base.preset` is *present*. Its value flows to `preset_schema::base_for_name`, whose `match` ends in a catch-all returning a plausible `rank: 16, seq_len: 512` profile — so `preset: qwen3_16gb` (a typo) trains at the wrong size, silently. `KNOWN_PRESETS` already exists as the registry and is contract-mirrored in `contracts/mens/training-presets.v1.yaml`; this wires the validator to it. This is what makes "add or remove a spoke by editing YAML" safe rather than merely easy.

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/spoke_validate.rs:31-35` (inside the `fine_tune(base.method)` branch)
- Test: same file, existing `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `preset_schema::KNOWN_PRESETS: &[&str]` (`preset_schema.rs:57`); `SpokeViolation(pub String)` (`spoke_validate.rs:9`).
- Produces: no new public items — one additional violation variant in `validate`'s returned `Vec<SpokeViolation>`.

- [x] **Step 1: Write the failing tests**

```rust
    #[test]
    fn flags_unknown_preset() {
        let yaml = r#"
profiles:
  typo:
    description: "x"
    mix_config: mens/config/mix-vox-lang.yaml
    base: { model: strong_code_default, method: qlora, preset: qwen3_16gb }
"#;
        let file: DomainProfilesFile = serde_yaml::from_str(yaml).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let v = validate(&file, root);
        assert!(
            v.iter().any(|x| x.0.contains("not in KNOWN_PRESETS")),
            "a typo'd preset must be rejected, got {v:?}"
        );
    }

    #[test]
    fn accepts_known_preset() {
        let yaml = r#"
profiles:
  ok:
    description: "x"
    mix_config: mens/config/mix-vox-lang.yaml
    base: { model: strong_code_default, method: qlora, preset: qwen3_16g }
"#;
        let file: DomainProfilesFile = serde_yaml::from_str(yaml).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let v = validate(&file, root);
        assert!(
            !v.iter().any(|x| x.0.contains("KNOWN_PRESETS")),
            "a real preset must pass, got {v:?}"
        );
    }

    #[test]
    fn every_shipped_spoke_preset_is_known() {
        // Drift guard: the real domain-profiles.yaml must never reference a
        // preset the trainer would silently default on.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let raw = std::fs::read_to_string(root.join("mens/config/domain-profiles.yaml")).unwrap();
        let file: DomainProfilesFile = serde_yaml::from_str(&raw).unwrap();
        let v = validate(&file, root);
        assert!(v.is_empty(), "shipped spoke SSOT has violations: {v:?}");
    }
```

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vox-populi --lib spoke_validate 2>&1 | tail -20`
Expected: FAIL on `flags_unknown_preset` — no violation is produced for the typo (the other two may already pass; that is fine and expected).

- [x] **Step 3: Write the implementation**

In `crates/vox-populi/src/mens/tensor/spoke_validate.rs`, replace the `base.preset` presence check (lines 31-35) with presence **and** membership:

```rust
            match &base.preset {
                None => v.push(SpokeViolation(format!(
                    "spoke '{name}': fine-tune method requires base.preset"
                ))),
                Some(preset)
                    if !crate::mens::tensor::preset_schema::KNOWN_PRESETS
                        .contains(&preset.as_str()) =>
                {
                    v.push(SpokeViolation(format!(
                        "spoke '{name}': base.preset '{preset}' is not in KNOWN_PRESETS — \
                         add it to preset_schema::KNOWN_PRESETS and \
                         contracts/mens/training-presets.v1.yaml, or fix the typo"
                    )));
                }
                Some(_) => {}
            }
```

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vox-populi --lib spoke_validate 2>&1 | tail -20`
Expected: PASS — all three new tests plus the four pre-existing ones.

- [x] **Step 5: Verify the CI gate agrees**

Run: `cargo run -q -p vox-cli -- ci spoke-check 2>&1 | tail -20`
Expected: exit 0, no violations reported against the shipped `domain-profiles.yaml`.

- [x] **Step 6: Commit**

```bash
git add crates/vox-populi/src/mens/tensor/spoke_validate.rs
git commit -m "fix(mens): reject unknown base.preset in the spoke SSOT

base_for_name ends in a catch-all returning a rank-16 profile, so a typo'd
preset trained at the wrong size with no error. Validate base.preset against
KNOWN_PRESETS so adding a spoke fails loudly instead of silently mis-sizing.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: Add the Mac-tier rung to the base ladder

`train_bases` tops out at `floor_mb: 60000` → Qwen3-32B QLoRA. With ~116 GiB usable, `pick_base` picks that rung and leaves roughly half the machine idle. The ladder already encodes the "more memory ⇒ quantize less" idea (`Qwen3-14B` appears twice: `floor_mb: 20000, methods: [qlora]` and `floor_mb: 44000, methods: [lora]`), so this follows the established pattern rather than inventing one. **The floor is an estimate and must be replaced by a measured number in Step 5** — every other constant in this system was calibrated from a real run, and inventing precision here is how the 4080's 4B OOM happened.

**Files:**
- Modify: `mens/config/gpu-specs.yaml:262-277` (`strong_code_default` and `agentic_default` ladders)
- Test: `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` (existing `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `pick_base(overlay, tag, vram_mb) -> Result<&TrainBase>` (`spoke_base_resolver.rs:17`), `load_overlay(root)` (line 38). Both unchanged.
- Produces: no new code items — a new ladder rung consumed by the existing resolver.

- [x] **Step 1: Write the failing test**

Add to `mod tests` in `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`:

```rust
    #[test]
    fn mac_128g_prefers_unquantized_32b_lora() {
        // 128 GiB physical - 12 GiB GUI reserve = 116 GiB usable = 118_784 MB.
        // At that budget the un-quantized 32B rung must outrank the QLoRA one.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let overlay = load_overlay(root).expect("load overlay");
        let base = pick_base(&overlay, "strong_code_default", 118_784).expect("a base fits");
        assert!(
            base.hf_id.contains("Qwen3-32B"),
            "116 GiB should resolve Qwen3-32B, got: {}",
            base.hf_id
        );
        assert!(
            base.methods.iter().any(|m| m == "lora"),
            "at 116 GiB the un-quantized LoRA rung should win, got methods: {:?}",
            base.methods
        );
    }
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo test -p vox-populi --lib spoke_base_resolver 2>&1 | tail -20`
Expected: FAIL — resolves `Qwen3-32B` but with `methods: ["qlora"]`, because no `lora` rung exists above `floor_mb: 60000`.

- [x] **Step 3: Add the rung**

In `mens/config/gpu-specs.yaml`, append one line to **both** the `strong_code_default` ladder (after line 269) and the `agentic_default` ladder (after line 277):

```yaml
    # Apple Silicon 128 GB tier (usable ~116 GiB after the GUI reserve): 32B
    # un-quantized LoRA. floor_mb is a provisional estimate — 64 GB of BF16
    # weights plus optimizer/activation headroom — and is re-calibrated from a
    # measured peak in the plan's Step 5.
    - { hf_id: "Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137",    floor_mb: 100000, methods: [lora] }
```

- [x] **Step 4: Run the test to verify it passes**

Run: `cargo test -p vox-populi --lib spoke_base_resolver 2>&1 | tail -20`
Expected: PASS — including the pre-existing `qwen3_code_48g_prefers_unquantized_14b`, `strong_code_default_16g_resolves_qwen3_8b`, `agentic_default_16g_resolves_qwen3_8b`, and `small_code_default_caps_at_8b` tests, which must be unaffected.

- [ ] **Step 5: Calibrate `floor_mb` against a measured run**

Corrected (found while running this step): the binary is `vox-ml-cli`, not
`vox-cli`, and its feature is `gpu` (which pulls in `vox-populi/mens-train`
transitively) — `vox-ml-cli` has no `mens-train` feature of its own. Also,
`--skip-train` stops the pipeline *before* the Train stage, which is exactly
the code path being verified — use `--stages train` instead so only that
stage runs, still under `--dry-run` so nothing is downloaded or trained yet:

```bash
cargo run -q -p vox-ml-cli --features gpu -- mens pipeline \
  --profile rust --dry-run --stages train 2>&1 | tail -10
```

Confirmed on this machine: `resolved training selection
model=Some("Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137")
preset=qwen3_16g backend=CandleQlora` — the base model correctly scales to
32B via the Task 4 rung; **the preset does not** (worth knowing, not a bug):
`rust`'s `domain-profiles.yaml` entry pins `preset: qwen3_16g` explicitly, so
`auto_preset_for` (Task 2) is never consulted — a spoke's static preset always
wins over auto-detection. Training a 32B QLoRA run under 16 GB-tuned
batch/seq-len hyperparameters is safe (conservative, not incorrect) but wastes
most of the Mac's headroom; wiring per-machine preset auto-upgrade is a
reasonable follow-up, out of scope here.

Then get a real resident peak, which requires actually starting training briefly — a plan-only path cannot measure it. Launch a short run and watch memory, then stop it:

```bash
cargo run -q -p vox-ml-cli --features gpu -- mens train \
  --model Qwen/Qwen3-32B --preset qwen3_96g --device metal \
  --data-dir target/dogfood --output-dir mens/runs/calib-32b 2>&1 | tail -40
```

While it runs, in a second shell: `sudo footprint -a 2>/dev/null | head -20` or Activity Monitor's Memory tab → the `vox` process's "Real Memory". Once the first few steps complete and the figure plateaus, stop the run (Ctrl-C — the trainer's `ctrlc` handler checkpoints).

If the observed peak differs from 100000 MB by more than ~10%, edit the `floor_mb` you just added to the observed value rounded up to the next 1000 MB, change the comment from "provisional estimate" to `measured 2026-09-XX`, and re-run Step 4's test. **If the observed peak exceeds 118_784 MB, the 32B LoRA rung does not fit this machine — delete the rung rather than forcing it**, and let the existing 60000 MB QLoRA rung remain the top of the ladder. A rung that does not fit is exactly the failure the 4080's 4B OOM taught this codebase to avoid.

- [x] **Step 6: Commit**

```bash
git add mens/config/gpu-specs.yaml crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
git commit -m "feat(mens): add the 128GB Apple Silicon rung to the base ladder

The ladder stopped at 60GB/32B-QLoRA, so a 128GB Mac left half its memory
unused. Add an un-quantized 32B LoRA rung following the existing
'more memory, quantize less' pattern already used for 14B at 44GB.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 5: End-to-end verification on this machine

Tasks 1-4 are unit-tested in isolation. This proves the whole chain — detect memory → pick a preset → resolve a base → validate the SSOT — actually works on the target hardware, which is the thing no unit test can assert.

**Files:**
- Test: no new files; this task runs existing gates and records results.
- Modify (only if Step 3 reveals a gap): `docs/src/reference/mens-training.md` (add an Apple Silicon row to the device/preset guidance).

**Interfaces:**
- Consumes: everything produced by Tasks 1-4.
- Produces: a verified, documented resolution path; no new code items.

- [x] **Step 1: Confirm the full resolution chain on macOS**

```bash
cargo test -p vox-populi --lib -- vram_autodetect spoke_base_resolver spoke_validate 2>&1 | tail -20
```

Expected: PASS, zero failures across all three modules.

- [x] **Step 2: Confirm the SSOT CI gate is green**

```bash
cargo run -q -p vox-cli -- ci spoke-check
```

Expected: exit 0. Confirmed: `spoke-check OK`.

- [x] **Step 3: Confirm a spoke resolves a Mac-sized base end to end**

`--profile` and `--dry-run` are `vox mens pipeline` flags (`action_populi_enum.rs:12-48`); the binary is `vox-ml-cli` under the `gpu` feature (see Task 4 Step 5's correction), and `--stages train` — not `--skip-train` — is what actually exercises the resolution path:

```bash
cargo run -q -p vox-ml-cli --features gpu -- mens pipeline \
  --profile rust --dry-run --stages train 2>&1 | tail -10
```

Confirmed on this machine: `resolved training selection
model=Some("Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137")
preset=qwen3_16g backend=CandleQlora` — no "no GPU VRAM detected" error, and
the base model correctly scales to the Task 4 rung.

Expected: the plan prints a resolved base model and preset without the error
"no GPU VRAM detected; cannot size base tag". Use `--profile rust` rather than
`--profile vox-lang`: `vox-lang` uses `small_code_default`, which deliberately
caps at 8B (see the `small_code_default_caps_at_8b` test), so it would not
exercise the new Mac-tier rung. `rust` uses `strong_code_default` and should
resolve the 32B rung.

- [ ] **Step 4: Run the full local gate**

```bash
vox ci pre-push --complete
```

Expected: green. This tier includes clippy and tests; the default fast tier does not and is not sufficient for a code change. Fix anything it reports before committing.

- [ ] **Step 5: Format**

```bash
cargo fmt -p vox-populi
```

Never `cargo fmt --all` (Global Constraints).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "test(mens): verify Mac base resolution end to end

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Out of scope for this plan

Deliberately separate subsystems, each worth its own plan:

- **VLM text-tower loading** (unlocks Qwen3.8-27B as the hub base) — `vox-hf-layout` currently hard-rejects any checkpoint with `vision_config`. See the spec's §3.2r.
- **The harness/skill-retrieval petal** — replacing the bespoke BM25 skill index with the `vox-search` hybrid stack, and feeding the reliability producer that `chat_tools/mod.rs` reads but nothing writes.
- **Turning on `system-metrics`** so `ScalingService`'s CPU/memory guard stops being inert — a one-feature-flag fix that restores a safety guard the GUI already exposes knobs for. Highest value-per-line item found in the whole audit, and not a training change at all.
