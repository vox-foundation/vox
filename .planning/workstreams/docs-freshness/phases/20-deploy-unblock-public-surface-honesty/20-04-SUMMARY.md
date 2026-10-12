---
phase: 20-deploy-unblock-public-surface-honesty
plan: 04
subsystem: docs-tutorials
tags: [docs, tutorials, verification, voxscript, honesty]
status: complete

requires: []
provides:
  - "scripts/docs/tutorial-verify.vox: verifies all docs/src/tutorials/*.md (registry commands + --help flags, strict vox doctests, install versions vs installation.md, relative links); default writes the record, --check writes nothing and exits 1 on a failing check or stale blob"
  - "contracts/documentation/tutorial-verification.v1.json: generated record, verified_at_commit b10e20dfdab2ae5b43e59352c94336416c43934f, 7 tutorials x 4 checks all pass"
  - "docs-astro/tests/unit/tutorial-record.test.mjs: blob-SHA freshness, all-pass and vox:skip-reason assertions, run by docs-quality CI's tests/unit glob"
  - "All 7 tutorials carry status: and describe current code"
affects: [20-05, 21]

actuals:
  tokens: 90000
  tasks: 3
  commits: 3
plan_head_before: a0060c8492d9bc8fef7dc49cc8d893666ca1d0d1

tech-stack:
  added: []
  patterns:
    - "Generated verification record keyed by git blob SHA, checked by a node:test in docs-quality CI (P20-D10: no verified_against frontmatter)"
    - "VoxScript interp scripts: caps directive must be line 1; normalize '..' lexically before fs.* calls; avoid `break` inside if-in-for"

key-files:
  created:
    - scripts/docs/tutorial-verify.vox
    - contracts/documentation/tutorial-verification.v1.json
    - docs-astro/tests/unit/tutorial-record.test.mjs
  modified:
    - docs/src/tutorials/tut-getting-started.md
    - docs/src/tutorials/tut-first-app.md
    - docs/src/tutorials/tut-first-vox-app-checkpoints.md
    - docs/src/tutorials/tut-workflow-durability.md
    - docs/src/tutorials/tut-actor-basics.md
    - docs/src/reference/installation.md

key-decisions:
  - "Node.js >= 22.13 and pnpm >= 11 are the user-facing minimums (evidence below); CI's Node 24 / pnpm 11 pin is cited, not used as the minimum"
  - "Command scanning covers shell-tagged fences AND inline code spans outside fences, so inline-only tutorials (checkpoints) are checked too"
  - "Actor tutorial rewritten to shipped behavior (Rule 1) rather than lightly edited: most of its claims had no implementation"

duration: ~3h (verifier runs take 1.5–4 min each under the build broker)
completed: 2026-10-09
---

# Phase 20 Plan 04: Tutorial Verification Summary

**Every tutorial is now checked against current code by a VoxScript verifier whose generated record (all 28 checks pass, keyed by blob SHA) is enforced by a docs-quality unit test; five tutorials and the install reference were corrected, including an honest rewrite of the actor tutorial.**

## Performance

- **Duration:** ~3h
- **Completed:** 2026-10-09
- **Tasks:** 3/3
- **Files:** 3 created, 6 modified

## Accomplishments

Final record: `verified_at_commit` = `b10e20dfdab2ae5b43e59352c94336416c43934f`; all 7 tutorials `pass` on `commands_in_registry`, `snippets_compile`, `install_versions_match`, `links_resolve`.

Per-tutorial fixes:

1. **tut-getting-started** — explains the real `vox init` scaffold verbatim (`table Note`, `server add_note`, `server list_notes`, `component App`, `routes`); removed `query get_notes` / `mutation create_note`; claims checked against a real build (`POST /api/add_note` routes in `target/generated/src/main.rs`, `add_note` in `dist/vox-client.ts`, `Note` struct + SQL in `lib.rs`); Node/pnpm numbers replaced by a link to installation.md; `status: "current"`.
2. **tut-first-app** — `mkdir vox-task-list && cd && vox init` nests the project one level deeper (verified), so it is now `vox init vox-task-list && cd vox-task-list`; dropped the redundant `--kind application`; `vox build ... -o dist`; the assembled file passes `vox check` and `vox build`; removed the "Hello Vox"/"collaborative in action" overclaim.
3. **tut-first-vox-app-checkpoints** — removed the stale `script-execution` caveat (`default = ["keyring-store", "script-execution"]` in `crates/vox-cli/Cargo.toml`); `vox check --json` emits `error_code`/`severity`/`message`/`span`, not `category` (verified on a type error); populi needs `vox-ml-cli --features populi` with a link to `installation.md#beyond-the-cli`; golden link points at `../examples/golden.md`; `status: "current"`.
4. **tut-workflow-durability** — the dead `{{#include …getting_started.vox:logic}}` replaced by the `ANCHOR: display` region of `examples/golden/durable_workflow_real.vox` verbatim + `Source:` note; duplicate sentence removed; `with { retries: 3, timeout: "30s", initial_backoff: "1s" }` backed by a compiled snippet; `vox mens workflow` attributed to `vox-ml-cli` (`workflow-runtime` feature for the journaled interpreted runtime).
5. **tut-actor-basics** — rewritten (see deviations); `status: "current"`; `#vox-run-file----args` anchor kept.
6. **tut-ui-integration** — `table Task { task_id: Id[Task] … }` and view calls compile under strict doctest; no drift, no edit.
7. **use-a-react-component-from-vox** — excerpt matches `examples/golden/react_interop.vox`; no edit.

Node/pnpm evidence (installation.md now says Node.js >= 22.13 and pnpm >= 11, consistently, and the doctor table no longer calls them Optional — `doctor/checks_standard/toolchain.rs` requires both outside the `minimal` tier):
- `crates/vox-cli/src/templates/spa.rs` `pnpm_workspace_yaml()` emits pnpm 11 `allowBuilds` (pnpm < 11 does not understand it).
- pnpm 11.25.0 `package.json` engines: `"node": ">=22.13"`.
- Generated frontend deps are less strict: `@tailwindcss/oxide` 4.3.3 `node >= 20`; `vite` 6.4.3 `^18 || ^20 || >=22`.
- `contracts/toolchain/workspace-toolchain.v1.yaml` pins Node 24 / pnpm 11 for CI.

Actor codegen location checked: `crates/vox-codegen/src/codegen_rust/emit/durability_lower.rs` `emit_actor_body` (mailbox loop via `::vox_actor_runtime::spawn_process`, `ctx.receive()`, `Envelope::Message` / `Request` + `ProcessContext::reply` / `Signal` ignored, JSON `{"event","args"}`), `emit_actor_dispatch_arm`; `crates/vox-codegen/src/codegen_rust/emit/workflow.rs` `emit_actor_state_structs` and `emit_fn_with_actor_handlers`; parser `crates/vox-compiler/src/parser/descent/decl/mid.rs` `parse_actor_decl`.

## Task Commits

1. **Task 1: Verifier, record, freshness test, getting-started rewrite** — `f1cb89e99`
2. **Task 2: first-app, checkpoints, workflow-durability, installation prerequisites** — `b10e20dfd`
3. **Task 3: actor rewrite, ui/react re-verified, all-pass assertion** — `c0b7fc9cc`

## Verification

- `vox run --mode interp scripts/docs/tutorial-verify.vox -- --check`: exit 0, "all 7 tutorials pass and the record is fresh".
- `node --test tests/unit/tutorial-record.test.mjs`: 7/7 pass; whole `tests/unit/*.test.mjs` glob: 17/17.
- Mutation check: appending a line to `tut-ui-integration.md` made the node test fail ("tutorial changed since verification …") and `--check` exit 1 with `STALE …`; file restored (blob hash re-checked).
- `vox ci doctest-md --strict` fails closed on a type-error snippet (checked on a scratch file), so `snippets_compile` is not vacuous.
- Task 2 grep counts `{{#include` / duplicate sentence / `script-execution` / `^status:` = `0,1,0,1`.
- `grep -L '^status:' docs/src/tutorials/*.md`: empty; jq all-pass count: 7.
- `cargo run -q -p vox-doc-pipeline -- --lint-only --paths tutorials,reference/installation.md`: no hard errors.
- Every commit: pin script exit 0, `git diff --cached | rg lean-ctx` empty, numstat reviewed.

## Deviations from Plan

**1. [Rule 1 - Bug] tut-actor-basics rewritten, not just its lowering sentence fixed**
- **Issue:** Its `state count: int` field, `state_load`/`state_save`, `spawn X()`, `ref.send`, `await ref.get()`, `Codex::get_actor_state`, `enum XMessage` and `mpsc`/`oneshot` claims have no implementation; `state …` inside an actor is a parse error, `spawn Counter()` is a parse error, and a real `vox build` emits an empty dispatch `match` (handlers routed only when state fields exist; the parser never produces any).
- **Fix:** Retitled "Actor Basics"; compiled `actor Counter { on … }` snippet; table of what is actually emitted; a "Current Limits" section. Code defect logged in `deferred-items.md` (no Rust changes in this plan).

**2. [Rule 2] Inline code spans scanned for commands**
- The plan's check reads shell fences; checkpoints lists its commands only as inline spans, so the verifier also scans backtick spans outside fences.

**3. [Rule 3 - Blocking] VoxScript interpreter workarounds in the verifier**
- `// vox:caps` is only honored on line 1 (moved there); `fs.*` rejects any path with `..` (link targets are normalized lexically, escaping the repo fails); `break` inside `if` inside `for` returns a `_Break` sentinel from the function (replaced by `return`/loop-condition flags); `x is not "lit"` parses as `x is (not "lit")` (kept `!=`). Interpreter bugs logged in `deferred-items.md`.
- `VOX_BIN` env var lets the doctest step use a specific `vox` binary (default `vox` on PATH).

**4. [Rule 1] installation.md doctor table**
- "Node.js >= 18 | Optional" was wrong on both counts; split into Node.js and pnpm rows marked required outside the `minimal` tier, and the example doctor output updated to what doctor prints (bare versions, plus a pnpm line).

## Known Limits of the Verifier

- `install_versions_match` compares the first version after a tool name on a line; it is deliberately simple and only covers Node.js, pnpm and Rust.
- Flag checks run `cargo run -q -p vox-cli -- <path> --help`; commands delegated to `vox-ml-cli` are noted, not flag-checked.

## Deferred Issues

Logged in `deferred-items.md`: empty actor dispatch in `vox build`; interpreter `break` leak; `is not` literal precedence vs. the `==` lint; the record not listed in `contracts/index.yaml`.

## Known Stubs

None.

## Self-Check: PASSED
