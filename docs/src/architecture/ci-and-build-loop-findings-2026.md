---
title: "CI and build loop findings (2026-10-03)"
description: "Measured causes of the slow local edit-commit-push loop and the slow, red hosted CI, with the evidence for each and the decisions that follow from them."
category: "Architecture SSOTs"
status: "roadmap"
sort_order: 51
---

# CI and build loop findings (2026-10-03)

Everything here was read from files, logs, `gh` and the build broker's log. No builds were run to collect it.
Each fact carries its source so it can be re-checked. The plan that acts on it is
[`2026-10-03-hosted-ci-fast-local-loop.md`](../../superpowers/plans/2026-10-03-hosted-ci-fast-local-loop.md).
It extends [`rust-build-loop-ssot-2026.md`](rust-build-loop-ssot-2026.md), which measured the orchestrator loop and the
kernel-time anomaly.

## Decisions taken with the owner

1. **Hosted CI is the only gate.** GitHub-hosted runners run clippy, the full suites, nightly and release builds.
   The laptop runs `cargo check` and the tests of the crate being edited, nothing more. Local or self-hosted runners
   are a backup, never a dependency.
2. **Work reaches CI as a short-lived draft PR per batch.** Agents push a branch, open a draft PR, PR CI gates it,
   and it merges through the queue, so `main` stays CI-green. Committing straight to `main` without CI stops.

## Local loop: why a commit or push takes minutes to hours

| Finding | Evidence |
|---|---|
| Git hooks build binaries. `pre-push` runs `cargo run -p vox-cli -- ci pre-push`, which rebuilds the debug `vox-cli` (73 workspace deps) before any check runs. | `lefthook.yml:79-80` |
| Pre-commit `tdd-guard` runs `cargo run -p vox-code-audit --bin toestub` on any staged `.rs` file; four other pre-commit commands run `cargo run -p vox-cli` when their files are staged. | `lefthook.yml:30-48,58-59` |
| `workspace-hack` (hakari) makes every small tool (toestub, drift-check) compile gix, sqlx, axum, hyper, rustls, wasmtime-environ and two reqwest versions. | `crates/workspace-hack/Cargo.toml:17-195` |
| `vox ci pre-push --complete` runs `cargo clippy --workspace --all-targets`. Hosted CI runs clippy on affected crates only, so the local tier is broader than the gate it imitates. Observed: 2.5 h, still compiling. | `pre_push.rs:563-567,1102-1114`; `ci.yml:110` |
| Hosted CI already runs the fast tier (`./target/debug/vox ci pre-push`), so the local pre-push duplicates it. | `ci.yml:53-54` |
| Scoped TOESTUB builds into `$TMPDIR/vox-targets/<hash>/nested-ci`, a separate target that starts cold. | `matrix.rs:588-603`; `vox-cli-core/src/artifact_policy.rs:36-52` |
| `[build] jobs = 24` with a broker cap of 6 allows about 144 concurrent rustc. Load average 15 to 40 observed. | `.cargo/config.toml:23`; `~/.vox/build-broker/broker.log` |
| The broker holds a slot for the whole runtime of `cargo run`, not just its build, and has 145 stale `inflight/` entries. | `crates/vox-cargo-shim/src/main.rs:162-181` |
| Worktrees that symlink `target` to the main checkout share one cargo build-dir lock. First-party artifacts are keyed by path, so the sharing buys only registry deps. | `readlink`; `.cargo/config.toml:71-86` |
| 80 to 92% of build CPU is kernel time; `syspolicyd` was at 25.7% CPU on 2026-10-03. The suspect is macOS assessing every new binary. The owner approved a Developer Tools exemption; it is a security setting only the owner can apply. | `rust-build-loop-ssot-2026.md:69-77`; `ps` |
| Measured: a 48.5-minute debug build of `vox-cli` and a 21.6-minute `cargo run` in a sibling worktree. | broker log |
| macOS has no fast linker configured (lld is set for Linux and Windows only). | `.cargo/config.toml:39-67` |

## Hosted CI: why it is slow and red

| Finding | Evidence |
|---|---|
| CI already runs on GitHub-hosted runners. The only self-hosted job is `ml_data_extraction.yml` (`extract` could move to hosted; `train` needs a GPU). The one registered self-hosted runner is offline and no workflow targets its labels. | `gh api …/actions/runners`; grep of `runs-on:` |
| `ci.yml` triggers on `pull_request` and `merge_group` only. Pushes to `main` get no CI; local `main` was 57 commits ahead of `origin/main`. | `ci.yml:7-9`; `git rev-list` |
| Nightly (`0 6 * * *`, started 5 to 7 h late) failed every day since at least 2026-09-28. Root cause: `Cargo.lock` on `origin/main` is out of sync, so `--locked` fails in `setup`, which skips ten downstream jobs, and in `full`. Local `main` already has the one-line lock fix. | `gh run view --log-failed` (37119390406, 37005197255, 36864157289) |
| Nightly also has real failures: all-features compile errors (vox-actor-runtime E0599 `DbConfig::resolve_canonical`, vox-codegen E0432/E0277, vox-cli E0275, objc2 on Linux in vox-ml-cli/vox-populi), 7 to 11 `cargo audit` advisories (wasmtime, h2, a TLS 1.3 issue), a CDP smoke where Chromium never answers, Playwright cancelled at its 25-minute cap, and a 93 to 124-minute Windows GUI smoke. | nightly logs |
| PR test shards compile cold: 17 to 19 minutes of compiling, then 6 to 7 minutes of tests (4,278 tests). Ten of 24 recent shards hit the 30-minute cap. `ci.yml` never saves caches (`save-if: false`), and the only writer of the `workspace` cache is nightly's `full` job, which never succeeds. | `ci.yml:45,168`; `nightly.yml:1546`; run 36306369144 |
| All three shards compile the same test binaries; `--partition hash:K/3` only splits execution. | `ci.yml:180` |
| A small affected set compiles for 4 to 6 minutes per shard and runs tests for 2 to 5 seconds. | run 36775994599 |
| Actions cache is at 14.48 GB against a 10 GB budget: sccache 7.0 GB (7,232 entries), Windows cargo 4.5 GB, differential gate 1.6 GB. | `actions/cache/usage` |
| 13 open `nightly-failure` issues (#574 to #578, #580, #581, #583 to #586, #592, #593). | `gh issue list` |
| PR outcomes since 2026-09-20: 4 success, 10 failure, 39 cancelled. | `gh run list` |

## Test suite: what is expensive

| Finding | Evidence |
|---|---|
| 14,144 tests. 735 integration-test files, each its own binary to link; 372 hold two tests or fewer (vox-cli 58, vox-integration-tests 43, vox-compiler 33, vox-codegen 33). No crate merges them into `tests/it/main.rs`. | `rg` counts |
| The slowest tests run a nested `cargo build`: `emit_compile_harness` golden tests (185 to 401 s each), `build_e2e build_component_state` (291 s), trybuild `compile_fail` (over 180 s), ten `emission_ladder *_compiles_as_rust_script` (43 to 72 s), `workflow_runtime_no_db` (88 s). Their `static Mutex` does nothing under nextest's process-per-test model. | PR log; `emission_ladder_test.rs:20,87-93` |
| The `slow` nextest test-group is defined but nothing is assigned to it. | `.config/nextest.toml:26-27` |
| The #569 quarantine holds 32 tests (19 codegen emit/compile); nightly runs them anyway, so nightly is always red. | `.config/nextest.toml:37-70` |
| Nightly runs the whole workspace twice: once under llvm-cov and once plain in `full`. | `nightly.yml:809,1561` |
| Any `contracts/**` change forces a full-workspace PR run, including generated reports; 231 of 2,075 commits since August touched `contracts/`. A change to vox-crypto, vox-telemetry, vox-secrets or vox-config reaches 70+ of 128 crates. | `affected.rs:5-12,33-44,143-151`; crate-graph closure |
| 71 `#[ignore]` with a sunset of 2026-08-01, already passed (69 in vox-compiler); 12 ignores assert retired `activity`/`@v0` behaviour. | `rg`; `codegen_rust_test.rs:20-51` |
| The test-tier budgets file says the full run takes about 27 s; `ci.yml` records about 21 minutes. | `contracts/budgets/test-tier-budgets.v1.yaml:41-46`; `ci.yml:118-120` |
| Playwright on PRs runs with 1 worker; nightly uses 4. | `playwright.config.ts:12`; `nightly.yml:1438` |

## What this means

- The local loop is slow mostly because the hooks build things the hosted gate already builds, on an oversubscribed
  machine whose kernel taxes every new binary. Removing the builds from the hooks is the largest and cheapest win.
- Hosted CI is slow because nothing ever writes a warm cache and each shard compiles everything. It is red because of
  one unpushed lockfile line plus a backlog of real failures that the quarantine and the all-features matrix expose.
- The laptop is not a CI dependency today; the habit of committing to `main` without CI is what made it feel like one.

## Measured gains (2026-10-03, after phase 06.1 tasks 1.1–1.9, 2.1–2.3, 3.1, 3.4)

### Local loop (this laptop, measured)

| Measure | Before | After | Change |
|---|---|---|---|
| `git push` (pre-push hook), same 66-commit landing push | 4,001 s (67 min; 36 min of it building `vox-drift-check`) | 14 s hook, 17 s total | about 280x faster |
| `git commit` with hooks | `cargo run` of `vox-cli` or `toestub` whenever a matching file was staged (minutes cold) | under 1 s (three commits timed) | no build at all |
| `cargo run` time in the broker log, 2026-09-22 to 10-03 | 34.9 h over 1,298 runs; 49 runs averaged about 43 min each (hooks and pre-push) | hooks issue no cargo command | that whole class is gone |
| All broker-logged cargo time over the same 11.5 days | 74.6 h (run 34.9, test 22.2, clippy 9.6, check 4.8, build 3.1) | — | about 47% of it was the `cargo run` class above |
| Installing the hook tools during first-time setup | — | 20–24 s for `vox`, `toestub` and `vox-drift-check` together (debug install reusing the dev build) | one time |
| Concurrent rustc ceiling | 6 broker slots x `jobs = 24` = 144 on 18 cores | load-gated: another build starts only while the load average is below the core count; `jobs` follows the machine | no fixed oversubscription |

Not yet measured: the kernel-time share after the owner applies the macOS Developer Tools exemption (task 1.8).

### Hosted CI (measured on PR #596, run 37175489714, cold dependency cache)

| Measure | Before (measured) | After (measured) | Notes |
|---|---|---|---|
| Full-workspace PR, test execution per shard | 6–7 min (hash shards; nested-cargo tests included) | 1.7–2.3 min (crate shards; nested-cargo tests moved to nightly) | 4,558 / 3,677 / ~4,100 tests per shard |
| Full-workspace PR, compile per shard | 17–19 min (every shard compiled every test binary) | 17–22 min | Unchanged: every test crate pulls in most of the workspace and all third-party deps, and this run's dependency cache was cold (Cargo.lock changed) |
| Full-workspace PR, worst shard wall time | 22–30 min; 10 of 24 recent shards hit the 30-min cap | 19.3–24.4 min; none at the cap | |
| Full-workspace PR, runner minutes | about 89 | about 82 (linux 15.8 + shards 22.7 + 19.3 + 24.4) | |
| Build-once (one archive job) | — | cancelled at the 30-min cap | Rejected: one job cannot hold the whole first-party compile; replaced by crate shards |
| Small affected PR | 3 shards each compiling the set | 1–3 crate groups, each compiling only its own crates | |
| PR that changes only generated `contracts/reports/` | full-workspace run | no clippy or tests | task 2.3 |
| Nightly test executions | the suite ran twice | once | task 3.4 |

### Warm cache (measured 2026-10-04)

Per-shard compile was dominated by building the same dependency graph cold, so `cache-seed.yml` writes a
main-scope `workspace` cache. Its first run (37182816937, dispatched on `main`) took **25.5 min** from cold
(build step 23 min 54 s, cache save 32 s) and wrote the cache. The next PR with a `Cargo.lock` change (#599, run
37195946939) restored it through the restore-key (`full match: false`, because the lockfile changed) and ran:

| Job | Cold (#596, run 37175489714) | Warm (#599, run 37195946939) |
|---|---|---|
| Test shards, job wall time | 22.7 / 19.3 / 24.4 min | 9.2 / 10.7 / 12.4 min |
| Test step of a shard | 17 min 57 s (shard 2) | 7 min 47 s / 9 min 10 s / 10 min 44 s |
| `linux` job | 15.8 min | 14.7 min (`Build vox CLI and fast-tier tools` 8 min 17 s → 5 min 51 s; clippy 5 min 12 s → 6 min 30 s) |

So warming the cache roughly **halves** the test shards (worst 24.4 → 12.4 min), not the "few minutes" the
projection assumed: the first-party crates still recompile after a lockfile change, and a restore-key hit is a
partial one. The `linux` job barely moved because it builds a different set (the CLI and clippy). Caveats: the
two runs are different changes (both touch `Cargo.lock`, so both plan the full workspace), and this is one
sample each; the 6-hourly `cache-seed` run keeps the cache fresh, so an exact-key hit on a no-lockfile PR
should be faster still. Re-measure on the next two ordinary PRs before treating these numbers as stable.


