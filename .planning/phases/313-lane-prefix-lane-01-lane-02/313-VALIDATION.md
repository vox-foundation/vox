---
phase: "313"
slug: "lane-prefix-lane-01-lane-02"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-26"
---

# Phase 313 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> No model is loaded. Proof is string and serde unit tests. Commands below were named from the research pass and have not been executed yet.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | built-in `cargo test` |
| **Config file** | none |
| **Quick run command** | the single-crate filter for the crate the task touched |
| **Full suite command** | the per-crate filters in the map below, then `cargo check -p vox-ml-cli -p vox-plugin-mens-candle-cuda -p vox-plugin-mens-candle-metal` |
| **Estimated runtime** | ~120 seconds |

---

## Sampling Rate

- **After every task commit:** Run the single-crate filter for the crate touched
- **After every plan wave:** Run every command in the map, crate by crate
- **Before `/gsd-verify-work`:** Those commands must be green
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 313-01-01 | 01 | 0 | LANE-01 | — | `Lane::parse` accepts only `codegen`, `hub`, `agent` and the error names all three | unit | `cargo test -p vox-eval qwen_reasoning` | ❌ W0 | ⬜ pending |
| 313-01-02 | 01 | 1 | LANE-01 | — | Hub and agent prompts have no `/no_think` and no empty think block; codegen stays byte-identical | unit | `cargo test -p vox-eval qwen_reasoning` | ❌ W0 | ⬜ pending |
| 313-01-03 | 01 | 1 | LANE-01 | — | Raw wrap by lane, codegen byte-identical | unit | `cargo test -p vox-plugin-mens-candle-core training_text` | ❌ W0 | ⬜ pending |
| 313-01-04 | 01 | 1 | LANE-01 | — | Chat flatten: codegen unchanged, hub/agent bare assistant, last user turn only | unit | `cargo test -p vox-ml-cli flatten_chat_messages` | ✅ / ❌ new | ⬜ pending |
| 313-01-05 | 01 | 1 | LANE-01 | T-313-01 | Unknown `lane` is 400 on chat and generate before any worker call; error names the three values | unit | `cargo test -p vox-ml-cli unknown_lane` | ❌ W0 | ⬜ pending |
| 313-01-06 | 01 | 1 | LANE-01 | — | `inference_payload` carries `lane` | unit | `cargo test -p vox-ml-cli the_payload_carries` | ✅ / ❌ new | ⬜ pending |
| 313-01-07 | 01 | 1 | LANE-01 | — | CUDA and Metal `assemble_prompt` match the lane; already-ChatML prompts stay unchanged | unit | `cargo test -p vox-plugin-mens-candle-cuda assemble_prompt` and `cargo test -p vox-plugin-mens-candle-metal assemble_prompt` | ✅ / ❌ new | ⬜ pending |
| 313-01-08 | 01 | 1 | LANE-02 | — | Empty think stays outside the loss; a non-empty think body is not rewritten | unit | `cargo test -p vox-plugin-mens-candle-core direct_answer_puts_empty_think` and `cargo test -p vox-plugin-mens-candle-core reasoning_trace_keeps` | ✅ | ⬜ pending |
| 313-01-09 | 01 | 1 | LANE-02 | — | Same fusion locks on the populi copy, including a reasoning message row | unit | `cargo test -p vox-populi training_text` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/vox-eval/src/qwen_reasoning.rs` — `Lane`, `user_for_lane`, `assistant_opener_for_lane` and tests
- [ ] `crates/vox-plugin-mens-candle-core` `training_text.rs` — lane prefix helper and a message-row reasoning lock
- [ ] `crates/vox-ml-cli` serve tests — fake worker counter so an unknown lane is proven to skip inference
- [ ] `crates/vox-populi` `training_text.rs` — fusion locks matching candle-core

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
