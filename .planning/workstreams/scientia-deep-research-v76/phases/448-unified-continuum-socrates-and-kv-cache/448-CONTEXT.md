---
phase: 448
title: "Unified Continuum, Socrates, and KV-Cache"
slug: "unified-continuum-socrates-and-kv-cache"
milestone: "scientia-deep-research-v76"
status: "ready"
---

# Phase 448: Unified Continuum, Socrates, and KV-Cache — Context

**Gathered:** 2026-11-15
**Status:** Ready for planning
**Requirements:** CONT-01, CONT-02, CONT-03, CONT-04

<domain>
## Phase Boundary

Phase 448 unifies Vox's chat/research routing into a single orthogonal continuum (`ResearchContinuumMode` × `ResearchDomainMode` × `ResearchScope` × `waves`), wires Socrates pre-classification and warm-seeded CRAG escalation (`LocalRag → QuickWeb → DeepMultiWave`), enforces posture-before-retrieval ordering in `chat_message`, and stabilizes the static KV-cache system prompt prefix across multi-turn sessions.

Scope includes ONLY:
- `crates/vox-orchestrator-mcp/src/chat_tools/` (`mod.rs`, `params.rs`, `chat/system_prompt_vars.rs`, `chat/message.rs`, `chat/message/posture_and_routing.rs`, `chat/message/preamble_and_retrieval.rs`, `chat/research_intent.rs`, `chat/research_turn.rs`, `dispatch_timeout.rs`, `input_schemas/chat_and_memory.rs`)
- `crates/vox-research-shim/src/research/orchestrator/` (`mod.rs`, `pipeline.rs`, new `domain_modes.rs`)
- `crates/vox-gui/` composer continuum wiring (`ui/src/App.tsx`, `ui/src/components/surfaces/Chat/ResearchModeDropdown.tsx`, `ui/src/lib/buildChatTurn.ts`, `ui/src/transport.ts`, `ui/src/types/tauri.ts`, `src/commands/chat_turn.rs`)

Explicitly deferred to Phase 449+: SSE/WS telemetry streaming & chunk spill (`STRM-01..04`), claim verification & novelty hardening (Phase 450), MENS corpus/bench (Phase 451), and final before/after audit verification (Phase 452).
</domain>

<decisions>
## Implementation Decisions

### Orthogonal Continuum Routing (`CONT-01`)
- **Explicit separation of axes:** Never overload `research_scope` (`"deep"` / `"quick"` / `"auto"`) to express depth/mode. Introduce dedicated `research_mode: Option<String>` (`off | auto | fast | deep`) and `waves: Option<usize>` (`1..=5`, clamped) alongside orthogonal `research_scope` (`web | local | both`), `domain_mode` (`general | codegen | shopping | scholarly`), and `site_scope`.
- **Legacy backward compatibility:** `classify_research_intent` and `ChatMessageParams` continue accepting legacy `research_scope = "deep" | "quick" | "none"` from older callers as fallback aliases mapped onto `ResearchContinuumMode` when `research_mode` is `None`, while emitting canonical `ResearchContinuumMode` and `ResearchScope` cleanly.
- **No silent overwrites in composer:** In `crates/vox-gui/ui/src/App.tsx` (`submitFromComposer`), respect caller-provided `p.research_mode ?? researchMode`, `p.force_research ?? ...`, and `p.research_scope ?? ...` so explicit overrides (such as `/research` / `/deepresearch` or E2E fixtures, verified by `bug_probes::probe_composer_scope_overwritten`) are never silently clobbered.
- **Dynamic query propagation in `research_turn.rs`:** `run_deep` must no longer hardcode `domain_mode: ResearchDomainMode::General`, `scope: ResearchScope::Web`, `site_scope: None`, and `waves: 1` (verified by `bug_probes::probe_research_turn_hardcoded`); all four fields come from `ResearchIntent` / continuum routing (`waves` defaults to `3` in `DeepMultiWave` and accepts `1..=5`).

### Socrates Pre-Classification & Warm-Seeded Escalation (`CONT-02`)
- **Contract reuse:** Use `vox_orchestrator_types::socrates_policy::continuum::{ResearchContinuumMode, ResearchContinuumTier, EscalationReason, CragEscalationPolicy}` (landed in Phase 447) as the single source of truth — do not duplicate thresholds (`0.68`, `0.25`, `0.55`, `min_sources: 3`, `default_max_waves: 3`).
- **Warm-seeded escalation:** When `Auto` / `Fast` starts at `LocalRag` or `QuickWeb` and `CragEscalationPolicy::evaluate_escalation` returns `Some(reason)`, carry the already-fetched `ResearchHit`s (`warm_seed_hits`) forward into the escalated `QuickWeb` / `DeepMultiWave` pipeline instead of discarding them and re-querying from scratch.
- **All four domains first-class:** Extract unified domain behavior (`General`, `CodeGen`, `Shopping`, `Scholarly`) into `crates/vox-research-shim/src/research/orchestrator/domain_modes.rs` (`< 400` LoC) so `Scholarly` receives full multi-wave synthesis instructions, authoritative-domain boosting, and citation rules alongside `General`, `CodeGen`, and `Shopping`.
- **Metadata persistence:** Record `continuum_tier`, `escalated_from`, and `escalation_reason` via `VoxDb::update_session_continuum_metadata` (landed in Phase 447 `research_session_pages.rs`).

### Posture-Before-Retrieval Ordering (`CONT-03`)
- In `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`, resolve and merge `PersistedSessionChatPosture` (`merge_posture_opt!` including `research_mode`, `domain_mode`, `kb_scope`, `tier`, `clutch`, `risk`, `grounding_check_enabled`) **before** calling `classify_research_intent_with_socrates` and `prepare_turn_context_and_retrieval`, guaranteeing retrieval scope (`KbScopeMask`) and research depth reflect the effective turn posture on the very first pass.

### Day-Stable KV-Cache System Prompt Prefix (`CONT-04`)
- **Static prefix separation:** `build_system_prompt_with_skill` (`crates/vox-orchestrator-mcp/src/chat_tools/mod.rs`) MUST produce a day-stable prefix (`>= 99%` byte prefix overlap across multi-turn sessions as measured by `bug_probes::probe_kv_cache_prefix_stability`). Remove per-turn `attention_budget` (`mod.rs:341-356`), per-turn `env.operating_mode.system_rider()` (`mod.rs:369-373`), and per-turn `active_file` / `session_id` / `model_id` / `cognitive_profile` interpolation from `build_system_prompt_with_skill`.
- **Volatile turn rider:** Add `build_volatile_turn_system_rider(...)` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs` to format per-turn volatile state (`attention_budget`, `operating_mode`, `active_file`, `session_id`, `model_id`, `cognitive_profile`, `current_datetime`) as a trailing rider appended **after** the immutable system prompt prefix (or injected into the trailing user turn preamble) without re-interpolating `raw_system_prompt` in `message.rs:303-320`.

### Architectural & Repo Invariants
- Zero new Cargo or npm dependencies (`Cargo.toml`, `Cargo.lock`, `package.json` untouched).
- Every new `.rs` / `.tsx` file stays strictly under 400 LoC.
- 100% of LLM calls route through `vox_actor_runtime::llm`.
- Zero raw `<select>`, `<input type="range">`, emojis, or BMP dingbats in `crates/vox-gui/ui/src`.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Contracts (from Phase 447)
- `crates/vox-orchestrator-types/src/socrates_policy/continuum.rs` (374 LoC): `ResearchContinuumMode`, `ResearchContinuumTier`, `EscalationReason`, `CragEscalationPolicy`, `initial_tier_for_mode`, `has_coverage_paradox`, `evaluate_escalation`.
- `crates/vox-research-shim/src/research/types.rs`: `ResearchDomainMode::{General, Shopping, CodeGen, Scholarly}`, `ResearchScope::{Web, Local, Both}`, `ResearchQuery`.
- `crates/vox-db/src/research_session_pages.rs` (391 LoC): `update_session_continuum_metadata`, `get_session_continuum_metadata`.

### Bugs Confirmed by Live Inspection & `bug_probes.rs`
- `crates/vox-orchestrator-mcp/src/chat_tools/mod.rs:341-379`: `build_system_prompt_with_skill` embeds `attention_budget`, `env.operating_mode.system_rider()`, and interpolates `runtime_vars` into the base prompt.
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:142-146,194,295-320`: runs `classify_research_intent` before retrieval/posture completion and re-interpolates `raw_system_prompt` per turn with changing `active_file` / `session_id` / `model_id` / `cognitive_profile`.
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs:620-630`: `run_deep` hardcodes `scope: ResearchScope::Web`, `site_scope: None`, `domain_mode: ResearchDomainMode::General`, `waves: 1`.
- `crates/vox-gui/ui/src/App.tsx:1967-1974`: `submitFromComposer` unconditionally overwrites `p.force_research` and `p.research_scope`.

</code_context>

<specifics>
## Specific Ideas & Parallel Wave Boundaries

- **Wave B parallel file-disjoint workers:**
  1. `p448-kvcache` owns `chat_tools/mod.rs` + `chat_tools/chat/system_prompt_vars.rs`.
  2. `p448-router` owns `params.rs`, `message.rs`, `posture_and_routing.rs`, `preamble_and_retrieval.rs`, `research_intent.rs`, `research_turn.rs`, `dispatch_timeout.rs`, `input_schemas/chat_and_memory.rs`, `orchestrator/pipeline.rs`, `orchestrator/domain_modes.rs`, `orchestrator/mod.rs`.
  3. `p448-ui` owns `App.tsx`, `ResearchModeDropdown.tsx`, `ResearchModeDropdown.test.tsx`, `buildChatTurn.ts`, `buildChatTurn.test.ts`, `transport.ts`, `types/tauri.ts`.
- **Wave C (`rust-verifier`)**: wires the small `research_mode` + `waves` serde passthrough in `crates/vox-gui/src/commands/chat_turn.rs`, and runs serialized Cargo tests + UI TypeScript/Vitest/guard checks.

</specifics>

<deferred>
## Deferred Ideas

- Cross-session persistent prompt prefix caching headers for external OpenAI/Anthropic APIs beyond local/generic system-prefix stability (`CONT-04` covers the deterministic prefix invariant and volatile rider split; provider cache-control annotations remain in `vox-actor-runtime`).
- Resumable research SSE/WS deltas and virtualized research canvas (Phase 449).

</deferred>

---

*Phase: 448-unified-continuum-socrates-and-kv-cache*
*Context gathered: 2026-11-15*
