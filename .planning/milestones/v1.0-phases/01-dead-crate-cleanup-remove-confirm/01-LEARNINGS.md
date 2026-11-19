---
phase: 1
phase_name: "dead-crate-cleanup-remove-confirm"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 13
  lessons: 5
  patterns: 2
  surprises: 2
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 1 Learnings: dead-crate-cleanup-remove-confirm

## Decisions

### D-03 verification ran vox graph refresh --auto first (all 5 corpora were...
D-03 verification ran vox graph refresh --auto first (all 5 corpora were stale with git_drift+worktree_drift); coverage --kind crate still returned empty post-refresh, confirming RESEARCH.md's Open Question 2 as a real tool gap, not a cache staleness artifact

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-01-SUMMARY.md

---

### The plan's git_grep verify command for ROADMAP SC#2 (\b word-boundary re...
The plan's git_grep verify command for ROADMAP SC#2 (`\b` word-boundary regex) is non-portable to macOS/BSD grep — it silently produces 0 matches instead of the correct 3, because BSD grep -E does not support \b. Verified the actual requirement directly instead (ctx_search + git grep without \b, cross-checked against source).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-01-SUMMARY.md

---

### Machine-verified (not assumed from RESEARCH.md prose) that all 10 crates...
Machine-verified (not assumed from RESEARCH.md prose) that all 10 crates named in REQ-dead-crate-delete are absent from `cargo tree -p vox-cli` and `cargo metadata`

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-01-SUMMARY.md

---

### Machine-verified zero surviving vox_socrates_policy:
Machine-verified zero surviving `vox_socrates_policy::` imports and confirmed all 3 migrated types (`ConfidencePolicy`, `ComplexityBand`, `RiskBand`) are defined in `vox-orchestrator-types`

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-01-SUMMARY.md

---

### Machine-verified vox-plugin-publication reaches the ingest logic without...
Machine-verified `vox-plugin-publication` reaches the ingest logic without a `vox-cli` dependency and enables `vox-publisher`'s `scholarly-external-jobs` feature

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-01-SUMMARY.md

---

### Ran vox graph refresh --auto (all 5 corpora were stale) then vox graph c...
Ran `vox graph refresh --auto` (all 5 corpora were stale) then `vox graph coverage --kind crate` — confirmed the empty-result gap RESEARCH.md flagged is a real tool limitation, not cache staleness; fell back to per-crate `vox graph query` for all 10 names as the plan specifies (fuzzy/inconclusive results, consistent with but not superseding the cargo ground truth)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-01-SUMMARY.md

---

### Repointed the all-features check CI matrix from the deleted vox-scientia...
Repointed the `all-features check` CI matrix from the deleted `vox-scientia-ingest` to `vox-scientia`, verified `cargo check -p vox-scientia --all-features` is green, committed as an exact one-line diff (`1d81d7ae9`)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-01-SUMMARY.md

---

### Frozen-crate annotation placed as a new comment block between the file h...
Frozen-crate annotation placed as a new comment block between the file header and the first plugin section, not inside/near the unrelated execution-api/stub-check block -- keeps two distinct disposition records visually separate

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-02-SUMMARY.md

---

### Confirmed execution-api/stub-check have zero live catalog entries and ze...
Confirmed `execution-api`/`stub-check` have zero live catalog entries and zero matching workspace packages; the existing removal comment is present and accurate — REQ-dead-crate-catalog-cleanup was already satisfied, this plan only verified it

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-02-SUMMARY.md

---

### Confirmed all three KEEP-FROZEN crates (vox-workflow-runtime, vox-integr...
Confirmed all three KEEP-FROZEN crates (`vox-workflow-runtime`, `vox-integration-tests`, `vox-test-harness`) are git-tracked and untouched by any Phase 1 commit

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-02-SUMMARY.md

---

### Confirmed CatalogFile's schema (exactly 4 rename = "..." tables, no deny...
Confirmed `CatalogFile`'s schema (exactly 4 `rename = "..."` tables, no `deny_unknown_fields`) forces the D-02 annotation to be a comment, not a new table

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-02-SUMMARY.md

---

### Added the KEEP-FROZEN disposition comment block and verified the commit'...
Added the KEEP-FROZEN disposition comment block and verified the commit's diff is comment-only, single-file, and leaves the generated docs untouched

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-02-SUMMARY.md

---

### Placed the new block as its own section rather than appending to or near...
Placed the new block as its own section rather than appending to or near the execution-api/stub-check block, keeping the two disposition records (ghost-entry removal vs. frozen-crate retention) visually distinct despite both being "crate lifecycle" bookkeeping.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-02-SUMMARY.md

## Lessons

### Not caused by this task, resolved along the way
**Not caused by this task, resolved along the way:** the working tree had an in-progress, unrelated `git merge` (the operator's own `repo-history-graph` branch) whose conclusion initially failed a pre-commit hook: `crates/vox-research-shim/src/research/orchestrator/pipeline.rs` had 6 real compile errors from an incomplete upstream change (`synthesize_answer_with_llm` now returns `(String, bool)`; 5 call sites and a defined-but-never-applied `TEMPLATE_FALLBACK_QUALITY_CAP` constant were unwired). Fixed with the operator's explicit direction ("mechanical fix" — destructure and discard the flag, no behavior change) after presenting both a mechanical and a feature-completing option. A second, unrelated pre-existing compile break then surfaced (`ResearchLane` missing `FromStr`, blocking commits workspace-wide via the `command-sync` pre-commit hook regardless of merge status) — fixed after the operator chose "propose a fix, show me first" and approved it. The operator ultimately resolved the merge itself via `git merge --abort` in a separate, concurrent terminal session while these fixes were in flight; both fixes survived as local edits and were committed cleanly once the dust settled. Two of my own commits were silently reverted mid-attempt during this window (git index reset by the concurrent `git merge --abort`) and had to be reapplied and re-verified — no data was permanently lost, but it cost several retry cycles.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-01-SUMMARY.md

---

### SC#3's literal wording ("reachable from vox-publisher") doesn't match th...
SC#3's literal wording ("reachable from vox-publisher") doesn't match the actual host crate (vox-plugin-publication reaching vox-scientia via runtime plugin-host ABI dispatch, not vox-publisher). The substance is satisfied and exceeds the letter — operator ratified via Task 3's checkpoint:human-verify: 'Accept' (full plugin extraction satisfies SC#3 as written).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-01-SUMMARY.md

---

### None specific to this plan
None specific to this plan. (Phase-level git-merge interruption is recorded in `01-01-SUMMARY.md`.)

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-02-SUMMARY.md

---

### None. The phase's one human-decision point (SC#3's host-crate reinterpre...
None. The phase's one human-decision point (SC#3's host-crate reinterpretation) was a blocking `checkpoint:human-verify` gate during execution, already resolved — the SUMMARY records the operator's verbatim reply ("Accept"), and the plan's gate structure (blocking, explicit resume-signal required, "any other reply or silence: stay paused") means this could not have silently defaulted to proceeding.

**Context:** Verified in phase acceptance checks.
**Source:** 01-VERIFICATION.md

---

### No gaps found
No gaps found. All four ROADMAP success criteria are independently re-verified against the live codebase (not copied from SUMMARY.md), both commits (`1d81d7ae9`, `ccc29d5cf`) match their claimed diffs exactly, and the three requirement IDs (delete, catalog-cleanup, keep-frozen) all have concrete evidence. I additionally re-ran both of the phase's build-affecting commands live (`cargo check -p vox-scientia --all-features`, `cargo build -p vox-plugin-catalog`) and confirmed both exit 0. Nothing outstanding.

**Context:** Verified in phase acceptance checks.
**Source:** 01-VERIFICATION.md

## Patterns

### vox graph coverage --kind crate returns {"entries"
vox graph coverage --kind crate returns {"entries": []} even fresh — use per-crate vox graph query as fallback, cargo tree/metadata remain ground truth

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-01-SUMMARY.md

---

### catalog.toml records crate disposition as a prose #-comment block when t...
catalog.toml records crate disposition as a prose `#`-comment block when the schema (no deny_unknown_fields, 4 renamed tables) would silently swallow a new table kind

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-02-SUMMARY.md

## Surprises

### Estimate variance on 01-01
Estimate variance on 01-01: planned 45000 tokens vs 5100 actual (0.11x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-01-PLAN.md, 01-01-SUMMARY.md

---

### Estimate variance on 01-02
Estimate variance on 01-02: planned 22000 tokens vs 3400 actual (0.15x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-02-PLAN.md, 01-02-SUMMARY.md

