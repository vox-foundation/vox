---
phase: 01-dead-crate-cleanup-remove-confirm
verified: 2026-09-23T02:10:00Z
status: passed
score: 7/7 must-haves verified
covered_files:
  - ".github/workflows/ci.yml"
  - ".planning/REQUIREMENTS.md"
  - ".planning/phases/01-dead-crate-cleanup-remove-confirm/01-01-PLAN.md"
  - ".planning/phases/01-dead-crate-cleanup-remove-confirm/01-01-SUMMARY.md"
  - ".planning/phases/01-dead-crate-cleanup-remove-confirm/01-02-PLAN.md"
  - ".planning/phases/01-dead-crate-cleanup-remove-confirm/01-02-SUMMARY.md"
  - "crates/vox-plugin-catalog/catalog.toml"
covered_digest: "v1:sha256:cd7e9269698387168692deece77571a096481508ce1531025a9380796642c207"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 1: Dead Crate Cleanup — Remove & Confirm Verification Report

**Phase Goal:** The workspace no longer carries zero-consumer crates or stale catalog entries; frozen crates are explicitly documented as intentionally inactive.
**Verified:** 2026-09-23T02:10:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

This phase's own RESEARCH.md establishes it is a *confirmation-and-residue-cleanup* phase (all ten named crates were deleted months before this milestone, in commits `e828828a9`, `72bde3718`, `0a6aae51e`). I independently re-ran the phase's own verification commands against the live working tree rather than trusting the SUMMARY's pasted output, plus checked the two committed diffs byte-for-byte against their claims.

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | ROADMAP SC#1 — `cargo tree -p vox-cli` and workspace `cargo metadata` show zero references to the ten named crates | ✓ VERIFIED | Independently re-ran both commands (not copy of SUMMARY output): `cargo tree -p vox-cli --offline` (4375 lines) → 0 matches for the 10-name pattern; `cargo metadata --format-version 1 --offline` → 0 exactly-quoted matches. |
| 2 | ROADMAP SC#2 — `ConfidencePolicy`/`ComplexityBand`/`RiskBand` live in `vox-orchestrator-types`, zero `vox_socrates_policy::` imports remain | ✓ VERIFIED | `git grep -nE "vox_socrates_policy::" -- '*.rs'` → 0 matches (exit 1). Direct grep of `crates/vox-orchestrator-types/src/socrates_policy/policy_types.rs` → `pub enum RiskBand` (L6), `pub enum ComplexityBand` (L33), `pub struct ConfidencePolicy` (L153), all present. |
| 3 | ROADMAP SC#3 — scholarly-external-jobs ingest reachable with no crate taking a `vox-cli` compile dependency | ✓ VERIFIED | `cargo tree -p vox-plugin-publication --offline` → 0 lines containing `vox-cli`. `crates/vox-plugin-publication/Cargo.toml:16` enables `scholarly-external-jobs` on its `vox-publisher` dep. Runtime dispatch chain confirmed by direct source read: `crates/vox-cli/src/commands/db/publication/ingest.rs:24` calls `vox_plugin_host::cached_code_plugin("publication")`; `crates/vox-plugin-publication/src/ingest.rs:19` defines `ingest_tick`, importing `vox_scientia::ingest::{FeedCrawler, IngestDeduplicator}` at line 7. The literal wording ("reachable from vox-publisher") doesn't match the actual host crate (`vox-plugin-publication`, dispatching to `vox-publisher` and `vox-scientia` at runtime) — SUMMARY records the operator's checkpoint reply "Accept" ratifying this divergence. This is a human-in-the-loop decision from execution I cannot replay, but the plan's `checkpoint:human-verify` gate structure (blocking, requires an explicit resume-signal reply) makes a fabricated "Accept" an unlikely silent failure mode, and the underlying technical facts it ratifies are independently confirmed above. |
| 4 | ROADMAP SC#4 clause 1 — catalog.toml shows execution-api/stub-check as already-removed | ✓ VERIFIED | `grep -nE '^id = "(execution-api\|stub-check)"' catalog.toml` → 0 matches. `cargo metadata` → 0 matching package names. Pre-existing comment `# execution-api and stub-check removed 2026-05-08:` present at line 84, untouched. |
| 5 | ROADMAP SC#4 clause 2 — the 3 KEEP-FROZEN crates present, unmodified by this phase, and annotated | ✓ VERIFIED | `git ls-files` confirms all 3 `Cargo.toml` tracked. `git log --oneline -- crates/vox-workflow-runtime crates/vox-integration-tests crates/vox-test-harness` since 2026-09-22 → empty (no phase commit touched them). New comment block at catalog.toml lines 11-18 names all three verbatim, states KEEP-FROZEN disposition, cites the PRD, and states removal requires a new PRD. |
| 6 | All-features CI matrix names only resolvable workspace packages (plan-added residue fix) | ✓ VERIFIED | `grep -n 'vox-scientia-ingest' .github/workflows/ci.yml` → 0 matches. `grep -nE '^ *- vox-scientia$'` → line 1980 present. `cargo metadata` → `vox-scientia` resolves as a real package (8 hits in the dependency graph). **Live re-run of the literal CI command:** `cargo check -p vox-scientia --all-features` → `Finished \`dev\` profile ... in 12m 22s`, exit code 0 (ran myself, in background). Committed diff `1d81d7ae9` is exactly `1 insertion(+), 1 deletion(-)` — confirmed via `git show --stat`, matching the plan's tightest acceptance gate. |
| 7 | Frozen-crate annotation is a prose comment, not a new TOML table (D-02 schema constraint) | ✓ VERIFIED | `crates/vox-plugin-catalog/src/lib.rs` has exactly 4 `#[serde(rename = "...")]` tables (plugin/bundle/component/skill-bundle), zero `deny_unknown_fields`. `grep -nE '^\[\[[a-z-]+\]\]' catalog.toml \| sort -u` → only `bundle`, `component`, `plugin`, `skill-bundle` — no new table kind introduced. Committed diff `ccc29d5cf` is `9 insertions(+), 0 deletions(-)`, all lines `#`-prefixed or blank. **Live re-run:** `cargo build -p vox-plugin-catalog` → `Finished \`dev\` profile ... in 64m 16s` (build-broker queue delay, not a code issue), exit code 0 — the file still parses cleanly post-edit. |

**Score:** 7/7 truths verified (0 present-but-behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.github/workflows/ci.yml` | one matrix item substituted, `vox-scientia-ingest` → `vox-scientia` | ✓ VERIFIED | Diff confirmed exact (1/1), no comment added, position preserved between `vox-skills`/`vox-doc-pipeline`. |
| `crates/vox-plugin-catalog/catalog.toml` | new KEEP-FROZEN comment block, execution-api/stub-check comment untouched | ✓ VERIFIED | Both blocks present and byte-consistent; diff is comment-only (`9 insertions`, all `#`/blank); `cargo build -p vox-plugin-catalog` exits 0. |
| `.planning/phases/.../01-01-SUMMARY.md` | D-03 verification evidence, SC#3 checkpoint outcome, D-04 batching record | ✓ VERIFIED | Present, contains literal command output organized by SC, checkpoint reply "Accept" recorded verbatim. |
| `.planning/phases/.../01-02-SUMMARY.md` | SC#4 verification evidence, schema justification, comment block quoted | ✓ VERIFIED | Present, matches required structure. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `.github/workflows/ci.yml` all-features matrix | `vox-scientia` package | matrix `crate:` list item | ✓ WIRED | `cargo metadata` resolves the name; `cargo check -p vox-scientia --all-features` re-run live by the verifier, exit code 0 (12m22s cold build). |
| `crates/vox-cli/src/commands/db/publication/ingest.rs` | `crates/vox-plugin-publication/src/ingest.rs::ingest_tick` | `vox_plugin_host::cached_code_plugin("publication")` (runtime ABI dispatch) | ✓ WIRED | Confirmed by direct source read at both ends — line 24 and line 19/7 respectively. |
| `crates/vox-plugin-publication/Cargo.toml` | `vox-publisher` `scholarly-external-jobs` feature | Cargo dependency feature list | ✓ WIRED | Line 16, confirmed directly. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo check -p vox-scientia --all-features` exits 0 | `cargo check -p vox-scientia --all-features` | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 12m 22s`, exit code 0 | ✓ PASS — ran live in background during this verification session (queued behind the build broker, cap 6, ~12 min cold) |
| `cargo build -p vox-plugin-catalog` exits 0 | `cargo build -p vox-plugin-catalog` | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 64m 16s`, exit code 0 | ✓ PASS — ran live in background (queued behind the build broker, cap 6, 3 builds ahead; the 64-minute wall time is broker-queue contention on a busy shared machine, not a build problem — the crate itself is trivial) |

Both of this phase's file-level changes are now confirmed, live, to still compile cleanly after the edit.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| REQ-dead-crate-delete | 01-01 | Delete zero-consumer crates (10 named) | ✓ SATISFIED | Truths 1-3 above; all 10 crates confirmed absent, migrations confirmed landed. |
| REQ-dead-crate-catalog-cleanup | 01-02 | Confirm catalog-only ghost entries fully reflected | ✓ SATISFIED | Truth 4 above. |
| REQ-dead-crate-keep-frozen | 01-02 | Explicitly preserve/annotate frozen crates | ✓ SATISFIED | Truth 5 above. |

**Orphaned requirements check:** `.planning/REQUIREMENTS.md`'s traceability table maps only these three REQ IDs to Phase 1 (REQ-dead-crate-wire-up → Phase 2, REQ-dead-crate-extract-to-plugin / REQ-dead-crate-misplaced → Phase 3). No orphaned requirements for this phase.

Note: `.planning/REQUIREMENTS.md`'s per-requirement checkboxes (`- [ ]`) and its traceability table (`Pending`) are still unchecked/unmarked for all three IDs. This is a milestone-level document not touched by either phase plan (neither plan's `files_modified` lists it), so its staleness is expected bookkeeping outside this phase's scope, not a phase gap — but it means REQUIREMENTS.md itself does not yet reflect Phase 1's completion.

### Anti-Patterns Found

None. Both modified files (`.github/workflows/ci.yml`, `crates/vox-plugin-catalog/catalog.toml`) were scanned for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`placeholder` — zero matches. Both diffs are minimal and match their stated shape exactly (1/1 line substitution; 9-line comment-only addition).

### Human Verification Required

None. The phase's one human-decision point (SC#3's host-crate reinterpretation) was a blocking `checkpoint:human-verify` gate during execution, already resolved — the SUMMARY records the operator's verbatim reply ("Accept"), and the plan's gate structure (blocking, explicit resume-signal required, "any other reply or silence: stay paused") means this could not have silently defaulted to proceeding.

### Gaps Summary

No gaps found. All four ROADMAP success criteria are independently re-verified against the live codebase (not copied from SUMMARY.md), both commits (`1d81d7ae9`, `ccc29d5cf`) match their claimed diffs exactly, and the three requirement IDs (delete, catalog-cleanup, keep-frozen) all have concrete evidence. I additionally re-ran both of the phase's build-affecting commands live (`cargo check -p vox-scientia --all-features`, `cargo build -p vox-plugin-catalog`) and confirmed both exit 0. Nothing outstanding.

---

_Verified: 2026-09-23T02:10:00Z_
_Verifier: Claude (gsd-verifier)_
