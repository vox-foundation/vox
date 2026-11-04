---
phase: "318"
slug: "live-vram-selector-size-02"
status: verified
verdict: SECURED
# threats_open = count of OPEN threats at or above block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-10-27"
---

# Phase 318 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail. The register is taken from the `<threat_model>` blocks of plans 318-01 to 318-04. ASVS L1, `block_on: high`. Code range audited: `git diff 47973e78f..HEAD -- . ':(exclude).planning'` on `gsd/mens-compare-316` (HEAD `229b3ce72`), including the 318-REVIEW-FIX changes.

**Verdict: SECURED.** All 21 registered threats are closed (18 mitigated, 3 accepted). There are 4 unregistered flags, all low severity and non-blocking (see below).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| OS / driver → selector | Free memory comes from the Metal working set (`recommendedMaxWorkingSetSize` capped by `vm_stat`), from nvidia-smi `index,memory.free`, or from the NVML plugin | MB figures |
| Repo YAML → selector | `mens/config/gpu-specs.yaml` (workspace file, or the embedded copy) sets the sizes, pins and floors | `hf_id` pins, `floor_mb` |
| Operator → trainer / serve | `--model`, `--base`, `VOX_MENS_DEFAULT_MODEL`, `--preset`, `VOX_MENS_FORCE_TRAIN`, `CUDA_VISIBLE_DEVICES`, `CUDA_DEVICE_ORDER`, `--device` | Strings and flags |
| Run dir → serve / resume | `training_manifest.json` `base_model`; checkpoint `base_model` | Strings |
| Rust → webview | `AutoModelSelection` or the refusal string, across Tauri IPC | JSON |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Evidence | Status |
|-----------|----------|-----------|----------|-------------|----------|--------|
| T-318-01 | DoS | auto pick with no reading | high | mitigate | `size_select.rs:172` and `:200`: `let Some(free) = … else bail!(NO_READING)` in both `select_size` and `recheck_before_load`. `resolve_default_model_id` and the `maybe_resolve_metal_default_base` default fill are gone from `run_train.rs`. CR-01: `metal_reading(…, live=false)` returns `None` (`size_select.rs:60-65`, `accel_budget.rs:84`) | closed |
| T-318-02 | DoS | total VRAM or an override used as free memory | high | mitigate | `read_free_memory` (`size_select.rs:69-79`) reads only `query_accel_budget` on Metal (live-gated) or `query_nvidia_smi_free_mb` off macOS. Grep gate: `get_system_vram_info`, `get_system_vram_gb`, `VOX_VRAM_OVERRIDE` and `probe_gpu` appear in `size_select.rs` only in a comment (line 73). `FreeMemory::cuda` applies `SAFETY_FRACTION` (0.88) (`:39-44`) | closed |
| T-318-03 | Tampering | off-by-one fit or missing preset | medium | mitigate | `fits` uses `<=` (`:303-305`), with a boundary test (12000 → 8B, 11999 → 4B). `size_ladder` bails on a size with no preset (`:289-291`), covered by `ladder_fails_closed_on_a_size_without_a_preset`. The presets are asserted against `KNOWN_PRESETS` (`:522-527`). `qwen3_6g` and `qwen3_12g` were added to `KNOWN_PRESETS`, `preset_schema.rs` and `training-presets.v1.yaml` | closed |
| T-318-04 | Spoofing | size name resolving to a wrong or moving revision | medium | mitigate | `resolve_base_pin` (`:229-263`) is semantically identical to the deleted `detect.rs` copy and requires exactly one distinct pin (`if let [only]`). `split_pin` accepts only 40 lowercase hex characters (`:348-359`). `detect.rs` re-exports it. `reasoning_compare.rs` applies `DEFAULT_MODEL_ID` itself (IN-02) | closed |
| T-318-05 | EoP | explicit size shrunk, or refusal bypassed | medium | mitigate | `explicit_pick` bails unless `force` (`:321-330`). `recheck_before_load` sends explicit picks back through `explicit_pick`, never `largest_fit` (`:197-199`). Tests: `explicit_size_that_does_not_fit_is_refused_not_shrunk`, `recheck_never_steps_an_explicit_size_down` | closed |
| T-318-06 | DoS | re-check stepping up | low | mitigate | `recheck_before_load` keeps a pick that fits unchanged (`:203-210`) and searches the ladder only when it does not fit | closed |
| T-318-07 | Repudiation | off-ladder concrete id skips the fit check | low | accept | AR-318-1 | closed |
| T-318-08 | DoS | training downloads a size that no longer fits | high | mitigate | `gpu.rs:97-109` calls `recheck_size_pick` and `adopt_size_pick` before `download_model_blocking` (`gpu.rs:216`). `run_gpu_training` has one caller (`run_train.rs:468`) | closed |
| T-318-09 | DoS | `train_arm` sizes from total VRAM or 16 GB | high | mitigate | Grep gate: no `get_system_vram_gb`, `get_system_vram_info`, `16384` or `16_384` in `train_arm.rs`. `resolve_local_spoke_base` passes `None` (live). `16384` remains only in `training_selection.rs` tests, for the cloud fixed tier | closed |
| T-318-10 | Tampering | spoke YAML tag chooses a larger size | medium | mitigate | `resolve_training_selection` returns CLI values when `fixed_tier_mb` is `None` (`training_selection.rs:56-62`). `pipeline.rs:471` passes `None`. The YAML comment is in `domain-profiles.yaml:36-38`. Tests: `live_mode_leaves_size_and_preset_to_the_selector`, `domain_rust_leaves_size_to_the_live_selector` | closed |
| T-318-11 | EoP | CPU run reads the GPU budget | low | mitigate | `live_free_memory`: `DeviceKind::Cpu => None` (`run_train.rs`). Test: `cpu_training_has_no_accelerator_reading` | closed |
| T-318-12 | Repudiation | operator cannot see which size or why | low | mitigate | `eprintln!("size: {note}")` at pick time (`run_train.rs`) and at re-check (`recheck_size_pick`). `preset_for_stats` (`gpu.rs:516`) reads the adopted preset. CR-03: the card records `trained_preset(cli, final_pick)` | closed |
| T-318-13 | DoS | chat or GUI sized from total VRAM | high | mitigate | `detect_hardware_vram_gb`, `select_tier_for_vram`, `is_host_apple_silicon` and `select_optimal_local_model` are gone from the code. `host_free_memory` (`auto_select.rs:32-36`) checks the NVML probe first, then `cuda_free_memory` (x0.88), then `read_free_memory` | closed |
| T-318-14 | DoS | auto-local guesses a default | high | mitigate | `select_size(…, None, …)` with no reading bails. `chat.rs:574-576` applies `.map_err(…)?` and `chat.rs:225` applies `?`. Test: `auto_selection_refuses_without_a_reading` | closed |
| T-318-15 | DoS | serve loads a base that no longer fits | medium | mitigate | In `serve/mod.rs`, `serve_fit_note(…)?` (line 132) runs before `spawn_inference_worker` (line 140). Test: `serve_fit_note_refuses_a_ladder_base_that_does_not_fit` | closed |
| T-318-16 | Tampering | hostile `training_manifest.json` | low | mitigate | `load_manifest(run_dir).ok().flatten()`: a missing or unreadable file yields `None` and no check. `base_model` is only passed to `resolve_base_pin` and `select_size`, and is never fetched or executed (`serve/mod.rs:204-221`) | closed |
| T-318-17 | Spoofing | registry advertises nonexistent local models | low | mitigate | `register_mens_local_candidates` and its test were deleted from `registry.rs`. Both call sites were removed | closed |
| T-318-18 | Tampering | TS type drifts from Rust | low | mitigate | The `transport.ts` `AutoModelRecommendation` fields match `AutoModelSelection` exactly. `tauriMock.families.test.ts` asserts the exact key set | closed |
| T-318-19 | Info disclosure | recommendation exposes host details | low | accept | AR-318-2 | closed |
| T-318-20 | Repudiation | GUI hides a refusal as an empty recommendation | low | mitigate | `models_host.rs:37` returns `select_optimal_local_model_for_host()` directly, with no `Ok(...)` wrapper | closed |
| T-318-SC | Tampering | supply chain | low | accept | AR-318-3. `git diff --name-only` shows no `Cargo.toml`, `Cargo.lock`, `package.json` or `pnpm-lock` change in the range | closed |

---

## REVIEW-FIX areas: requested probes

| Area | Probe | Result |
|------|-------|--------|
| Checkpoint `base_model` and the resume refusal (CR-02) | Is a checkpoint field trusted without validation? | **Not trusted for action.** The field is only compared for string equality with the run's base and echoed in the error (`checkpoint.rs:43-56`). It is never loaded or fetched. A mismatch refuses. Core's `apply_checkpoint_resume` is the single implementation, and both the metal and cuda plugins re-export it. Every `CheckpointState` writer sets the field (struct literals, no `..Default`). `VOX_MENS_FORCE_TRAIN` does not bypass the refusal; only `--force-restart` does. Weak spot: a checkpoint with no `base_model` resumes with a warning (UF-318-2) |
| `working_set_is_live` (CR-01) | Is free memory unbounded or from an untrusted source? | **Holds.** It is `live_pressure_budget_bytes().is_some()` on Metal and `false` on CUDA. Only `size_select.rs:72` reads it. A Metal reading is `min(driver advisory, vm_stat)`, so it is bounded. A live 0 is filtered by `f.mb > 0`. A CUDA reading has no upper bound but comes only from the driver (nvidia-smi or NVML), with no env override |
| `VOX_MENS_FORCE_TRAIN` passed to serve (WR-04) | Does force skip more checks than it should? | **No.** In serve, `force` reaches only `explicit_pick`'s "does not fit" arm. It does not skip the no-reading refusal for auto picks, the "nothing fits" refusal, or the checkpoint base guard. `force_train_env` was moved unchanged from `gpu.rs` (`"1"` or `"true"`, case-insensitive). Note: a value exported for training now also silences serve's fit check; this is documented in the how-to |
| `CUDA_VISIBLE_DEVICES` parsing (WR-02) | Does env parsing panic? | **No panic paths.** `env::var(..).ok()` treats non-UTF-8 as unset. Entries are parsed with `.parse().ok()` and collected into `Option<Vec>`, so any UUID, `-1`, empty or overflowing entry means every device counts. Counting every device is conservative: the minimum over all devices is never above the minimum over the visible ones. An index that is not present gives an empty set, which means no reading and a refusal. `parse_nvidia_smi_free` fails the whole reading on any bad row. No `unwrap` or indexing |
| `gpu-specs.yaml` overlay in chat and the GUI (WR-06) | Can an attacker influence the overlay path? | **Partially (UF-318-1).** The path is `<find_cargo_workspace_root()>/mens/config/gpu-specs.yaml`, which walks up from the **cwd** to the first `Cargo.toml` with `[workspace]`, in any Rust workspace. Chat, the GUI and serve now read it. Training already did, so that part predates this phase. A missing file falls back to the embedded copy. A file that is unreadable or fails to parse refuses (fail-closed) |

---

## Unregistered Flags

None of these were raised in any SUMMARY `## Threat Flags` section; all four plans report "None". All are low severity and below `block_on: high`, so they are non-blocking and not counted in `threats_open`.

| Flag ID | Category | Severity | Finding | Suggested action |
|---------|----------|----------|---------|------------------|
| UF-318-1 | Tampering | low | Running `vox chat --model auto-local`, the GUI, `vox mens serve` or `vox mens train` inside an untrusted checkout loads that checkout's `mens/config/gpu-specs.yaml`. `size_ladder` validates only the size suffix (`Qwen3-<n>B`), not the owner, so a planted file can lower floors (bypassing the fit check, leading to a local OOM) or point a rung at `evil/Qwen3-8B@<40hex>`. An auto training pick would then download that repo's safetensors. This is the same trust as training's spoke resolution before this phase. What is new is the read from chat, the GUI and serve. Chat sends the id only to the local endpoint. | Register it as a threat (accept, citing the workspace-config trust model), or mitigate by requiring ladder rungs to be `Qwen/` and, for known sizes, to match the embedded pin unless an opt-in is set |
| UF-318-2 | Tampering | low | A checkpoint without `base_model` (a legacy one, or one with the field removed) resumes onto any base with only a warning. A missing adapter file still keeps the old epoch, step and offset with fresh weights; the fixer deferred this. It requires write access to the run's output dir. | Track it. Once legacy checkpoints have aged out, make a missing `base_model` refuse unless `--force-restart` is given |
| UF-318-3 | DoS | low | NVML path: the probe skips devices it cannot open (`probe.rs` `continue`), and `parse_device_free_mb` drops entries without `memory_free_mb`. The minimum can then come from a subset of devices, unlike the nvidia-smi path, which fails the whole reading on an unreadable row. This affects chat and GUI advice only; training reads nvidia-smi. | Have the probe emit `memory_free_mb: 0` for devices it cannot open, so the existing 0-means-no-reading rule applies |
| UF-318-4 | Info disclosure | low | AR-318-2's rationale covers only the success struct. The refusal string that crosses IPC can include the workspace path (`load_overlay`'s `read {path}: {e}`) or YAML parse text. It goes only to the user's own webview. | Amend AR-318-2 to cover the error arm |

Informational: the IN-07 out-of-phase commit `371c198fe` only removes dead code from `history.rs` (six handlers with no callers) and adds no attack surface.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-318-1 | T-318-07 | Concrete off-ladder ids (D-04) have no floor to check and pass through unchanged. The post-download `plan_for`/`sweep` refusal in `gpu.rs` is the backstop. **Caveat found in the audit:** that backstop decides only on a *calibrated* lane. On an uncalibrated lane (candle-metal today), it prints "UNVERIFIED" and proceeds (`gpu.rs:299-311`), so an off-ladder id on Metal gets no fit check at all. This is pre-existing behavior. | plan 318-01 threat model | 2026-10-27 |
| AR-318-2 | T-318-19 | `AutoModelSelection` carries only a public pin, a size label, two MB figures and a `reason` whose `source` is a static string. It has no path, serial or device name. See UF-318-4 for the error arm. | plan 318-04 threat model | 2026-10-27 |
| AR-318-3 | T-318-SC | The phase installs no packages and adds no dependency or crate edge. Verified: no manifest or lockfile changed in `47973e78f..HEAD`. | plans 318-01..04 threat models | 2026-10-27 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-27 | 21 | 21 | 0 (+4 unregistered flags, all low and non-blocking) | gsd-security-auditor (ASVS L1 grep depth, plus code reading of every REVIEW-FIX area). `cargo nextest run -p vox-populi --features mens-train --lib size_select`: 27 passed |
