---
phase: 01-retrieval-eval-baseline
plan: 07
subsystem: cli-eval-gate
tags: [kb, ratchet, canary, tighten-only, vox-cli]
requires: ["01-05", "01-06", "01-12"]
provides:
  - "vox kb eval --gate / --base: one-pass regression + tighten-only + canary report, fail-closed"
  - "vox kb eval --tighten and --write-baseline --epoch-note: the only baseline writers"
  - "index-time canary assert on every index-building mode"
affects: [01-11]
tech-stack:
  added: []
  patterns: ["CLI gathers inputs, decisions stay in 01-05 pure functions", "all failures collected, sorted, deduped", "typed CorpusHasCanary error so the gate tags it [canary] not [regression]"]
key-files:
  created:
    - crates/vox-cli/src/commands/kb/gate.rs
  modified:
    - crates/vox-cli/src/commands/kb.rs
    - crates/vox-cli/src/commands/kb/eval.rs
    - docs/src/reference/cli.md
key-decisions:
  - "Typed CorpusHasCanary error (downcast in run_gate) instead of string matching, so a GUID in the corpus is reported as [canary]"
  - "Local jsonl scan reads one file at a time and passes it to texts_containing_guid (memory bounded to one file)"
  - "Base ref starting with '-' is rejected before any git call (option-injection guard on top of the read_only allowlist)"
requirements-completed: []
status: complete
duration: ~45min
completed: 2026-10-04
commits: 2
plan_head_before: 276929bff0bf0603f39520243c9d28074962cb8c
actuals:
  tokens: 7300
  tasks: 2
  commits: 2
---

# Phase 1 Plan 07: `vox kb eval --gate`, `--tighten`, `--write-baseline` Summary

The ratchet and canary logic from 01-05 is now reachable from `vox kb eval`: a fail-closed gate that reports every regression, loosening and contamination in one pass, and two noted, explicit baseline writers. EVAL-03 and EVAL-04 are also served by 01-11, so they are NOT marked complete here. The gate is not yet in the pre-push fast tier (01-11 wires it).

## What was built

- **Task 1 (`bc160d852`)**: new flags `--gate`, `--base REF` (default `origin/main`, requires `--gate`), `--tighten`, `--write-baseline`, `--epoch-note` (mutually required with write-baseline). Gate/tighten/write-baseline are mutually exclusive and conflict with `--split/--query/--headings/--snapshot-corpus`; all use `ratchet_split()`. `gate.rs` holds `run_tighten` (never lowers, writes only on change, errors without a baseline), `run_write_baseline` (needs a non-empty note, epoch 1 or +1) and the shared `dev_report`. `ensure_clean` (index-time canary assert) is called by `dev_report`, `run_report` and `run_query`; it names doc ids, never the GUID.
- **Task 2 (`c0c476ff1`)**: `run_gate(root, base)` collects every failure: report load/regression vs the working-tree baseline; base ref resolved fail-closed; base baseline via `git show BASE:...` (absent = bootstrap); golden-changed via `git diff --name-only BASE...HEAD` on queries/qrels/corpus (AP-15) feeding `check_tighten_only`; canary sweep (repo scan over `git ls-files`, root disjointness, real MENS docs extractor via `walk_and_extract_docs` on `root/docs`, local `mens/data` + `target/dogfood` jsonl, 40-char query fingerprints in tracked `docs/`+`crates/` markdown minus archive and the golden dir). Output sorted `[canary] < [regression] < [tighten-only]`, deduped; hints and elapsed go to stderr. `cli.md` documents the flags, categories and the dev-split rule.

## Verification

- `cargo nextest run -p vox-cli --lib commands::kb`: 18 passed (gate tests: 8). `cargo clippy -p vox-cli --all-targets -- -D warnings`: clean. `rustfmt --edition 2024 --config-path .` over the kb module.
- Fail-closed command (`kb eval --gate --base refs/heads/definitely-missing-kb-eval`): exit 1, output contains `cannot resolve base ref`, alongside the other two failures it can see (no `queries.jsonl` yet, since 01-10 has not run).
- `vox-doc-pipeline --lint-only --paths reference/cli.md`: no hard errors.
- Acceptance greps: `ratchet_split()` in gate.rs; `walk_and_extract_docs` present; no `Command::new("git")` under `commands/kb/`; gate.rs 485 non-blank lines (<500); `--write-baseline` appears once in cli.md.
- Gate elapsed in debug on this machine: 0.98 s on the real worktree, but this is a PARTIAL run (golden set not yet present, so the fingerprint step and scoring were skipped). The ≤10 s budget must be re-measured by 01-11 on the finished golden set.

## Mutation proofs (before/after presence checks)

- `ensure_clean` guard: `if !hits.is_empty()` count 1 -> mutated to `if false && !hits.is_empty()` (count 1 of the mutated form) -> `index_build_rejects_corpus_with_guid` FAILED -> restored (mutated form 0, original 1).
- `mens_extract_guid_hits`: `hits.sort();` count 1, stub marker 0 -> stubbed to return no hits (marker 1) -> `mens_extract_scan_flags_planted_guid_then_clears` and `canary_sweep_names_every_source_and_never_the_guid` FAILED (2 failed) -> restored (marker 0, sort 1). Re-run after the final refactor.
- `resolve_base`: made to return Ok -> `resolve_base_rejects_missing_ref` FAILED -> restored.
- Clap: removed `split` from `--gate` conflicts -> `ratchet_modes_reject_explicit_split` FAILED -> restored.
- Not mutation-tested: `tighten_failures` (needs a real git history; no raw git in tests per acceptance). Its fail-closed path is covered by the real-repo command above; the happy path is only exercised end to end in 01-11.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug avoided] A GUID in the corpus would be mislabelled `[regression]`**
- **Found during:** Task 2 design. `dev_report` bails on a corpus carrying the GUID, and the gate wrapped every load error as "golden set failed to load".
- **Fix:** `ensure_clean` returns a typed `CorpusHasCanary`; `run_gate` downcasts it and tags it `[canary]`.
- **Commit:** `c0c476ff1`.

### Notes

- Tests were written together with the implementation and passed first time, so no separate RED run was observed; mutation proofs above stand in for it.
- `ci command-compliance` exits non-zero ONLY because `docs/src/reference/cli-command-surface.generated.md` is stale (pre-existing since 01-12; regenerated by CI `ssot-autoregen`; `ci command-sync` deliberately not run). The plan's acceptance wording "still exits 0" cannot hold until then. The cli.md mention requirement it also checks is satisfied.
- The commit-1 `run_gate` was a placeholder that bailed; commit 2 replaced it.
- A `git` base-baseline `show` failure of any non-zero kind is treated as "absent at base" (bootstrap), as the plan specifies; a missing path and an unrelated git error are not distinguished.

## Known Stubs

None.

## Threat Flags

None. Git access is only through `vox_git::read_cmd::read_only`; failures name files, doc ids and query ids, never the GUID (asserted in tests).

## Self-Check: PASSED

Commits `bc160d852` and `c0c476ff1` exist; `git rev-list --count` from the ledger base = 2; `gate.rs`, `kb.rs`, `eval.rs`, `cli.md` are in those commits.
