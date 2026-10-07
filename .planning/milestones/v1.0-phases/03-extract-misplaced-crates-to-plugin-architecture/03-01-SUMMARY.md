---
phase: 03-extract-misplaced-crates-to-plugin-architecture
plan: 01
status: complete
subsystem: crate-graph / grammar-export / planning docs
tags: [dead-code, crate-edges, derived-contracts, D-11, D-05]
requires: []
provides:
  - "vox-grammar-export without the zero-consumer automaton module, with in-file tests"
  - "vox-populi without its unused vox-grammar-export dependency"
  - "ROADMAP SC#1/SC#3 and REQUIREMENTS acceptance amended per D-11 / D-05"
affects: [vox-grammar-export, vox-populi, contracts/ci/crate-graph.v1.json, contracts/ci/crate-build-map.v1.json]
tech-stack:
  added: []
  patterns: ["derived build-map applied as generator before/after delta", "HEAD-blob staging for shared dirty files"]
key-files:
  created: []
  modified:
    - crates/vox-grammar-export/src/lib.rs
    - crates/vox-populi/Cargo.toml
    - Cargo.lock
    - contracts/ci/crate-graph.v1.json
    - contracts/ci/crate-build-map.v1.json
    - docs/src/architecture/where-things-live.md
    - .planning/ROADMAP.md
    - .planning/REQUIREMENTS.md
  deleted:
    - crates/vox-grammar-export/src/automaton.rs
decisions:
  - "D-11 honoured: no grammar-export plugin and no GrammarExportPlugin ABI; the library stays CORE."
  - "D-05 honoured: vox-ssg recorded as already folded into vox-cli (L4), not CORE."
  - "Stale crate-edges baseline edge vox-populi -> vox-grammar-export left in place (no --tighten while 3 unrelated violations stand)."
metrics:
  duration: "~9 min wall clock (start 2026-09-26T08:13Z, end 08:23Z)"
  completed: 2026-09-26
plan_head_before: 44f029900d1264120672f577d413ac320b39b389
actuals:
  tokens: 2800
  tasks: 2
  commits: 3
---

# Phase 3 Plan 01: Grammar-export dead code out, SC#1/SC#3 closed per D-11/D-05 Summary

vox-grammar-export stays a CORE library (D-11). Its zero-consumer `automaton` module is gone, and so is vox-populi's unused dependency on it. The lockfile line, crate-graph element and single build-map field landed in the same commit as that manifest change. SC#1 and SC#3 now cite D-11 (crate-audit D-4/D-18) and D-05.

## Commits

| Task | Commit | Message | Paths |
|---|---|---|---|
| 1a | `ab63e98b2` | refactor(03-01): drop grammar-export's zero-consumer automaton module | `crates/vox-grammar-export/src/automaton.rs` (deleted), `crates/vox-grammar-export/src/lib.rs` |
| 1b | `0cfd80e4a` | refactor(03-01): drop vox-populi's unused vox-grammar-export dependency | `Cargo.lock` (0/1), `contracts/ci/crate-build-map.v1.json` (1/1), `contracts/ci/crate-graph.v1.json` (0/1), `crates/vox-populi/Cargo.toml` (0/1) |
| 2 | `83e1cd0ab` | docs(03-01): close SC#1 per crate-audit D-4 and SC#3 as not-CORE evidence | `.planning/REQUIREMENTS.md` (2/2), `.planning/ROADMAP.md` (2/2), `docs/src/architecture/where-things-live.md` (1/1) |

## Verification verdicts

- Precondition: index empty, no merge, the three owned paths clean. Met.
- Step 1 zero-use proof: `target/phase03-01-zero-use.txt` is 0 bytes. Neither `git grep` found anything.
- Step 0 HEAD row: `{"crate":"vox-grammar-export","compile_s":0.8,"dependents":53,"blast_s":588.0,"fan_in":6}`.
- Step 3(b) tests ran green on the unmodified code first: `test result: ok. 6 passed` (3 new, 3 existing compact_prompt tests).
- Task 1 verify 1: `test=0 check=0`. vox-grammar-export had 6 tests pass and vox-constrained-gen had 23 + 18 pass. vox-compiler and vox-populi check clean.
- Step 5(c) build-map delta: exactly one row changed, `{"crate":"vox-grammar-export","dependents":53,"blast_s":588.0,"fan_in":6}` -> `...,"fan_in":5}`.
- Task 1 verify 2 (staged blobs): `verify2=0`. The graph is the start graph minus one element, `affected-crates --check` passes, the build map is HEAD plus the single fan_in change (`measured_on` kept), and the lock patch is exactly `- "vox-grammar-export",`.
- Task 1 verify 3 (crate-edges): `rc=1`, verify `verify3=0`. The verdict line reads `Error: crate-edges: 3 violation(s)`, all three pre-existing: `NEW EDGE not in baseline: vox-gui -> vox-db-types`, `vox-gui -> vox-research-shim`, `vox-research-shim -> vox-compiler`. No NEW/UPWARD line names vox-populi or vox-grammar-export. There is also the expected warning `stale baseline edge vox-populi -> vox-grammar-export (gone; run vox ci crate-edges --tighten)`, left as it is.
- Task 1 verify 4 (tdd-guard command on lib.rs): `rc=0`, "No issues found".
- Step 7(b)/8(c) staged-set checks: `rc=0` both.
- Step 8(e) lock integrity: `cargo metadata` `rc=0`. `git diff --stat -- Cargo.lock` still shows the other sessions' 19 inserted lines.
- Task 2 verify 1: `verify1=0`. Task 2 verify 2 (doc lint): `rc=0`, "no hard errors".
- Task 2 staged-set check: `rc=0`. The committed ROADMAP numstat is `2 2`.
- Plan verification 3: `git status --porcelain -- crates/vox-grammar-export crates/vox-populi/Cargo.toml contracts/ci/crate-graph.v1.json` is empty.

## SSG evidence (D-05), from `target/phase03-01-ssg-evidence.txt`

```text
9d385a60b988f0ee8d6b19b69f963d5de441518b Tue May 12 08:11:46 2026 -0400 chore: Consolidate workspace crates and align dependencies
 .../{vox-ssg/src/lib.rs => vox-cli/src/utils/ssg/mod.rs}   |  0
 crates/vox-ssg/Cargo.toml                                  | 14 --------------
ABSENT
crates/vox-cli/src/utils/ssg/mod.rs
false
4
```

## Deviations from Plan

1. **Step 2 `crate_audit.json` left as found (not overwritten).** The compile_s comparison differed only in that HEAD's map lists `vox-mesh-transport` and `vox-plugin-mens-candle-core` at `compile_s: 0.0`. Those are exactly the map's `crates_without_compile_times: 2`, and the file has no entries for them. The existing file is the input that produced HEAD's map: the baseline run reported `missing=2`, and its rows equal HEAD's rows exactly. Overwriting it would have given those two crates explicit 0.0 times without changing the delta. The before and after runs used the same input, so the delta is unaffected.
2. **Shared-file state had changed since planning.** At start, `contracts/ci/crate-build-map.v1.json` and `docs/src/architecture/where-things-live.md` were clean, because the other sessions' edits had already been committed. Only `Cargo.lock` was dirty. The HEAD-blob protocol was still followed. For where-things-live, the first awk-built blob added a trailing newline that HEAD lacks (numstat 2/2). I rebuilt the blob as HEAD plus the one row: the working-tree file, clean before my edit. That gave numstat 1/1.
3. **Commit 1b message wording.** The plan's template body says HEAD's map "carries unrelated parity drift". At execution HEAD's rows matched the generator exactly (per the parent's note), so the message says the delta was one field and HEAD already matched.
4. **Pre-commit `fmt-fix` side effect (hook behaviour, not my edit).** On commit 1a the hook printed `fmt: formatted 48 file(s).` It runs `scripts/fmt.vox`, which formats every dirty `.rs` file in the shared working tree, including other sessions' uncommitted files. Only this commit's two staged paths were re-staged and committed, and I did not touch or revert the others. They may now carry rustfmt-only changes from the hook.

No `VOX_SKIP_FRESHNESS_CHECK` was needed. No `--no-verify`, stash, checkout, restore or reset. No `exceptions` entry, no `edges` edit, no `--tighten`, and fan-in-snapshot.v1.json was not touched. STATE.md and ROADMAP progress rows were not updated (the orchestrator owns them).

## Known Stubs

None.

## Self-Check: PASSED

- `crates/vox-grammar-export/src/automaton.rs` absent; `pub mod automaton;` count in lib.rs 0; vox-populi manifest has no `vox-grammar-export` line.
- Commits `ab63e98b2`, `0cfd80e4a`, `83e1cd0ab` present on `main`; `git rev-list --count 44f029900..HEAD` = 3 before this SUMMARY commit.
