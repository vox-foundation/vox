---
phase: 447
phase_name: "before-audit-baseline-and-continuum-contracts"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 7
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 447 Learnings: before-audit-baseline-and-continuum-contracts

## Decisions

### Establish the pure L0 and L1 type contracts required across SCIENTIA Dee...
Establish the pure L0 and L1 type contracts required across SCIENTIA Deep Research v7.6: 1. Add `ResearchContinuumMode::{Off, Auto, Fast, Deep}`, `ResearchContinuumTier`, `EscalationReason`, and `CragEscalationPolicy` in a dedicated `<400`-LoC child module `crates/vox-orchestrator-types/src/socrates_policy/continuum.rs` (zero new dependencies) plus `ResearchDomainMode::Scholarly` in `vox-research-shim::research::types` (updating all exhaustive match arms so workspace compilation stays green). 2. Add `SequencedResearchEvent` (`seq: u64` monotonic watermark + `ResearchEventSequencer`) and five continuum/wave/DAG `ResearchEvent` variants (`WaveStarted`, `WaveCompleted`, `SubqueryDispatched`, `ContinuumEscalated`, `EpistemicDagDelta`) in L1 `vox-research-events`, and harden L1 `vox_research_events::schema_types::NoveltyEvidenceBundle` as the single canonical novelty bundle representation for `DISC-01`. Purpose: Unblock downstream Wave 2 (`447-02`), Wave 3 `vox-publisher` novelty consolidation (`447-03`), and Phases 448–452 without introducing any `layers.toml` or `crate-edges` violations. Output: Tested L0/L1 continuum, domain, sequenced event, and novelty bundle contracts.

**Rationale:** Primary phase objective established in plan.
**Source:** 447-01-PLAN.md

---

### Deliver Wave 2 of Phase 447 after 447-01 completes — the L3 storage and ...
Deliver Wave 2 of Phase 447 after `447-01` completes — the L3 storage and embedding foundations for SCIENTIA Deep Research v7.6: 1. Add additive `VoxDb` baseline `v99` schema (`research_session_sources` table + `continuum_tier` and `escalated_from` columns on `scientia_research_sessions` and `research_sessions`, PRAGMA-guarded idempotent `ALTER TABLE` migration on open, `contracts/db/baseline-version-policy.yaml` sync, and `codex_legacy.rs` export table parity) plus `<400`-LoC cursor-paginated queries `list_session_claims_page` and `list_session_sources_page` in `crates/vox-db/src/research_session_pages.rs`. 2. Lift `Embedder`, `cosine_similarity`, `embed_cache_key` (via `vox-crypto`), `LocalHashEmbedder` (deterministic pure-Rust feature-hashed fallback), and `CachedLlmEmbedder` (`vox_actor_runtime::llm::llm_embed` + `VoxDb` `scientia_embedding_cache`) into `<400`-LoC `crates/vox-scientia/src/embedder.rs` (`DISC-01`). Purpose: Enable bounded IPC pagination for 100+ source / 200+ claim sessions and share `CachedLlmEmbedder` across CLI, MCP, orchestrator, and publisher from L3 `vox-scientia` with zero upward dependencies. Output: Tested `VoxDb` v99 schema + cursor pagination module and L3 `vox-scientia::embedder` module.

**Rationale:** Primary phase objective established in plan.
**Source:** 447-02-PLAN.md

## Lessons

### Authorized Mechanical Build Unblock (Step A0)
**Authorized Mechanical Build Unblock (Step A0):** Two unrelated pre-existing compile blockers in `crates/vox-orchestrator-mcp` (`dispatch_domains/terminal_tools.rs` lines ~57 & ~386 `scrub_terminal_mcp_str` `Option<String>` mismatch, and `chat_tools/chat/term_mentions.rs` lines ~99 & ~184 match arm / borrow split) were fixed under `@coordinator` authorization (`#62`) and formatted with `rustfmt --edition 2024`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 447-03-SUMMARY.md

---

### Non-blocking Cleanup Note
**Non-blocking Cleanup Note:** `crates/vox-cli-research/src/db/publication/embedder.rs:3` emits a single compiler unused-imports warning during `vox-cli-research` compilation; functionality and re-exports are unaffected.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 447-03-SUMMARY.md

---

### Blocking Issues
**Blocking Issues:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 447-REVIEW.md

---

### Architectural Violations (vox-arch-check)
**Architectural Violations (`vox-arch-check`):** `0` errors (12 pre-existing release-delta LoC warnings across workspace crates; no new violations introduced)

**Context:** Surfaced in structured phase code review.
**Source:** 447-REVIEW.md

---

### Non-Blocking Findings
**Non-Blocking Findings:** `3` documented scope/hygiene notes below (no phase blockers)

**Context:** Surfaced in structured phase code review.
**Source:** 447-REVIEW.md

---

### vox-orchestrator-types
`vox-orchestrator-types`: `51 + 3 + 5 + 2` tests passed

**Context:** Verified in phase acceptance checks.
**Source:** 447-VERIFICATION.md

---

### vox-research-events
`vox-research-events`: `37` tests passed

**Context:** Verified in phase acceptance checks.
**Source:** 447-VERIFICATION.md

## Patterns

## Surprises

