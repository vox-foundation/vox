---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 11
subsystem: gui
tags: [kb_findings, gui-side-doors, ingest-url, ingest-text, publish-research-doc, trust-tier, playwright]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "capture_finding / CaptureInput (03-04), research_repo_root pub(crate) and GUI Save-to-KB findings (03-06)"
provides:
  - "kb_tools::findings::{FindingFiled, file_gui_finding, PublishFiled, file_publish_draft}"
  - "GUI ingest_url / ingest_text / publish_research_doc file pending Untrusted findings; no knowledge_nodes, no docs/src writes"
  - "PublishDocResult {finding_id, proposed_path, status} (Rust + TS)"
  - "Playwright stepper kb-inbox-side-doors.spec.ts with kb-inbox-*.png evidence"
affects: [03-12, 03-13, phase-12-inbox-gui]
tech-stack:
  added: []
  patterns: ["side-door logic and tests live in mcp helpers (vox-gui Rust tests need the Tauri sidecar)", "findings DTO status is pending|duplicate; duplicate is a toast, not an error"]
key-files:
  created:
    - crates/vox-orchestrator-mcp/tests/kb_findings_gui.rs
    - crates/vox-gui/ui/e2e/kb-inbox-side-doors.spec.ts
  modified:
    - crates/vox-orchestrator-mcp/src/kb_tools/findings.rs
    - crates/vox-gui/src/commands/knowledge.rs
    - crates/vox-gui/src/commands/research.rs
    - crates/vox-gui/src/main.rs
    - crates/vox-gui/ui/src/components/surfaces/Memory/MemoryView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Memory/MemoryView.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts
    - crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.tsx
    - crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx
    - contracts/reports/gui-surface-coverage.v1.json
key-decisions:
  - "D-05 interim visibility: toasts say 'Filed for review ... pending review and not yet searchable'; no visible text says 'Published' (Playwright-asserted)"
  - "D-06: every side-door finding is Untrusted; dedupe surfaces as status duplicate ('Already pending review'), not an error"
  - "publish keeps store_research_artifact as a best-effort episodic log (its failure no longer reports as indexed:false; the field is gone)"
  - "Empty slug is rejected too (beyond the old GUI rules), since it would propose docs/src/architecture/-2026.md"
patterns-established:
  - "Tracer-first: Ingest URL proved the helper -> Tauri command -> inbox -> toast slice before Ingest Text / Publish expanded"
requirements-completed: [GROW-06, TRUST-02]
duration: ~45 min wall (two long cargo builds on a loaded host)
completed: 2026-10-08
status: complete
commits: 5
plan_head_before: 865ed9fddfadbf9b7c68b7020921c9c204a1da18
actuals:
  tokens: 28000
  tasks: 3
  commits: 5
---

# Phase 3 Plan 11: GUI side doors into the inbox Summary

Ingest URL, Ingest Text and Publish research doc now file pending `Untrusted` findings through two small mcp helpers; the caller-less `save_research_doc` command (which wrote `docs/src/architecture` with `status: current`) is deleted.

## Commits

- `d3937de87` test(03-11): failing test for GUI ingest-url inbox filing (RED, Task 1)
- `12ebd7b30` feat(03-11): GUI Ingest URL files a pending Untrusted finding (tracer, Task 1 GREEN)
- `976c3d149` test(03-11): failing tests for publish-draft inbox filing (RED, Task 2)
- `6ffa45757` feat(03-11): Ingest Text and Publish file pending findings; delete save_research_doc (Task 2 GREEN)
- `72a42a2e3` feat(03-11): side-door toasts say filed for review; add Playwright stepper (Task 3)

## What was built

- **mcp helpers.** `file_gui_finding(db, root, origin, title, text, source_url, provenance) -> FindingFiled` wraps `capture_finding` (text = full_text) and maps `finding_id` Some/None to `pending`/`duplicate`. `file_publish_draft(db, root, session_id, slug, content) -> PublishFiled` validates the slug (rejects empty, `/`, `\`, `..`, leading `.`), builds `docs/src/architecture/<slug>-2026.md` as a string without touching disk, and captures with origin `publish_research_doc`, `dedupe_scope = proposed_path`, provenance `{proposed_path, session_id}`.
- **GUI Rust.** `ingest_url` keeps URL validation, robots check and scrape, then files via `file_gui_finding` (origin `GuiIngestUrl`, provenance title + text density). `ingest_text` keeps the empty-content check and files with origin `GuiIngestText`. `publish_research_doc` calls `file_publish_draft` then a best-effort `store_research_artifact`; the atomic docs write and its import are gone. `save_research_doc` and its `main.rs` registration deleted; `gui-surface-coverage.v1.json` regenerated (one line removed).
- **GUI UI.** `FindingFiled` and `PublishDocResult` TS types mirror the DTOs; `onPublished(proposedPath, status)`; toasts `Filed for review` / `Already pending review`; Save-to-KB body "N source(s) filed for review; not yet searchable"; Memory URL/Text card descriptions and the text button ("File for Review") no longer promise knowledge-graph storage; the publish modal blurb says nothing is written until a reviewer approves.
- **Playwright.** `kb-inbox-side-doors.spec.ts` (installTauriMock + addInvokeOverrides, 4 tests): URL pending, URL duplicate, Text, Save-to-KB + Publish; asserts toast copy, the proposed path, and `getByText(/Published/)` count 0; screenshots `kb-inbox-ingest-url.png`, `-ingest-url-duplicate.png`, `-ingest-text.png`, `-save-to-kb.png`, `-publish-doc.png` in `review-bundle/latest/` (gitignored, not committed).

## TDD / tracer

- RED evidence: `gui_inbox_ingest_url_writes_no_knowledge_node` failed on `unwrap()` of the stub's `Err("not implemented")` (target test, planned behavior); likewise `gui_inbox_publish_writes_no_docs_file` and `gui_inbox_publish_does_not_double_the_suffix_and_reports_duplicates` in Task 2. `test(...)` commits precede each `feat(...)` commit. (The `gui_inbox_empty_text...` / `rejects_traversal_slugs` tests pass trivially against a stub that always errs; their guard strength is proven by the mutations below.)
- Tracer gate: interactive mode, automated-only `<verify>` re-run end to end (mcp tests, MemoryView vitest, `cargo check -p vox-gui`) green before expanding.

## Mutation proofs (grep -c presence counts; mutations applied with the Edit tool)

| Guard | Mutation | before / mutated / restored | Result |
|-------|----------|-----------------------------|--------|
| T-03-31 no knowledge_nodes (file_gui_finding) | add `db.upsert_knowledge_node(..)` after capture | `upsert_knowledge_node` in findings.rs: 0 / 1 / 0 | `gui_inbox_ingest_url_writes_no_knowledge_node` FAILED at the `list_knowledge_nodes(..).is_empty()` assertion (kb_findings_gui.rs:40) |
| T-03-12 no docs/src write (file_publish_draft) | add `create_dir_all` + `fs::write` under `<root>/docs/src/architecture` | lines matching `docs/src/architecture\|create_dir_all`: 1 / 3 / 1 (`create_dir_all` alone 0 after restore) | `gui_inbox_publish_writes_no_docs_file` FAILED at the docs/src existence assertion (kb_findings_gui.rs:123) |
| T-03-45 traversal slug | delete `slug.contains("..")` and `slug.starts_with('.')` | `slug.contains("..")`: 1 / 0 / 1 | `gui_inbox_publish_rejects_traversal_slugs` FAILED: `slug ".." must be rejected` |

All restored; after each restore `kb_findings_gui` ran 7/7 green.

## Verification run locally

- `cargo nextest run -p vox-orchestrator-mcp --test kb_findings_gui`: 7 passed (includes `common::vox_home_is_hermetic`).
- `cargo check -p vox-gui`: green (after Task 1 and again after Task 2).
- vitest, only the touched suites (`src/components/surfaces/Memory`, `.../Research`): 13 files, 101 tests passed. `pnpm --dir crates/vox-gui/ui typecheck`: clean.
- `pnpm exec playwright test kb-inbox-side-doors`: 4 passed; 5 `kb-inbox-*.png` produced; the publish screenshot was viewed (toast "Filed for review ... Proposed docs/src/architecture/session-1-research-2026.md is pending review and not yet searchable").
- `vox ci gui-surface-coverage --write` then `vox ci gui-surface-coverage`: "report is up to date" (see deviation 1).
- Acceptance greps: `save_research_doc\b` in `crates/vox-gui` and `contracts` prints nothing; `atomic_write_secure` in `publish_research_doc` 0; `upsert_knowledge_node` in `ingest_url` / `ingest_text` 0; no `Published Architecture SSOT|URL Ingested|Document Stored` under `ui/src`.

## Deferred to CI (lean verification policy)

- Full `pnpm --dir crates/vox-gui/ui test` (not just the touched suites) and the full Playwright suite / `test:e2e` (only the new spec ran).
- `cargo check -p vox-gui` is the only vox-gui Rust check; no `cargo nextest -p vox-gui` run (the new logic is tested in mcp).
- `cargo clippy`, `vox ci crate-edges` / `sql-surface-guard` / other guards: not run (no new crate edge: vox-gui already depends on vox-orchestrator-mcp; tests avoid raw `.connection()` SQL).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - blocking] `cargo run -p vox-cli -- ci gui-surface-coverage` hung**
- **Found during:** Task 2
- **Issue:** rustc for `vox_cli` sat at ~0% CPU for 12+ min (the known host hang; another IDE-owned `cargo check` was also contending on this worktree).
- **Fix:** killed my cargo/rustc, ran the pre-built `target/debug/vox ci gui-surface-coverage --write` (the generator reads `crates/vox-gui/src/main.rs` and catalogs at runtime). The diff is exactly the removal of `"save_research_doc",`, and the check mode reports up to date. Nothing hand-edited. `target/debug/incremental` was not deleted. Stale-binary warning applies (built at 604a35abd) but the generator output is verified by the diff.

**2. [Rule 2 - missing validation] empty slug**
- `file_publish_draft` also rejects an empty slug (it would propose `docs/src/architecture/-2026.md`); covered by `gui_inbox_publish_rejects_traversal_slugs`.

**3. [Scope note] copy beyond the listed strings**
- The Memory URL/Text card descriptions, the text-ingest button label ("Store Document" -> "File for Review") and the publish modal blurb also promised knowledge-graph storage / publication; changed to match D-05. Button labels "Publish Architecture SSOT" / "Approve & Publish" were left (action names; vitest asserts them).

## Known gaps / limits

- `PublishDocResult.indexed` is removed: the episodic `store_research_artifact` result is no longer surfaced.
- The ResearchView publish toast is covered by the Playwright stepper, not a vitest case (it requires driving the modal inside ResearchView); the Save-to-KB toast has a vitest case.
- The `×2` badge visible in the publish screenshot is the existing toast de-dup counter (Save-to-KB and publish both read "Filed for review").
- Findings are only visible via toasts (and `vox kb status` in 03-13) until the Phase 12 Inbox GUI (D-05, accepted regression).

## Known Stubs

None.

## Threat Flags

None. No new endpoints or trust-boundary surface; two sinks (S5/S6 and the docs writer) were closed.

## Self-Check: PASSED

Files found: findings.rs, kb_findings_gui.rs, kb-inbox-side-doors.spec.ts, this SUMMARY. Commits found: d3937de87, 12ebd7b30, 976c3d149, 6ffa45757, 72a42a2e3.
