---
phase: 87-doc-pipeline-consumer-legacy-mermaid-ratchet
fixed_at: 2026-10-17T19:00:00Z
review_path: .planning/workstreams/voxdiagram/phases/87-doc-pipeline-consumer-legacy-mermaid-ratchet/87-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 87: Code Review Fix Report

**Fixed at:** 2026-10-17
**Source review:** `.planning/workstreams/voxdiagram/phases/87-doc-pipeline-consumer-legacy-mermaid-ratchet/87-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (CR-01, WR-01..WR-04; Info excluded by `fix_scope`)
- Fixed: 5
- Skipped: 0

All five fixes landed in one commit, `5f25715f4`. They share files (`mermaid_gate.rs`, `mermaid_gate_tests.rs`, `mod.rs`), so splitting them per finding was not practical. Each regression test was written and run red against the unfixed code first (4 tree tests failed as expected), then went green. The edits ran in the main checkout, not a separate worktree, because `.planning/config.json` sets `workflow.use_worktrees: false`.

## Fixed Issues

### CR-01: Whole-tree Mermaid lint is skipped in the fast tier for `.mdx`-only PRs

**Files modified:** `crates/vox-doc-pipeline/src/pipeline/mod.rs`, `crates/vox-cli/src/commands/ci/pre_push.rs`
**Commit:** 5f25715f4
**Status:** fixed: requires human verification
**Applied fix:** `lint_in` gains `--mermaid-only`, which skips the facet, file and README-sync lints and runs only `mermaid_gate::lint` over the whole tree. `build_steps` now has an unconditional step, "legacy-Mermaid ratchet (whole docs/src)", that calls it, independent of the scoped doc lint and of which extensions changed. The misleading comment in `mod.rs` now says what `--mermaid-only` does.

I did not widen the `.md` filter in `changed_docs_md_rel_paths` to `.mdx`. The scoped `doctest-md` step shares that path list, and I did not verify it handles `.mdx`.

Regression tests:
- `mermaid_only_runs_the_ratchet_for_any_extension` (vox-doc-pipeline): an `.mdx` fence fails, and a clean tree needs no facet contracts.
- `mermaid_lint_step_ignores_which_extensions_changed` (vox-cli): the step fails on an `.mdx`-only fence while the scoped step still skips on an empty set.

### WR-01: Any path containing `SUMMARY.md` is invisible to the ratchet

**Files modified:** `crates/vox-doc-pipeline/src/pipeline/lint.rs`
**Commit:** 5f25715f4
**Applied fix:** `gather_md_files` now uses one helper, `is_lintable_doc`. It excludes only a path whose components end with `docs/src/SUMMARY.md`. `x-SUMMARY.md` and `SUMMARY.md.d/y.md` are linted again. Because the Mermaid gate reuses this function, the other doc checks regain those paths too.

Regression test: `only_the_generated_summary_is_skipped`.

### WR-02: Stale baseline slack is non-blocking

**Files modified:** `crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs`, `crates/vox-doc-pipeline/src/pipeline/mod.rs`, `crates/vox-doc-pipeline/src/pipeline/types.rs`
**Commit:** 5f25715f4
**Status:** fixed: requires human verification
**Applied fix:** A stale entry (count below allowance) is now a hard error, and its message names `--tighten-mermaid-baseline`. I removed `MermaidIssue::is_hard`. With a constant `true` body it tripped the toestub hollow-body warning in the pre-commit hook, so `mod.rs` now treats every `LegacyMermaid` finding as `ERROR` and counts it as hard. `tighten` is still shrink-only. The `Stale` doc comment in `types.rs` is updated.

Regression test: `stale_entry_is_a_hard_error` replaces `stale_entry_warns_but_exits_zero`. It checks that lint exits 1, that tighten exits 0, and that lint then exits 0.

### WR-03: Fences inside JSX/HTML blocks in `.mdx` are not counted

**Files modified:** `crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs`
**Commit:** 5f25715f4
**Status:** fixed: requires human verification
**Applied fix:** For `.mdx` files, `counts` also does a raw line scan for `` ``` `` or `~~~` openers whose info string starts with `mermaid`. It uses the larger of that count and the pulldown count, and the raw line numbers when raw wins. This fails closed. A mermaid opener quoted inside a longer example fence in an `.mdx` file is over-counted and needs a baseline entry (user-authorized).

Not covered: the same HTML-block bypass can happen in `.md` files (for example, a fence directly inside `<details>` with no blank line). The review scoped WR-03 to `.mdx`, and applying the raw scan to `.md` would change existing counts and force a user-authorized re-seed. I left that as a known gap.

Regression test: `mdx_fences_inside_jsx_blocks_are_counted`, covering both the `<Tabs>` and the over-count cases.

### WR-04: Moving or renaming a baselined doc deadlocks

**Files modified:** `crates/vox-doc-pipeline/src/pipeline/mermaid_base_gate.rs` (new), `crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs`, `crates/vox-doc-pipeline/src/pipeline/mod.rs`, `crates/vox-cli/src/commands/ci/mermaid_baseline.rs`, `docs/src/contributors/local-ci-pre-push.md`, `docs/src/architecture/where-things-live.md`
**Commit:** 5f25715f4
**Status:** fixed: requires human verification
**Applied fix:**
- **Base-ref rule.** `mermaid_baseline_gate_failures(base, head, renames)` now takes the renames git detected. A baselined path may not exceed its base count. A path absent from the base baseline is admitted only as the target of an unused git-detected rename of a baselined path, where the old entry is removed from head and the new count does not exceed the old one. The total is non-increasing by construction. This is why there is no separate total check, which would be dead code. A delete-and-add of unrelated docs is rejected. The upward self-edit test (`gate_fails_on_upward_edit_against_the_base_ref`) is unchanged and green.
- **Gathering renames.** `run_mermaid_baseline_gate` gets the renames from `git diff -M --name-status -z <base> -- docs/src`. A move must be staged or committed to be detected, so an unstaged move fails closed.
- **Author path.** `tighten` hands the quota of a baselined doc that no longer exists on disk to one unlisted path whose count fits. Total and per-file counts never grow, and the base-ref gate demands the matching git rename. `MERMAID_HEAL` documents the rule.
- **File size.** The rule moved into the new `mermaid_base_gate.rs` (about 150 lines). `mermaid_gate.rs` is now about 360 lines, where it was 414.

Regression tests:
- `a_git_detected_rename_carries_the_quota_but_never_grows_it` (pure rule cases)
- `tighten_hands_a_moved_docs_quota_to_its_new_path_only`
- `gate_admits_a_git_detected_move_but_not_a_laundered_one` (real git repo)
- `gate_text_cases` moved to the new file.

## Verification

Gates ran in the main checkout, `/Users/brbrainerd/dev/vox-voxdiagram`, on branch `milestone/v1.7-voxdiagram`.

- `cargo nextest run -p vox-doc-pipeline`: 98 of 98 passed.
- `cargo nextest run -p vox-cli mermaid mermaid_lint_step`: 3 of 3 passed.
- `cargo clippy -p vox-doc-pipeline -p vox-cli --all-targets`: no findings in any touched file. I did not review the pre-existing `claims/` findings.
- `cargo fmt -p vox-doc-pipeline -p vox-cli`: clean.
- `cargo run -p vox-doc-pipeline -- --mermaid-only` on the real tree: no hard errors, so the committed baseline has no stale slack.
- The commit hooks (fmt-fix, tdd-guard/toestub) pass. The first attempt was blocked by the hollow `is_hard` body, which is fixed.
- `./target/debug/vox ci pre-push` (fast tier) aborts at `vox ci ssot-drift` with the known pre-existing "capability-index.v1.json is stale" drift. Steps after it, including the new Mermaid steps, were not reached in that run. Their coverage is the tests above. I did not commit a regeneration, and the run left no tracked changes.

## Other notes

- The `.planning/active-workstream` modification and the untracked `milestone.lock` were already present at the start and are untouched.
- I did not modify `/Users/brbrainerd/dev/vox`, any Cargo or lock file, or `crate-edges`.

---

_Fixed: 2026-10-17_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
