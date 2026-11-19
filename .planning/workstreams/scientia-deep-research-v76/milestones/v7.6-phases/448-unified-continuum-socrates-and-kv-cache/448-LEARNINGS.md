---
phase: 448
phase_name: "unified-continuum-socrates-and-kv-cache"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 3
  lessons: 3
  patterns: 4
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 448 Learnings: unified-continuum-socrates-and-kv-cache

## Decisions

### Preserved pub async fn build_system_prompt_with_skill signature identica...
Preserved `pub async fn build_system_prompt_with_skill` signature identically across callers while moving volatile per-turn fields into `build_volatile_turn_system_rider`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 448-01-SUMMARY.md

---

### Retained legacy research_scope mode inference strictly when research_mod...
Retained legacy `research_scope` mode inference strictly when `research_mode` is `None` (`classify_research_intent_from_params`), preventing regressions on legacy callers.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 448-01-SUMMARY.md

---

### Migrated chat_tools/mod.rs off direct sha2::Sha256 onto vox_crypto::inte...
Migrated `chat_tools/mod.rs` off direct `sha2::Sha256` onto `vox_crypto::interop_sha256` + `vox_crypto::hex_encode`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 448-01-SUMMARY.md

## Lessons

### Blocking Issues
**Blocking Issues:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 448-REVIEW.md

---

### Phase 448 Architectural / Guard Violations
**Phase 448 Architectural / Guard Violations:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 448-REVIEW.md

---

### Non-Blocking Findings
**Non-Blocking Findings:** `3` noted below (unrelated `pty/tests.rs:141` arch-check failure, 2 dead-code warnings in `system_prompt_vars.rs`, and compile-only status of `autonomous_chat_research_test`)

**Context:** Surfaced in structured phase code review.
**Source:** 448-REVIEW.md

## Patterns

### merge_persisted_posture_into_params merges session research_mode and dom...
`merge_persisted_posture_into_params` merges session `research_mode` and `domain_mode` before `classify_research_intent_from_params` and `prepare_turn_context_and_retrieval`.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 448-01-SUMMARY.md

---

### Day-stable KV-cache prefix invariant
Day-stable KV-cache prefix invariant: volatile `AttentionBudget`, `OperatingMode` rider, and `$active_file` injected solely into `turn_user_prompt` via `build_volatile_turn_system_rider`

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 448-01-SUMMARY.md

---

### Warm-seeded multi-tier continuum
Warm-seeded multi-tier continuum: `LocalRag -> QuickWeb -> DeepMultiWave` carries prior hits forward via `run_research_with_warm_seed` without re-fetching URLs

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 448-01-SUMMARY.md

---

### Orthogonal frontend/backend wire contract
Orthogonal frontend/backend wire contract: `research_mode` + `waves` serialized alongside `research_scope`, `domain_mode`, and `site_scope` using nullish coalescing (`??`) in `App.tsx`

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 448-01-SUMMARY.md

## Surprises

