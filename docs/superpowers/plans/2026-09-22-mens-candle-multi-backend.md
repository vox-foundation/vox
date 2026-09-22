# MENS Candle Multi-Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the MENS Candle crates compile again, remove the latent CUDA double-Q/K-norm bug that the compile failure hid (no build ever ran it), and have training, serving, merging and eval pick the right backend for the host automatically: CUDA on an NVIDIA machine, Metal on any Mac, CPU otherwise.

**Architecture:** Part 1 (Tasks 1–3) repairs damage from merge `a7cdfdb8e` in the three `vox-plugin-mens-candle-*` crates and moves the duplicated Q/K-norm loader into the shared `-core` crate. Part 2 (Tasks 4–7) replaces two divergent backend-selection paths (training's OS guess and serving's probe) with one pure selector in `vox-populi`. It also makes the Metal plugin compile with Metal on macOS without flags, tags every Mac as Metal-capable, and adds a `mens-candle-cpu` plugin artifact for non-Mac hosts without a GPU. Task 8 merges the result into the integration branch.

**Tech Stack:** Rust 1.98.1, candle-core/candle-nn 0.10, qlora-rs (patched), abi_stable plugins via `vox-plugin-host`, GitHub Actions release workflow.

**Spec:** No separate spec file. The design was agreed in chat on 2026-09-22 (the brainstorming session that led to this plan). Its decisions are recorded under "Design decisions" below, and executors must treat that section as the spec.

## Design decisions

1. **CUDA Q/K norm:** keep the application that runs *before* the transpose, via `rms_norm_f32` (commit `1516d80ab`). Delete the second application that runs after the transpose (`b9f05601f`), its second loader, and the duplicate struct fields. Metal's `model.rs` already applies the norm exactly once, and that is the reference behaviour. The double application existed only in the non-compiling merge result, so no shipped build or trained adapter is affected. <!-- AMENDED: grill Q2 — latent, not live -->
2. **Q/K-norm loader:** a single helper, `vox_plugin_mens_candle_core::qk_norm::load_qk_norm`, used by both plugins' training code. Both old loaders read through a `VarBuilder` built with `DType::F32`, so they are numerically equivalent. The Metal "shadowing" warning is dead code, not a numeric bug. Inference loaders (`inference.rs`) use a different accessor and are out of scope.
3. **Metal on macOS by default:** the Metal plugin enables Candle's `metal` features through `[target.'cfg(target_os = "macos")'.dependencies]`. Its own code gates change from `feature = "metal"` to `any(feature = "metal", target_os = "macos")`. The `metal` Cargo feature stays, so the release workflow's `--features metal` keeps working. Linux builds of the crate are unaffected.
4. **Probe tags:** every macOS host gets the `metal` tag. Only `aarch64` macOS gets `apple-silicon`. The Metal plugin's catalog `requires-tag` changes from `apple-silicon` to `metal`, because the release already ships an `x86_64-apple-darwin` artifact that Intel Macs could not install until now.
5. **One selector:** `vox_populi::mens::select_mens_backend(requested: DeviceKind, caps: &CapabilitySet) -> &'static str`. It honours an explicit `--device`. For `Best` it picks CUDA if the host has `nvidia-gpu`, else Metal if it has `metal`, else the host's CPU plugin. The CPU plugin is `mens-candle-metal` on macOS (that plugin already falls back to CPU) and `mens-candle-cpu` everywhere else. An explicit `--device cpu` on an `nvidia-gpu` host reuses `mens-candle-cuda` (its `device_select.rs` maps `Cpu` to `Device::Cpu` without touching CUDA). <!-- AMENDED: grill Q5 --> Training, serve, `merge-qlora` and `eval-local` all call it. `ML_BACKEND_CANDIDATES`, `resolve_extension_point` and `ExtensionCandidate` are deleted. <!-- AMENDED: grill Q9 -->
6. **CPU plugin:** built from `vox-plugin-mens-candle-cuda` with no features. That crate cannot serve as the CPU fallback as-is, because its CUDA build links `libcuda` at load time (cudarc `dynamic-linking`) and so won't load on a machine without NVIDIA drivers. The plugin's identity comes from the packaged manifest: the release job ships `Plugin.cpu.toml` (id `mens-candle-cpu`) instead of `Plugin.toml`. The host never reads the plugin's own `manifest_json`/`id()` (only `Plugin.toml`), so no compile-time id is added. <!-- AMENDED: grill Q7 --> Only a `linux-x86_64` artifact is built for now.
7. **Out of scope, listed so nobody widens the plan:**
   - a Windows CPU artifact;
   - the compile-time `--device cuda` bail in `run_train.rs`;
   - real Metal device enumeration (instead of the OS-based tag);
   - de-duplicating the rest of `model.rs`, `inference.rs` and `candle_qlora_train/mod.rs` between the two plugins.

## Global Constraints

- Toolchain: Rust **1.98.1** (`rust-toolchain.toml`). Do not change it.
- **No new workspace crate edges.** `vox-ml-cli → vox-populi` and `vox-populi → vox-plugin-host` (optional, via `mens-train`) already exist. Never edit `contracts/ci/crate-edges.allow.v1.json`.
- Test first. Every task writes a failing test, runs it and sees it fail, then implements.
- Scope every cargo call to one package: `cargo test -p <crate> ...`. Never `cargo build --workspace` or `cargo check --workspace` (Task 8 included). Never `cargo fmt --all`; use `cargo fmt -p <crate>`.
- The machine may be shared. Bound long commands with `timeout 1800s`.
- Commits: imperative subject under 72 characters, a body that explains why, ending with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`. One commit per task.
- **Never push.** Work on branch `fix/mens-candle-backends` in `/Users/brbrainerd/dev/vox-mens-backends` (based on `origin/main` `7103fbdad`).
- A CUDA runtime is not available on the development Mac. CUDA-feature code is only type-checked there (`cargo check -p vox-plugin-mens-candle-cuda`). Tests that need `--features cuda` run in CI or on an NVIDIA host. Say so in the task report rather than claiming they ran.

---

## File Structure

| File | Responsibility | Tasks |
|---|---|---|
| `crates/vox-plugin-mens-candle-core/src/merge.rs` | Adapter merge. Test module needs `serialize` import restored | 1 |
| `crates/vox-plugin-mens-candle-cuda/src/model.rs` | CUDA attention forward. Remove the second Q/K-norm application and the duplicate fields in two test literals; add an apply-once test | 2 |
| `crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs` | CUDA training model build. Remove the second loader and duplicate fields; call the core helper | 2, 3 |
| `crates/vox-plugin-mens-candle-core/src/qk_norm.rs` (new) | Shared Q/K-norm loader and its tests | 3 |
| `crates/vox-plugin-mens-candle-core/src/lib.rs` | Register `qk_norm` module | 3 |
| `crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs` | Metal training model build. Drop the shadowing loader; call the core helper; Metal gates | 3, 5 |
| `crates/vox-plugin-host/src/capability.rs` | `metal` tag on every macOS | 4 |
| `crates/vox-plugin-catalog/catalog.toml` | Metal `requires-tag = "metal"`; new `mens-candle-cpu` entry | 4, 7 |
| `crates/vox-plugin-catalog/tests/catalog_validation.rs` | Pin the three MlBackend `requires-tag`s | 4, 7 |
| `crates/vox-plugin-mens-candle-metal/Cargo.toml` | macOS-target Candle `metal` features | 5 |
| `crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/device_select.rs`, `src/inference.rs` | Metal gates | 5 |
| `crates/vox-populi/src/mens/tensor/backend_select.rs` (new) | `select_mens_backend` + tests | 6 |
| `crates/vox-populi/src/mens/tensor/mod.rs`, `src/mens/mod.rs` | Register and re-export selector | 6 |
| `crates/vox-populi/src/mens/tensor/backend_candle_qlora.rs` | Training dispatch uses the selector | 6 |
| `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs`, `commands/mens/eval_local.rs`, `commands/ai/serve/worker.rs` | Use the selector; delete `ML_BACKEND_CANDIDATES` and `resolve_ml_backend_plugin` | 6 |
| `crates/vox-plugin-host/src/lib.rs` | Delete `ExtensionCandidate`, `resolve_extension_point` and their tests | 6 |
| `crates/vox-ml-cli/src/commands/schola/train/run_train.rs` | Heal the selected plugin, not a hard-coded one | 6 |
| `crates/vox-plugin-mens-candle-cuda/Plugin.toml` | Drop the `macos-aarch64` artifact; rewrite the `requires` comment | 7 |
| `crates/vox-plugin-mens-candle-cuda/Plugin.cpu.toml` (new) | CPU artifact manifest | 7 |
| `.github/workflows/release-binaries.yml` | `gpu-plugin-cpu` job; publish `needs` | 7 |
| `CHANGELOG.md` | Unreleased entries | 2, 6, 7 |

---

### Task 1: Restore the `serialize` import in core's merge tests

The test module of `merge.rs` calls `serialize(...)` three times, but merge `a7cdfdb8e` kept main's tests while taking the other branch's imports, which dropped `use safetensors::serialize;`. `vox-plugin-mens-candle-core`'s test target does not compile.

**Files:**
- Modify: `crates/vox-plugin-mens-candle-core/src/merge.rs` (the `#[cfg(test)] mod tests` block near the end of the file)

**Interfaces:**
- Consumes: nothing.
- Produces: a compiling `vox-plugin-mens-candle-core` test target, which Task 3 needs.

- [ ] **Step 1: Confirm the failure**

Run: `timeout 1800s cargo test -p vox-plugin-mens-candle-core --lib merge::`
Expected: FAIL to compile with `error[E0425]: cannot find function 'serialize' in this scope` pointing into `merge.rs`'s tests (three occurrences).

- [ ] **Step 2: Add the import**

In `crates/vox-plugin-mens-candle-core/src/merge.rs`, change the head of the test module from:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::DType;
    use safetensors::SafeTensors;
```

to:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::DType;
    use safetensors::SafeTensors;
    use safetensors::serialize;
```

- [ ] **Step 3: Run the tests**

Run: `timeout 1800s cargo test -p vox-plugin-mens-candle-core --lib merge::`
Expected: PASS (all `merge::tests::*`).

- [ ] **Step 4: Commit**

```bash
git add crates/vox-plugin-mens-candle-core/src/merge.rs
git commit -m "fix(mens-core): restore serialize import in merge tests" -m "Merge a7cdfdb8e took the non-test half of merge.rs from the deep-research branch (which switched to serialize_to_file and dropped the top-level import) but kept main's tests, which still call safetensors::serialize. The crate's test target stopped compiling." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: Apply CUDA's Q/K norm exactly once

Merge `a7cdfdb8e` combined two independent implementations of Dense Qwen3's per-head Q/K RMSNorm in the CUDA plugin:
- `candle_qlora_train/mod.rs` now lists `q_norm`/`k_norm` twice in the `Qwen2Attention` literal, which is a compile error.
- `model.rs::Qwen2Attention::forward` applies the norm twice: once before the transpose (`rms_norm_f32`) and again after it (`q_norm.forward`). That compiles, but it would be numerically wrong for any non-uniform norm weight. Because the crate never compiled after the merge, no build ran it; this is a latent bug.
- `model.rs` also lists `q_norm: None,`/`k_norm: None,` twice in two test-only literals (`build_layer` in the checkpoint-grad tests and `tiny_attention` in `mod bf16_activation_tests`), so the `--lib` test target doesn't compile either. <!-- AMENDED: grill Q1, Q2 -->

**Files:**
- Modify: `crates/vox-plugin-mens-candle-cuda/src/model.rs` (`Qwen2Attention::forward`; new test in `mod bf16_activation_tests`)
- Modify: `crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs` (the second `let q_norm = vb_mmap.get(...)`/`let k_norm = ...` pair and the duplicate literal fields)
- Modify: `CHANGELOG.md` (`## [Unreleased]` → `### Fixed`)

**Interfaces:**
- Consumes: nothing.
- Produces: `Qwen2Attention` in the CUDA crate whose struct literal has `q_norm`/`k_norm` exactly once. Task 3 relies on the literal using the variables named `q_norm` and `k_norm`.

- [ ] **Step 1: Write the failing test**

Append this test inside `mod bf16_activation_tests` in `crates/vox-plugin-mens-candle-cuda/src/model.rs`. That module already has `use super::*;`, `QLoraConfig`, `QuantizedLinear` and `ComputeDType` in scope, and `super::*` exposes the private `rms_norm_f32` and `causal_mask`.

```rust
    /// Q/K RMSNorm must be applied exactly once, before RoPE. A second
    /// application is invisible with a uniform norm weight (RMSNorm of an
    /// already-normalised vector times a constant is idempotent), so this uses
    /// a non-uniform weight like real Qwen3 checkpoints have, and compares the
    /// forward pass against a reference built from the attention's own
    /// projections with a single `rms_norm_f32` per side.
    #[test]
    fn qk_norm_is_applied_exactly_once() {
        let device = Device::Cpu;
        let (d, n_heads, head_dim) = (8usize, 2usize, 4usize);
        let mut cfg = QLoraConfig::preset_all_bf16(4, 8);
        cfg.quantization.compute_dtype = ComputeDType::F32;
        let w = Tensor::arange(0u32, (d * d) as u32, &device)
            .unwrap()
            .to_dtype(DType::F32)
            .unwrap()
            .reshape((d, d))
            .unwrap()
            .affine(0.01, 0.0)
            .unwrap();
        let mk = || QuantizedLinear::from_weight(&w, None, &cfg, &device).unwrap();
        let norm_w = Tensor::new(&[2.0f32, 0.5, 3.0, 1.5], &device).unwrap();
        let attn = Qwen2Attention {
            q_proj: mk(),
            k_proj: mk(),
            v_proj: mk(),
            o_proj: mk(),
            q_bias: None,
            k_bias: None,
            v_bias: None,
            n_heads,
            n_kv_heads: n_heads,
            head_dim,
            q_norm: Some(RmsNorm::new(norm_w.clone(), 1e-6)),
            k_norm: Some(RmsNorm::new(norm_w, 1e-6)),
        };

        let x = Tensor::randn(0f32, 1f32, (1, 3, d), &device).unwrap();
        let actual = attn.forward(&x, 0, None, None).unwrap();

        // Reference: same projections, one norm per side, no RoPE (inv_freq = None),
        // n_heads == n_kv_heads so repeat_kv is the identity.
        let (b, s, _) = x.dims3().unwrap();
        let q = attn.q_proj.forward(&x).unwrap().reshape((b, s, n_heads, head_dim)).unwrap();
        let q = rms_norm_f32(attn.q_norm.as_ref().unwrap(), &q).unwrap().transpose(1, 2).unwrap();
        let k = attn.k_proj.forward(&x).unwrap().reshape((b, s, n_heads, head_dim)).unwrap();
        let k = rms_norm_f32(attn.k_norm.as_ref().unwrap(), &k).unwrap().transpose(1, 2).unwrap();
        let v = attn
            .v_proj
            .forward(&x)
            .unwrap()
            .reshape((b, s, n_heads, head_dim))
            .unwrap()
            .transpose(1, 2)
            .unwrap();
        let scale = 1.0 / (head_dim as f64).sqrt();
        let att = (q.contiguous().unwrap().matmul(&k.transpose(2, 3).unwrap().contiguous().unwrap()).unwrap()
            * scale)
            .unwrap();
        let att = att.broadcast_add(&causal_mask(s, &device).unwrap()).unwrap();
        let att = candle_nn::ops::softmax(&att, candle_core::D::Minus1).unwrap();
        let y = att
            .matmul(&v.contiguous().unwrap())
            .unwrap()
            .transpose(1, 2)
            .unwrap()
            .contiguous()
            .unwrap()
            .reshape((b, s, n_heads * head_dim))
            .unwrap();
        let expected = attn.o_proj.forward(&y).unwrap();

        let (a, e) = (
            actual.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            expected.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
        );
        assert_eq!(a.len(), e.len());
        for (i, (av, ev)) in a.iter().zip(e.iter()).enumerate() {
            assert!(
                (av - ev).abs() < 1e-4,
                "element {i}: forward={av} reference={ev} — Q/K norm is not applied exactly once"
            );
        }
    }
```

- [ ] **Step 2: Remove the duplicate struct-literal fields so the crate compiles**

The test can't run while the crate fails to compile. In `crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs`, find the second loader pair that sits immediately above `let attn = crate::model::Qwen2Attention {`:

```rust
                let q_norm = vb_mmap
                    .get(head_dim, &format!("{layer_prefix}.self_attn.q_norm.weight"))
                    .ok()
                    .map(|w| candle_nn::RmsNorm::new(w, 1e-6));
                let k_norm = vb_mmap
                    .get(head_dim, &format!("{layer_prefix}.self_attn.k_norm.weight"))
                    .ok()
                    .map(|w| candle_nn::RmsNorm::new(w, 1e-6));
```

Delete those eight lines. The earlier `let q_norm = load_norm(&q_key);` / `let k_norm = load_norm(&k_key);` stay. Then change the struct literal from:

```rust
                    v_bias,
                    q_norm,
                    k_norm,
                    n_heads,
                    n_kv_heads,
                    head_dim,
                    q_norm,
                    k_norm,
                };
```

to:

```rust
                    v_bias,
                    n_heads,
                    n_kv_heads,
                    head_dim,
                    q_norm,
                    k_norm,
                };
```

<!-- AMENDED: grill Q1 --> Also in `crates/vox-plugin-mens-candle-cuda/src/model.rs`, in the two test literals `build_layer` (checkpoint-grad tests) and `tiny_attention` (`mod bf16_activation_tests`), each of which sets the fields twice, delete the **second** `q_norm: None,` / `k_norm: None,` pair (the one after `head_dim`). Verify: `rg -c 'q_norm: None' crates/vox-plugin-mens-candle-cuda/src/model.rs` drops by exactly 2.

- [ ] **Step 3: Run the test and see it fail on the double application**

Run: `timeout 1800s cargo test -p vox-plugin-mens-candle-cuda --lib model::bf16_activation_tests::qk_norm_is_applied_exactly_once -- --exact`
Expected: output contains `running 1 test` (if it says `running 0 tests`, the test is in the wrong module — fix that first; a 0-test run is not a pass or a fail). <!-- AMENDED: R8 --> The crate now compiles and the test FAILS with `element N: forward=… reference=… — Q/K norm is not applied exactly once`. If it passes at this point, stop: the reference or the weights are not discriminating, and the task must not proceed.

- [ ] **Step 4: Remove the second application in `forward`**

In `crates/vox-plugin-mens-candle-cuda/src/model.rs`, inside `Qwen2Attention::forward`, delete this block, which comes right after the `let v = v.reshape(...)?.transpose(1, 2)?;` statement:

```rust
        let q = if let Some(q_norm) = &self.q_norm {
            q_norm.forward(&q)?
        } else {
            q
        };
        let k = if let Some(k_norm) = &self.k_norm {
            k_norm.forward(&k)?
        } else {
            k
        };
```

Keep the earlier `match &self.q_norm { Some(norm) => rms_norm_f32(norm, &q)?, None => q }` blocks, which run before the transpose.

<!-- AMENDED: R1 — Module becomes unused --> That block was the only user of the `Module` trait in `model.rs` (added by `b9f05601f`). Change the import `use candle_nn::{Module, RmsNorm};` to `use candle_nn::RmsNorm;`, otherwise Step 5's clippy fails on `unused_imports`.

- [ ] **Step 5: Run the test and the crate's model tests**

Run: `timeout 1800s cargo test -p vox-plugin-mens-candle-cuda --lib model::`
Expected: PASS, including `qk_norm_is_applied_exactly_once`.

Run: `timeout 1800s cargo clippy -p vox-plugin-mens-candle-cuda --lib --no-deps -- -D warnings`
Expected: no warnings in `model.rs` or `candle_qlora_train/mod.rs`. Warnings elsewhere in the crate may predate this plan. Record them, don't fix them.

- [ ] **Step 6: Record the fix in the CHANGELOG**

In `CHANGELOG.md`, under `## [Unreleased]` → `### Fixed`, add as the first bullet:

```markdown
- **CUDA Candle plugin compiles again, with Dense Qwen3 Q/K RMSNorm applied once.** Merge `a7cdfdb8e` combined two Q/K-norm implementations in `vox-plugin-mens-candle-cuda`: duplicate `q_norm`/`k_norm` struct fields, which broke the build, plus a second norm in `Qwen2Attention::forward` after the head transpose. No build ever ran that code. The pre-transpose `rms_norm_f32` path (matching Metal) is kept, and a test pins it as applied once.
```
<!-- AMENDED: grill Q2; R12 — marker kept outside the snippet so it is not pasted into CHANGELOG.md -->

- [ ] **Step 7: Commit**

```bash
git add crates/vox-plugin-mens-candle-cuda/src/model.rs crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs CHANGELOG.md
git commit -m "fix(mens-cuda): apply Qwen3 Q/K RMSNorm once, not twice" -m "Merge a7cdfdb8e kept both 1516d80ab's and b9f05601f's Q/K-norm implementations: the training model build and two model.rs test literals listed q_norm/k_norm twice (a compile error), and Qwen2Attention::forward normalised Q and K before and again after the head transpose, a latent bug the compile failure hid. Keep the pre-transpose rms_norm_f32 path, which matches Metal, and pin it with a reference-forward test that fails on a double application." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: One shared Q/K-norm loader in core

Both plugins' `candle_qlora_train/mod.rs` define the same `load_norm` closure. Metal also has a second, shadowing loader (the `unused variable: q_norm/k_norm` warnings). Move the loader into core and call it from both plugins.

**Files:**
- Create: `crates/vox-plugin-mens-candle-core/src/qk_norm.rs`
- Modify: `crates/vox-plugin-mens-candle-core/src/lib.rs` (add `pub mod qk_norm;` in the alphabetical module list, before `pub mod qlora_preflight;`)
- Modify: `crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs`
- Modify: `crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs`

**Interfaces:**
- Consumes: Task 1 (core test target compiles); Task 2 (CUDA literal uses `q_norm`/`k_norm` once).
- Produces: `pub fn load_qk_norm(vb: &candle_nn::VarBuilder, proj_weight_key: &str, head_dim: usize) -> Option<candle_nn::RmsNorm>` in `vox_plugin_mens_candle_core::qk_norm`.

- [ ] **Step 1: Write the failing test (and module skeleton)**

Create `crates/vox-plugin-mens-candle-core/src/qk_norm.rs`:

```rust
//! Dense Qwen3's per-head Q/K RMSNorm loader, shared by the Metal and CUDA
//! training model builds.
//!
//! Qwen3 checkpoints store `…self_attn.q_norm.weight` / `…k_norm.weight`
//! (shape `[head_dim]`). Qwen2/Qwen2.5 checkpoints have neither, so a missing
//! tensor is `None`, not an error. The key is derived from the projection
//! key (`…self_attn.q_proj.weight` → `…self_attn.q_norm.weight`) — a plain
//! `.weight` suffix swap would wrongly produce `q_proj_norm.weight`.

use candle_core::DType;
use candle_nn::{RmsNorm, VarBuilder};

/// RMSNorm epsilon Qwen3 uses for its Q/K norms.
const QK_NORM_EPS: f64 = 1e-6;

/// Load the Q or K norm that belongs to `proj_weight_key`, as F32.
pub fn load_qk_norm(vb: &VarBuilder, proj_weight_key: &str, head_dim: usize) -> Option<RmsNorm> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{Device, Tensor};
    use std::collections::HashMap;

    /// `vb_dtype` is the dtype the VarBuilder hands tensors out as — use BF16
    /// to model a builder that does NOT cast for us, so the helper's own F32
    /// cast is what's under test.
    fn vb_with(entries: &[(&str, Tensor)], vb_dtype: DType) -> VarBuilder<'static> {
        let map: HashMap<String, Tensor> = entries
            .iter()
            .map(|(k, t)| ((*k).to_string(), t.clone()))
            .collect();
        VarBuilder::from_tensors(map, vb_dtype, &Device::Cpu)
    }

    #[test]
    fn loads_the_norm_named_after_the_projection() {
        let dev = Device::Cpu;
        let w = Tensor::new(&[2.0f32, 0.5, 3.0, 1.5], &dev).unwrap();
        let vb = vb_with(&[("model.layers.0.self_attn.q_norm.weight", w)], DType::F32);
        let norm = load_qk_norm(&vb, "model.layers.0.self_attn.q_proj.weight", 4)
            .expect("q_norm must be found from the q_proj key");
        // All-ones input has RMS 1, so the output is the weight (up to epsilon).
        let x = Tensor::ones((1, 4), DType::F32, &dev).unwrap();
        let y = norm.forward_diff(&x).unwrap().flatten_all().unwrap().to_vec1::<f32>().unwrap();
        for (got, want) in y.iter().zip([2.0f32, 0.5, 3.0, 1.5]) {
            assert!((got - want).abs() < 1e-4, "stored weight not applied: got {y:?}");
        }
    }

    #[test]
    fn missing_norm_is_none_not_an_error() {
        let vb = vb_with(&[], DType::F32);
        assert!(load_qk_norm(&vb, "model.layers.0.self_attn.k_proj.weight", 4).is_none());
    }

    #[test]
    fn the_norm_weight_is_f32_even_from_a_bf16_builder() {
        let dev = Device::Cpu;
        let w = Tensor::new(&[1.0f32, 1.0, 1.0, 1.0], &dev).unwrap();
        let vb = vb_with(&[("m.self_attn.k_norm.weight", w)], DType::BF16);
        let norm = load_qk_norm(&vb, "m.self_attn.k_proj.weight", 4).unwrap();
        // A BF16 weight against F32 activations is a dtype-mismatch error in
        // forward_diff; this only succeeds if the helper cast the weight to F32.
        let x = Tensor::ones((1, 4), DType::F32, &dev).unwrap();
        assert!(norm.forward_diff(&x).is_ok(), "norm weight must be F32 to match F32 activations");
    }
}
```

Add `pub mod qk_norm;` to `crates/vox-plugin-mens-candle-core/src/lib.rs` directly before `pub mod qlora_preflight;`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `timeout 1800s cargo test -p vox-plugin-mens-candle-core --lib qk_norm::`
Expected: FAIL. All three tests panic with `not yet implemented`.

- [ ] **Step 3: Implement**

Replace the `todo!()` body with:

```rust
pub fn load_qk_norm(vb: &VarBuilder, proj_weight_key: &str, head_dim: usize) -> Option<RmsNorm> {
    let norm_key = proj_weight_key.replace("_proj.weight", "_norm.weight");
    vb.get((head_dim,), &norm_key)
        .ok()
        .and_then(|t| t.to_dtype(DType::F32).ok())
        .map(|w| RmsNorm::new(w, QK_NORM_EPS))
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `timeout 1800s cargo test -p vox-plugin-mens-candle-core --lib qk_norm::`
Expected: PASS (3 tests).

- [ ] **Step 5: Use the helper in the CUDA plugin**

In `crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs`, replace the whole `load_norm` closure with its comment and the two calls, from the `// Dense Qwen3's per-head q_norm/k_norm (frozen, not LoRA-adapted;` comment through `let k_norm = load_norm(&k_key);`, with:

```rust
                // Dense Qwen3's per-head q_norm/k_norm (frozen, not LoRA-adapted;
                // absent on Qwen2/Qwen2.5). Must match inference.rs's loader or a
                // served model drifts from what it trained against.
                let q_norm = vox_plugin_mens_candle_core::qk_norm::load_qk_norm(&vb_mmap, &q_key, head_dim);
                let k_norm = vox_plugin_mens_candle_core::qk_norm::load_qk_norm(&vb_mmap, &k_key, head_dim);
```

- [ ] **Step 6: Use the helper in the Metal plugin and drop its shadowing loader**

In `crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs`, make the same replacement of the `load_norm` closure and its two calls as in Step 5. Then delete the second pair above `let attn = crate::model::Qwen2Attention {`:

```rust
                let q_norm = vb_mmap
                    .get(head_dim, &format!("{layer_prefix}.self_attn.q_norm.weight"))
                    .ok()
                    .map(|w| candle_nn::RmsNorm::new(w, 1e-6));
                let k_norm = vb_mmap
                    .get(head_dim, &format!("{layer_prefix}.self_attn.k_norm.weight"))
                    .ok()
                    .map(|w| candle_nn::RmsNorm::new(w, 1e-6));
```

If `layer_prefix` becomes unused in either file after this, the compiler will say so. Remove that binding too only if nothing else reads it.

- [ ] **Step 7: Check both plugins build clean**

Run: `timeout 1800s cargo clippy -p vox-plugin-mens-candle-metal --lib --no-deps -- -D warnings`
Expected: no `unused variable: q_norm`/`k_norm` warnings.

Run: `timeout 1800s cargo clippy -p vox-plugin-mens-candle-cuda --lib --no-deps -- -D warnings`
Expected: no new warnings in `candle_qlora_train/mod.rs`.

Run: `timeout 1800s cargo test -p vox-plugin-mens-candle-cuda --lib model::` and `timeout 1800s cargo test -p vox-plugin-mens-candle-metal --lib model::`
Expected: PASS (Task 2's test still passes; Metal's `qk_norm_changes_output_when_present` still passes).

- [ ] **Step 8: Commit**

```bash
git add crates/vox-plugin-mens-candle-core/src/qk_norm.rs crates/vox-plugin-mens-candle-core/src/lib.rs crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs
git commit -m "refactor(mens): share the Qwen3 Q/K-norm loader via candle-core" -m "Both plugins carried the same load_norm closure, and Metal also kept a second, shadowing loader from merge a7cdfdb8e (the unused q_norm/k_norm warnings). One helper in vox-plugin-mens-candle-core now owns the q_proj -> q_norm key mapping and the F32 cast, so the two training builds cannot drift apart again." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: Tag every Mac as Metal-capable

`probe()` only adds `metal` on `aarch64` macOS, and the Metal plugin's catalog entry requires `apple-silicon`. Intel Macs have Metal and the release already builds an `x86_64-apple-darwin` Metal artifact. On an Intel Mac, though, `resolve_extension_point` skips the Metal plugin, so serve, `merge-qlora` and `eval-local` fail with "no ML backend plugin matches". (Install is not refused: no install path checks `requires_tag`. Training is unaffected because it uses `plugin_id_for_device`'s OS check.) <!-- AMENDED: grill Q3 -->

**Files:**
- Modify: `crates/vox-plugin-host/src/capability.rs` (`probe`; tests module)
- Modify: `crates/vox-plugin-catalog/catalog.toml` (`mens-candle-metal` entry)
- Modify: `crates/vox-plugin-catalog/tests/catalog_validation.rs` (`ml_backend_requires_tag_matches_the_hand_mirrored_candidate_list`)

**Interfaces:**
- Consumes: nothing.
- Produces: the `metal` tag on every macOS host, and the `mens-candle-metal` plugin with `requires-tag = "metal"`. Task 6's selector matches on `"metal"`.

- [ ] **Step 1: Write the failing tests**

In the tests module of `crates/vox-plugin-host/src/capability.rs`, add:

```rust
    #[test]
    #[cfg(target_os = "macos")]
    fn every_mac_is_tagged_metal() {
        let caps = probe();
        assert!(caps.satisfies(Some("metal")), "all Macs since 2012 support Metal: {caps:?}");
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    fn intel_mac_is_not_tagged_apple_silicon() {
        assert!(!probe().satisfies(Some("apple-silicon")));
    }
```

In `crates/vox-plugin-catalog/tests/catalog_validation.rs`, change the `EXPECTED` table in `ml_backend_requires_tag_matches_the_hand_mirrored_candidate_list` to:

```rust
    const EXPECTED: &[(&str, &str)] = &[
        ("mens-candle-cuda", "nvidia-gpu"),
        ("mens-candle-metal", "metal"),
    ];
```

- [ ] **Step 2: Run the catalog test to see it fail**

Run: `timeout 1800s cargo test -p vox-plugin-catalog --test catalog_validation ml_backend_requires_tag`
Expected: FAIL: `plugin 'mens-candle-metal' requires-tag must stay 'metal' … got Some("apple-silicon")`.

(On an Apple Silicon Mac, `every_mac_is_tagged_metal` already passes, and the Intel test is compiled out. The probe change is still required for Intel Macs; Step 3 makes it explicit.)

- [ ] **Step 3: Implement**

In `crates/vox-plugin-host/src/capability.rs`, replace the block inside `probe()`:

```rust
    if cfg!(target_arch = "aarch64") && cfg!(target_os = "macos") {
        tags.insert("apple-silicon".to_string());
        // Every Apple Silicon Mac has a Metal-capable GPU. There is no
        // lightweight Metal-probe library in this tree (candle_core's Metal
        // support requires its heavy `metal` cargo feature, which
        // vox-plugin-host must not depend on just to answer "is there a
        // GPU"), so `metal` is derived from `apple-silicon` at compile time
        // rather than probed at runtime. Revisit only as a deliberate,
        // separate follow-up if a false positive is ever observed.
        tags.insert("metal".to_string());
    }
```

with:

```rust
    if cfg!(target_os = "macos") {
        // Every Mac vox can run on (macOS 11+, 2012 hardware onward) has a
        // Metal-capable GPU, Intel ones included. There is no lightweight
        // Metal-probe library in this tree (candle_core's Metal support needs
        // its heavy `metal` feature, which vox-plugin-host must not depend on
        // just to answer "is there a GPU"), so `metal` is derived from the
        // target OS rather than probed at runtime. The Metal plugin still
        // falls back to CPU with a warning if device creation fails.
        tags.insert("metal".to_string());
        if cfg!(target_arch = "aarch64") {
            tags.insert("apple-silicon".to_string());
        }
    }
```

In `crates/vox-plugin-catalog/catalog.toml`, in the `mens-candle-metal` entry, change `requires-tag = "apple-silicon"` to `requires-tag = "metal"`.

<!-- AMENDED: R2 — rewrite the whole comment, or Task 6's `rg ML_BACKEND_CANDIDATES crates` check still matches --> Replace the **entire** comment block above the `EXPECTED` table (from `// \`crates/vox-ml-cli/src/commands/schola/merge_qlora.rs\`` through `// This test exists solely to guard against that drift.`) with:

```rust
    // The MlBackend selector (`vox_populi::mens::select_mens_backend`) matches
    // on these `requires-tag` values and cannot depend on vox-plugin-catalog
    // (see AGENTS.md Dependency Discipline), so it hand-mirrors them. If a tag
    // changes here without the selector, backend selection silently stops
    // matching the right plugin; this test exists to catch that drift.
```

<!-- AMENDED: grill leftover --> Also change the assertion message in that test from `"... to match ML_BACKEND_CANDIDATES, got {:?}"` to `"... to match vox_populi::mens::select_mens_backend, got {:?}"` (Task 6 deletes the constant).

- [ ] **Step 4: Run the tests**

Run: `timeout 1800s cargo test -p vox-plugin-catalog --test catalog_validation`
Expected: PASS.

Run: `timeout 1800s cargo test -p vox-plugin-host --lib capability::`
Expected: PASS.

<!-- AMENDED: R3 — the generated catalog doc has no requires-tag column (`plugin-catalog.generated.md:15`), so Task 4 changes nothing there; the regeneration moved to Task 7. Step numbering kept. -->

- [ ] **Step 6: Commit**

```bash
git add crates/vox-plugin-host/src/capability.rs crates/vox-plugin-catalog/catalog.toml crates/vox-plugin-catalog/tests/catalog_validation.rs
git commit -m "fix(plugin-host): tag every Mac as metal, not just Apple Silicon" -m "The release ships an x86_64-apple-darwin Metal plugin, but probe() only added the metal tag on aarch64 and the catalog gated the plugin on apple-silicon, so Intel Macs could not be matched to it (serve, merge and eval switch to the shared selector in a later commit). Metal is available on every supported Mac; apple-silicon stays aarch64-only." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```


---

### Task 5: Build the Metal plugin with Metal on macOS without flags

A plain `cargo build -p vox-plugin-mens-candle-metal` on a Mac compiles Candle without Metal. `--device best` then quietly logs "No GPU feature compiled — using CPU". Only the release workflow passes `--features metal`.

**Files:**
- Modify: `crates/vox-plugin-mens-candle-metal/Cargo.toml`
- Modify: `crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/device_select.rs`
- Modify: `crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs`
- Modify: `crates/vox-plugin-mens-candle-metal/src/inference.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: on macOS, `select_candle_device(DeviceKind::Best, true)` returns a label other than `"cpu(no-gpu-build)"` with no features passed.

- [ ] **Step 1: Write the failing test**

Append to the end of `crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/device_select.rs`:

```rust
#[cfg(all(test, target_os = "macos"))]
mod macos_default_build_tests {
    use super::*;

    /// On macOS the plugin must be Metal-capable without `--features metal`.
    /// Metal may still be unavailable at runtime (e.g. a headless VM), which
    /// is a `cpu(fallback)`, but never `cpu(no-gpu-build)`.
    #[test]
    fn best_device_on_macos_is_metal_capable_without_feature_flags() {
        let (_device, label) = select_candle_device(DeviceKind::Best, true).unwrap();
        // Positive match: "cpu(forced)" (VOX_CANDLE_DEVICE=cpu) must not count as a pass.
        assert!(
            label.starts_with("metal") || label == "cpu(fallback)",
            "macOS build compiled without Metal (label: {label})"
        );
    }
}
```

- [ ] **Step 2: Run the test to see it fail**

Run (on macOS, no features): `env -u VOX_CANDLE_DEVICE timeout 1800s cargo test -p vox-plugin-mens-candle-metal --lib macos_default_build_tests`
Expected: `running 1 test`, FAIL: `macOS build compiled without Metal (label: cpu(no-gpu-build))`. <!-- AMENDED: R7 -->

Before writing the assertion, read the success-path labels in `device_select.rs` (the `feature = "metal"` branches) and make `label.starts_with(...)` match the real Metal label exactly.

- [ ] **Step 3: Enable Candle's Metal backend for macOS targets**

In `crates/vox-plugin-mens-candle-metal/Cargo.toml`, add this table directly after the `[dependencies]` table (before `[features]`):

```toml
# Metal on every macOS build, not only when `--features metal` is passed:
# a plain `cargo build` on a Mac used to produce a CPU-only plugin that
# silently fell back. Target-specific so Linux builds of this crate (e.g. a
# workspace-wide `cargo check` in CI) never try to compile Metal.
[target.'cfg(target_os = "macos")'.dependencies]
candle-core = { workspace = true, features = ["metal"] }
candle-nn = { workspace = true, features = ["metal"] }
vox-plugin-mens-candle-core = { path = "../vox-plugin-mens-candle-core", features = ["metal"] }
```

Leave the existing `[features] metal = [...]` line in place. The release workflow passes `--features metal`, and that must keep compiling.

- [ ] **Step 4: Widen the crate's Metal code gates**

<!-- AMENDED: R6 — the three-step sed was non-idempotent and rewrote the comment at inference.rs:1056 (which describes core's gates) --> Edit only the `#[cfg(...)]` attribute lines. There are 8: `device_select.rs` (4), `candle_qlora_train/mod.rs` (2), `inference.rs` (1 gate plus 1 comment that must NOT change). First confirm nothing is already widened:

```bash
rg -c 'target_os = "macos"' crates/vox-plugin-mens-candle-metal/src   # must print nothing
sed -i '' -E '/^[[:space:]]*#\[cfg/ s/feature = "metal"/any(feature = "metal", target_os = "macos")/g' crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/device_select.rs crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs crates/vox-plugin-mens-candle-metal/src/inference.rs
```

(On Linux, use `sed -i -E` without the `''`.) A single substitution is enough: `not(feature = "metal")` becomes `not(any(feature = "metal", target_os = "macos"))`.

Then verify: `rg -n 'feature = "metal"' crates/vox-plugin-mens-candle-metal/src` shows every `#[cfg` hit inside `any(feature = "metal", target_os = "macos")` (7 gate lines), plus the unchanged `// \`#[cfg(not(any(feature = "cuda", feature = "metal")))]\`` comment in `inference.rs`.

- [ ] **Step 5: Run the test to see it pass**

Run: `env -u VOX_CANDLE_DEVICE timeout 1800s cargo test -p vox-plugin-mens-candle-metal --lib macos_default_build_tests`
Expected: PASS.

Run: `timeout 1800s cargo test -p vox-plugin-mens-candle-metal --lib`
Expected: PASS (all existing tests; tests that were `--features metal`-only now run on macOS too. If a previously `#[ignore]`d live-Metal test exists, it stays ignored.)

Run: `timeout 1800s cargo clippy -p vox-plugin-mens-candle-metal --lib --no-deps -- -D warnings`
Expected: no warnings on the 7 gate lines or in Task 5's test. <!-- AMENDED: R16 --> Code behind `feature = "metal"` is now linted on macOS for the first time; warnings inside those pre-existing bodies are not this plan's to fix. Record them in the report (file:line + lint) and do not edit them.

- [ ] **Step 6: Commit**

```bash
git add crates/vox-plugin-mens-candle-metal/Cargo.toml crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/device_select.rs crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs crates/vox-plugin-mens-candle-metal/src/inference.rs
git commit -m "fix(mens-metal): compile with Metal on macOS without --features" -m "Only the release workflow passed --features metal, so a local cargo build of the Metal plugin on a Mac was CPU-only and --device best quietly logged 'No GPU feature compiled'. Enable Candle's Metal backend for macOS targets and widen the crate's gates to any(feature = \"metal\", target_os = \"macos\"); Linux builds are unchanged and --features metal still works." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 6: One backend selector for train, serve, merge and eval

Training picks a plugin with `plugin_id_for_device`, which returns Metal on macOS and CUDA everywhere else, including hosts without a GPU. Serve, `merge-qlora` and `eval-local` probe the host and fail outright when nothing matches. Replace both with one selector.

**Files:**
- Create: `crates/vox-populi/src/mens/tensor/backend_select.rs`
- Modify: `crates/vox-populi/src/mens/tensor/mod.rs` (register module)
- Modify: `crates/vox-populi/src/mens/mod.rs` (re-export)
- Modify: `crates/vox-populi/src/mens/tensor/backend_candle_qlora.rs`
- Modify: `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs`
- Modify: `crates/vox-ml-cli/src/commands/mens/eval_local.rs`
- Modify: `crates/vox-ml-cli/src/commands/ai/serve/worker.rs`
- Modify: `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: Task 4 (`metal` tag semantics).
- Produces:
  - `pub fn vox_populi::mens::select_mens_backend(requested: DeviceKind, caps: &vox_plugin_host::CapabilitySet) -> &'static str`
  - `pub const MENS_CANDLE_CUDA: &str = "mens-candle-cuda"`, `MENS_CANDLE_METAL: &str = "mens-candle-metal"`, `MENS_CANDLE_CPU: &str = "mens-candle-cpu"` in `vox_populi::mens::tensor::backend_select`, re-exported from `vox_populi::mens`.
  - Task 7 builds the `mens-candle-cpu` plugin this selector can return.

- [ ] **Step 1: Write the failing tests (and module skeleton)**

Create `crates/vox-populi/src/mens/tensor/backend_select.rs`:

```rust
//! Which MENS Candle plugin services a request on this host.
//!
//! One selector for training, serving, `merge-qlora` and `eval-local`, so
//! they cannot disagree about the backend. An explicit `--device` always
//! wins (the plugin then reports a clear error if the host can't do it).
//! `Best` prefers CUDA, then Metal, then the CPU plugin for this OS. `Cpu`
//! reuses the CUDA plugin on an NVIDIA host (it runs on CPU without touching
//! CUDA), else the CPU plugin for this OS.
//!
//! The `requires-tag` strings here mirror `vox-plugin-catalog/catalog.toml`;
//! `catalog_validation.rs` pins them on the catalog side.

use vox_plugin_host::CapabilitySet;

use crate::mens::tensor::device::DeviceKind;

/// CUDA build of the Candle plugin (`requires-tag = "nvidia-gpu"`).
pub const MENS_CANDLE_CUDA: &str = "mens-candle-cuda";
/// Metal build of the Candle plugin (`requires-tag = "metal"`); also runs on CPU.
pub const MENS_CANDLE_METAL: &str = "mens-candle-metal";
/// CPU-only build for non-Mac hosts without an NVIDIA GPU.
pub const MENS_CANDLE_CPU: &str = "mens-candle-cpu";

/// Whether a release asset of `plugin_id` exists for the platform this binary
/// was built for. `vox plugin install`/auto-heal can only fetch these; anything
/// else must be built from source.
#[must_use]
pub fn has_prebuilt_artifact(plugin_id: &str) -> bool {
    has_prebuilt_artifact_for(plugin_id, std::env::consts::OS, std::env::consts::ARCH)
}

// Mirrors the release jobs in .github/workflows/release-binaries.yml.
fn has_prebuilt_artifact_for(plugin_id: &str, os: &str, arch: &str) -> bool {
    match plugin_id {
        MENS_CANDLE_METAL => os == "macos",
        MENS_CANDLE_CUDA | MENS_CANDLE_CPU => os == "linux" && arch == "x86_64",
        _ => false,
    }
}

/// Pick the plugin id for `requested` on a host with capabilities `caps`.
#[must_use]
pub fn select_mens_backend(requested: DeviceKind, caps: &CapabilitySet) -> &'static str {
    select_for_os(requested, caps, cfg!(target_os = "macos"))
}

fn select_for_os(requested: DeviceKind, caps: &CapabilitySet, is_macos: bool) -> &'static str {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(tags: &[&str]) -> CapabilitySet {
        CapabilitySet::from_tags(tags.iter().copied())
    }

    #[test]
    fn best_prefers_cuda_on_an_nvidia_host() {
        let c = caps(&["cpu-only", "nvidia-gpu"]);
        assert_eq!(select_for_os(DeviceKind::Best, &c, false), MENS_CANDLE_CUDA);
    }

    #[test]
    fn best_uses_metal_on_any_mac() {
        let c = caps(&["cpu-only", "metal"]);
        assert_eq!(select_for_os(DeviceKind::Best, &c, true), MENS_CANDLE_METAL);
    }

    #[test]
    fn best_falls_back_to_the_cpu_plugin_off_mac() {
        let c = caps(&["cpu-only"]);
        assert_eq!(select_for_os(DeviceKind::Best, &c, false), MENS_CANDLE_CPU);
    }

    #[test]
    fn cpu_on_a_mac_uses_the_metal_plugin() {
        let c = caps(&["cpu-only", "metal", "apple-silicon"]);
        assert_eq!(select_for_os(DeviceKind::Cpu, &c, true), MENS_CANDLE_METAL);
    }

    #[test]
    fn cpu_on_an_nvidia_host_reuses_the_cuda_plugin() {
        // The CUDA plugin maps DeviceKind::Cpu to Device::Cpu without touching
        // CUDA; don't make the user install a second plugin.
        let c = caps(&["cpu-only", "nvidia-gpu"]);
        assert_eq!(select_for_os(DeviceKind::Cpu, &c, false), MENS_CANDLE_CUDA);
    }

    #[test]
    fn cpu_off_mac_without_a_gpu_uses_the_cpu_plugin() {
        let c = caps(&["cpu-only"]);
        assert_eq!(select_for_os(DeviceKind::Cpu, &c, false), MENS_CANDLE_CPU);
    }

    #[test]
    fn prebuilt_artifacts_match_the_release_jobs() {
        assert!(has_prebuilt_artifact_for(MENS_CANDLE_METAL, "macos", "x86_64"));
        assert!(has_prebuilt_artifact_for(MENS_CANDLE_CPU, "linux", "x86_64"));
        assert!(has_prebuilt_artifact_for(MENS_CANDLE_CUDA, "linux", "x86_64"));
        assert!(!has_prebuilt_artifact_for(MENS_CANDLE_CPU, "windows", "x86_64"));
        assert!(!has_prebuilt_artifact_for(MENS_CANDLE_CPU, "linux", "aarch64"));
        assert!(!has_prebuilt_artifact_for(MENS_CANDLE_CUDA, "macos", "aarch64"));
        assert!(!has_prebuilt_artifact_for(MENS_CANDLE_METAL, "linux", "x86_64"));
    }

    #[test]
    fn an_explicit_device_is_honoured_even_without_the_capability() {
        let c = caps(&["cpu-only"]);
        assert_eq!(select_for_os(DeviceKind::Cuda, &c, false), MENS_CANDLE_CUDA);
        assert_eq!(select_for_os(DeviceKind::Metal, &c, false), MENS_CANDLE_METAL);
    }
}
```

In `crates/vox-populi/src/mens/tensor/mod.rs`, add next to `mod backend_candle_qlora;`:

```rust
#[cfg(feature = "mens-train")]
pub mod backend_select;
```

In `crates/vox-populi/src/mens/mod.rs`, add after the existing `pub use tensor::{ DeviceKind, …` block:

```rust
#[cfg(feature = "mens-train")]
pub use tensor::backend_select::{
    MENS_CANDLE_CPU, MENS_CANDLE_CUDA, MENS_CANDLE_METAL, has_prebuilt_artifact, select_mens_backend,
};
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `timeout 1800s cargo test -p vox-populi --lib --features mens-train backend_select::`
Expected: FAIL (`not yet implemented`).

- [ ] **Step 3: Implement**

Replace the `todo!()` body:

```rust
fn select_for_os(requested: DeviceKind, caps: &CapabilitySet, is_macos: bool) -> &'static str {
    let cpu_plugin = if is_macos { MENS_CANDLE_METAL } else { MENS_CANDLE_CPU };
    match requested {
        DeviceKind::Cuda => MENS_CANDLE_CUDA,
        DeviceKind::Metal => MENS_CANDLE_METAL,
        DeviceKind::Cpu if caps.satisfies(Some("nvidia-gpu")) => MENS_CANDLE_CUDA,
        DeviceKind::Cpu => cpu_plugin,
        DeviceKind::Best => {
            if caps.satisfies(Some("nvidia-gpu")) {
                MENS_CANDLE_CUDA
            } else if caps.satisfies(Some("metal")) {
                MENS_CANDLE_METAL
            } else {
                cpu_plugin
            }
        }
    }
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `timeout 1800s cargo test -p vox-populi --lib --features mens-train backend_select::`
Expected: PASS (8 tests). <!-- AMENDED: grill Q5; R9 -->

- [ ] **Step 5: Route training through the selector**

In `crates/vox-populi/src/mens/tensor/backend_candle_qlora.rs`:
- Replace `let plugin_id = plugin_id_for_device(device_kind);` with:

```rust
        let plugin_id = super::backend_select::select_mens_backend(device_kind, &vox_plugin_host::probe());
```

- Delete the `fn plugin_id_for_device` function.
- In the test module, change `use super::{CandleQloraBackend, plugin_id_for_device};` to `use super::CandleQloraBackend;` and delete the test `candle_qlora_plugin_id_for_metal_is_mens_candle_metal`. The selector's tests cover it.

- [ ] **Step 6: Route merge, eval-local and serve through the selector**

In `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs`, delete the `ML_BACKEND_CANDIDATES` constant and the comment block above it (from `// Candidate plugins for the `MlBackend` extension point,` through the constant's closing `];`). Replace the call:

```rust
        let plugin_id = vox_plugin_host::resolve_extension_point(
            "MlBackend",
            ML_BACKEND_CANDIDATES,
            &vox_plugin_host::probe(),
        )
        .context("no ML backend plugin matches this host's capabilities")?;
```

with:

```rust
        let plugin_id = vox_populi::mens::select_mens_backend(
            vox_populi::mens::DeviceKind::Best,
            &vox_plugin_host::probe(),
        );
```

Update the comment above it to read: `// Dispatch to the MlBackend plugin this host should use (CUDA on an NVIDIA host, Metal on a Mac, CPU otherwise) — see vox_populi::mens::select_mens_backend.`

In `crates/vox-ml-cli/src/commands/mens/eval_local.rs`, make the same replacement for its `resolve_extension_point(... crate::commands::schola::merge_qlora::ML_BACKEND_CANDIDATES, &vox_plugin_host::probe()) .context(...)?` call (use `vox_populi::mens::DeviceKind::Best`), and update its comment the same way.

<!-- AMENDED: grill Q9 --> In `crates/vox-ml-cli/src/commands/ai/serve/worker.rs`, the selector can't fail, so the `Err` arm becomes dead. Replace the whole `let plugin_id = match resolve_ml_backend_plugin(&vox_plugin_host::probe()) { Ok(id) => id, Err(e) => { … return; } };` statement in the worker thread with:

```rust
        let plugin_id = vox_populi::mens::select_mens_backend(
            vox_populi::mens::DeviceKind::Best,
            &vox_plugin_host::probe(),
        );
```

Then delete `fn resolve_ml_backend_plugin` and the test `metal_capability_selects_metal_serve_plugin`, and drop `resolve_ml_backend_plugin` from the test module's `use super::{InferenceRequest, inference_payload, resolve_ml_backend_plugin};` line. <!-- AMENDED: R11 --> `backend_select::tests` covers both cases.

In `crates/vox-plugin-host/src/lib.rs`, delete `pub struct ExtensionCandidate`, `pub fn resolve_extension_point` (with their doc comments) and `mod resolve_extension_point_tests`. Fix the one doc link that pointed at `resolve_extension_point` if any remain (`cargo doc` would warn).

Then run `rg -n "ML_BACKEND_CANDIDATES|resolve_extension_point|ExtensionCandidate|resolve_ml_backend_plugin" crates` and confirm it prints nothing.

- [ ] **Step 7: Heal the selected plugin before training**

In `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`, inside `if matches!(train_backend, vox_populi::mens::PopuliTrainBackend::CandleQlora) { … }`:
- Keep the `#[cfg(not(feature = "mens-candle-cuda"))] anyhow::bail!(…)` for `DeviceKind::Cuda` unchanged.
- Delete the `#[cfg(feature = "mens-candle-cuda")] crate::commands::mens::plugin_heal::ensure_cuda_plugin(true)?;` line and the comment block above it.
- Delete the whole `// `--device metal` dispatches through mens-candle-cuda's …` comment and the `#[cfg(target_os = "macos")] if matches!(device_kind, …Metal) { … ensure_metal_plugin(true)?; }` block.
- Add, as the last statement inside the `if matches!(train_backend, …CandleQlora)` block:

```rust
        // Make sure the plugin the selector will dispatch to is installed and
        // loadable, self-healing a missing/stale one (opt out with
        // `--no-auto-heal` / VOX_MENS_NO_AUTO_HEAL). Same selector the
        // training dispatch in vox-populi uses, so they can't disagree.
        // Skip the heal where no release asset exists for this platform (it
        // would try a download that can't succeed); dispatch then fails at load
        // with a build-from-source hint.
        #[cfg(feature = "gpu")]
        {
            let plugin_id =
                vox_populi::mens::select_mens_backend(device_kind, &vox_plugin_host::probe());
            if vox_populi::mens::has_prebuilt_artifact(plugin_id) {
                crate::commands::mens::plugin_heal::ensure_plugin(plugin_id, true)?;
            }
        }
```

<!-- AMENDED: grill Q4; R4, R9 — hint must work where it is shown --> In `crates/vox-populi/src/mens/tensor/backend_candle_qlora.rs` (Step 5's file), make the load-error hint depend on whether a prebuilt asset exists. Replace the `"… Install it with: vox plugin install {plugin_id}"` message's last line with a computed hint:

```rust
        let hint = if super::backend_select::has_prebuilt_artifact(plugin_id) {
            format!("Install it with: vox plugin install {plugin_id}")
        } else {
            format!(
                "No prebuilt '{plugin_id}' exists for this platform. Build it from source \
                 (the vox-plugin-mens-candle-* crate; mens-candle-cpu is vox-plugin-mens-candle-cuda \
                 without features, packaged with Plugin.cpu.toml), add this platform to the \
                 manifest's [plugin.payload.artifacts] table, then: vox plugin install --path <dir>"
            )
        };
```

and use `{hint}` in the `anyhow!` message in place of the old install line.

`ensure_cuda_plugin` / `ensure_metal_plugin` stay in `plugin_heal.rs`. They are public and `crates/vox-ml-cli/tests/no_runtime_cargo.rs` calls `ensure_cuda_plugin`.

- [ ] **Step 8: Build and test the touched crates**

Run: `timeout 1800s cargo test -p vox-populi --lib --features mens-train mens::`
Expected: PASS.

Run: `timeout 1800s cargo test -p vox-ml-cli --lib --features gpu`
Expected: PASS.

Run: `timeout 1800s cargo test -p vox-plugin-host --lib`
Expected: PASS (the deleted `resolve_extension_point_tests` are gone, everything else unchanged).

Run: `timeout 1800s cargo clippy -p vox-ml-cli --lib --features gpu --no-deps -- -D warnings`
Expected: no new warnings in the four touched `vox-ml-cli` files; `cargo clippy -p vox-plugin-host --lib --no-deps -- -D warnings` clean. Pre-existing warnings elsewhere in the crate (e.g. `populi_lifecycle.rs`) are out of scope.

Run: `timeout 600s cargo run -q -p vox-cli -- ci crate-edges`
Expected: pass (no new edges).

- [ ] **Step 9: CHANGELOG**

Under `## [Unreleased]` → `### Changed`, add:

```markdown
- **MENS picks its Candle backend the same way everywhere.** `vox mens train`, `vox mens serve`, `merge-qlora` and `eval-local` now share `vox_populi::mens::select_mens_backend`: an explicit `--device` wins; otherwise CUDA on an NVIDIA host, Metal on any Mac (Intel included), and a CPU plugin elsewhere. Previously training guessed from the OS (CUDA on every non-Mac host, GPU or not) while serving probed hardware and failed outright on a host with no GPU.
```

- [ ] **Step 10: Commit**

```bash
git add crates/vox-plugin-host/src/lib.rs crates/vox-populi/src/mens/tensor/backend_select.rs crates/vox-populi/src/mens/tensor/mod.rs crates/vox-populi/src/mens/mod.rs crates/vox-populi/src/mens/tensor/backend_candle_qlora.rs crates/vox-ml-cli/src/commands/schola/merge_qlora.rs crates/vox-ml-cli/src/commands/mens/eval_local.rs crates/vox-ml-cli/src/commands/ai/serve/worker.rs crates/vox-ml-cli/src/commands/schola/train/run_train.rs CHANGELOG.md
git commit -m "feat(mens): one backend selector for train, serve, merge and eval" -m "Training chose a plugin from the OS alone (CUDA on every non-Mac host, GPU or not) while serve/merge/eval probed hardware and failed when nothing matched. select_mens_backend honours an explicit --device and otherwise prefers CUDA, then Metal, then this OS's CPU plugin; every entry point and the pre-train plugin heal now go through it. ML_BACKEND_CANDIDATES and the now-unused resolve_extension_point are gone." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 7: A CPU plugin for non-Mac hosts without a GPU

The CUDA build links `libcuda` at load time (cudarc's `dynamic-linking`), so it can't load on a machine without NVIDIA drivers. The same crate built without `cuda` is a working CPU backend. Ship it under its own manifest. <!-- AMENDED: grill Q7 — identity comes from the packaged Plugin.toml; the host never reads manifest_json/id() -->

**Files:**
- Modify: `crates/vox-plugin-mens-candle-cuda/Plugin.toml` (drop `macos-aarch64` artifact; rewrite `requires` comment)
- Create: `crates/vox-plugin-mens-candle-cuda/Plugin.cpu.toml`
- Modify: `crates/vox-plugin-catalog/catalog.toml` (new entry)
- Modify: `crates/vox-plugin-catalog/tests/catalog_validation.rs` (pin)
- Modify: `.github/workflows/release-binaries.yml` (new job; publish `needs`)
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: Task 6's `MENS_CANDLE_CPU = "mens-candle-cpu"`.
- Produces: a catalog entry `mens-candle-cpu` with `requires-tag = "cpu-only"` and `bundled-in = []`, `Plugin.cpu.toml`, and a release asset `mens-candle-cpu-v<version>-linux-x86_64.zip`.

- [ ] **Step 1: Write the failing test**

In `crates/vox-plugin-catalog/tests/catalog_validation.rs`, extend `EXPECTED` in `ml_backend_requires_tag_matches_the_hand_mirrored_candidate_list`:

```rust
    const EXPECTED: &[(&str, &str)] = &[
        ("mens-candle-cuda", "nvidia-gpu"),
        ("mens-candle-metal", "metal"),
        ("mens-candle-cpu", "cpu-only"),
    ];
```

<!-- AMENDED: R5 — the one-off diff doesn't stop future drift --> Append to `crates/vox-plugin-mens-candle-cuda/tests/plugin_toml_version_matches_crate.rs`:

```rust
/// The CPU build ships with Plugin.cpu.toml; the installer derives the asset
/// name from its version, so it must track the crate and Plugin.toml too.
#[test]
fn plugin_cpu_toml_matches_plugin_toml() {
    let get = |src: &str| -> (String, i64) {
        let v: toml::Value = src.parse().unwrap();
        (
            v["plugin"]["version"].as_str().unwrap().to_string(),
            v["plugin"]["payload"]["abi-version"].as_integer().unwrap(),
        )
    };
    let main = get(include_str!("../Plugin.toml"));
    let cpu = get(include_str!("../Plugin.cpu.toml"));
    assert_eq!(cpu, main, "Plugin.cpu.toml version/abi-version must match Plugin.toml");
    assert_eq!(cpu.0, env!("CARGO_PKG_VERSION"));
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `timeout 1800s cargo test -p vox-plugin-mens-candle-cuda --test plugin_toml_version_matches_crate`
Expected: FAIL to compile: `include_str!` can't find `../Plugin.cpu.toml`.

Run: `timeout 1800s cargo test -p vox-plugin-catalog --test catalog_validation ml_backend_requires_tag`
Expected: FAIL: `expected plugin 'mens-candle-cpu' in the catalog`.

- [ ] **Step 3: Make `Plugin.toml` describe only the CUDA build**

<!-- AMENDED: grill Q7 --> In `crates/vox-plugin-mens-candle-cuda/Plugin.toml`:
- Delete the `macos-aarch64` artifact line and the `# CPU-only build (no `cuda` feature — see note above).` comment above it. On macOS the selector never picks `mens-candle-cuda`; CPU work goes to `mens-candle-metal`. Keep `windows-x86_64` (a from-source CUDA build on Windows is a real case; out of scope per Design decision 7).
- In the comment above `[plugin.payload.requires]`, replace the sentences saying that without the `cuda` feature (the default, and the only option on macOS) the crate compiles as a CPU-only backend with: `This manifest describes the \`cuda\`-feature build only. The no-feature build ships as a separate plugin, mens-candle-cpu, packaged with Plugin.cpu.toml.` Keep the "informational only" sentence.
- Drop `"macos"` from `os = [...]`.

- [ ] **Step 4: Add the CPU plugin manifest**

Create `crates/vox-plugin-mens-candle-cuda/Plugin.cpu.toml`:

```toml
# Manifest for the CPU-only build of this crate (built WITHOUT `--features
# cuda`). Packaged instead of Plugin.toml by the
# release workflow's gpu-plugin-cpu job. It exists because the CUDA build
# links libcuda at load time and cannot load on hosts without NVIDIA drivers.
[plugin]
id = "mens-candle-cpu"
name = "Mens (Candle, CPU)"
version = "0.6.0"
description = "ML training/inference backend using Candle on the CPU, for hosts without a supported GPU."
license = "Apache-2.0"

[plugin.host]
min-vox-version = "0.5.0"

[plugin.payload]
kind = "code"
abi-version = 12

[plugin.payload.provides]
extension-points = ["MlBackend"]

[plugin.payload.requires]
os = ["linux"]
arch = ["x86_64"]

[plugin.payload.artifacts]
"linux-x86_64" = "libvox_plugin_mens_candle_cuda.so"
```

Keep `version` and `abi-version` equal to `Plugin.toml`'s; Step 1's `plugin_cpu_toml_matches_plugin_toml` enforces it.

- [ ] **Step 5: Add the catalog entry**

In `crates/vox-plugin-catalog/catalog.toml`, insert directly after the `mens-candle-metal` entry:

```toml
[[plugin]]
id = "mens-candle-cpu"
payload-kind = "code"
description = "ML training/inference backend using Candle on the CPU, for hosts without a supported GPU."
status = "beta"
extension-points = ["MlBackend"]
# Every host has cpu-only. On macOS the selector uses mens-candle-metal for
# CPU work instead, and no macOS artifact of this plugin is published.
requires-tag = "cpu-only"
# Same first-party release-asset model as mens-candle-cuda above; built from
# vox-plugin-mens-candle-cuda without the `cuda` feature.
default-source = "github:vox-foundation/vox"
bundled-in = []
```

<!-- AMENDED: grill Q6 --> `bundled-in` stays empty and neither `[[bundle]]` changes. Adding it to `vox-dev` would make `vox bundle apply vox-dev` fail on a Mac (no macOS artifact; `install.rs` errors, `apply.rs` reports "partially failed"), and `vox-ml` is the NVIDIA stack. The pre-train heal installs it on demand on linux-x86_64.

- [ ] **Step 6: Run the tests to see them pass**

Run: `timeout 1800s cargo test -p vox-plugin-mens-candle-cuda --test plugin_toml_version_matches_crate`
Expected: PASS (2 tests).

Run: `timeout 1800s cargo test -p vox-plugin-catalog`
Expected: PASS. That includes `every_default_source_resolves` if it exists on this base; a `github:vox-foundation/vox` source is allowed.

- [ ] **Step 6b: Regenerate catalog-derived docs** <!-- AMENDED: R3 -->

The new `[[plugin]]` entry adds a row to `docs/src/reference/plugin-catalog.generated.md`.

Run: `timeout 1800s cargo run -q -p vox-cli -- ci generate-plugin-catalog-docs`
Expected: a new `mens-candle-cpu` row. (The lefthook `plugin-catalog-docs` hook also regenerates on commit if hooks are installed in this worktree; running it explicitly makes it not depend on that.) If the build can't finish within the timeout on a loaded machine, skip and say so; the `ssot-autoregen` CI job regenerates it on the PR.

- [ ] **Step 7: Add the release job**

In `.github/workflows/release-binaries.yml`, add a job after `gpu-plugin-cuda` (before `dist-verify`):

```yaml
  gpu-plugin-cpu:
    name: Build mens-candle-cpu plugin
    runs-on: ubuntu-latest
    timeout-minutes: 240
    # Same non-blocking contract as gpu-plugin-metal/gpu-plugin-cuda: a plugin
    # build failure must never take down the core release.
    continue-on-error: true
    env:
      CARGO_BUILD_JOBS: "2"
    steps:
      - uses: actions/checkout@v7

      - name: Install Rust toolchain
        uses: ./.github/actions/setup-rust

      # The same crate as mens-candle-cuda, built WITHOUT `--features cuda`:
      # a CPU-only Candle backend that loads on hosts with no NVIDIA driver.
      - name: Build the CPU cdylib
        run: cargo build -p vox-plugin-mens-candle-cuda --profile dist

      - name: Package the plugin zip
        run: |
          set -euo pipefail
          version="$(cargo pkgid -p vox-plugin-mens-candle-cuda | sed -E 's/.*[#@]//')"
          staging="$(mktemp -d)"
          cp crates/vox-plugin-mens-candle-cuda/Plugin.cpu.toml "$staging/Plugin.toml"
          cp target/dist/libvox_plugin_mens_candle_cuda.so "$staging/"
          mkdir -p dist
          zip -j "dist/mens-candle-cpu-v${version}-linux-x86_64.zip" "$staging"/*
          echo "Packaged: mens-candle-cpu-v${version}-linux-x86_64.zip"

      - name: Upload plugin artifact
        uses: actions/upload-artifact@v7
        with:
          name: release-gpu-cpu
          path: dist/mens-candle-cpu-v*.zip
          if-no-files-found: error
```

In the publish job, change `needs: [build, dist-verify, gpu-plugin-metal, gpu-plugin-cuda]` to `needs: [build, dist-verify, gpu-plugin-metal, gpu-plugin-cuda, gpu-plugin-cpu]`.

Then check the workflow guards. Both only read YAML, so there's no heavy build:

Run: `timeout 900s cargo run -q -p vox-cli -- ci runner-policy-check`
Expected: pass. `ubuntu-latest` is already a registered exception for `release-binaries.yml` in `docs/src/ci/github-hosted-exceptions.md`.

Run: `timeout 900s cargo run -q -p vox-cli -- ci workflow-concurrency-guard`
Expected: pass.

- [ ] **Step 8: CHANGELOG**

Under `## [Unreleased]` → `### Added`, add:

```markdown
- **`mens-candle-cpu` plugin** for Linux hosts without an NVIDIA GPU: the Candle backend built without CUDA, published as `mens-candle-cpu-v<version>-linux-x86_64.zip`. The CUDA build links `libcuda` at load time and cannot load on such hosts, so until now MENS training, serving and merging had no backend there at all. On macOS the Metal plugin already covers CPU.
```

- [ ] **Step 9: Commit**

```bash
git add crates/vox-plugin-mens-candle-cuda/Plugin.toml crates/vox-plugin-mens-candle-cuda/Plugin.cpu.toml crates/vox-plugin-mens-candle-cuda/tests/plugin_toml_version_matches_crate.rs docs/src/reference/plugin-catalog.generated.md docs/src/reference/distribution-bundles.generated.md crates/vox-plugin-catalog/catalog.toml crates/vox-plugin-catalog/tests/catalog_validation.rs .github/workflows/release-binaries.yml CHANGELOG.md
git commit -m "feat(mens): ship a CPU Candle plugin for hosts without a GPU" -m "The CUDA plugin links libcuda at load time (cudarc dynamic-linking), so on a Linux host without NVIDIA drivers MENS had no backend at all. The same crate built without the cuda feature is a working CPU backend; ship it as mens-candle-cpu with its own manifest, catalog entry and release job so select_mens_backend's CPU fallback has something to load." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 8: Merge into the integration branch and verify

**Files:** none authored. This task only merges and checks.

**Interfaces:**
- Consumes: Tasks 1–7 on `fix/mens-candle-backends`.
- Produces: `integrate/suite-fixes-2026-09-22` (worktree `/Users/brbrainerd/dev/vox-integration`) containing this branch, with the two previously broken crates compiling.

- [ ] **Step 1: Confirm this branch is complete**

Run in `/Users/brbrainerd/dev/vox-mens-backends`: `git status --short` (must be empty) and `git log --oneline origin/main..HEAD` (the plan commits — currently 3 — plus 7 task commits) <!-- AMENDED: R10 -->.

- [ ] **Step 2: Merge**

Run in `/Users/brbrainerd/dev/vox-integration`: `git merge --no-ff fix/mens-candle-backends -m "merge: fix/mens-candle-backends"`

If `CHANGELOG.md` or `contracts/toestub/suppressions.v1.json` conflict, the integration branch and this branch both appended entries. Keep both sides' entries and drop only the conflict markers. For any other conflicting file, stop and report; do not guess.

<!-- AMENDED: grill leftover --> Expect a possible nearby-hunk conflict in `crates/vox-plugin-catalog/tests/catalog_validation.rs`: the integration branch adds a test right after the MlBackend pin test. If it conflicts, keep both tests whole, then `grep -c 'fn ml_backend_requires_tag_matches_the_hand_mirrored_candidate_list'` must print 1. `catalog.toml` and `release-binaries.yml` also changed on both sides but in non-overlapping places.

- [ ] **Step 3: Per-package checks, now including the Candle crates**

<!-- AMENDED: grill Q8 — no workspace-wide cargo (Global Constraints) --> Run each, one at a time; all must finish with no `error`:
- `timeout 1800s cargo check -p vox-plugin-mens-candle-core --all-targets`
- `timeout 1800s cargo check -p vox-plugin-mens-candle-cuda --all-targets`
- `timeout 1800s cargo check -p vox-plugin-mens-candle-metal --all-targets`
- `timeout 1800s cargo check -p vox-plugin-host --all-targets`
- `timeout 1800s cargo check -p vox-plugin-catalog --all-targets`
- `timeout 1800s cargo check -p vox-populi --all-targets --features mens-train`
- `timeout 1800s cargo check -p vox-ml-cli --all-targets --features gpu`
- `timeout 1800s cargo check -p vox-cli --all-targets`

If one fails, stop and report the error; do not patch it in Task 8.

- [ ] **Step 4: Run the tests this plan added, on the merged tree**

Run each; all must PASS:
- `timeout 1800s cargo test -p vox-plugin-mens-candle-core --lib`
- `timeout 1800s cargo test -p vox-plugin-mens-candle-cuda --lib`
- `timeout 1800s cargo test -p vox-plugin-mens-candle-metal --lib`
- `timeout 1800s cargo test -p vox-plugin-host --lib`
- `timeout 1800s cargo test -p vox-plugin-catalog`
- `timeout 1800s cargo test -p vox-populi --lib --features mens-train backend_select::`
- `timeout 1800s cargo test -p vox-ml-cli --lib --features gpu`

- [ ] **Step 5: Report**

Report the merge commit SHA, the check and test results, and the tests that could not run here. The `cuda`-feature variants (`cargo check/test -p vox-plugin-mens-candle-cuda --features cuda`, and `qk_norm_is_applied_exactly_once` on a real CUDA device) need an NVIDIA host or CI. Do not push.

---

## Execution Order

Sequential constraints (shared-file collisions, CANNOT parallelize):
- Task 2 → Task 3: both modify `crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs` (Task 3 relies on Task 2's single `q_norm`/`k_norm` fields).
- Task 3 → Task 5: both modify `crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs`.
- Task 4 → Task 7: both modify `crates/vox-plugin-catalog/catalog.toml` and `tests/catalog_validation.rs` (same `EXPECTED` table).
- Task 2 → Task 6 → Task 7: all append to `CHANGELOG.md` `## [Unreleased]`.
- Task 1 → Task 3: Task 3's tests need core's test target to compile.
- Task 4 → Task 6: the selector matches the `metal` tag semantics.
- Task 6 → Task 7: the selector returns `mens-candle-cpu`, which Task 7 adds to the catalog.
- Machine constraint: the build broker is not installed, so run one cargo command at a time.

Pre-flight checklist:
- [ ] Worktree isolated: `/Users/brbrainerd/dev/vox-mens-backends`, branch `fix/mens-candle-backends`.
- [ ] Target git history confirmed: `git merge-base HEAD origin/main` = `7103fbdad`.
- [ ] Next migration sequence verified: N/A (no schema changes).
- [ ] Local test database verified: N/A (no DB).
- [ ] `env | grep VOX_CANDLE_DEVICE` is empty (or run the Task 5 tests with `env -u`).

Recommended task sequence: Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 6 → Task 7 → Task 8. Tasks 4 and 5 are file-independent but offer no real parallelism gain on a broker-less machine.

SDD ledger pre-population (copy into progress.md):
- Conflict on `cuda/.../candle_qlora_train/mod.rs`, `metal/.../candle_qlora_train/mod.rs`, `catalog.toml`, `catalog_validation.rs`, `CHANGELOG.md`: sequential execution enforced. Ruling: settled.
- R1: Task 2 Step 4 also narrows `use candle_nn::{Module, RmsNorm};` to `use candle_nn::RmsNorm;`. Ruling: settled.
- R2: Task 4 rewrites the whole pin-test comment; no `ML_BACKEND_CANDIDATES` text may remain. Ruling: settled.
- R3: catalog docs regeneration lives in Task 7 (Step 6b), not Task 4. Ruling: settled.
- R4/R9: `has_prebuilt_artifact` in `backend_select.rs` is the single place that encodes which release assets exist; both the heal skip and the load hint call it. Ruling: settled.
- R5: `Plugin.cpu.toml` version/abi parity is enforced by a test, not a one-off diff. Ruling: settled.
- R6: the Task 5 sed touches `#[cfg` lines only; the `inference.rs` comment about core's gates stays unchanged. Ruling: settled.
- R7: the Task 5 test asserts a positive label and runs with `env -u VOX_CANDLE_DEVICE`. Ruling: settled.
- R16: pre-existing clippy warnings in newly compiled Metal bodies are recorded, not fixed. Ruling: settled.
- Grill Q4–Q9 decisions (Cpu→CUDA on NVIDIA hosts, `bundled-in = []`, no compile-time plugin id, per-package Task 8 checks, `resolve_extension_point` deleted): settled.

## Deferred Minor Issues

1. **The Intel-Mac probe path never goes red on this Apple Silicon host** (Task 4). Optional: `rustup target add x86_64-apple-darwin` and run `cargo test -p vox-plugin-host --lib --target x86_64-apple-darwin capability::` under Rosetta. Deferred because it installs a toolchain target.
2. **The public `select_mens_backend` wrapper is never called by a test** (only `select_for_os` is). Optional: add a `#[cfg(target_os = "macos")]` test asserting `select_mens_backend(DeviceKind::Best, &vox_plugin_host::probe()) == MENS_CANDLE_METAL`.
3. **`MENS_CANDLE_CUDA`/`MENS_CANDLE_METAL` could be private.** Outside `backend_select.rs` only `MENS_CANDLE_CPU` and `has_prebuilt_artifact` are used. They could also reuse `plugin_heal.rs`'s `CUDA_PLUGIN_ID`/`METAL_PLUGIN_ID`, but that is a different crate, so the duplication stays.
4. **`cargo hakari generate --diff`.** The macOS target dependencies on Candle `metal` may add transitive crates to the workspace-hack graph. Run the hakari check before pushing (CI `ci.yml:860-861`).
5. **Heal now runs for `--device best`** on Macs and linux-x86_64 NVIDIA hosts. Previously it ran only for explicit `--device cuda`/`metal`. Mention this in the Task 6 report and CHANGELOG wording if the reviewer wants it.
6. **Windows host with no GPU:** it previously got `mens-candle-cuda` (a from-source CPU build) and now gets `mens-candle-cpu`, which has no Windows asset. The load error's build-from-source hint covers it (Design decision 7 keeps Windows out of scope).
7. **Task 4's commit is intermediate:** between the Task 4 and Task 6 commits, `ML_BACKEND_CANDIDATES` still says `apple-silicon` while the catalog says `metal`. This is harmless on this branch because both land before any merge.
8. **The CPU release job duplicates the CUDA job** (~38 lines). A two-leg matrix would save ~25 lines but touches the least-proven release job. Kept separate on purpose.
