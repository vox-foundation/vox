---
title: "CI/CD audit findings (2026-09)"
description: "Audit of every GitHub workflow and vox ci guard on the hosted-primary CI branch: execution cost, relevance, what to remove, and coverage gaps."
category: "CI & Quality"
status: "research"
---

# CI/CD audit findings (2026-09)

Audited on branch `claude/cicd-complexity-tradeoffs-b62a67` (the hosted-primary CI
migration — spec: `docs/superpowers/specs/2026-09-21-hosted-primary-ci-design.md`), against
the codebase as of 2026-09-22. Inputs: all 43 files in `.github/workflows/`, GitHub run
history (`gh run list` / jobs API), the 160-variant `CiCmd` surface in
`crates/vox-cli-ci/src/cmd_enums.rs`, the `vox ci pre-push` tiers, and `git log` activity on
each workflow's target paths.

**Read the run history with one caveat.** Almost all "never succeeded / always cancelled"
history comes from `main`, where jobs target a self-hosted fleet with zero registered
runners (e.g. `vox-mental-tracker.yml` is `[self-hosted, linux]` on `main`, `ubuntu-latest`
here). That history says the fleet is dead — which this branch fixes — not that the
workflows are useless. The verdicts below separate "starved" from "genuinely broken."

## 1. Execution time

| Where | Cost | Finding |
|---|---|---|
| `vox ci pre-push` (fast tier, runs locally **and** as step 2 of the PR gate) | 126 s measured | `vox-doc-pipeline` scoped lint = **69 s (58%)** to lint 9 files — fixed startup cost, not proportional work. |
| Same | ~3 s per step floor | **Bug:** `run_step_with_heartbeat` (`crates/vox-cli/src/commands/ci/pre_push.rs:320`) sleeps its heartbeat thread in 3 s blocks and `join()`s it after the step finishes, so every step is padded up to the next multiple of 3 s (the uniform `OK (3000ms)` / `6009ms` readings). Up to ~3 s × 12 steps per push, locally and in CI. Fix: wake the heartbeat via a channel `recv_timeout` or condvar instead of `thread::sleep`. |
| `ssot-drift` (inside the fast tier) | 15 s | 26 nested stages; `command_compliance` (4.3 s), `completion_quality` (3.3 s), `affected_cmd::check_graph` (2.6 s) are ~68% of it. |
| PR gate (`ci.yml`) | unmeasured on hosted | Never executed end-to-end (Docker unavailable locally; branch unpushed). The 30-min budget is unvalidated until the first real run, and PR runs compile cold until a nightly on `main` seeds the shared cache. |
| `ci.yml` `gate` + `ssot-autoregen` | 2 × vox-cli compile per PR | Different feature sets (`ssot-autoregen` needs `completion-toestub,extras-ludus,ars,coderabbit`), so not shareable as-is; add a path filter so autoregen only runs when a generator input changed. |
| `nightly.yml` | 19 jobs, ~1,800 lines | The `full` job repeats the clippy/nextest/deny/audit work of the `lints`/`tests`/`guards-fast` jobs — two workspace compiles per night. |

## 2. Remove or disable

| Item | Evidence | Action |
|---|---|---|
| Self-hosted fleet tooling: `runner_scale.rs` + `queue.rs` (~3.1 k LoC), `oom_watch.rs`, `unexpected_exit_watch.rs`, `vox doctor`'s `hook_guard_check` | Zero callers in any workflow, hook, script, or `.claude/settings.json` after this migration | Delete (planned as Task 10 of the migration plan — separate PR). |
| `qwen35-native-nightly.yml` | `disabled_manually`; needs a GPU runner; subsumed by `ml_data_extraction.yml`'s `native_train` input | Delete, keep `ml_data_extraction.yml` (also disabled) as the single GPU lane. |
| `docker-telemetry.yml` + `deploy-telemetry.yml` | Deploy has **never** succeeded (3/3 failed); dormant since 2026-07-29; `server/telemetry` 2 commits/90 d | Disable both until telemetry deploy is revived. |
| `vox-visus-audit.yml` | Weekly `windows-latest` build whose audit step no-ops (`continue-on-error`) without `VOX_VISUS_STAGING_URL`; mostly cancelled | Disable until a staging preview exists. |
| `scorecard.yml` | 10/10 failures, zero signal | Fix permissions (`publish_results: true` vs top-level `read-all`) or delete. |
| `link_checker.yml` | 10/10 failures; external-URL lychee is inherently flaky; `docs-deploy.yml` already runs lychee (advisory) | Delete, or make it weekly and advisory. |
| `pm-provenance-verify.yml` | Dispatch-only, **never run** | Fold one step into `nightly.yml`, or delete if registry publish isn't an active feature. |
| ~45–50 orphan `vox ci` subcommands + 4 placeholder no-ops (`mens-corpus-health`, `grpo-reward-baseline`, `collateral-damage-gate`, `constrained-gen-smoke`, `check-frozen`) | No workflow / hook / script / `ssot-drift` / pre-push caller | Prune in a dedicated PR; each deletion regenerates command-sync + gui-surface-coverage + doc-inventory. |
| Dead YAML inside live workflows | `nightly.yml`: ~8 `pull_request`/`merge_group` branches that can never fire (setup `filter`/`affected` steps, the unreachable "Shadow" step, 4+ job guards); `cross-platform-check.yml`: no-op `path-check` job; `gui-cross-build.yml`: dead PR branch; `compile-matrix.yml`: paths for deleted `crates/vox-release-artifacts`, `crates/vox-assets` | Strip in one cleanup PR. |
| Stale CodeRabbit reminder | `pre_push.rs:315` prints "comment `@coderabbitai review`"; CodeRabbit is retired per AGENTS.md | Remove. |

**Keep** (relevant, targets active code): `ci.yml`, `nightly.yml`, `nightly-report.yml`,
`nightly-artifacts.yml`, `bench-nightly`, `mutation-nightly`, `differential-gate-nightly`
(sole runner of the golden interp-vs-native gate), `harness-eval-nightly`, `cr-l-gates`
+ `cr-l8-corpus-feedback` (v1.0 GA gate, not ML), `gitleaks`, `codeql`, `workflow-lint`,
`docs-quality`, `ts-emit-noemit`, `distribution-parity` (10/10 green), `compile-matrix`,
`cross-platform-check`, `gui-cross-build`, `os-compat-report`, `vox-mental-tracker` +
the three mobile lanes, `deploy-hetzner` (green, live), `docker-eval`, `coolify-eval-sync`,
and the release family.

## 3. Broken but needed — fix, don't delete

| Workflow | State |
|---|---|
| `docs-deploy.yml` | 10/10 failures — docs are **not publishing** to Pages. Highest-value fix. |
| `release-gui.yml` | 0/10 successes (target `vox-gui` is the most active crate: 478 commits/90 d). |
| `release-installers.yml` | 0/5 successes. |
| `setup-e2e.yml` | Red every night 2026-09-18 → 09-22 (onboarding script). |
| `mobile-e2e-ios.yml` | Red every night 2026-09-18 → 09-22. |

`nightly-artifacts.yml` exists to rehearse the release lane nightly; once green, it should
catch these before a real tag does.

## 4. Coverage gaps (ranked)

| # | Risk | Gap | Evidence | Cheapest fix |
|---|---|---|---|---|
| 1 | High | **Toolchain-bump lint wave has no pre-merge check** (AGENTS.md's #1 perennial bug) | `toolchain-lint-wave` exists on `main` (`ci.yml:322`) and was dropped by this branch; the nightly `full` job only catches it after merge | Re-add as a path-filtered PR job on `rust-toolchain.toml` changes; shard fresh clippy/rustdoc to fit the 30-min cap. |
| 2 | High | **No pre-merge Windows/macOS signal** | `cross-platform-check.yml` (weekly) and `gui-cross-build.yml` (nightly) left PR/merge_group; `ci.yml` is `ubuntu-latest` only; AGENTS.md documents Windows-only failure classes (`os error 206`) | `cargo check --workspace --exclude vox-gui` on `windows-latest`, on `merge_group` only (check, not test). |
| 3 | High | **`vox-gui` is never tested** | Stripped from PR affected args; `--exclude vox-gui` in nightly `full`; `gui-cross-build` only builds | Add `cargo test -p vox-gui` to `gui-cross-build.yml`, which already stages `ui/dist` and the sidecar. |
| 4 | High | **GUI Playwright e2e is nightly-only**, contradicting AGENTS.md's "must pass `test:e2e` before merging" | `gui-playwright-smoke` lives in `nightly.yml` | Path-filtered PR job on `crates/vox-gui/ui/**`, or amend the policy — the two must agree. |
| 5 | Med | **Golden `.vox` examples / `scripts/*.vox` / full doc lint only nightly** | PR fast tier lints touched docs only | Path-filtered PR step on `crates/vox-compiler/**`, `examples/golden/**`. |
| 6 | Med | **No dead-man's switch** for scheduled workflows | `nightly-report.yml` reacts only to completed runs; GitHub auto-disables crons after 60 days of inactivity | Daily job: for each listed workflow, open a `nightly-failure` issue if the last run is older than 2× its cadence. |
| 7 | Med | **No automatic runtime-vs-cap measurement** | `ci-timings.yml` deleted; `vox ci job-timings` (`crates/vox-cli-ci/src/job_timings.rs`) has no caller | Run `vox ci job-timings --run-id` from `nightly-report.yml`; warn at 80% of the 30/180 caps. |
| 8 | Med | `all-features-matrix` in nightly is gated on the affected set | `nightly.yml` ~1692 | Force full on schedule. |
| 9 | Low | `cargo-deny`/`cargo-audit` only nightly (gitleaks is per-PR) | — | Path-filtered PR job on `Cargo.lock` changes (~2 min). |
| 10 | Low | `workflow-permissions-guard` is advisory, has no `CiCmd` entry | `pre_push.rs` step, `run(root, false)` | Clear the backlog and flip to strict, or drop the step. |

Out of scope here: `crates/vox-db` carries 14 pre-existing `collapsible_if` clippy errors on
this branch (toolchain lint wave — gap #1 in action); the full nightly will be red on them
until fixed.

## 5. Suggested order

1. Merge the hosted-primary branch, run the PR gate once for real, and record actual per-step
   times (validates the 30-min cap).
2. Fix `docs-deploy.yml`; fix the heartbeat join bug; drop the stale CodeRabbit message.
3. Re-add the toolchain-bump job (gap 1); add merge_group Windows check (gap 2); add
   `cargo test -p vox-gui` (gap 3).
4. Disable telemetry chain, visus audit, scorecard (or fix), link checker; delete qwen35.
5. Task 10: delete fleet tooling; prune orphan `vox ci` subcommands; strip dead YAML.
6. Dead-man's switch + runtime-vs-cap reporting in `nightly-report.yml` (gaps 6–7).
