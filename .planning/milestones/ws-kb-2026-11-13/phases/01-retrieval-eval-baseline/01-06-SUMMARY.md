---
phase: 01-retrieval-eval-baseline
plan: 06
subsystem: retrieval-eval
tags: [vox-cli, eval, bm25, beir, kb, snapshot]
requires: ["01-01"]
provides:
  - "vox kb eval: report (+ --json, --split), --query/--top BM25 pooling, --headings DOC_ID, --snapshot-corpus"
  - "contracts/kb/eval/corpus.jsonl: frozen BEIR corpus snapshot (AP-01), deterministic and GUID-free"
  - "Bm25Index::rank_scored: ranked chunks with their BM25 score"
affects: [01-07, 01-08, 01-09, 01-10, 01-11, 01-12]
tech-stack:
  added: []
  patterns: ["mode fns take explicit paths + a writer; stdout deterministic, diagnostics on stderr"]
key-files:
  created:
    - crates/vox-cli/src/commands/kb.rs
    - crates/vox-cli/src/commands/kb/eval.rs
    - contracts/kb/eval/corpus.jsonl
  modified:
    - crates/vox-cli/src/lib.rs
    - crates/vox-cli/src/cli_dispatch/mod.rs
    - crates/vox-cli/src/commands/mod.rs
    - crates/vox-search/src/kb_eval/bm25.rs
    - .gitignore
    - .gitattributes
    - .voxignore
    - .cursorignore
    - .aiignore
    - .aiexclude
key-decisions:
  - "snapshot_decision: frozen-snapshot (user decision recorded in-session; 01-04 SUMMARY not yet written because 01-04 is paused at its query-authoring checkpoint)"
  - "run_report validates splits only for --split test; dev never calls validate_splits (it would read test qrels)"
requirements-completed: []
status: complete
duration: ~25min
completed: 2026-10-04
commits: 2
plan_head_before: 6f421cad4d5e5a74a6548e24a15cb7643383bed9
actuals:
  tokens: 2800
  tasks: 2
  commits: 2
---

# Phase 1 Plan 06: `vox kb eval` command and frozen corpus snapshot Summary

`vox kb eval` (report, BM25 pooling, headings, snapshot) wired into the CLI, plus the committed 670-doc frozen corpus snapshot with attribute and AI-context-ignore entries. EVAL-01 and EVAL-02 stay open: later plans (01-07, 01-08..01-12) also serve them.

## What was built

- **Task 1 (`161919567`)**: `commands/kb.rs` (`KbCmd::Eval`, `run`) and `commands/kb/eval.rs` (`KbEvalArgs`, `run_report`, `run_query`, `run_headings`, `run_snapshot`). `Cli::Kb` wired at all three dispatch match sites. 7 tests (3 clap, 4 fixture-driven). No raw `git`: the snapshot lists files through `vox_git::read_cmd::read_only`.
- **Task 2 (`1151d3aa6`)**: `contracts/kb/eval/corpus.jsonl`, `linguist-generated=true -diff` in `.gitattributes`, `.voxignore` entry with `.cursorignore`/`.aiignore`/`.aiexclude` regenerated via `vox ci sync-ignore-files`.

## Snapshot facts (for 01-11 `epoch_note`)

- Docs: **670**; bytes: **10,099,600**; source: tracked non-archive `docs/src` markdown at commit **`161919567a2e61274ebb30408f6e1cb03caa2089`** (docs/src is unchanged since `6f421cad4`).
- Two consecutive runs produce identical bytes (shasum equal); every `_id` starts with `docs/src/`, none contain `/archive/`; the canary GUID read from `contracts/kb/eval/README.md` is absent from the file.

## Verification

- `cargo nextest run -p vox-cli --lib commands::kb`: 7 passed. `cargo nextest run -p vox-search --lib kb_eval::bm25`: 2 passed.
- `cargo clippy -p vox-cli -p vox-search --all-targets -- -D warnings`: clean.
- `vox kb eval --help` lists `--snapshot-corpus`; `vox kb eval --query "pre-push fast tier" --top 5` prints exactly 5 lines; `--headings` works on a real snapshot doc.
- Acceptance: `Cli::Kb` count 3; `pub mod kb;` present once; no `Command::new("git")` under `commands/kb*`; `eval.rs` 167 non-blank lines.
- Not done: no mutation proof of the clap exclusivity/`requires` rules (tested only by parse success/failure). Plain `vox kb eval` on the real dir has no queries yet, so the report mode is exercised only against the vox-search fixture.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `*.jsonl` in `.gitignore` hid the snapshot**
- **Found during:** Task 2 staging (`git status` did not list `corpus.jsonl`; `git check-ignore` named `.gitignore:189`).
- **Fix:** added `!contracts/kb/eval/*.jsonl` beside the existing test-fixture exceptions (also covers 01-04's `queries.jsonl`).
- **Files modified:** `.gitignore`. **Commit:** `1151d3aa6`.

**2. [Rule 3 - Blocking] BM25 scores were not exposed, but `--query` prints them**
- **Found during:** Task 1 design (`Bm25Index::rank` returns only `RankedChunk{id, bytes}`).
- **Fix:** added `Bm25Index::rank_scored` (score quantized to 1e-9, same ordering); `rank` now delegates to it, behavior unchanged. Test extended in the same file.
- **Files modified:** `crates/vox-search/src/kb_eval/bm25.rs`. **Commit:** `161919567`.

### Notes

- The precondition on `01-04-SUMMARY.md` (`snapshot_decision: frozen-snapshot`) was satisfied by the user's recorded in-session decision and the committed 01-04 Task 2 README, per the dispatch instruction; golden-draft was not touched and no queries/qrels were created.
- `ci command-compliance` flagging `kb eval` as unregistered is expected until 01-12.

## Known Stubs

None.

## Threat Flags

None. `--snapshot-corpus` writes only `contracts/kb/eval/corpus.jsonl`; paths come solely from `git ls-files docs/src` filtered by `is_kb_corpus_path`.

## Self-Check: PASSED

Created files present (`kb.rs`, `kb/eval.rs`, `corpus.jsonl`), commits `161919567` and `1151d3aa6` exist, `git rev-list --count` from the ledger base = 2.
