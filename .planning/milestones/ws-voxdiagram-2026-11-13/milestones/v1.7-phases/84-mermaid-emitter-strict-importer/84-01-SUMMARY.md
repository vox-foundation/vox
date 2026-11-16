---
phase: 84-mermaid-emitter-strict-importer
plan: 01
subsystem: diagram-mermaid
tags: [rust, mermaid, oracle, jsdom, pnpm, fixtures, corpus]
status: complete

requires:
  - phase: 83-07
    provides: "vox_diagram check/fmt, Ir/GraphIr, SequenceIr, fence::extract/normalize, limits, RESERVED"
provides:
  - "vox_diagram::mermaid public API: ImportOpts, Unsupported, ImportReport (with `sequence`), import, export"
  - "Tracer: flowchart TD / a --> b through import, canonical DSL, export, committed fixtures and the pinned real parser (three-way set equality)"
  - "escape.rs: the only Mermaid text encoder (all RESEARCH rows) and keeps_id"
  - "Node oracle (mermaid 11.17.2 under jsdom 30.0.1): accepts, decodeLabel, canon for flowchart/state/sequence, sameSets"
  - "Fixture loops for import-cases, corpus-roundtrip and export-hostile, wired into docs-quality CI by a path filter plus guard test"
  - "Frozen 33-fence corpus with MANIFEST.json and a completeness test"
affects: [84-02, 84-03, 84-04, 84-05, 87, 90]

actuals:
  tokens: 13400   # chars/4 over the diff excluding pnpm-lock.yaml and the 33 corpus data fixtures
  tasks: 3
  commits: 3
plan_head_before: 23f77cf556cd77d3ada9a0a15efda9481ea1c42d
plan_head_after: 12561ffb6734f0e1acb6d54b152b95882667c29c

tech-stack:
  added: ["mermaid@11.17.2 (docs-astro devDependency, exact)", "jsdom@30.0.1 (docs-astro devDependency, exact)"]
  patterns:
    - "Rust writes golden fixtures, Node reads them (UPDATE_GOLDEN=1 blesses)"
    - "Oracle independence: Rust canonical sets come from the IR alone; Node sets come from the real parser"

key-files:
  created:
    - crates/vox-diagram/src/mermaid/mod.rs
    - crates/vox-diagram/src/mermaid/escape.rs
    - crates/vox-diagram/src/mermaid/export.rs
    - crates/vox-diagram/src/mermaid/import/mod.rs
    - crates/vox-diagram/tests/support/roundtrip.rs
    - crates/vox-diagram/tests/mermaid_roundtrip.rs
    - crates/vox-diagram/tests/freeze_corpus.rs
    - crates/vox-diagram/tests/mermaid_corpus.rs
    - crates/vox-diagram/tests/fixtures/import-cases/00-minimal.mmd
    - crates/vox-diagram/tests/fixtures/import-cases-roundtrip/00-minimal.{vdg,export.mmd,sets.json}
    - crates/vox-diagram/tests/fixtures/corpus/ (33 .mmd + MANIFEST.json)
    - docs-astro/tests/lib/mermaid-oracle.mjs
    - docs-astro/tests/unit/mermaid-oracle.test.mjs
  modified:
    - crates/vox-diagram/src/lib.rs
    - crates/vox-diagram/src/print.rs
    - crates/vox-diagram/tests/support/mod.rs
    - docs-astro/package.json
    - docs-astro/pnpm-lock.yaml
    - docs-astro/tests/unit/workflow-paths.test.mjs
    - .github/workflows/docs-quality.yml

key-decisions:
  - "Unit tests live inline in each new src file (all four are well under 400 lines), not in sibling tests.rs files"
  - "Importer id grammar rejects ids ending in '-' so `a--->b` fails closed instead of reading as `a-` --> `b`"
  - "Dangling edge endpoints are exported as implicit Mermaid nodes (bare id, `n<i>` when unsafe), as the plan specifies; containers are 84-02's slice"

patterns-established:
  - "family(dir, suffix, body) in the oracle test: a fixture directory is skipped only when absent, and fails when present but empty"
  - "Goldens fail on a missing file and on any byte difference; UPDATE_GOLDEN=1 rewrites"

requirements-completed: []  # REND-06, MIG-01, MIG-02 are only tracer-proven here; the phase delivers them across plans 84-02..84-05

duration: ~50min
completed: 2026-10-16
---

# Phase 84 Plan 01: Tracer, Oracle and Frozen Corpus Summary

**Mermaid tracer proven end to end: `flowchart TD / a --> b` imports to canonical DSL, exports, and the pinned mermaid 11.17.2 parser (jsdom shim) agrees on id-free sets three ways; the oracle covers flowchart, state and sequence, runs in docs-quality CI on fixture changes, and the 33 live corpus fences are frozen with provenance.**

## Accomplishments

- **Task 1 (tracer), `54ba24560`.** `vox_diagram::mermaid` with the final public types, a flat-graph `export`, the tracer `import` (header, `<id> --> <id>` lines, fail-closed `unsupported` with block-relative lines, `limit-source` gate), and `escape.rs` with every RESEARCH encoding row and `keeps_id`. `print::push_quoted` extracted from `label()` with no behaviour change. Test support `roundtrip.rs` builds canonical sets from the IR only. Node helper + test drive the three-way assertions against the committed `00-minimal` fixtures. `mermaid`/`jsdom` added as exact devDependencies with pnpm 11.28.6.
- **Task 2 (oracle breadth + CI), `ca7bdb8d9`.** `canonState` (`[*]` to Start/End; composite/note/other states throw) and `canonSeq` (actors by index, solid/dashed messages, `over` notes; anything else throws). Fixture loops for `import-cases-roundtrip`, `corpus-roundtrip` (accounting against `unsupported.json`, `markdown-string` must be rejected by Mermaid) and `export-hostile`. `docs-quality.yml` path filter on both `push` and `pull_request`, guarded by a `workflow-paths` test.
- **Task 3 (corpus), `12561ffb6`.** `freeze_corpus` (`#[ignore]`, the only code reading `docs/`) writes 33 fixtures and `MANIFEST.json` from tracked `docs/src` files via `fence::extract`, skipping `docs/src/archive/`. `corpus_manifest_matches_fixtures` enforces 33 fixtures, 21 sources, no archive, header census 30/1/1/1. Block 10 is the gantt, 19 the sequence, 29 the how-to-rust-crate-imports line 53 block, 32 the populi state diagram (matches the RESEARCH inventory). A second freezer run left `git status` on the corpus directory empty.

## TDD evidence (RED observed before GREEN)

| Task | RED observed | GREEN |
|------|--------------|-------|
| 1 Rust | `cargo nextest run -p vox-diagram -E 'binary(mermaid_roundtrip)'`: compile failure, unresolved import `vox_diagram::mermaid` (exit 101) | 41 tests across `mermaid_roundtrip`, `mermaid::`, `print::tests`, `fmt_golden` pass |
| 1 Node | `node --test tests/unit/mermaid-oracle.test.mjs`: `ERR_MODULE_NOT_FOUND` for `mermaid-oracle.mjs` (exit 1) | 2 pass, 0 fail (`import-cases-roundtrip/00-minimal` among them) |
| 2 Node | 4 failures: state canon, composite/header throws, sequence canon, `docs-quality runs the Mermaid oracle...` | 13 pass, 0 fail across the two files |
| 3 Rust | `corpus_manifest_matches_fixtures` panicked: `corpus/MANIFEST.json is missing` | passes after the freezer ran |

A mid-task test bug (my own): the duplicate-id unit test used id `ok`, which `keeps_id` correctly rejects (starts with `o`); corrected the test to `kb`, no source change.

## Path-filter mutation proof (Task 2)

Before: `grep -c 'crates/vox-diagram/tests/fixtures' .github/workflows/docs-quality.yml` printed 2. Deleted the line from the `push` list: count 1, `node --test tests/unit/workflow-paths.test.mjs` FAILED (`docs-quality runs the Mermaid oracle when vox-diagram fixtures change`). Restored from a saved copy: count 2, both files green (13 pass), and `git diff` of the workflow shows only the four added lines (a comment and the path, per trigger).

## Verification

| Check | Result |
|-------|--------|
| `cargo check -p vox-diagram --all-targets` | clean, no warnings |
| `cargo clippy -p vox-diagram --all-targets -- -D warnings` | clean (after final commit) |
| `INSTA_UPDATE=no cargo nextest run -p vox-diagram` | 209 passed, 1 skipped (the ignored freezer) |
| `node --test tests/unit/mermaid-oracle.test.mjs tests/unit/workflow-paths.test.mjs` | 13 pass, 0 fail |
| `npx pnpm@11.28.6 install --frozen-lockfile` | exit 0, "Already up to date" |
| `lockfileVersion` / lockfile numstat vs `85ac1f7c8` | `'9.0'`; `1150 0` (zero deleted lines) |
| `pnpm-workspace.yaml`, `Cargo.lock`, `crates/vox-diagram/Cargo.toml` vs `85ac1f7c8` | unchanged |
| Acceptance greps (pins, `await import('mermaid')`, `pub sequence`, golden `cmp`s, 33 fixtures, 33 `"source"`) | all as specified |
| Largest new file (non-blank lines) | `import/mod.rs` 161; all well under 400 |

## Deviations from Plan

### Auto-fixed / judgement calls

**1. [Rule 2 - fail-closed] Importer rejects ids ending in `-`**
- **Found during:** Task 1 importer grammar.
- **Issue:** the specified id class `[A-Za-z_][A-Za-z0-9_-]*` would read `a--->b` (a longer Mermaid arrow) as `a-` --> `b`.
- **Fix:** `is_id` also requires the id not to end with `-`; unit test `reserved_words_and_chained_edges_are_refused` covers `a- --> b`. Tracer goldens are unaffected; 84-03 owns the full grammar.
- **Files:** `crates/vox-diagram/src/mermaid/import/mod.rs`. **Commit:** `54ba24560`.

**2. Unit tests inline rather than sibling `tests.rs`**
- The research suggests the sibling-file pattern for the 400-line cap and tdd-guard; the new files are 41 to 161 non-blank lines and tdd-guard passed, so inline `#[cfg(test)] mod tests` keeps the diff small. Revisit if 84-03 grows `import/`.

Otherwise: plan executed as written.

## Deferred Issues

- `docs-astro/tests/unit/tutorial-record.test.mjs` ("tutorial verification record parses and matches git blob hashes") fails on `docs/src/tutorials/tut-workflow-durability.md` (blob `7077299c...` vs recorded `644b8bae...`). Pre-existing and unrelated to this plan (no `docs/` file touched); it is not in this plan's verify command. The full `tests/unit/*.test.mjs` run shows 877 pass, 1 fail, that one. Needs `vox run --mode interp scripts/docs/tutorial-verify.vox` by its owner.
- The generated API page (`docs/src/api/vox-diagram.generated.md`) does not yet list `mermaid`; plan 84-05 regenerates it with `vox ci crate-api-sync --write` as part of its governance task.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-84-02 holds: the JSDOM is built without `runScripts`/`resources`, `securityLevel: 'strict'`, helper under `tests/` only. T-84-SC: exact pins, lockfile written by pnpm 11.28.6, additive-only diff (1150 inserted, 0 deleted), `allowBuilds` unchanged.

## Notes for the next plans

- Node must always be run from `docs-astro/`; `node_modules/` there is gitignored.
- The Rust and Node canonical-set sort orders never need to agree (Node re-sorts both sides by `JSON.stringify`).
- `UPDATE_GOLDEN=1 cargo nextest run -p vox-diagram -E 'binary(mermaid_roundtrip)'` blesses `import-cases-roundtrip/`.

## Self-Check: PASSED

- Files exist: `src/mermaid/{mod,escape,export}.rs`, `src/mermaid/import/mod.rs`, `tests/{mermaid_roundtrip,freeze_corpus,mermaid_corpus}.rs`, `tests/support/roundtrip.rs`, `tests/fixtures/corpus/MANIFEST.json` (+33 `.mmd`), `docs-astro/tests/lib/mermaid-oracle.mjs`, `docs-astro/tests/unit/mermaid-oracle.test.mjs`.
- Commits `54ba24560`, `ca7bdb8d9`, `12561ffb6` are ancestors of HEAD.
