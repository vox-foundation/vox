---
phase: "337"
slug: "eval-throughput"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-06"
---

# Phase 337 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Derived from
> 337-RESEARCH.md §Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (std harness; nextest in CI) |
| **Config file** | per-crate; Metal-gated tests in `crates/vox-plugin-mens-candle-metal` need `--features metal` and are `#[ignore]` |
| **Quick run command** | `cargo test -p vox-eval --lib -- decode_protocol sequential_mcnemar` (or the touched crate's filter) |
| **Full suite command** | `cargo test -p vox-eval --lib && cargo test -p vox-plugin-mens-candle-metal --lib && cargo test -p vox-ml-cli --features gpu --lib -- item_checkpoint reasoning_eval eval_collateral eval_gate` |
| **Estimated runtime** | ~60-300 seconds per crate (build-dominated); background every build/test |

Per-crate only (AGENTS.md). No workspace-wide clippy/test. Metal/real-model tests run only in
non-autonomous checkpoints with the GPU idle (`ps aux | grep -c "[v]ox-ml-cli"` is 0).

---

## Sampling Rate

- **After every task commit:** the touched crate's quick command (CPU tests only).
- **After every plan wave:** full suite command.
- **Before `/gsd-verify-work`:** full suite green plus the recorded THRU-01/05/12 artifacts.
- **Max feedback latency:** 300 seconds.

---

## Per-Task Verification Map

| Req | Behavior | Test Type | Automated Command | File Exists |
|-----|----------|-----------|-------------------|-------------|
| THRU-01 | step spans + decomposition bench produce F, A per batch width and size | ignored hardware test + CPU unit for span plumbing | `cargo test --release -p vox-plugin-mens-candle-metal --features metal -- --ignored --nocapture decode_step_decomposition` | ❌ W0 |
| THRU-02 | new attention path bitwise equal to `attend_reference` | unit (CPU synthetic) + ignored Metal | `cargo test -p vox-plugin-mens-candle-metal --lib -- attention_fast_path` | ❌ W0 |
| THRU-02/03/04 | batched greedy decode token-identical to batch=1 | unit (synthetic engine) | `cargo test -p vox-plugin-mens-candle-metal --lib -- batch::tests` | ✅ extend |
| THRU-03 | row-batched norm/rope/gate bitwise equal to per-row | unit | `cargo test -p vox-plugin-mens-candle-metal --lib -- rows_prelude_identity` | ❌ W0 |
| THRU-04 | dense linear bitwise equal to qlora base + zero-LoRA | unit | `cargo test -p vox-plugin-mens-candle-metal --lib -- dense_linear_matches_qlora_zero_lora` | ❌ W0 |
| THRU-05 | width derivation uses exact KV size, no constants; real-model identity at widest width | unit + ignored real-model checkpoint | `cargo test -p vox-plugin-mens-candle-metal --lib -- fit_width` | partial |
| THRU-06 | one load per side across slices; cancel stops consumption; failed batch falls back | unit with fake backend | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval::tests eval_collateral` | partial |
| THRU-07 | f32 key unchanged (golden), bf16 adds field, unknown tag errors, mixed-tag pair is Err | unit | `cargo test -p vox-eval --lib -- decode_protocol` ; `cargo test -p vox-ml-cli --features gpu --lib -- item_checkpoint` | ❌ W0 |
| THRU-08 | bf16 path selected by tag; LM head stays f32 | unit + ignored Metal | `cargo test -p vox-plugin-mens-candle-metal --lib -- compute_dtype_for_protocol` | ❌ |
| THRU-09 | base reuse across two adapters; base never removed by adapter restart | unit | `cargo test -p vox-ml-cli --features gpu --lib -- item_checkpoint eval_collateral` | partial |
| THRU-10 | worst-case size over all paths <= alpha; DP equals brute force on a tiny schedule; loader refuses invalid YAML | unit (exhaustive) | `cargo test -p vox-eval --lib -- sequential_mcnemar` | ❌ W0 |
| THRU-11 | crossed boundary cancels stream; report records rule/look/items_used; replay equals live | unit with fake backend | `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral::sequential` | ❌ |
| THRU-12 | measured before/after table; 4B gate under 10 h | non-autonomous run | checkpoint in the measurement plan | ❌ |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Mutation proofs (record a before/after presence check for each)
- Loosen an interim `look_alpha` to `alpha` → THRU-10 test fails.
- `<` to `<=` in the boundary comparison → THRU-10 test fails.
- Reintroduce wrong `repeat_kv` index order / swap head-row order in the GQA reshape → THRU-02 test fails.
- Remove "omit when f32" → golden-key test fails (THRU-07).
- Pass `force_restart` to the base side in `eval_collateral.rs` → base-never-restarted test fails (THRU-09).
- Drop the pairing-protocol check → mixed-tag test fails (THRU-07).

---

## Wave 0 Requirements

- [ ] `decode_step_decomposition` ignored test (extend `inference/batch_bench.rs`)
- [ ] `attend_reference` kept under `#[cfg(test)]` + synthetic bitwise tests (CPU, Metal-ignored)
- [ ] golden checkpoint-key test (f32 key bytes equal today's) in `item_checkpoint.rs` tests
- [ ] `crates/vox-eval/src/decode_protocol.rs` and `sequential_mcnemar.rs` with tests first
- [ ] env-var entry in `contracts/config/env-vars.v1.yaml` for the protocol variable

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Expected wall-clock |
|----------|-------------|------------|---------------------|
| R1 step decomposition before/after, 0.6B-14B, B in {1,4,8,16,32} | THRU-01, THRU-12 | needs idle Metal GPU and real snapshots | ~5/8/15/25/40 min per size per pass |
| R2 real-model byte identity (0.6B 8 items cap 256; 4B 4 items cap 128) | THRU-05 | real model | ~5 min; ~10 min |
| R3 `decode_dtype_bench` | THRU-08 priority | real model | ~5 min |
| R4 bf16 base + adapter baseline, 0.6B 128/slice | THRU-08 | real model, new protocol baseline | ~1.5-2 h |
| R5 4B f32 128/slice collateral gate | THRU-12 | overnight real run | ~3.8 h expected, ~5.8 h conservative, budget 10 h |
| R6 optional 14B gate | THRU-12 stretch | real run | ~8 h f32 expected, 11 h conservative |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 300s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
