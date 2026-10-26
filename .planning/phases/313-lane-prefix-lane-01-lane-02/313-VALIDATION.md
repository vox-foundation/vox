---
phase: "313"
slug: "lane-prefix-lane-01-lane-02"
status: draft
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-26"
---

# Phase 313 — Validation Strategy

> Per-phase validation contract. Commands are the `<automated>` lines from the four plans. No model is loaded.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | built-in `cargo test` |
| **Config file** | none |
| **Quick run command** | the single-crate filter for the crate the task touched |
| **Full suite command** | every command in the map below, then `cargo check -p vox-ml-cli --tests` |
| **Estimated runtime** | ~120 seconds |

---

## Sampling Rate

- **After every task commit:** Run that task's automated commands
- **After every plan wave:** Run every command in the map for plans in that wave
- **Before `/gsd-verify-work`:** Every command in the map must be green
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 313-01-01 | 01 | 1 | LANE-01 | — | `Lane::parse` accepts only `codegen`, `hub`, `agent` | unit | `cargo test -p vox-eval qwen_reasoning -- --nocapture` | ❌ created in task | ⬜ pending |
| 313-01-01 | 01 | 1 | LANE-01 | — | Hub `/generate` reaches the worker payload as `"lane":"hub"` | unit | `cargo test -p vox-ml-cli do_generate_hub_lane_reaches_the_worker_payload` | ❌ created in task | ⬜ pending |
| 313-01-01 | 01 | 1 | LANE-01 | T-313-03 | `inference_payload` always emits `lane` | unit | `cargo test -p vox-ml-cli the_payload_carries` | ❌ created in task | ⬜ pending |
| 313-01-02 | 01 | 1 | LANE-01 | T-313-01 | Unknown lane on `/generate` is HTTP 400 before any worker call | unit | `cargo test -p vox-ml-cli unknown_lane` | ❌ created in task | ⬜ pending |
| 313-01-02 | 01 | 1 | LANE-01 | — | Repair retries keep the request lane | unit | `cargo test -p vox-ml-cli do_generate_repair_retries_keep_the_lane` | ❌ created in task | ⬜ pending |
| 313-01-02 | 01 | 1 | LANE-01 | — | Serve tests compile after the `lane` field is added | unit | `cargo check -p vox-ml-cli --tests` | ✅ | ⬜ pending |
| 313-02-01 | 02 | 2 | LANE-01 | — | Shared raw-prompt builder is lane-aware and codegen stays byte-identical | unit | `cargo test -p vox-plugin-mens-candle-core chatml_prefix_for_lane` | ❌ created in task | ⬜ pending |
| 313-02-01 | 02 | 2 | LANE-01 | — | Existing training-text fusion tests still pass | unit | `cargo test -p vox-plugin-mens-candle-core training_text` | ✅ | ⬜ pending |
| 313-02-02 | 02 | 2 | LANE-01 | — | CUDA and Metal `assemble_prompt` follow the lane; already-ChatML prompts stay unchanged | unit | `cargo test -p vox-plugin-mens-candle-cuda assemble_prompt` | ✅ / ❌ new | ⬜ pending |
| 313-02-02 | 02 | 2 | LANE-01 | — | Metal twin of the CUDA assemble tests | unit | `cargo test -p vox-plugin-mens-candle-metal assemble_prompt` | ✅ / ❌ new | ⬜ pending |
| 313-02-02 | 02 | 2 | LANE-01 | — | Plugin rejects an unknown lane string | unit | `cargo test -p vox-plugin-mens-candle-cuda resolve_lane` | ❌ created in task | ⬜ pending |
| 313-03-01 | 03 | 2 | LANE-01 | — | Chat flatten is codegen-identical by default and bare-thinking for hub and agent | unit | `cargo test -p vox-ml-cli flatten_chat_messages` | ✅ / ❌ new | ⬜ pending |
| 313-03-01 | 03 | 2 | LANE-01 | — | `ChatCompletionRequest.lane` defaults to none and accepts `agent` | unit | `cargo test -p vox-ml-cli chat_completion_request_` | ❌ created in task | ⬜ pending |
| 313-03-02 | 03 | 2 | LANE-01 | T-313-07 | Unknown lane on chat is HTTP 400 before the worker | unit | `cargo test -p vox-ml-cli unknown_lane` | ❌ created in task | ⬜ pending |
| 313-03-02 | 03 | 2 | LANE-01 | — | Hub chat reaches the worker as a bare thinking prompt | unit | `cargo test -p vox-ml-cli do_chat_completions` | ✅ / ❌ new | ⬜ pending |
| 313-04-01 | 04 | 3 | LANE-02 | — | Empty think stays outside the loss; a non-empty think body is not rewritten | unit | `cargo test -p vox-eval qwen_reasoning` | ✅ / ❌ new | ⬜ pending |
| 313-04-01 | 04 | 3 | LANE-02 | — | Candle-core message-row reasoning lock | unit | `cargo test -p vox-plugin-mens-candle-core training_text` | ✅ / ❌ new | ⬜ pending |
| 313-04-02 | 04 | 3 | LANE-02 | — | Same fusion locks on the populi copy, feature `mens-train` | unit | `cargo test -p vox-populi --features mens-train training_text` | ❌ created in task | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. New tests are written inside the tasks that need them (TDD), not as a separate wave.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

## Validation Audit 2026-10-26

| Metric | Count |
|---|---|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
