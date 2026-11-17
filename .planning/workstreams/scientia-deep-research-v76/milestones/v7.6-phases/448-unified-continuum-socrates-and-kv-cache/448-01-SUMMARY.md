---
phase: 448-unified-continuum-socrates-and-kv-cache
plan: 01
subsystem: orchestrator-mcp
tags: [rust, typescript, react, continuum, socrates, kv-cache, domain-modes, vox-orchestrator-mcp, vox-research-shim, vox-gui]

# Dependency graph
requires:
  - phase: 447-before-audit-baseline-and-continuum-contracts
    provides: Pure L0 continuum enums (`ResearchContinuumMode`, `ResearchContinuumTier`, `EscalationReason`, `CragEscalationPolicy`), `ResearchDomainMode::Scholarly`, and immutable `before` baseline
provides:
  - Day-stable KV-cache system prompt prefix (`build_system_prompt_with_skill` + `interpolate_static_system_prompt_variables`) with turn-volatile context moved to `build_volatile_turn_system_rider` on `turn_user_prompt`
  - Orthogonal `research_mode` (`off|auto|fast|deep`), `research_scope` (`local|web|both`), `domain_mode` (`general|shopping|codegen|scholarly`), `site_scope`, and `waves` (`1..=5`) propagation from Axis UI -> Tauri `ChatTurnInput` -> `vox_chat_message` -> `vox-research-shim`
  - Warm-seeded `T1 -> T2 -> T3` CRAG auto-escalation (`run_research_with_warm_seed`, `evaluate_continuum_escalation`) and unified `domain_modes.rs` across all 4 domain modes
affects:
  - Phase 449–452 (`STRM`, `VERI`, `DISC`, `PUBL`, `AUDT-02` paired `--stage after` diff)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Day-stable KV-cache prefix invariant: volatile `AttentionBudget`, `OperatingMode` rider, and `$active_file` injected solely into `turn_user_prompt` via `build_volatile_turn_system_rider`"
    - "Warm-seeded multi-tier continuum: `LocalRag -> QuickWeb -> DeepMultiWave` carries prior hits forward via `run_research_with_warm_seed` without re-fetching URLs"
    - "Orthogonal frontend/backend wire contract: `research_mode` + `waves` serialized alongside `research_scope`, `domain_mode`, and `site_scope` using nullish coalescing (`??`) in `App.tsx`"

key-files:
  created:
    - crates/vox-research-shim/src/research/orchestrator/domain_modes.rs
    - crates/vox-gui/ui/src/components/surfaces/Chat/ResearchModeDropdown.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/ResearchModeDropdown.test.tsx
  modified:
    - crates/vox-orchestrator-mcp/src/chat_tools/mod.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/params.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/message/posture_and_routing.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/message/preamble_and_retrieval.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/message/tests.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/research_intent.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs
    - crates/vox-orchestrator-mcp/src/dispatch_timeout.rs
    - crates/vox-orchestrator-mcp/src/input_schemas/chat_and_memory.rs
    - crates/vox-orchestrator-mcp/tests/autonomous_chat_research_test.rs
    - crates/vox-research-shim/src/research/orchestrator/mod.rs
    - crates/vox-research-shim/src/research/orchestrator/pipeline.rs
    - crates/vox-gui/src/commands/chat_turn.rs
    - crates/vox-gui/ui/src/App.tsx
    - crates/vox-gui/ui/src/lib/buildChatTurn.ts
    - crates/vox-gui/ui/src/lib/buildChatTurn.test.ts
    - crates/vox-gui/ui/src/transport.ts
    - crates/vox-gui/ui/src/types/tauri.ts

key-decisions:
  - "Preserved `pub async fn build_system_prompt_with_skill` signature identically across callers while moving volatile per-turn fields into `build_volatile_turn_system_rider`."
  - "Retained legacy `research_scope` mode inference strictly when `research_mode` is `None` (`classify_research_intent_from_params`), preventing regressions on legacy callers."
  - "Migrated `chat_tools/mod.rs` off direct `sha2::Sha256` onto `vox_crypto::interop_sha256` + `vox_crypto::hex_encode`."

patterns-established:
  - "`merge_persisted_posture_into_params` merges session `research_mode` and `domain_mode` before `classify_research_intent_from_params` and `prepare_turn_context_and_retrieval`."

requirements-completed: [CONT-01, CONT-02, CONT-03, CONT-04]

# Metrics
duration: 65min
completed: 2026-11-15
---

# Phase 448 Plan 01: Unified Light ↔ Deep Continuum, Socrates Escalation & KV-Cache Summary

**Phase 448 Plan 01 unified Chat and Deep Research behind one orthogonal 4-tier continuum (`Off | Auto | Fast | Deep`), wired warm-seeded `T1 -> T2 -> T3` CRAG escalation across all 4 domain modes, and stabilized day-level KV-cache system prompt prefixes.**

## Accomplishments

### Task 1: Day-Stable KV-Cache Prefix & Volatile Rider Extraction (`CONT-04`)
- Refactored `crates/vox-orchestrator-mcp/src/chat_tools/mod.rs` (`728` LoC) and `crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs` (`399` LoC, `< 400`) to keep `build_system_prompt_with_skill` signature intact while stripping per-turn `attention_budget` (`## Budget Status`), `OperatingMode` riders, and volatile tokens via `interpolate_static_system_prompt_variables`.
- Added `build_volatile_turn_system_rider`, `resolve_session_operating_mode`, and `interpolate_volatile_tokens` so `Attention Budget Signal`, active file/spoke, model ID, cognitive profile, and turn timestamp inject strictly into `turn_user_prompt`.
- Verified `>= 0.99` byte-prefix similarity across 5 varying turns in `kv_cache_prefix_stability_across_varying_turns` and replaced pre-existing direct `sha2::Sha256` in `chat_tools/mod.rs` with `vox_crypto::interop_sha256`.

### Task 2: Orthogonal Continuum Router, Socrates Escalation & Domain Modes (`CONT-01..03`)
- Added `crates/vox-research-shim/src/research/orchestrator/domain_modes.rs` (`161` LoC, `< 400`) and wired `run_research_with_warm_seed` + `merge_warm_seed_hits` in `pipeline.rs`, unifying `General`, `CodeGen`, `Shopping`, and `Scholarly` query expansion, hit scoring, and synthesis decoration.
- Extended `ChatMessageParams` (`params.rs`), `PersistedSessionChatPosture` (`posture_and_routing.rs`), `vox_chat_message` / `vox_research_*` schemas (`input_schemas/chat_and_memory.rs`), and continuum timeout scaling (`dispatch_timeout.rs`) with `research_mode` (`off|auto|fast|deep`) and `waves` (`1..=5`).
- Updated `research_intent.rs`, `research_turn.rs`, `preamble_and_retrieval.rs`, and `message.rs` to merge persisted posture before classification, support `waves: 1..=5`, persist continuum metadata via `VoxDb::update_session_continuum_metadata`, and execute `LocalRag -> QuickWeb -> DeepMultiWave` warm-seeded CRAG escalation without duplicate URL fetches.
- Completed Wave C exhaustive struct-literal updates in `history.rs`, `message/tests.rs`, and `tests/autonomous_chat_research_test.rs`.

### Task 3: Axis Composer Continuum UI & Rust Tauri Command Wiring (`CONT-01`, `CONT-02`)
- Created `crates/vox-gui/ui/src/components/surfaces/Chat/ResearchModeDropdown.tsx` (`156` LoC, Radix `Popover` + `Tooltip` + `Icon.*`, zero raw `<select>`/`<input type="range">`, zero emojis/dingbats) and `ResearchModeDropdown.test.tsx` (`92` LoC).
- Updated `App.tsx` (`submitFromComposer` nullish-coalesced overrides preserving `research_scope`), `buildChatTurn.ts`, `transport.ts`, and `types/tauri.ts`.
- Wired `research_mode: Option<String>` and `waves: Option<usize>` into `ChatTurnInput` and `sync_tool_args` in `crates/vox-gui/src/commands/chat_turn.rs`, adding `test_sync_tool_args_forwards_research_mode_and_waves_orthogonally` and updating the 144-case `full_knob_matrix_roundtrips_across_sync_background_and_plan_tracks`.

## Verification Receipts (Observed Outputs)

1. **`vox-research-shim --lib`:** `163 passed; 0 failed; 0 ignored; 0 measured; finished in 39.17s`
2. **`vox-orchestrator-mcp --lib --tests --no-run`:** `exit 0` (`4m 25s`, 4 non-blocking dead-code warnings)
   - `./target/debug/deps/vox_orchestrator_mcp-caf61828b67e474b chat_tools`: `226 passed; 0 failed; 0 ignored; 825 filtered out; finished in 60.15s`
   - `./target/debug/deps/vox_orchestrator_mcp-caf61828b67e474b dispatch_timeout`: `17 passed; 0 failed; 0 ignored; 1034 filtered out; finished in 23.26s`
   - `./target/debug/deps/vox_orchestrator_mcp-caf61828b67e474b input_schemas`: `15 passed; 0 failed; 0 ignored; 1036 filtered out; finished in 0.01s`
   - `tests/autonomous_chat_research_test.rs`: compiled cleanly (`--no-run` `exit 0`; not executed offline as it invokes live `chat_message`)
3. **`vox-gui --bins --no-run`:** `exit 0` (`25m 01s`)
   - `./target/debug/deps/vox_gui-e384194f9b0254e9 chat_turn`: `18 passed; 0 failed; 0 ignored; 374 filtered out; finished in 0.03s`
4. **Frontend TypeScript & Combined Vitest:**
   - `pnpm --dir crates/vox-gui/ui exec tsc --noEmit`: `exit 0`
   - Combined Vitest (`7` files): `64 passed; 0 failed` (`ResearchModeDropdown.test.tsx`: `8`, `buildChatTurn.test.ts`: `13`, `noRawNativeControlsGuard.test.ts`: `2`, `zeroEmojiAndDingbat.test.ts`: `2`, `transportIpcGuard.test.ts`: `2`, plus Phase 449 `ResearchDagCanvas.test.tsx` `4` & `ResearchView.test.tsx` `33`)
5. **Architecture & Static Guards:**
   - All new `.rs`/`.tsx` modules `< 400` LoC (`domain_modes.rs`: `161`, `system_prompt_vars.rs`: `399`, `ResearchModeDropdown.tsx`: `156`, `ResearchModeDropdown.test.tsx`: `92`).
   - `rg` banned OS path scan (`"/tmp|"/var|"/home`), direct crypto import scan (`sha2|sha3|blake3` outside `vox-crypto`), and `vox-gui` DB handle check: `0` Phase 448 violations.
   - `cargo run -p vox-arch-check`: exited `1` solely due to 1 unrelated concurrent/pre-existing violation at `crates/vox-gui/src/commands/pty/tests.rs:141` (`[no-hardcoded-abs-path] "/tmp`); `0` violations in Phase 448 files (`12` pre-existing release-delta LoC warnings).

---
*Phase: 448-unified-continuum-socrates-and-kv-cache*
*Completed: 2026-11-15*
