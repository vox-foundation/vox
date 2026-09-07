---
title: "Script tier timings (2026-09)"
description: "Executed check and interp-run timings for every scripts/**/*.vox, plus the four automation entry points."
category: "Architecture SSOTs"
status: "current"
training_eligible: true
training_rationale: "Raw execution timings gathered for the interpreter-first execution program; informs Task 7's default-tier flip decision."
---

# Script tier timings (2026-09)

Task 0 of the [interpreter-first execution plan](../../superpowers/plans/2026-09-05-interpreter-first-execution.md):
measure by **executing**, not merely type-checking. `vox check` only proves a script
parses and typechecks; it says nothing about whether `main()` runs. This page records
both, plus hand-measured native-tier (`--mode script`) timings for the four scripts
that matter most for Task 7 (the default-tier flip): `fmt.vox`, `install-hooks.vox`,
`setup.vox`, `arch-check.vox`.

**Caveat that applies to every row in the generated table below:** every `run --mode
interp` invocation appends `-- --help` so scripts that dispatch on `argv` exit before
their real (often destructive) side effect runs. This short-circuits each script's own
collection loop, so this table **cannot see the `list.push` O(n²) class change** —
fixing that is a Task 7 prerequisite regardless of what this table shows. It also means
a script's `run --mode interp` time here is a *lower bound*, not its real-world cost.

**Second caveat, discovered while running this table, not by reading source first:**
`-- --help` is not a universal safe short-circuit. At least three scripts ignore it
entirely and perform their real, repo-wide write on every invocation:
`scripts/fmt.vox` (runs `cargo fmt` across all 128 crates — harmless here because the
tree was already formatted, so it produced a zero-line diff both times it ran),
`scripts/migrate-arrows.vox`, and `scripts/migrations/2026-phase1-contract-headers.vox`
(both rewrote real files — 213 tracked files touched — the first run these were hit
during Step 2 below). Those last two are **not** idempotent no-ops the way `fmt.vox`
is: they left 213 modified files in the working tree (128 `.md`, 60 `.json`, 19
`.yaml`, 7 `.vox`, mostly a `->` → `to` arrow-syntax rewrite and an `x-vox-version`
header insert) that had to be reverted with `git checkout --` before this could be
committed cleanly. `scripts/mens/**`, `scripts/train_local_qwen.vox`, and
`scripts/start-marquee.vox` were excluded from the `run --mode interp` column
*up front*, by reading their argv-dispatch logic before running anything (unbounded
GPU training / full release builds / background daemons); `migrate-arrows.vox` and the
`migrations/2026-phase1-*` scripts were **not** — they looked like read-only checks
from their names and were only caught by inspecting `git status` after the full run.
Treat "runs the whole table under `--help`" as unsafe in general; this table is a
one-time measurement, not a template for routine use.

## Machine

```
$ system_profiler SPHardwareDataType | grep Chip
Chip: Apple M5 Max
```

Apple M5 Max, 18 cores, 128 GB RAM, macOS (Darwin 25.5.0, arm64). `rustc`/`cargo`
1.96.0. Built at `1533f50dd` (`cargo build -q -p vox-cli --bin vox`).

**Load caveat:** this machine was running other agents' concurrent `cargo` builds for
most of Step 3 (`uptime` load average observed between 10 and 33 across the session,
up to 42 concurrent `cargo`/`rustc` processes). The native-tier numbers below are real,
single-sample, hand-measured wall-clock times on a **shared, contended** machine, not a
clean-room benchmark — see the per-row notes. One `setup.vox` cold attempt was aborted
after 746.9 s when a concurrent build raced the shared `~/.vox/script-cache` target
directory and failed with `couldn't create a temp dir ... No such file or directory`;
it is recorded below as a failed attempt, and a second cold attempt is recorded as the
usable cold number.

## Step 3: native tier (`--mode script`), by hand

Command: `/usr/bin/time -p cargo run -q -p vox-cli -- run --mode script <f> -- --help
2>&1 | grep real`. "Cold" = first native execution of that script in this session
(its own script-cache subdirectory did not exist yet); "warm" = immediate re-run
right after. `rm -rf ~/.vox/script-cache` (the brief's literal cold-cache recipe) was
**not** used — that directory is a machine-wide cache shared with other agents'
concurrently running sessions, and wiping it mid-build risks corrupting their
in-flight compiles. Relying on each script's own not-yet-built cache subdirectory for
"cold" gives the same measurement without that blast radius; this deviation is called
out here rather than silently substituted.

| script | cold (real) | cold exit | warm (real) | warm exit | notes |
|---|---|---|---|---|---|
| `scripts/fmt.vox` | 287.89 s | 0 | 138.55 s | 0 | Runs real `cargo fmt` across 128 crates both times (does not gate on `--help`); zero-diff both runs. Warm is faster but still dominated by the workspace-wide fmt scan, not by compile-cache warmup alone. |
| `scripts/install-hooks.vox` | 336.51 s | 1 | 135.99 s | 1 | Fails natively too — same defect class as under `--mode interp` (see FAIL list below), just a different exit path. First warm attempt was killed after >12 min stalled under machine contention (load avg 33, 42 concurrent `cargo`/`rustc` procs); the 135.99 s number is a clean re-attempt after contention eased. |
| `scripts/setup.vox` | 746.88 s (**failed**, contention) / 635.94 s (usable) | n/a / 0 | 216.08 s | 0 | First cold attempt failed after 12.4 min with a shared-cache race (`couldn't create a temp dir`), not a script defect — recorded above. **Unlike under `--mode interp`, `setup.vox` runs to completion natively** (installs pre-commit hooks fails gracefully with a warning, then runs `cargo check --workspace`); the `Option.unwrap()` `AssertionFailed` is interp-tier-specific, not a native-tier bug. |
| `scripts/arch-check.vox` | 156.50 s | 1 | 293.10 s | 1 | Exit 1 both times is pre-existing (real arch-check findings, not a script bug — see the plan). Warm slower than cold: `arch-check` re-scans the whole workspace on every invocation, so wall time here is dominated by that scan plus concurrent-build contention, not by native compile-cache warmup. |

**Reading these numbers:** native-tier cost for these four scripts is dominated by (a)
the one-time cost of compiling the large shared dependency set the generated
script-crate pulls in (paid once per session, not once per script — `arch-check.vox`,
run first, paid ~150 s of it; every script after benefited from the now-warm shared
`target/` even on its own "cold" first run) and (b) for `fmt.vox`/`setup.vox`/
`arch-check.vox`, real work that scales with the whole workspace regardless of
compile caching. None of the four is close to `run --mode interp`'s sub-second-to-
low-double-digit-millisecond range in the generated table below.

## Step 2: full `scripts/**/*.vox` table (generated)

Generated by [`scripts/bench-script-tiers.vox`](../../../scripts/bench-script-tiers.vox)
via `cargo build -q -p vox-cli --bin vox && cargo run -q -p vox-cli -- run --mode interp
scripts/bench-script-tiers.vox`, at HEAD `1533f50dd`. 84 scripts measured; 9 excluded
from the `run --mode interp` column up front for unbounded side effects (GPU training,
full release builds, background daemons — see the script's own header comment for the
per-script reasoning). `grep -c FAIL` over the raw output is 43 lines (41 rows have a
`run --mode interp` `FAIL(n)`; 2 more have a `vox check` `FAIL` with no matching
`run`-column `FAIL(n)` on the same line: `plugin-candidacy.vox` and `start-marquee.vox`).

| script | `vox check` | status | `run --mode interp` | status |
|---|---|---|---|---|
| scripts/arch-check.vox | 19 ms | ok | 163 ms | FAIL(101) |
| scripts/audit-skill-licenses.vox | 21 ms | ok | 17 ms | FAIL(1) |
| scripts/ci-proximity-drift.vox | 18 ms | ok | 17 ms | ok |
| scripts/ci-runners-up.vox | 19 ms | ok | 69 ms | FAIL(1) |
| scripts/ci/compile_kernels.vox | 19 ms | ok | 19 ms | FAIL(1) |
| scripts/ci/corpus_prep.vox | 20 ms | ok | 161 ms | ok |
| scripts/ci/gui-e2e-check.vox | 19 ms | ok | 2618 ms | FAIL(1) |
| scripts/ci/gui-registry-check.vox | 20 ms | ok | 20 ms | ok |
| scripts/ci/install-runner-schedule.vox | 20 ms | ok | 25 ms | FAIL(1) |
| scripts/ci/script-hygiene.vox | 23 ms | ok | 45 ms | FAIL(1) |
| scripts/ci/test.vox | 18 ms | ok | 18 ms | ok |
| scripts/clean-build-artifacts.vox | 31 ms | ok | 19 ms | ok |
| scripts/crate-build-audit.vox | 34 ms | ok | 3090 ms | ok |
| scripts/db-table-census.vox | 37 ms | ok | 35 ms | FAIL(1) |
| scripts/db-test-census.vox | 22 ms | ok | 6661 ms | FAIL(1) |
| scripts/docs-corpus-census.vox | 34 ms | FAIL | 3395 ms | FAIL(1) |
| scripts/docs-reality-audit-cycle.vox | 13 ms | ok | 97 ms | FAIL(101) |
| scripts/docs/architecture-staleness-report.vox | 16 ms | ok | 1998 ms | ok |
| scripts/docs/strip-last-updated-frontmatter.vox | 32 ms | ok | 394 ms | ok |
| scripts/fix-doc-categories.vox | 40 ms | ok | 364 ms | ok |
| scripts/fmt.vox | 19 ms | ok | 31436 ms | FAIL(1) |
| scripts/frontend-review.vox | 13 ms | ok | 467 ms | FAIL(1) |
| scripts/generate-bench-scaffold.vox | 12 ms | ok | 15 ms | FAIL(1) |
| scripts/generate-grammars.vox | 12 ms | ok | 10 ms | FAIL(1) |
| scripts/graphify-coverage.vox | 11 ms | ok | 45 ms | FAIL(1) |
| scripts/graphify-refresh.vox | 15 ms | ok | 61 ms | FAIL(1) |
| scripts/graphify-study-source.vox | 12 ms | ok | 16 ms | FAIL(1) |
| scripts/gui-build.vox | 11 ms | ok | 212 ms | ok |
| scripts/install-hooks.vox | 12 ms | ok | 10 ms | FAIL(1) |
| scripts/mens-corpus/harvest.vox | 12 ms | ok | 11 ms | ok |
| scripts/mens-corpus/harvest_small.vox | 14 ms | ok | 1080 ms | ok |
| scripts/mens-corpus/helpers/jsonl_writer.vox | 12 ms | ok | 10 ms | FAIL(1) |
| scripts/mens-corpus/helpers/walk_docs.vox | 11 ms | ok | 10 ms | FAIL(1) |
| scripts/mens-corpus/helpers/walk_sources.vox | 13 ms | ok | 10 ms | FAIL(1) |
| scripts/mens/_probe.vox | 12 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/full-pipeline.vox | 14 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/gate_safe.vox | 12 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/run_4080_cycles.vox | 11 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/train_dogfood.vox | 11 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/train_resilient.vox | 13 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/train_watch.vox | 11 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/migrate-arrows.vox | 11 ms | ok | 719 ms | ok |
| scripts/migrate-corpus.vox | 13 ms | ok | 12 ms | FAIL(1) |
| scripts/migrations/2026-phase1-contract-headers.vox | 12 ms | ok | 247 ms | ok |
| scripts/migrations/2026-phase1-delete-empty-schemas-dir.vox | 12 ms | ok | 11 ms | ok |
| scripts/migrations/2026-phase1-delete-repo-root-strays.vox | 10 ms | ok | 11 ms | ok |
| scripts/migrations/2026-phase7-target-cleanup.vox | 11 ms | ok | 10 ms | ok |
| scripts/orchestrator/model_discover.vox | 12 ms | ok | 345 ms | FAIL(1) |
| scripts/orchestrator/scoreboard_rollup.vox | 12 ms | ok | 31 ms | ok |
| scripts/perf/coverage-report.vox | 12 ms | ok | 56 ms | FAIL(2) |
| scripts/perf/test-baseline.vox | 13 ms | ok | 12 ms | FAIL(1) |
| scripts/plugin-candidacy.vox | 13 ms | FAIL | 12 ms | ok |
| scripts/pre-claim.vox | 12 ms | ok | 178 ms | FAIL(101) |
| scripts/profile-crate-count.vox | 25 ms | FAIL | 14 ms | FAIL(1) |
| scripts/quality/audit-dependency-layers.vox | 16 ms | ok | 57 ms | FAIL(1) |
| scripts/quality/audit-telemetry.vox | 15 ms | ok | 167 ms | ok |
| scripts/quality/audit-workspace-health.vox | 14 ms | ok | 13 ms | FAIL(1) |
| scripts/quality/doc-policy-lint.vox | 15 ms | ok | 758 ms | FAIL(1) |
| scripts/quality/generate-matrix-doc.vox | 12 ms | ok | 18 ms | ok |
| scripts/render-durable-animation.vox | 12 ms | ok | 4130 ms | FAIL(1) |
| scripts/scientia/acceptance-matrix.vox | 15 ms | ok | 128 ms | FAIL(101) |
| scripts/scientia/atlas-draft.vox | 16 ms | ok | 15 ms | ok |
| scripts/scientia/atlas-publish.vox | 16 ms | ok | 13 ms | ok |
| scripts/scientia/probe-run.vox | 15 ms | ok | 14 ms | ok |
| scripts/scientia/profile-rollup.vox | 17 ms | ok | 15 ms | ok |
| scripts/serve_mens_v1.vox | 16 ms | ok | 13 ms | ok |
| scripts/setup.vox | 14 ms | ok | 325 ms | FAIL(1) |
| scripts/show/cross-post.vox | 15 ms | ok | 11 ms | FAIL(1) |
| scripts/show/publish.vox | 15 ms | ok | 13 ms | ok |
| scripts/show/script.vox | 16 ms | ok | 12 ms | FAIL(1) |
| scripts/show/title-workshop.vox | 14 ms | ok | 11 ms | FAIL(1) |
| scripts/show/topic-suggest.vox | 14 ms | ok | 13 ms | FAIL(1) |
| scripts/smoke-llm.vox | 14 ms | ok | 2126 ms | FAIL(1) |
| scripts/start-marquee.vox | 27 ms | FAIL | - | SKIPPED (unbounded side effects, see header) |
| scripts/sync-cursor-skills.vox | 37 ms | ok | 30 ms | ok |
| scripts/sync-superpowers-skills.vox | 21 ms | ok | 20 ms | ok |
| scripts/sync_golden_vox.vox | 20 ms | ok | 37 ms | ok |
| scripts/target-gc.vox | 14 ms | FAIL | 15 ms | FAIL(1) |
| scripts/test_for.vox | 18 ms | ok | 15 ms | ok |
| scripts/test_fs.vox | 15 ms | ok | 15 ms | ok |
| scripts/test_process_primitives.vox | 16 ms | ok | 17 ms | ok |
| scripts/test_recursion.vox | 16 ms | ok | 15 ms | ok |
| scripts/train_local_qwen.vox | 17 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/vendor-skills.vox | 35 ms | ok | 15 ms | FAIL(1) |

## FAIL list, with first `stderr` line

The 41 rows above whose `run --mode interp` column is `FAIL(n)`, in table order, with
the first line each printed to `stderr` (captured with `2>&1 | head -1` per script;
blank means the script's first stderr line was empty — see `docs-corpus-census.vox`).
Most are pre-existing behavior (missing external tools/daemons, capability denials,
real lint/audit findings) exposed by *executing* rather than only checking; a smaller
group is a genuine interpreter defect (`UndefinedVariable`, `TypeError`, one
`AssertionFailed`).

| script | exit | first stderr line |
|---|---|---|
| scripts/arch-check.vox | 101 | `Running vox-arch-check (layer + fan-in + LoC + orphan + docstring)...` |
| scripts/audit-skill-licenses.vox | 1 | `audit-skill-licenses: missing LICENSE.upstream or LICENSE.txt:` |
| scripts/ci-runners-up.vox | 1 | `2026-09-06T21:52:43.449208Z ERROR vox_compiler::eval::builtins: ci-runners-up: docker daemon not reachable — start the WSL2 Docker Engine (`wsl -d Ubuntu -u root -- service docker start`), then retry.` |
| scripts/ci/compile_kernels.vox | 1 | `Compiling CUDA kernels to PTX (Windows MSVC shim)...` |
| scripts/ci/gui-e2e-check.vox | 1 | `Running GUI E2E validation suite via Playwright...` |
| scripts/ci/install-runner-schedule.vox | 1 | `install-runner-schedule: registering VoxCIRunnerScale from scripts/ci/voxcirunnerscale.task.xml` |
| scripts/ci/script-hygiene.vox | 1 | `Running script-hygiene check...` |
| scripts/db-table-census.vox | 1 | `db-table-census: scanning .vox/store.db` |
| scripts/db-test-census.vox | 1 | `db-test-census: loading quarantine targets from graphify-out/table_usage_report.json` |
| scripts/docs-corpus-census.vox | 1 | *(blank)* |
| scripts/docs-reality-audit-cycle.vox | 101 | `docs-reality-audit: verify…` |
| scripts/fmt.vox | 1 | `Formatting Rust across 128 crates (Windows-safe, per-crate)...` |
| scripts/frontend-review.vox | 1 | `[frontend-review] capturing matrix (chromium + firefox)...` |
| scripts/generate-bench-scaffold.vox | 1 | `Migrating bench scaffold generator to VoxScript...` |
| scripts/generate-grammars.vox | 1 | `Regenerating grammar fragments from SSOT...` |
| scripts/graphify-coverage.vox | 1 | `Graphify coverage verification script started` |
| scripts/graphify-refresh.vox | 1 | `Checking freshness of Graphify corpora...` |
| scripts/graphify-study-source.vox | 1 | `Cloning Graphify upstream source into .vox/cache/graphify-src ...` |
| scripts/install-hooks.vox | 1 | `Error: Eval failed calling main: AssertionFailed("called \`Option.unwrap()\` on a None value")` |
| scripts/mens-corpus/helpers/jsonl_writer.vox | 1 | `Error: Eval failed calling main: UndefinedVariable("main")` |
| scripts/mens-corpus/helpers/walk_docs.vox | 1 | `Error: Eval failed calling main: UndefinedVariable("main")` |
| scripts/mens-corpus/helpers/walk_sources.vox | 1 | `Error: Eval failed calling main: UndefinedVariable("main")` |
| scripts/migrate-corpus.vox | 1 | `Error: Eval failed calling main: TypeError { expected: "List", found: "Result" }` |
| scripts/orchestrator/model_discover.vox | 1 | `Running Nightly Model Discovery...` |
| scripts/perf/coverage-report.vox | 2 | `=== coverage-report.vox ===` |
| scripts/perf/test-baseline.vox | 1 | `=== test-baseline.vox ===` |
| scripts/pre-claim.vox | 101 | `Running pre-claim: refreshing CR-L gate snapshot...` |
| scripts/profile-crate-count.vox | 1 | `Error: Eval failed calling main: UndefinedVariable("Process")` |
| scripts/quality/audit-dependency-layers.vox | 1 | `Vox Dependency Layer Auditor (2026-05-23, JSON-RFC API)` |
| scripts/quality/audit-workspace-health.vox | 1 | `Vox Dynamic Workspace Health Audit (2026-05-23, JSON-RFC API)` |
| scripts/quality/doc-policy-lint.vox | 1 | `Doc policy violations (1514):` |
| scripts/render-durable-animation.vox | 1 | `Installing puppeteer-core in apps/build-tools/render-durable-animation/ ...` |
| scripts/scientia/acceptance-matrix.vox | 101 | `Running SCIENTIA acceptance slice...` |
| scripts/setup.vox | 1 | `Initializing Vox Workspace Setup` |
| scripts/show/cross-post.vox | 1 | `Capability denied: script missing capability for 'env' namespace` |
| scripts/show/script.vox | 1 | `Capability denied: script missing capability for 'env' namespace` |
| scripts/show/title-workshop.vox | 1 | `Capability denied: script missing capability for 'env' namespace` |
| scripts/show/topic-suggest.vox | 1 | `Capability denied: script missing capability for 'env' namespace` |
| scripts/smoke-llm.vox | 1 | `smoke-llm: running \`vox doctor --probe\` …` |
| scripts/target-gc.vox | 1 | `Error: Failed to read file` |
| scripts/vendor-skills.vox | 1 | `vendor-skills: 2 source entries` |

**Confirmed:** `scripts/install-hooks.vox` and `scripts/setup.vox` are both in this
list, both with `AssertionFailed`-class/exit-1 failures under `--mode interp` as the
brief said to expect — `install-hooks.vox` with the exact
`AssertionFailed("called Option.unwrap() on a None value")` from the brief;
`setup.vox`'s interp-tier stderr does not repeat that message on its first line (the
assertion fires later in its longer body), but its exit code is non-zero and its
native-tier run (Step 3, above) proves the same script runs to completion under
`--mode script`, isolating the defect to the interpreter tier.

## Task 5b: re-measure after fs scoping (2026-09-07)

Re-ran [`scripts/bench-script-tiers.vox`](../../../scripts/bench-script-tiers.vox)
at HEAD `d901cec0a` with `target/debug/vox run --mode interp` on the same Apple M5
Max machine. The question is whether Task 5's parent-walk resolver denied relative
writes that Task 0 executed. It did not: the four Task 7 entry points now show
`ok` (or arch-check's usual findings exit), not `CapabilityDenied`.

**Task 7 gates** (also probed individually before the full table):

| script | `run --mode interp -- --help` | notes |
|---|---|---|
| `scripts/fmt.vox` | **ok** (18130 ms) | Real per-crate `cargo fmt`. Relative writes not denied. |
| `scripts/install-hooks.vox` | **ok** (56 ms) | First probe this session was `FAIL(1)` with `Failed to spawn lefthook` — environment, not a capability denial. After `brew install lefthook`, exit 0. |
| `scripts/setup.vox` | **ok** (69441 ms) | Task 0 was `FAIL(1)` (`AssertionFailed` on `Option.unwrap()`). Now finishes `cargo check --workspace --exclude vox-gui`. |
| `scripts/arch-check.vox` | **FAIL(1)** (6381 ms) | Stderr is `vox-arch-check: FAILED (exit 1)` — the usual findings report, not `CapabilityDenied`. Allowed by the Task 5b brief. |

**`skip_run` expanded** so `-- --help` does not launch unbounded or mutating work.
Task 0 already skipped `scripts/mens/**`, `train_local_qwen.vox`, and
`start-marquee.vox`. This re-measure also skips:

- Playwright: `frontend-review.vox`, `ci/gui-e2e-check.vox` (Task 0's 2618 ms
  `FAIL(1)` on e2e was a fast Playwright miss; with browsers installed it runs
  the full suite)
- `cargo test` / `cargo run` extractors: `scientia/acceptance-matrix.vox`,
  `ci/corpus_prep.vox`, `docs-reality-audit-cycle.vox`, `quality/audit-telemetry.vox`,
  `pre-claim.vox`
- Full GUI / puppeteer: `gui-build.vox`, `render-durable-animation.vox`
- Tree mutators: `migrate-arrows.vox`, `migrations/2026-phase1-contract-headers.vox`

`vendor-skills.vox` still ran (3035 ms, **ok**). It rewrote files under
`assets/skills/`; those writes were reverted and are not in the Task 5b commit.

Generated table (84 scripts; skipped rows still `vox check`):

| script | `vox check` | status | `run --mode interp` | status |
|---|---|---|---|---|
| scripts/arch-check.vox | 29 ms | ok | 6381 ms | FAIL(1) |
| scripts/audit-skill-licenses.vox | 24 ms | ok | 19 ms | FAIL(1) |
| scripts/ci-proximity-drift.vox | 26 ms | ok | 20 ms | ok |
| scripts/ci-runners-up.vox | 31 ms | ok | 63 ms | FAIL(1) |
| scripts/ci/compile_kernels.vox | 23 ms | ok | 18 ms | FAIL(1) |
| scripts/ci/corpus_prep.vox | 21 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/ci/gui-e2e-check.vox | 21 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/ci/gui-registry-check.vox | 22 ms | ok | 17 ms | ok |
| scripts/ci/install-runner-schedule.vox | 23 ms | ok | 17 ms | FAIL(1) |
| scripts/ci/script-hygiene.vox | 22 ms | ok | 20 ms | FAIL(1) |
| scripts/ci/test.vox | 22 ms | ok | 17 ms | ok |
| scripts/clean-build-artifacts.vox | 30 ms | ok | 19 ms | ok |
| scripts/crate-build-audit.vox | 26 ms | ok | 717 ms | ok |
| scripts/db-table-census.vox | 35 ms | ok | 34 ms | FAIL(1) |
| scripts/db-test-census.vox | 25 ms | ok | 3751 ms | FAIL(1) |
| scripts/docs-corpus-census.vox | 33 ms | FAIL | 2137 ms | FAIL(1) |
| scripts/docs-reality-audit-cycle.vox | 27 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/docs/architecture-staleness-report.vox | 32 ms | ok | 5896 ms | ok |
| scripts/docs/strip-last-updated-frontmatter.vox | 30 ms | ok | 135 ms | ok |
| scripts/fix-doc-categories.vox | 38 ms | ok | 137 ms | ok |
| scripts/fmt.vox | 27 ms | ok | 18130 ms | ok |
| scripts/frontend-review.vox | 23 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/generate-bench-scaffold.vox | 22 ms | ok | 20 ms | FAIL(1) |
| scripts/generate-grammars.vox | 22 ms | ok | 19 ms | FAIL(1) |
| scripts/graphify-coverage.vox | 22 ms | ok | 3758 ms | ok |
| scripts/graphify-refresh.vox | 23 ms | ok | 727 ms | FAIL(1) |
| scripts/graphify-study-source.vox | 25 ms | ok | 1542 ms | ok |
| scripts/gui-build.vox | 25 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/install-hooks.vox | 24 ms | ok | 56 ms | ok |
| scripts/mens-corpus/harvest.vox | 24 ms | ok | 18 ms | ok |
| scripts/mens-corpus/harvest_small.vox | 24 ms | ok | 876 ms | ok |
| scripts/mens-corpus/helpers/jsonl_writer.vox | 22 ms | ok | 19 ms | FAIL(1) |
| scripts/mens-corpus/helpers/walk_docs.vox | 31 ms | ok | 28 ms | FAIL(1) |
| scripts/mens-corpus/helpers/walk_sources.vox | 26 ms | ok | 23 ms | FAIL(1) |
| scripts/mens/_probe.vox | 30 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/full-pipeline.vox | 37 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/gate_safe.vox | 31 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/run_4080_cycles.vox | 35 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/train_dogfood.vox | 34 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/train_resilient.vox | 31 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/mens/train_watch.vox | 27 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/migrate-arrows.vox | 26 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/migrate-corpus.vox | 27 ms | ok | 23 ms | FAIL(1) |
| scripts/migrations/2026-phase1-contract-headers.vox | 26 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/migrations/2026-phase1-delete-empty-schemas-dir.vox | 25 ms | ok | 20 ms | ok |
| scripts/migrations/2026-phase1-delete-repo-root-strays.vox | 25 ms | ok | 21 ms | ok |
| scripts/migrations/2026-phase7-target-cleanup.vox | 23 ms | ok | 18 ms | ok |
| scripts/orchestrator/model_discover.vox | 26 ms | ok | 623 ms | ok |
| scripts/orchestrator/scoreboard_rollup.vox | 24 ms | ok | 63 ms | ok |
| scripts/perf/coverage-report.vox | 23 ms | ok | 74 ms | FAIL(2) |
| scripts/perf/test-baseline.vox | 26 ms | ok | 20 ms | FAIL(1) |
| scripts/plugin-candidacy.vox | 23 ms | FAIL | 21 ms | ok |
| scripts/pre-claim.vox | 23 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/profile-crate-count.vox | 23 ms | FAIL | 18 ms | FAIL(1) |
| scripts/quality/audit-dependency-layers.vox | 23 ms | ok | 52 ms | FAIL(1) |
| scripts/quality/audit-telemetry.vox | 34 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/quality/audit-workspace-health.vox | 32 ms | ok | 23 ms | FAIL(1) |
| scripts/quality/doc-policy-lint.vox | 29 ms | ok | 704 ms | FAIL(1) |
| scripts/quality/generate-matrix-doc.vox | 26 ms | ok | 27 ms | ok |
| scripts/render-durable-animation.vox | 25 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/scientia/acceptance-matrix.vox | 23 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/scientia/atlas-draft.vox | 23 ms | ok | 19 ms | ok |
| scripts/scientia/atlas-publish.vox | 23 ms | ok | 18 ms | ok |
| scripts/scientia/probe-run.vox | 23 ms | ok | 18 ms | ok |
| scripts/scientia/profile-rollup.vox | 22 ms | ok | 16 ms | ok |
| scripts/serve_mens_v1.vox | 22 ms | ok | 18 ms | ok |
| scripts/setup.vox | 23 ms | ok | 69441 ms | ok |
| scripts/show/cross-post.vox | 29 ms | ok | 20 ms | FAIL(1) |
| scripts/show/publish.vox | 27 ms | ok | 19 ms | ok |
| scripts/show/script.vox | 25 ms | ok | 19 ms | FAIL(1) |
| scripts/show/title-workshop.vox | 25 ms | ok | 19 ms | FAIL(1) |
| scripts/show/topic-suggest.vox | 28 ms | ok | 19 ms | FAIL(1) |
| scripts/smoke-llm.vox | 25 ms | ok | 4562 ms | FAIL(1) |
| scripts/start-marquee.vox | 28 ms | FAIL | - | SKIPPED (unbounded side effects, see header) |
| scripts/sync-cursor-skills.vox | 28 ms | ok | 21 ms | ok |
| scripts/sync-superpowers-skills.vox | 25 ms | ok | 18 ms | ok |
| scripts/sync_golden_vox.vox | 27 ms | ok | 25 ms | ok |
| scripts/target-gc.vox | 26 ms | ok | 50 ms | FAIL(1) |
| scripts/test_for.vox | 26 ms | ok | 17 ms | ok |
| scripts/test_fs.vox | 26 ms | ok | 18 ms | ok |
| scripts/test_process_primitives.vox | 24 ms | ok | 19 ms | ok |
| scripts/test_recursion.vox | 21 ms | ok | 18 ms | ok |
| scripts/train_local_qwen.vox | 26 ms | ok | - | SKIPPED (unbounded side effects, see header) |
| scripts/vendor-skills.vox | 37 ms | ok | 3035 ms | ok |
