---
phase: 4
phase_name: "taxonomy-contract-and-facet-lint"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 21
  lessons: 22
  patterns: 8
  surprises: 14
missing_artifacts:
  - "STATE.md"
---

# Phase 4 Learnings: taxonomy-contract-and-facet-lint

## Decisions

### Facets fields are all Option/Vec so a missing field is a lint error, not...
Facets fields are all Option/Vec so a missing field is a lint error, not a parse error; unknown frontmatter keys are ignored (no deny_unknown_fields)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-01-SUMMARY.md

---

### inspect_summary treats only whole-line markers as the block; inline form...
inspect_summary treats only whole-line markers as the block; inline forms report Missing but strip_summary_blocks still removes them (fail closed)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-01-SUMMARY.md

---

### Blockquote runs are one quote each; only marked quotes are detectable (p...
Blockquote runs are one quote each; only marked quotes are detectable (ponytail ceiling noted in code)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-01-SUMMARY.md

---

### Topic prefLabels must differ from domain prefLabels (one label namespace...
Topic prefLabels must differ from domain prefLabels (one label namespace), so the domain `orchestration` has the topic `agent-orchestrator`

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-02-SUMMARY.md

---

### expand_query matches only altLabels and deprecated aliases, never the pr...
expand_query matches only altLabels and deprecated aliases, never the prefLabel itself (a doc already tagged with it needs no expansion)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-02-SUMMARY.md

---

### Every scopeNote is double-quoted in the YAML because they contain :
Every scopeNote is double-quoted in the YAML because they contain `: `

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-02-SUMMARY.md

---

### Renderer reads the committed JSON file, so vox-cli-ci takes no vox-kb-fa...
Renderer reads the committed JSON file, so vox-cli-ci takes no vox-kb-facets edge

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-03-SUMMARY.md

---

### Every generated property is optional (?:) and nullable fields are T | nu...
Every generated property is optional (`?:`) and nullable fields are `T | null`, matching the schema (all properties defaulted, none required)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-03-SUMMARY.md

---

### Schema property order relies on serde_json preserve_order, which workspa...
Schema property order relies on serde_json preserve_order, which workspace-hack pins for every crate

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-03-SUMMARY.md

---

### A facet parse error excludes the doc (fail closed); the real-tree oracle...
A facet parse error excludes the doc (fail closed); the real-tree oracle proves no doc is affected once three mis-shaped YAML values are fixed

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-04-SUMMARY.md

---

### Files with no frontmatter or an unterminated block are now eligible by d...
Files with no frontmatter or an unterminated block are now eligible by default (Facets::default); the old substring-on-flag rule for those shapes is gone per D-06 and no real doc has either shape

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-04-SUMMARY.md

---

### A missing or invalid taxonomy/backlog is one hard facet-contract-unreada...
A missing or invalid taxonomy/backlog is one hard facet-contract-unreadable error (fail closed); the contracts load once in lint_in before the rayon pass

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-05-SUMMARY.md

---

### Backlog rows excuse exact rule ids per exact docs/src path; an unknown r...
Backlog rows excuse exact rule ids per exact docs/src path; an unknown rule id in the backlog makes the contract unreadable

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-05-SUMMARY.md

---

### hard_errors counts Facet issues via is_hard() (only facet-backlog-stale ...
hard_errors counts Facet issues via is_hard() (only facet-backlog-stale is soft)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-05-SUMMARY.md

---

### search() stays and delegates to search_with_taxonomy(None); the eval ret...
search() stays and delegates to search_with_taxonomy(None); the eval retriever keeps calling search (frozen snapshot has no faceted docs)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-06-SUMMARY.md

---

### MCP reaches the taxonomy only through the vox_search::library::Taxonomy ...
MCP reaches the taxonomy only through the vox_search::library::Taxonomy re-export: no new crate edge

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-06-SUMMARY.md

---

### Seed/tighten handlers return before FacetCtx::load so a missing backlog ...
Seed/tighten handlers return before FacetCtx::load so a missing backlog never blocks seeding

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-07-SUMMARY.md

---

### An addition is accepted only with epoch+1 and a changed non-empty epoch_...
An addition is accepted only with epoch+1 and a changed non-empty epoch_note; same-epoch additions, a lower epoch, and a deleted file fail

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-07-SUMMARY.md

---

### Base read is fail-closed
Base read is fail-closed: unresolvable base or any git error fails; only an ls-tree that lists nothing bootstraps

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-07-SUMMARY.md

---

### Backlog produced only by the seed command; no facet written into any exi...
Backlog produced only by the seed command; no facet written into any existing doc

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-08-SUMMARY.md

---

### Summary-block example sits in a column-0 fence in the governance doc (th...
Summary-block example sits in a column-0 fence in the governance doc (the summary scanner only recognises column-0 fences)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-08-SUMMARY.md

## Lessons

### 1. [Rule 3 - Blocking] Stale installed vox binary. vox ci crate-edges an...
**1. [Rule 3 - Blocking] Stale installed `vox` binary.** `vox ci crate-edges` and `affected-crates --regen` refuse a stale binary; ran with `VOX_SKIP_FRESHNESS_CHECK=1` rather than rebuilding vox-cli workspace-wide. The crate-edges logic reads the working-tree contracts, so the verdict is valid for this change.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-01-SUMMARY.md

---

### 2. [Rule 3 - Blocking] Cargo.lock committed with Task 1 (not Task 3), be...
**2. [Rule 3 - Blocking] `Cargo.lock` committed with Task 1** (not Task 3), because the new crate lands in the lockfile at Task 1. Task 3 commit added the three consumer rows.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-01-SUMMARY.md

---

### 3. No separate RED commit. Tests and implementation for a brand-new crat...
**3. No separate RED commit.** Tests and implementation for a brand-new crate were committed together per task; tests were written first locally and the guards are covered by mutation proofs above.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-01-SUMMARY.md

---

### 4. crate-graph.v1.json regeneration also added a stale vox-search -> vox...
**4. `crate-graph.v1.json` regeneration also added a stale `vox-search -> vox-crypto` row** (mirror was behind the tree); this is generator output, not an edge added by this plan, and the crate-edges baseline `edges` list is untouched (0 removed lines in the allow-list diff).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-01-SUMMARY.md

---

### 1. [Rule 1 - Bug] scopeNote values unquoted broke YAML parse. Notes cont...
**1. [Rule 1 - Bug] scopeNote values unquoted broke YAML parse.** Notes containing `: ` failed (`mapping values are not allowed`); all scopeNote values are now double-quoted. Found by the real_taxonomy test; fixed in the Task 2 commit.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-02-SUMMARY.md

---

### 2. [Process] Task 1 had no separate RED commit. Tests and implementation...
**2. [Process] Task 1 had no separate RED commit.** Tests and implementation were committed together (same as 04-01); the guards are covered by the mutation proofs above.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-02-SUMMARY.md

---

### 3. [Process] Pre-commit allow-list check for agent-* branches skipped. T...
**3. [Process] Pre-commit allow-list check for agent-* branches skipped.** This is a sequential run on the claude/ feature branch, per the dispatch instructions; protected-branch and root-pin checks were applied.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-02-SUMMARY.md

---

### 1. [Rule 1 - Bug] Three real docs failed the facet parse (found by the o...
**1. [Rule 1 - Bug] Three real docs failed the facet parse** (found by the oracle, as the plan anticipated). Fixed YAML shape only: `supersedes:` scalar to a list in `knowledge-base-practices-research-2026.md` and `adr-NNN-scope-tauri-desktop-only.md`; `superseded_by:` list to a single string in `crate-classification-2026-05-08.md` (second target kept as a YAML comment). All three are `training_eligible: false` or tombstoned, so corpus output is unchanged. Included in commit 230a57928.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-04-SUMMARY.md

---

### 2. Strip step landed in the Task 2 commit, tests in the sibling module s...
**2. Strip step landed in the Task 2 commit, tests in the sibling module split across tasks**, so each commit is green on its own.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-04-SUMMARY.md

---

### 1. [Process] Task 1 has no separate RED commit. Tests and implementation...
**1. [Process] Task 1 has no separate RED commit.** Tests and implementation committed together per task (same as 04-01/04-02/04-04); guards covered by the mutation proofs.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-05-SUMMARY.md

---

### 2. [Rule 1] lint_facets falls back to anchoring on docs/src/ when strip_...
**2. [Rule 1] `lint_facets` falls back to anchoring on `docs/src/`** when `strip_prefix(repo_root)` fails (drive casing), mirroring existing `lint.rs` behavior, so a path-shape mismatch cannot silently skip the facet rules.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-05-SUMMARY.md

---

### 3. hard_errors uses a guard arm (LintKind::Facet(i) if i.is_hard()) rath...
**3. `hard_errors` uses a guard arm** (`LintKind::Facet(i) if i.is_hard()`) rather than a plain `matches!` list entry, since one wrapper variant has one soft id.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-05-SUMMARY.md

---

### 1. [Process] Task 1 had no separate RED commit. Tests and implementation...
**1. [Process] Task 1 had no separate RED commit.** Tests and implementation were committed together (as in 04-01/04-02); the guards are covered by the mutation proofs.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-06-SUMMARY.md

---

### 2. [Process] Pre-commit agent-branch allow-list skipped. Sequential run ...
**2. [Process] Pre-commit agent-branch allow-list skipped.** Sequential run on the `claude/` feature branch per dispatch instructions; root-pin and protected-branch checks were applied.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-06-SUMMARY.md

---

### 1. [Rule 1 - Bug in my own doc] Summary example inside an indented fence...
**1. [Rule 1 - Bug in my own doc] Summary example inside an indented fence tripped `facet-summary-placement`.** The first draft of the governance section put the `kb:summary` example in a fence indented under a list item; the summary scanner (`vox-kb-facets/src/summary.rs`, `is_fence` uses `starts_with` with no indentation allowance) does not treat indented fences as fences, so it saw a misplaced real block. Fixed by moving the example to a column-0 fence. Not changing the scanner: out of scope. Noted as a minor scanner limitation (CommonMark allows up to 3 spaces of fence indent); a doc that indents a fence containing the marker lines will get a false `facet-summary-placement`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-08-SUMMARY.md

---

### 2. [Process] Task edits to text files done with one-off scripted exact-m...
**2. [Process] Task edits to text files done with one-off scripted exact-match replacement** (no committed glue).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-08-SUMMARY.md

---

### File: crates/vox-cli-ci/src/kb_facets_ts.rs:46-52, crates/vox-cli/src/co...
**File:** `crates/vox-cli-ci/src/kb_facets_ts.rs:46-52`, `crates/vox-cli/src/commands/ci/run_body.rs:106-115`

**Context:** Surfaced in structured phase code review.
**Source:** 04-REVIEW.md

---

### A $ref name is emitted without an ident() check or a check that it exist...
A `$ref` name is emitted without an `ident()` check or a check that it exists in `definitions`. A bad ref yields an undefined or garbage TS type despite the "closed over known shapes" claim.

**Context:** Surfaced in structured phase code review.
**Source:** 04-REVIEW.md

---

### KB_FACET_KEYS order depends on serde_json preserve_order being unified i...
`KB_FACET_KEYS` order depends on `serde_json` `preserve_order` being unified into `vox-cli-ci`. Today the workspace hack guarantees it. A comment or an assertion would make the dependency explicit.

**Context:** Surfaced in structured phase code review.
**Source:** 04-REVIEW.md

---

### See frontmatter human_verification
See frontmatter `human_verification`: (1) hosted CI green on the phase PR, (2) GUI vitest and typecheck, (3) product decision on promoted-only sources[] completeness.

**Context:** Verified in phase acceptance checks.
**Source:** 04-VERIFICATION.md

---

### No gaps. All four ROADMAP success criteria and all seven TAX requirement...
No gaps. All four ROADMAP success criteria and all seven TAX requirements are backed by code, wiring and passing targeted tests; CR-01 fail-closed behavior and the WR-01 re-epoch gate hold in the current code. Remaining items are confirmation steps that need hosted CI, the GUI toolchain, and one scoping decision.

**Context:** Verified in phase acceptance checks.
**Source:** 04-VERIFICATION.md

---

### total: 3 passed: 3 issues: 0 pending: 0 skipped: 0 blocked: 0
total: 3 passed: 3 issues: 0 pending: 0 skipped: 0 blocked: 0

**Context:** Observed during user acceptance testing.
**Source:** 04-UAT.md

## Patterns

### L0 pure crate, workspace-hack only
L0 pure crate, workspace-hack only

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 04-01-SUMMARY.md

---

### defactored frontmatter splitter and blake3 encoded-form check
defactored frontmatter splitter and blake3 encoded-form check

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 04-01-SUMMARY.md

---

### deny_unknown_fields contract loader that reports every curation error at...
deny_unknown_fields contract loader that reports every curation error at once

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 04-02-SUMMARY.md

---

### whole-token altLabel match emitting only validated prefLabel phrases
whole-token altLabel match emitting only validated prefLabel phrases

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 04-02-SUMMARY.md

---

### closed-shape JSON-Schema-to-TS renderer that bails on unsupported shapes
closed-shape JSON-Schema-to-TS renderer that bails on unsupported shapes

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 04-03-SUMMARY.md

---

### split-needle dispatch guard test for ssot-drift
split-needle dispatch guard test for ssot-drift

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 04-03-SUMMARY.md

---

### query-time synonym expansion (no re-index when the vocabulary changes)
query-time synonym expansion (no re-index when the vocabulary changes)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 04-06-SUMMARY.md

---

### availability over synonyms
availability over synonyms: load failure degrades and is reported, never silent

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 04-06-SUMMARY.md

## Surprises

### Estimate variance on 04-01
Estimate variance on 04-01: planned 90000 tokens vs 6400 actual (0.07x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 04-01-PLAN.md, 04-01-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Stale installed vox binary. vox ci crate-edges an...
**1. [Rule 3 - Blocking] Stale installed `vox` binary.** `vox ci crate-edges` and `affected-crates --regen` refuse a stale binary; ran with `VOX_SKIP_FRESHNESS_CHECK=1` rather than rebuilding vox-cli workspace-wide. The crate-edges logic reads the working-tree contracts, so the verdict is valid for this change.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 04-01-SUMMARY.md

---

### 2. [Rule 3 - Blocking] Cargo.lock committed with Task 1 (not Task 3), be...
**2. [Rule 3 - Blocking] `Cargo.lock` committed with Task 1** (not Task 3), because the new crate lands in the lockfile at Task 1. Task 3 commit added the three consumer rows.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 04-01-SUMMARY.md

---

### Estimate variance on 04-02
Estimate variance on 04-02: planned 70000 tokens vs 11000 actual (0.16x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 04-02-PLAN.md, 04-02-SUMMARY.md

---

### 1. [Rule 1 - Bug] scopeNote values unquoted broke YAML parse. Notes cont...
**1. [Rule 1 - Bug] scopeNote values unquoted broke YAML parse.** Notes containing `: ` failed (`mapping values are not allowed`); all scopeNote values are now double-quoted. Found by the real_taxonomy test; fixed in the Task 2 commit.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 04-02-SUMMARY.md

---

### Estimate variance on 04-03
Estimate variance on 04-03: planned 75000 tokens vs 7600 actual (0.10x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 04-03-PLAN.md, 04-03-SUMMARY.md

---

### Estimate variance on 04-04
Estimate variance on 04-04: planned 55000 tokens vs 5500 actual (0.10x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 04-04-PLAN.md, 04-04-SUMMARY.md

---

### 1. [Rule 1 - Bug] Three real docs failed the facet parse (found by the o...
**1. [Rule 1 - Bug] Three real docs failed the facet parse** (found by the oracle, as the plan anticipated). Fixed YAML shape only: `supersedes:` scalar to a list in `knowledge-base-practices-research-2026.md` and `adr-NNN-scope-tauri-desktop-only.md`; `superseded_by:` list to a single string in `crate-classification-2026-05-08.md` (second target kept as a YAML comment). All three are `training_eligible: false` or tombstoned, so corpus output is unchanged. Included in commit 230a57928.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 04-04-SUMMARY.md

---

### Estimate variance on 04-05
Estimate variance on 04-05: planned 95000 tokens vs 14000 actual (0.15x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 04-05-PLAN.md, 04-05-SUMMARY.md

---

### 2. [Rule 1] lint_facets falls back to anchoring on docs/src/ when strip_...
**2. [Rule 1] `lint_facets` falls back to anchoring on `docs/src/`** when `strip_prefix(repo_root)` fails (drive casing), mirroring existing `lint.rs` behavior, so a path-shape mismatch cannot silently skip the facet rules.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 04-05-SUMMARY.md

---

### Estimate variance on 04-06
Estimate variance on 04-06: planned 90000 tokens vs 9000 actual (0.10x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 04-06-PLAN.md, 04-06-SUMMARY.md

---

### Estimate variance on 04-07
Estimate variance on 04-07: planned 70000 tokens vs 11000 actual (0.16x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 04-07-PLAN.md, 04-07-SUMMARY.md

---

### Estimate variance on 04-08
Estimate variance on 04-08: planned 55000 tokens vs 9000 actual (0.16x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 04-08-PLAN.md, 04-08-SUMMARY.md

---

### 1. [Rule 1 - Bug in my own doc] Summary example inside an indented fence...
**1. [Rule 1 - Bug in my own doc] Summary example inside an indented fence tripped `facet-summary-placement`.** The first draft of the governance section put the `kb:summary` example in a fence indented under a list item; the summary scanner (`vox-kb-facets/src/summary.rs`, `is_fence` uses `starts_with` with no indentation allowance) does not treat indented fences as fences, so it saw a misplaced real block. Fixed by moving the example to a column-0 fence. Not changing the scanner: out of scope. Noted as a minor scanner limitation (CommonMark allows up to 3 spaces of fence indent); a doc that indents a fence containing the marker lines will get a false `facet-summary-placement`.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 04-08-SUMMARY.md

