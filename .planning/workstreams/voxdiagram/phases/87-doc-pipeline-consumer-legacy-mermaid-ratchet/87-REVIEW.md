---
phase: 87-doc-pipeline-consumer-legacy-mermaid-ratchet
reviewed: 2026-10-17T00:00:00Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - contracts/ci/crate-edges.allow.v1.json
  - contracts/ci/crate-graph.v1.json
  - contracts/documentation/legacy-mermaid-baseline.v1.yaml
  - contracts/index.yaml
  - crates/vox-cli/src/commands/ci/mermaid_baseline.rs
  - crates/vox-cli/src/commands/ci/mod.rs
  - crates/vox-cli/src/commands/ci/operations_catalog_exposure.rs
  - crates/vox-cli/src/commands/ci/pre_push.rs
  - crates/vox-doc-pipeline/Cargo.toml
  - crates/vox-doc-pipeline/src/lib.rs
  - crates/vox-doc-pipeline/src/pipeline/diagrams.rs
  - crates/vox-doc-pipeline/src/pipeline/lint.rs
  - crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs
  - crates/vox-doc-pipeline/src/pipeline/mermaid_gate_tests.rs
  - crates/vox-doc-pipeline/src/pipeline/mod.rs
  - crates/vox-doc-pipeline/src/pipeline/types.rs
  - docs/src/architecture/layers.toml
  - docs/src/architecture/where-things-live.md
  - docs/src/contributors/local-ci-pre-push.md
findings:
  critical: 1
  warning: 4
  info: 4
  total: 9
status: issues_found
---

# Phase 87: Code Review Report

**Reviewed:** 2026-10-17
**Depth:** standard (plus behavioural probes of the built `vox-doc-pipeline` binary against scratch trees under `scratchpad/probe87`; the worktree was not modified)
**Files Reviewed:** 19
**Status:** issues_found

## Summary

The core design is sound and mirrors the facet-backlog ratchet: fence-aware counting via `vox_diagram::fence::extract`, a fail-closed baseline load, a base-ref gate that checks `ls-tree` before treating an absent file as bootstrap, and a shrink-only `tighten`. The crate-edge exception entry matches the ledger's conventions (same field set, `user (explicit chat approval, 2026-10-16)`, layer direction stated). `vox ci crate-edges` reports OK, with one pre-existing stale-edge warning unrelated to this phase. Whole-tree scan cost is negligible: about 50 ms for a scoped lint over 1117 files. The `operations_catalog_exposure` initializer fix is correct and minimal. The `OperationRow` fields exist in `operations_catalog.rs`, and no other initializer in the crate lacks them. It was a pre-existing compile break on this branch.

The ratchet can still be bypassed. The most serious hole is that the whole-tree Mermaid lint does not run in the required fast tier for common PR shapes. Three smaller bypasses and one usability deadlock follow.

## Critical Issues

### CR-01: Whole-tree Mermaid lint is skipped in the fast tier (and so in the required `linux` CI job) for `.mdx`-only PRs

**File:** `crates/vox-cli/src/commands/ci/pre_push.rs:552-567, 914-933, 1193-1197` (claim at `crates/vox-doc-pipeline/src/pipeline/mod.rs:388`)
**Issue:** `mod.rs` documents that `mermaid_gate::lint` "always scans the whole docs/src tree so files nobody changed are still held to the baseline". That holds only if `lint_in` runs at all. In the fast tier it runs via `step_doc_frontmatter_scoped`, which returns early with "skipping" when `changed_docs_md_rel_paths` is empty. That function keeps only paths where `line.ends_with(".md")` (line 919). A PR that adds or edits `docs/src/**/*.mdx` therefore yields an empty set, and neither the Mermaid growth check nor the new `vox-diagram` fence lint ever executes.

The base-ref gate does not help. It compares only the baseline file against the base ref, and an `.mdx`-only PR does not touch it. So `docs/src/new-page.mdx` containing a `mermaid` fence passes the required `Check, Build, and Test (Rust)` gate. The only remaining catch is `docs-quality.yml` (`cargo run -p vox-doc-pipeline -- --lint-only`), which is a separate workflow. Nothing in this phase shows it is a required check. Today only `docs/src/index.mdx` exists, but Starlight pages are routinely `.mdx`.

**Fix:** Make the Mermaid ratchet independent of the scoped skip, and accept `.mdx` in the changed-files filter.
```rust
// pre_push.rs: changed_docs_md_rel_paths
if !(line.ends_with(".md") || line.ends_with(".mdx")) { continue; }
```
```rust
// pre_push.rs: step_doc_frontmatter_scoped, when rel_paths is empty,
// still enforce the whole-tree ratchet:
if rel_paths.is_empty() {
    return match vox_doc_pipeline::pipeline::lint_in(root, &["--lint-only".into(), "--paths=index.mdx".into()]) { ... };
}
```
A cleaner option is a `--mermaid-only` flag on `lint_in` that runs just `mermaid_gate::lint`, called unconditionally from a new fast-tier step. Add a regression test: an `.mdx`-only diff with a new fence must fail `vox ci pre-push`.

## Warnings

### WR-01: Any path containing the substring `SUMMARY.md` is invisible to the ratchet and to the diagram lint

**File:** `crates/vox-doc-pipeline/src/pipeline/lint.rs:151-181` (consumed by `mermaid_gate.rs:70`)
**Issue:** `gather_md_files` excludes with `path.to_str().contains("SUMMARY.md")`, a substring match on the full path rather than a file-name match. The Mermaid gate reuses this function, so it inherits the hole. Probed: `docs/src/x-SUMMARY.md` and `docs/src/SUMMARY.md.d/y.md` each containing a valid ```` ```mermaid ```` fence produced no lint output, while an identical `direct.md` failed with `legacy-mermaid-growth`. Those files are also skipped by the frontmatter, doctest and `vox-diagram` checks. The gate's own premise is "any new file with a fence fails", and this contradicts it. The old behaviour was harmless; making it load-bearing for a security-style ratchet is new in this phase. `find docs/src -iname '*SUMMARY.md*'` returns nothing today, so no current doc is affected.
**Fix:** Match the file name exactly, in both branches of `gather_md_files`:
```rust
fn is_generated_summary(p: &Path) -> bool {
    p.file_name().is_some_and(|n| n == "SUMMARY.md")
}
```

### WR-02: Stale baseline slack is non-blocking, so a migrated-away quota can be re-spent

**File:** `crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs:150-157, 300-302`
**Issue:** `count < allowed` is `Stale`, which is not hard. A PR that migrates three Mermaid diagrams in a doc to `vox-diagram` but does not run `--tighten-mermaid-baseline` leaves `allowed = 3`. A later PR can add three fresh ```` ```mermaid ```` fences to that doc. The lint passes (`count == allowed`) and the base-ref gate passes (baseline unchanged). That is growth the ratchet was meant to forbid, and Phase 90's bulk migration will create exactly this slack. The facet ratchet uses the same warn-only pattern, but there the slack re-admits a rule excuse, not an unbounded content type.
**Fix:** Make `Stale` hard (`is_hard` returns `true`) so a PR that shrinks a file must tighten in the same change. The message already names the one-command fix. Alternatively, have the base-ref gate fail when any path's live count is below its head baseline.

### WR-03: Fences inside JSX/HTML blocks in `.mdx` are not counted but are real code blocks to MDX

**File:** `crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs:95-104` (via `vox_diagram::fence::extract`, `crates/vox-diagram/src/fence.rs:5-6`)
**Issue:** `fence.rs` documents that MDX JSX blocks are HTML to CommonMark. For the ratchet this is a bypass, not a best-effort caveat. Probed: `<Tabs>\n<TabItem label="a">\n```mermaid ... ```\n</TabItem>\n</Tabs>` in a `.mdx` file produced no `legacy-mermaid-growth`. The same content with blank lines around the fence was caught. MDX renders the first form as a code block, so the diagram escapes the count and, in Phase 89, would escape the `vox-diagram` validation too. Only one `.mdx` page exists today, so exposure is low, but Starlight `Tabs`/`Steps` are the natural place for diagrams.
**Fix:** Add a cheap cross-check in `mermaid_gate::lint`. Count raw lines matching `^\s*(```+|~~~+)\s*mermaid\b` and emit a hard `InfoString`-style issue when that count exceeds the pulldown count and the file is not accounted for. That turns a silent miss into a loud one without parsing MDX.

### WR-04: Moving or renaming a baselined doc deadlocks with no legal path

**File:** `crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs:143-149, 276-286`
**Issue:** A `git mv docs/src/a.md docs/src/b.md` of a doc with a fence makes the lint fail (`b.md` is unlisted, so `allowed = 0`). The only way to satisfy it is to add `b.md: 1` to the baseline, but the base-ref gate then reports `b.md: 0 -> 1` and rejects it. The heal text sends the user to a re-seed that the gate also rejects. Total fence count did not grow, yet every route is blocked, so doc reorganisation (common in the Phase 90 migration window) needs a manual override of the guard. Not a bypass, but the gate fails closed in a case it cannot recover from.
**Fix:** Let the gate tolerate moves. In `run_mermaid_baseline_gate`, compute renames with `git diff -M --name-status <base>...HEAD -- docs/src` and treat `old -> new` as carrying `old`'s base allowance. Alternatively, compare total fence count alongside per-file counts, and document that a move needs `--tighten` and a user-authorized reseed.

## Info

### IN-01: Base-ref gate compares against the tip of `origin/main`, not the merge-base

**File:** `crates/vox-cli/src/commands/ci/mermaid_baseline.rs:17-19`; `pre_push.rs:878-880`
**Issue:** If `main` tightens the baseline after a branch forks and the branch is not rebased (typical for local pre-push), the unchanged head baseline reads as `n -> m` growth against the new tip, with a misleading message. CI is unaffected, since `pull_request` checks out the merge ref. This copies the facet gate's behaviour.
**Fix:** Resolve `git merge-base <base> HEAD` and use that as the comparison ref.

### IN-02: `vox-diagram` lint covers `docs/src/archive/` in full runs but its own regression test excludes it; unreadable files are skipped silently

**File:** `crates/vox-doc-pipeline/src/pipeline/diagrams.rs:75-83, 306-319`; `lint.rs:187`
**Issue:** `check_diagram_fences` has no archive exclusion, while `committed_docs_diagram_fences_are_valid` skips `archive/` and the Mermaid gate excludes it by design. The tombstoned tree is then linted by one check and not the other. Separately, `lint_one_file` uses `read_utf8_path_capped(...).unwrap_or_default()`, so an oversized or non-UTF-8 doc gets an empty string and its diagram fences go unchecked. For non-archive docs the Mermaid gate would hard-fail with `Unreadable`, which masks this. For archive docs it is a silent skip.
**Fix:** Pick one policy and apply it to both the lint and the test. Report the read error as a lint error instead of defaulting to empty.

### IN-03: `--seed-mermaid-baseline` has no friction comparable to the facet seed

**File:** `crates/vox-doc-pipeline/src/pipeline/mod.rs:276-278`; `mermaid_gate.rs:185-197`
**Issue:** `--seed-facet-backlog` takes an `--epoch-note` that records why the baseline was raised. The Mermaid seed silently overwrites with whatever the tree currently holds, so raising the baseline is a single flag. The base-ref gate is the real backstop, so this is a hardening point, not a hole. Local `VOX_PREPUSH_BASE=HEAD` also makes the gate trivially pass, as it does for the facet gate.
**Fix:** Optionally require a note or a refusal when a baseline already exists and the seed would raise any count, and print a loud USER-AUTHORIZED-ONLY banner.

### IN-04: Scope gap and size

**File:** `contracts/documentation/site-mounted-repo-docs.v1.json`; `crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs` (414 lines)
**Issue:** Repo files mounted into the site (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`) are outside `docs/src`, so Mermaid fences added there are neither counted nor validated. None exist today, and `docs/superpowers/` is explicitly deferred. `mermaid_gate.rs` also sits at the 400-line warning tier of the God Object limit, and `mermaid_baseline.rs::run_mermaid_baseline_gate` duplicates `run_facet_backlog_gate` almost line for line.
**Fix:** Record the mounted-source gap next to the deferred `docs/superpowers/` item. Factor the shared "read base/head baseline text" helper if a third ratchet appears.

---

_Reviewed: 2026-10-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
