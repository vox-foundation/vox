# macOS MENS Training Lane Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A bare `vox mens train` on Apple Silicon selects a Qwen3 model + `qwen3_*` preset that fit live-available unified memory. The 4080 Super CUDA lane stays byte-for-byte identical. Selection is the deliverable; Metal-plugin training remains a separate tracked project.

**Architecture:** Fix the zero-VRAM `MacosMetalProbe` stub by calling the leaf Apple-memory query (never `get_system_vram_info()`, which Priority-4-recurses into `hardware::probe()`). Make the omitted-`--preset` default **Metal-only** `"auto"`, and make `"auto"` on Metal skip the CUDA yaml `presets:` table and call `auto_preset_for(Metal, …)` → `base_for_name`. Resolve `--model` **before** the CandleQlora flat `DEFAULT_MODEL_ID` fill, fail-closed, in `vox-populi` not `run_train.rs`.

**Tech Stack:** Rust (`vox-populi`, `vox-ml-cli`), `mens/config/gpu-specs.yaml`, macOS `vm_stat`/`sysctl` shell-outs. Env knobs registered in `contracts/config/env-vars.v1.yaml` + `registry.v1.yaml`.

**Spec:** [docs/superpowers/specs/2026-09-06-macos-mens-training-lane-design.md](../specs/2026-09-06-macos-mens-training-lane-design.md)

## Locked design corrections

1. **`"auto"` on Metal skips yaml `presets:`.** Call `auto_preset_for(AcceleratorKind::Metal, Some(vram_gb))` then `base_for_name`. If that returns `None`, fail closed with a clear error (or `qwen3_dev_cpu` only when VRAM is in the documented smoke band 6–16 GiB — match `auto_preset_for`'s existing arms). Never `best_for_vram` for Apple.
2. **Omitted-preset default is Metal-only `"auto"`.** CUDA keeps `DEFAULT_PRESET` (`"4080"`). CPU/unknown keep `"4080"` (Linux CI unchanged).
3. **Model auto-resolve runs before** the CandleQlora `DEFAULT_MODEL_ID` fill in `run_train.rs`, Metal-only, **fail-closed**. Helper lives in `spoke_base_resolver`. Precedence: `--model` > `VOX_MENS_DEFAULT_MODEL` > Metal auto-resolve > `DEFAULT_MODEL_ID`.
4. **8 GB on `agentic_default` is a documented fail-closed gap.** Do not add a 0.6B rung. Tests assert `Err` below 11000 MB.
5. **Catalogue tests inject live-available MB**, not physical-GB×0.85. Expected rungs (tag `agentic_default`): 6144 → Err; 13926 → Qwen3-8B QLoRA; 20890 / 27853 / 31334 / 41779 → Qwen3-14B QLoRA; 55706 → Qwen3-14B LoRA; 83558 → Qwen3-32B QLoRA; 111411+ → Qwen3-32B LoRA.
6. **`probe_metal` calls the leaf query only.** Prefer setting `probe_failures` when the query returns `None`.
7. **No new `unsafe`/FFI.** `vm_stat` English prefixes only. Omit purgeable pages; document the omission.
8. **Do not edit `gpu.rs` CUDA auto-inject.**
9. **No `Co-Authored-By` trailers.**
10. **Keep hardware→tensor leaf call** (YAGNI vs a new `mens/platform` module).

## Global Constraints

- Zero CUDA behavior change: every existing `vram_autodetect` / `preset_schema` / `spoke_base_resolver` test stays unmodified in intent; new branches gated on Metal vendor.
- New `VOX_*` vars registered in both env contracts before merge. Tunables use `std::env::var`, not `vox_secrets`.
- Every new `pub fn` gets a same-file `#[test]`. Env-mutating tests use `#[serial]` from `serial_test` (already a `vox-populi` dev-dep).
- Format with `cargo fmt -p vox-populi -p vox-ml-cli` (never `cargo fmt --all`). Clippy: `cargo clippy -p vox-populi --all-targets --features mens-train -- -D warnings` and `cargo clippy -p vox-ml-cli --all-targets --features gpu -- -D warnings`.
- Feature matrix: `vram_autodetect` / `macos_metal` / `spoke_base_resolver` → `--features mens`; `preset_schema` → `--features mens-train`; `run_train` tests → `-p vox-ml-cli --features gpu`.

---

### Task 0: Rewrite spec + plan docs to match the code audit

**Files:**
- Modify: `docs/superpowers/specs/2026-09-06-macos-mens-training-lane-design.md`
- Modify: `docs/superpowers/plans/2026-09-06-macos-mens-training-lane.md`

- [x] Rewrite spec contradictions (yaml-auto hole, 8 GB fail-closed, Metal-only default, env SSOT, goal=selection).
- [x] Replace this plan file with the corrected plan.
- [ ] Commit: `docs: correct macos mens training-lane spec after code audit`

---

### Task 1: Live-available memory + env SSOT

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/vram_autodetect.rs`
- Modify: `contracts/config/env-vars.v1.yaml`
- Modify: `contracts/config/registry.v1.yaml`
- Modify: `docs/src/reference/env-vars.md`

**Produces:**
- `fn parse_vm_stat(stdout: &str) -> Option<(u64, u64, u64, u64)>`
- `pub(crate) fn apply_live_margin(reclaimable_gb: f32, margin_pct: f32, min_reserve_gib: f32) -> f32`
- `pub fn query_apple_available_memory() -> Option<VramInfo>` (macOS) / `None` stub
- `pub const MIN_LIVE_MEM_RESERVE_GIB: f32 = 2.0`
- Priority 3 of `get_system_vram_info` calls the live query (internal fallback to `query_apple_unified_memory` unchanged)

**Env contract rows** (same shape as `VOX_MENS_GRADIENT_CHECKPOINTING`):

- `VOX_MENS_LIVE_MEM_MARGIN_PCT` — `kind: float`, default 0.15, owner `vox-populi`
- `VOX_MENS_DISABLE_LIVE_MEM` — `kind: bool`, owner `vox-populi`
- `VOX_MENS_UNIFIED_MEM_RESERVE_GIB` — backfill pre-existing code

- [ ] Write failing tests first (16k + 4096 page-size samples; missing fields; `apply_live_margin` 40/5/1 GiB; margin-pct default + clamp; `#[serial(vox_mens_live_mem_env)]` disable-live-mem equals static heuristic).
- [ ] `cargo test -p vox-populi --features mens vram_autodetect::tests::` — expect missing-fn failures.
- [ ] Implement parser + margin + query. Wire Priority 3. Do **not** call `hardware::probe()` from the new query.
- [ ] Add the three env-var + registry rows. Run `vox ci config-hygiene`.
- [ ] Re-run tests. Put `#[serial]` on the existing `vram_override_env_is_respected` while touching the module.
- [ ] Commit: `feat(mens): size macOS training memory from vm_stat reclaimable pages`

---

### Task 2: Fix `MacosMetalProbe` zero-VRAM stub

**Files:**
- Modify: `crates/vox-populi/src/mens/hardware/macos_metal.rs`

**Consumes:** `query_apple_available_memory() -> Option<VramInfo>`
**Must not call:** `get_system_vram_gb` / `get_system_vram_info`.

- [ ] Failing unit test of `vram_mb_from_apple_info`: Some(116.0) → 118784; None → 0.
- [ ] Optional `#[cfg(target_os = "macos")]` live test: if query is Some, `probe_metal().vram_mb` equals the conversion; skip if None.
- [ ] `#[cfg(not(target_os = "macos"))] fn probe_metal_is_none_off_macos`.
- [ ] Implement: `probe_metal()` uses `vram_mb_from_apple_info(query_apple_available_memory())`. Keep returning `Some` on macOS. Set `probe_failures` when vram is 0.
- [ ] `cargo test -p vox-populi --features mens macos_metal::`
- [ ] Commit: `fix(mens): wire Apple live-memory probe into MacosMetalProbe`

---

### Task 3: `DeviceProfile.vendor` + `AcceleratorKind::from_vendor`

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/vram_autodetect.rs`
- Modify: `crates/vox-populi/src/mens/tensor/preset_schema.rs` (struct + `from_gpu_info` only — no resolve-logic change yet)
- Modify: `crates/vox-ml-cli/src/commands/schola/train/run_train.rs` (the `from_gpu_info` call)

**Call sites to update (enumerated):**

- `run_train.rs` — pass `&gpu_info.vendor`
- `preset_schema.rs` tests that call `from_gpu_info("rtx 4080 super", 16384)` — add `"nvidia"`

- [ ] Failing test: `from_vendor` maps nvidia/NVIDIA/apple/Apple/amd/unknown/"".
- [ ] Implement `from_vendor`. Add `vendor: String` to `DeviceProfile` and the third `from_gpu_info` argument. Update the enumerated call sites. **Do not change `resolve_effective_profile` yet.**
- [ ] `cargo build -p vox-populi -p vox-ml-cli --features mens-train` and `cargo test -p vox-populi --features mens-train tensor::preset_schema:: tensor::vram_autodetect::`
- [ ] Commit: `refactor(mens): thread GPU vendor into DeviceProfile`

---

### Task 4: Metal-only omitted-preset default + vendor-aware `"auto"`

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/preset_schema.rs` only

**Behavior:** Metal omitted-preset default is `"auto"`; CUDA/CPU/unknown keep `DEFAULT_PRESET`. Metal `"auto"` skips yaml `presets:` and uses `auto_preset_for(Metal, …)` → `base_for_name`. CUDA `"auto"` keeps today's yaml walk.

**Tests (strong, not `assert_ne!`):**

- `cuda_device_with_no_preset_matches_explicit_4080`
- `cpu_device_with_no_preset_still_uses_4080`
- `metal_16g_auto_is_qwen3_16g` (not yaml `prosumer_16g`)
- `metal_116g_auto_is_qwen3_96g` (not yaml `a100`)

- [ ] Write the four tests first; expect Metal cases to fail (still get `"4080"`).
- [ ] Implement the Metal-only default and Metal `"auto"` arm.
- [ ] Re-run the new tests plus the full `preset_schema` / `qwen3_preset_tests` modules.
- [ ] Commit: `fix(mens): Metal omitted-preset default uses qwen3 ladder not CUDA yaml`

---

### Task 5: Metal model auto-resolve (populi helper, then thin CLI wire)

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`
- Modify: `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`

**Produces:**

```rust
pub fn resolve_metal_default_base(
    workspace_root: &Path,
    vram_mb: u64,
) -> anyhow::Result<String>
```

**CLI wire:** before the CandleQlora `DEFAULT_MODEL_ID` fill, if `model.is_none()` and vendor is Metal and `VOX_MENS_DEFAULT_MODEL` is unset, set `model = Some(resolve_metal_default_base(...)?)`. One `probe_gpu()` per invocation (hoist).

- [ ] TDD in populi: 116_000 MB contains `Qwen3-32B`; 13926 contains `Qwen3-8B`; 6144 is `Err`.
- [ ] Implement helper; tests pass.
- [ ] Wire `run_train` (thin).
- [ ] `cargo test -p vox-populi --features mens spoke_base_resolver::` and `cargo build -p vox-ml-cli --features gpu`
- [ ] Commit: `feat(mens): fail-closed Metal base-model resolve before CandleQlora default`

---

### Task 6: Catalogue table (live-MB), CUDA sweep, Revision 7

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` (tests only)
- Modify: `docs/src/architecture/mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md` — append **Revision 7** (current max is 6).

**Sweep:**

```text
cargo test -p vox-populi --features mens-train
cargo test -p vox-ml-cli --features gpu
cargo clippy -p vox-populi --all-targets --features mens-train -- -D warnings
cargo clippy -p vox-ml-cli --all-targets --features gpu -- -D warnings
vox ci config-hygiene
```

- [ ] Table-driven live-MB catalogue test (6144 must be `Err`).
- [ ] Full sweep green.
- [ ] Append Revision 7.
- [ ] Commit: `test(mens): pin agentic_default rungs to live-available MB budgets`
