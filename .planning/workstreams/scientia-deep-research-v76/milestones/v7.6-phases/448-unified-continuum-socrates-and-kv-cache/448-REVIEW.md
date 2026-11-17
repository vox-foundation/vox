---
phase: 448-unified-continuum-socrates-and-kv-cache
review_scope: plan 448-01 (Tasks 1, 2, 3 & Wave C wiring)
reviewed_at: 2026-11-15
reviewer: rust-verifier
status: approved_with_notes
---

# Phase 448 Code & Architecture Review

## Scope Reviewed

- **KV-Cache Stabilization (`CONT-04`):** `crates/vox-orchestrator-mcp/src/chat_tools/mod.rs`, `crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs`.
- **Continuum Routing, CRAG Escalation & Domain Modes (`CONT-01..03`):** `crates/vox-research-shim/src/research/orchestrator/{domain_modes.rs,mod.rs,pipeline.rs}`, `crates/vox-orchestrator-mcp/src/{dispatch_timeout.rs,input_schemas/chat_and_memory.rs}`, `crates/vox-orchestrator-mcp/src/chat_tools/{params.rs,chat/message.rs,chat/message/posture_and_routing.rs,chat/message/preamble_and_retrieval.rs,chat/message/tests.rs,chat/research_intent.rs,chat/research_turn.rs,chat/history.rs}`, `crates/vox-orchestrator-mcp/tests/autonomous_chat_research_test.rs`.
- **Axis UI & Tauri Command Forwarding (`CONT-01`, `CONT-02`):** `crates/vox-gui/src/commands/chat_turn.rs`, `crates/vox-gui/ui/src/{App.tsx,transport.ts,types/tauri.ts,lib/buildChatTurn.ts,lib/buildChatTurn.test.ts,components/surfaces/Chat/ResearchModeDropdown.tsx,components/surfaces/Chat/ResearchModeDropdown.test.tsx}`.

## Findings Summary

- **Blocking Issues:** `0`
- **Phase 448 Architectural / Guard Violations:** `0`
- **Non-Blocking Findings:** `3` noted below (unrelated `pty/tests.rs:141` arch-check failure, 2 dead-code warnings in `system_prompt_vars.rs`, and compile-only status of `autonomous_chat_research_test`)

## 1. Boundary, Security & Policy Compliance

- **KV-Cache Prefix Purity:** `build_system_prompt_with_skill` preserves its public signature and uses `interpolate_static_system_prompt_variables`; `attention_budget` and `OperatingMode` riders are absent from the cached system prefix and delivered on `turn_user_prompt` via `build_volatile_turn_system_rider`. Verified by `kv_cache_prefix_stability_across_varying_turns`.
- **Crypto Compliance:** `chat_tools/mod.rs` migrated off direct `sha2::Sha256` to `vox_crypto::interop_sha256` + `vox_crypto::hex_encode`. Zero direct `sha2`/`sha3`/`blake3` imports remain in any Phase 448 touched `.rs` file.
- **Zero-`VoxDb` GUI Invariant:** `crates/vox-gui/src/commands/chat_turn.rs` forwards `research_mode` and `waves` strictly over `sync_tool_args` (`vox_chat_message`); `chat_turn_module_has_no_direct_db_access` passes.
- **UI Primitive & Source Guards:** `ResearchModeDropdown.tsx` uses Radix `Popover`/`Tooltip` + `Icon.*`; `noRawNativeControlsGuard.test.ts`, `zeroEmojiAndDingbat.test.ts`, and `transportIpcGuard.test.ts` all pass (`exit 0`).

## 2. File Size & Dependency Discipline

| New Module | Observed LoC | Budget | Status |
|---|---|---|---|
| `crates/vox-research-shim/src/research/orchestrator/domain_modes.rs` | `161` | `<400` | Pass |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs` | `399` | `<400` | Pass |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ResearchModeDropdown.tsx` | `156` | `<400` | Pass |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ResearchModeDropdown.test.tsx` | `92` | `<400` | Pass |

- Zero new Cargo crates or npm packages added; zero banned `"/tmp|"/var|"/home` literals in any Phase 448 file.

## 3. Non-Blocking Observations

1. **`vox-arch-check` exited `1` on unrelated `pty` file:** `cargo run -p vox-arch-check` flagged `[no-hardcoded-abs-path] crates/vox-gui/src/commands/pty/tests.rs:141 — "/tmp`, which belongs to an unrelated concurrent/pre-existing `pty` module outside Phase 448/449 scope. Zero violations exist in any Phase 448 file.
2. **Dead-code warnings in `system_prompt_vars.rs`:** `VolatileTurnContext` (`line 48`) and `VolatileTurnContext::render` (`line 56`) emit `#[warn(dead_code)]` notices during `vox-orchestrator-mcp` compilation because callers call `build_volatile_turn_system_rider` directly. Safe to remove or wire cleanly in a later pass.
3. **`tests/autonomous_chat_research_test.rs` compile-checked only:** Verified compilation under `cargo test -p vox-orchestrator-mcp --lib --tests --no-run`; execution deferred from offline verification because the test invokes live `chat_message` routing.

## Verdict

**APPROVED** — Phase 448 delivers orthogonal continuum routing, warm-seeded Socrates CRAG escalation, 4-mode domain execution, and day-stable KV-cache prompt prefixes with all scoped unit/frontend/type checks passing.
