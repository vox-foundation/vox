---
phase: 453
phase_name: "deep-research-reasoning-budget-separation-citation-scoped-au"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 6
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 453 Learnings: deep-research-reasoning-budget-separation-citation-scoped-au

## Decisions

### Cap OpenRouter synthesis reasoning tokens at 4_000 (candidate.reasoning_...
Cap OpenRouter synthesis reasoning tokens at `4_000` (`candidate.reasoning_max_tokens = Some(4_000)`) while allocating `synthesis_max_tokens >= 12_000` so reasoning tokens never starve visible synthesis output.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 453-01-SUMMARY.md

---

### Preserve full Markdown table rows as prose-safe masked spans and respect...
Preserve full Markdown table rows as prose-safe masked spans and respect sentence boundaries before Markdown list/heading markers in `claim_extractor/pipeline.rs`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 453-01-SUMMARY.md

---

### Key research_cache_key on normalized query, scope, max_sources, verify_c...
Key `research_cache_key` on normalized query, `scope`, `max_sources`, `verify_claims`, `lane`, `domain_mode`, `waves`, and `site_scope`, while refusing to store or serve empty or `[INSUFFICIENT_EVIDENCE]` responses.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 453-01-SUMMARY.md

---

### Bypass canonical daemon proxying whenever an explicit non-empty VOX_DB_P...
Bypass canonical daemon proxying whenever an explicit non-empty `VOX_DB_PATH` override is set so isolated/test DBs fail loudly rather than hitting the daemon.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 453-02-SUMMARY.md

---

### Keep daemon_db_proxy.rs at 397 LoC (< 400 LoC module budget) with determ...
Keep `daemon_db_proxy.rs` at `397` LoC (< `400` LoC module budget) with deterministic unit tests covering lock error matching, envelope unwrapping, and payload rendering.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 453-02-SUMMARY.md

## Lessons

## Patterns

### Quote-backed citation audit checks referenced [N] indices first and fall...
Quote-backed citation audit checks referenced `[N]` indices first and falls back to full source range only when no inline citations exist.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 453-01-SUMMARY.md

---

### OpenRouter reasoning budget separation via WireReasoningConfig (reasonin...
`OpenRouter` reasoning budget separation via `WireReasoningConfig` (`reasoning: { max_tokens: 4000 }`) alongside `max_tokens: 12000`

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 453-01-SUMMARY.md

---

### extract_referenced_citation_indices scoping audit_citations to actually ...
`extract_referenced_citation_indices` scoping `audit_citations` to actually cited `[N]` / `[lo-hi]` sources with `quote_overlaps` & lexical containment checks

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 453-01-SUMMARY.md

---

### CLI research & scientia commands degrade cleanly from local VoxDb::conne...
CLI research & scientia commands degrade cleanly from local `VoxDb::connect` to `daemon_tool::daemon_client()` when `vox-orchestrator-d` holds `vox.db`.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 453-02-SUMMARY.md

---

### is_canonical_db_lock_error gates IPC fallback on unset/empty VOX_DB_PATH...
`is_canonical_db_lock_error` gates IPC fallback on unset/empty `VOX_DB_PATH` + Turso/POSIX lock substrings (`failed locking file`, `databasebusy`, `database is locked`, `wouldblock`)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 453-02-SUMMARY.md

---

### 180s polling loop over orch.api_v2_get (/research/sessions/{id}) after s...
180s polling loop over `orch.api_v2_get` (`/research/sessions/{id}`) after submitting `dei_method::RESEARCH_RUN`

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 453-02-SUMMARY.md

## Surprises

