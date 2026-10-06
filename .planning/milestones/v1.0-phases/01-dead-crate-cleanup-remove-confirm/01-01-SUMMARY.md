---
phase: 01-dead-crate-cleanup-remove-confirm
plan: 01
subsystem: infra
tags: [cargo, ci, vox-graph, crate-lifecycle]

requires: []
provides:
  - "Machine-verified proof (not PRD-assumed) that all 10 REQ-dead-crate-delete crates are absent from the workspace"
  - "Confirmation that the ConfidencePolicy/ComplexityBand/RiskBand type migration and vox-scientia-ingest logic migration already landed"
affects: [01-02, phase-2, phase-3]

actuals:
  tokens: 5100
  tasks: 3
  commits: 1

tech-stack:
  added: []
  patterns: ["vox graph coverage --kind crate returns {\"entries\": []} even fresh — use per-crate vox graph query as fallback, cargo tree/metadata remain ground truth"]

key-files:
  created: []
  modified: [".github/workflows/ci.yml"]

key-decisions:
  - "D-03 verification ran vox graph refresh --auto first (all 5 corpora were stale with git_drift+worktree_drift); coverage --kind crate still returned empty post-refresh, confirming RESEARCH.md's Open Question 2 as a real tool gap, not a cache staleness artifact"
  - "The plan's git_grep verify command for ROADMAP SC#2 (`\\b` word-boundary regex) is non-portable to macOS/BSD grep — it silently produces 0 matches instead of the correct 3, because BSD grep -E does not support \\b. Verified the actual requirement directly instead (ctx_search + git grep without \\b, cross-checked against source)."

requirements-completed: [REQ-dead-crate-delete]

coverage:
  - id: D1
    description: "cargo tree -p vox-cli and cargo metadata show zero references to all 10 REQ-dead-crate-delete crates (ROADMAP SC#1)"
    requirement: "REQ-dead-crate-delete"
    verification:
      - kind: other
        ref: "cargo tree -p vox-cli --offline | grep -iE (10-crate pattern) — 0 matches"
        status: pass
      - kind: other
        ref: "cargo metadata --format-version 1 --offline | grep (10-crate exact-quoted pattern) — 0 matches"
        status: pass
    human_judgment: false
  - id: D2
    description: "ConfidencePolicy/ComplexityBand/RiskBand are defined under crates/vox-orchestrator-types/src/socrates_policy/, zero vox_socrates_policy:: imports remain (ROADMAP SC#2)"
    requirement: "REQ-dead-crate-delete"
    verification:
      - kind: other
        ref: "git grep -nE vox_socrates_policy:: -- '*.rs' — 0 matches"
        status: pass
      - kind: other
        ref: "policy_types.rs:6 pub enum RiskBand, :33 pub enum ComplexityBand, :153 pub struct ConfidencePolicy — read directly, all 3 present"
        status: pass
    human_judgment: false
  - id: D3
    description: "vox-scientia-ingest's functionality is reachable with no crate taking a vox-cli compile dependency (ROADMAP SC#3)"
    requirement: "REQ-dead-crate-delete"
    verification:
      - kind: other
        ref: "cargo tree -p vox-plugin-publication --offline | grep vox-cli — 0 matches"
        status: pass
      - kind: other
        ref: "grep scholarly-external-jobs crates/vox-plugin-publication/Cargo.toml — present (line 16)"
        status: pass
    human_judgment: true
    rationale: "SC#3's literal wording (\"reachable from vox-publisher\") doesn't match the actual host crate (vox-plugin-publication reaching vox-scientia via runtime plugin-host ABI dispatch, not vox-publisher). The substance is satisfied and exceeds the letter — operator ratified via Task 3's checkpoint:human-verify: 'Accept' (full plugin extraction satisfies SC#3 as written)."
  - id: D4
    description: "The all-features check CI matrix names only resolvable workspace packages (ROADMAP SC#1, residue fix)"
    requirement: "REQ-dead-crate-delete"
    verification:
      - kind: other
        ref: "grep -n vox-scientia-ingest .github/workflows/ci.yml — 0 matches; grep -nE '^ *- vox-scientia$' — present; cargo check -p vox-scientia --all-features — exit 0; commit 1d81d7ae9 diff is exactly 1 insertion/1 deletion in .github/workflows/ci.yml only"
        status: pass
    human_judgment: false

duration: 55min
completed: 2026-09-22
status: complete
---

# Phase 1 Plan 01: Dead Crate Cleanup Verification Summary

**D-03 verification pass confirms all 10 REQ-dead-crate-delete crates and their prerequisite type/logic migrations already landed; repointed the stale all-features CI matrix entry; operator ratified SC#3's actual host-crate disposition.**

## Performance

- **Duration:** ~55 min (including an operator-driven git-merge interruption between Task 1 and Task 2, not phase work)
- **Started:** 2026-09-22T20:20:00Z
- **Completed:** 2026-09-22T21:20:00Z
- **Tasks:** 3 of 3
- **Files modified:** 1 (`.github/workflows/ci.yml`)

## Accomplishments
- Machine-verified (not assumed from RESEARCH.md prose) that all 10 crates named in REQ-dead-crate-delete are absent from `cargo tree -p vox-cli` and `cargo metadata`
- Machine-verified zero surviving `vox_socrates_policy::` imports and confirmed all 3 migrated types (`ConfidencePolicy`, `ComplexityBand`, `RiskBand`) are defined in `vox-orchestrator-types`
- Machine-verified `vox-plugin-publication` reaches the ingest logic without a `vox-cli` dependency and enables `vox-publisher`'s `scholarly-external-jobs` feature
- Ran `vox graph refresh --auto` (all 5 corpora were stale) then `vox graph coverage --kind crate` — confirmed the empty-result gap RESEARCH.md flagged is a real tool limitation, not cache staleness; fell back to per-crate `vox graph query` for all 10 names as the plan specifies (fuzzy/inconclusive results, consistent with but not superseding the cargo ground truth)
- Repointed the `all-features check` CI matrix from the deleted `vox-scientia-ingest` to `vox-scientia`, verified `cargo check -p vox-scientia --all-features` is green, committed as an exact one-line diff (`1d81d7ae9`)
- Operator ratified SC#3's actual host-crate disposition ("Accept" — full plugin extraction satisfies the criterion as written)

## D-03 Verification Evidence

### ROADMAP Success Criterion 1 — zero references to the 10 deleted crates

```
$ cargo tree -p vox-cli --offline > target/phase01-vox-cli-tree.txt
$ grep -iE "vox-schola|vox-scientia-core|vox-scientia-social|vox-scientia-ingest|vox-socrates-policy|vox-spool|vox-tools|vox-mcp-meta|vox-browser|vox-audio-ingress" target/phase01-vox-cli-tree.txt
(0 matches, exit 1)

$ cargo metadata --format-version 1 --offline > target/phase01-metadata.json
$ grep -c -oE '"(vox-schola|vox-scientia-core|vox-scientia-social|vox-scientia-ingest|vox-socrates-policy|vox-spool|vox-tools|vox-mcp-meta|vox-browser|vox-audio-ingress)"' target/phase01-metadata.json
0 (exit 1)
```

Result: **PASS** — zero matches in both the vox-cli dependency tree and the full workspace metadata.

**D-03 graph-tooling check (required to have run, not required to be conclusive):**
```
$ vox graph status
(all 5 corpora reported stale: git_drift, worktree_drift)
$ vox graph refresh --auto
Rebuild repo-code-graph (stale) -> rebuilt
Rebuild vox-gui-surface (stale) -> rebuilt
Rebuild vox-config-graph (stale) -> rebuilt
Rebuild config-audit (stale) -> rebuilt
Rebuild crate-map (stale) -> rebuilt
$ vox graph coverage --kind crate
{"entries": []}
```
Coverage returned empty even against a freshly-rebuilt graph — this is RESEARCH.md Open Question 2 confirmed as a genuine tool gap (the `crate` coverage kind doesn't populate), not stale-cache noise. Fell back to `vox graph query "<name>"` per-crate for all 10 names; results were fuzzy full-text symbol matches (e.g. querying `vox-schola` surfaces unrelated symbols containing "vox" tokens) with no hit naming any of the 10 crates as an actual dependency — consistent with, but not independently probative beyond, the cargo ground truth above.

### ROADMAP Success Criterion 2 — type migration, zero stale imports

```
$ git grep -nE "vox_socrates_policy::" -- '*.rs'
(0 matches, exit 1)
```

Direct read of `crates/vox-orchestrator-types/src/socrates_policy/policy_types.rs`:
- Line 6: `pub enum RiskBand {`
- Line 33: `pub enum ComplexityBand {`
- Line 153: `pub struct ConfidencePolicy {`

Result: **PASS** — all 3 types present, zero stale imports.

**Portability note (not a phase blocker, worth fixing if this verify command is reused):** the plan's own verify command for this check (`git grep -cE "pub (struct|enum) (ConfidencePolicy|ComplexityBand|RiskBand)\b" -- '.../*.rs'`) relies on `\b` word-boundary support in extended regex. macOS's bundled `grep`/`git grep` (BSD grep) does not support `\b` in `-E` mode — it silently produced 0 matches (a false failure) when run as literally specified. Without `\b`, the same pattern over-matches (`ConfidencePolicyOverride` also matches `ConfidencePolicy`). Verified the actual requirement by direct source read instead, which is unambiguous.

### ROADMAP Success Criterion 3 — scholarly-external-jobs reachability, no vox-cli dependency

```
$ cargo tree -p vox-plugin-publication --offline > target/phase01-publication-tree.txt
$ grep -c vox-cli target/phase01-publication-tree.txt
0 (exit 1)

$ grep -n 'scholarly-external-jobs' crates/vox-plugin-publication/Cargo.toml
16:vox-publisher = { workspace = true, features = ["scientia-reddit", "scientia-youtube", "scholarly-external-jobs"] }
```

Result: **PASS**, with a caveat — see `coverage.D3.rationale` above. The literal path SC#3 describes ("reachable from vox-publisher") isn't the actual path (runtime plugin-host ABI dispatch through `vox-plugin-publication`, which separately enables `vox-publisher`'s feature). Substance holds; Task 3's checkpoint covers the reinterpretation formally.

### D-04 commit-batching — groups (1) and (2) produce no diff

Per D-04, this phase's crate deletions are batched into (1) the vox-scientia-* trio and (2) the seven standalone deletes. **Both groups are verification-only in this run — there is nothing to commit for either, because the underlying deletions were already committed months ago** (`e828828a9`, `72bde3718`, `0a6aae51e`, per 01-RESEARCH.md). This SUMMARY section *is* the D-04 record for groups (1) and (2); no commit exists or is expected for them. Only D-04's group (3) (KEEP-FROZEN documentation + catalog cleanup, in plan 01-02) and this plan's own residual CI fix (Task 2) produce real diffs.

### Task 3 — SC#3 checkpoint outcome (verbatim)

Presented to the operator: ROADMAP SC#3 names `vox-publisher` as the reachability target, but the actual path is `vox-plugin-publication` (host crate) → `vox-scientia::ingest` (logic) via `vox_plugin_host::cached_code_plugin("publication")` (runtime ABI dispatch, not a compile-time edge), with `vox-plugin-publication` separately enabling `vox-publisher`'s `scholarly-external-jobs` feature. Operator's reply: **"Accept"** — full plugin extraction satisfies SC#3 as written; the substance holds and exceeds the letter (runtime dispatch is a stronger decoupling than the criterion's literal "no vox-cli dependency" bar). No ROADMAP.md edit made or needed — SC#3's wording stands, its disposition is now ratified.

## Task Commits

Each committing task was committed atomically:

1. **Task 2: Repoint all-features CI matrix to vox-scientia** — `1d81d7ae9` (fix)
2. **Task 3: Checkpoint** — no commit (decision-only, recorded in this SUMMARY)

_Two unrelated compile-break fixes were also committed in this session, outside this plan's own scope, to unblock the operator's separate in-progress `git merge` — see Issues Encountered below. They are not part of this plan's deliverables: `b94490a9f` (vox-search: ResearchLane FromStr) and `ab3554261` (vox-research-shim: destructure synthesize_answer_with_llm's tuple return)._

## Files Created/Modified
- `.github/workflows/ci.yml` — one matrix list item substituted (`vox-scientia-ingest` → `vox-scientia`), exactly 1 insertion/1 deletion

## Decisions Made
- Ran the D-03-mandated `vox graph` check first, refreshed all 5 stale corpora, and used the documented per-crate-query fallback when `coverage --kind crate` returned empty post-refresh — confirming this is a tool gap, not a cache issue.
- Treated the plan's `\b`-based verify command as non-portable rather than as a failed check, and independently confirmed the underlying requirement by direct source read.
- Declined PATTERNS.md's suggested explanatory comment on the CI matrix line, per the plan's own instruction — a comment naming the old package would defeat the acceptance check's negative-grep.

## Deviations from Plan

None — all three tasks executed as specified. The git-merge interruption (below) delayed but did not change execution.

## Issues Encountered

**Not caused by this task, resolved along the way:** the working tree had an in-progress, unrelated `git merge` (the operator's own `repo-history-graph` branch) whose conclusion initially failed a pre-commit hook: `crates/vox-research-shim/src/research/orchestrator/pipeline.rs` had 6 real compile errors from an incomplete upstream change (`synthesize_answer_with_llm` now returns `(String, bool)`; 5 call sites and a defined-but-never-applied `TEMPLATE_FALLBACK_QUALITY_CAP` constant were unwired). Fixed with the operator's explicit direction ("mechanical fix" — destructure and discard the flag, no behavior change) after presenting both a mechanical and a feature-completing option. A second, unrelated pre-existing compile break then surfaced (`ResearchLane` missing `FromStr`, blocking commits workspace-wide via the `command-sync` pre-commit hook regardless of merge status) — fixed after the operator chose "propose a fix, show me first" and approved it. The operator ultimately resolved the merge itself via `git merge --abort` in a separate, concurrent terminal session while these fixes were in flight; both fixes survived as local edits and were committed cleanly once the dust settled. Two of my own commits were silently reverted mid-attempt during this window (git index reset by the concurrent `git merge --abort`) and had to be reapplied and re-verified — no data was permanently lost, but it cost several retry cycles.

## Next Phase Readiness

**Complete.** All three ROADMAP success criteria (1, 2, 3) for this plan are machine-verified with evidence, plus the residual CI matrix fix (contributing to Success Criterion 1's "cannot fail on a missing package" clause) is committed and green. Plan 01-02 (catalog.toml frozen-crate annotation, Success Criterion 4) is next.

---
*Phase: 01-dead-crate-cleanup-remove-confirm*
*Completed: 2026-09-22*
