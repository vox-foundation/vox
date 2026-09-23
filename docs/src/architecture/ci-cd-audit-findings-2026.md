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
| `qwen35-native-nightly.yml` | `disabled_manually`; needs a GPU runner; subsumed by `ml_data_extraction.yml`'s `native_train` input | Delete, keep `ml_data_extraction.yml` (also disabled) as the single GPU lane. **Done (2026-09-22, Task 9).** |
| `docker-telemetry.yml` + `deploy-telemetry.yml` | Deploy has **never** succeeded (3/3 failed); dormant since 2026-07-29; `server/telemetry` 2 commits/90 d | Disable both until telemetry deploy is revived. **Done (2026-09-22, Task 9): parked to `workflow_dispatch` only.** |
| `vox-visus-audit.yml` | Weekly `windows-latest` build whose audit step no-ops (`continue-on-error`) without `VOX_VISUS_STAGING_URL`; mostly cancelled | Disable until a staging preview exists. **Done (2026-09-22, Task 9): parked to `workflow_dispatch` only.** |
| `scorecard.yml` | 10/10 failures, zero signal | Fix permissions (`publish_results: true` vs top-level `read-all`) or delete. |
| `link_checker.yml` | 10/10 failures; external-URL lychee is inherently flaky; `docs-deploy.yml` already runs lychee (advisory) | Delete, or make it weekly and advisory. |
| `pm-provenance-verify.yml` | Dispatch-only, **never run** | Fold one step into `nightly.yml`, or delete if registry publish isn't an active feature. **Done (2026-09-22, Task 9): folded into `nightly.yml`'s `audits` job; file deleted.** |
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

## Resolution (2026-09-22)

Remediated by plan `docs/superpowers/plans/2026-09-22-ci-audit-remediation.md`, executed as
Tasks 1–14 (Task 11 was cut during plan review — a proposed static guard for stale `paths:`
filters was judged unnecessary because the runtime already fails loudly on a missing path;
see the plan's ledger, Important #7). Task 15 (this doc) is the closing task. Commit SHAs
below are on branch `claude/cicd-complexity-tradeoffs-b62a67`; the authoritative task-by-task
record (rulings, fix rounds, deferred findings) is
`.superpowers/sdd/2026-09-22-ci-audit-remediation/progress.md`.

### Findings → task/commit map

**§1 Execution time**

| Finding | Resolution |
|---|---|
| `vox-doc-pipeline` scoped lint = 58% of the fast tier (fixed startup cost) | Task 3, `b39397d3b` — scoped doc lint now runs in-process under the repo root instead of re-shelling out. |
| Heartbeat `thread::sleep` padding every step up to 3 s | Task 3, `b39397d3b` — heartbeat now wakes immediately instead of sleeping in fixed 3 s blocks. |
| `ssot-drift`'s 26 nested stages (~15 s) | **Not addressed.** No task in this plan touched `ssot-drift`'s internal stage cost. |
| PR gate (`ci.yml`) never run end-to-end; 30-min budget unvalidated | **Pending — user/post-merge.** Can only be measured after this branch merges and a real PR run completes; see the verification table below. |
| `ci.yml` `gate` + `ssot-autoregen` — two vox-cli compiles per PR, no path filter | **Not addressed.** `gate` was restructured (Task 5) into a thin aggregator whose one step checks that `linux` and `ui` both succeeded, but `ssot-autoregen` still runs unconditionally rather than being path-filtered to generator inputs. |
| `nightly.yml` `full` job repeats `lints`/`tests`/`guards-fast` work | Task 10, `44dd0340d` — nightly.yml's duplicate lint/test logic and ~325 lines of redundant job bodies removed. |

**§2 Remove or disable**

| Item | Resolution |
|---|---|
| Self-hosted fleet tooling (`runner_scale.rs`, `queue.rs`, `oom_watch.rs`, `unexpected_exit_watch.rs`, `hook_guard_check`) | Task 14, `e8106f370` — deleted, along with 9 `vox ci` subcommands: 4 fleet commands (`queue`, `runner-preflight`, `runner-scale`, `runner-status`) and 5 placeholder no-ops (`check-frozen`, `mens-corpus-health`, `grpo-reward-baseline`, `collateral-damage-gate`, `constrained-gen-smoke`). |
| `qwen35-native-nightly.yml` | Task 9, `7b0f433c7` — deleted; `ml_data_extraction.yml` remains the single GPU lane. |
| `docker-telemetry.yml` + `deploy-telemetry.yml` | Task 9, `7b0f433c7` — parked to `workflow_dispatch` only. |
| `vox-visus-audit.yml` | Task 9, `7b0f433c7` — parked to `workflow_dispatch` only. |
| `scorecard.yml` (10/10 failures) | Task 8, `3dc771a73` — fixed (signing/permissions), not deleted; kept as a live check. |
| `link_checker.yml` (10/10 failures) | **Not addressed** beyond an incidental `permissions: contents: read` block added by Task 13's blanket least-privilege sweep (`9b0718d18`). The audit's recommendation — delete, or make weekly and advisory — was not acted on. |
| `pm-provenance-verify.yml` | Task 9, `7b0f433c7` — folded one step into `nightly.yml`'s `audits` job; file deleted. |
| ~45–50 orphan `vox ci` subcommands + 4 placeholder no-ops | **Partially addressed.** Task 14, `e8106f370` deleted 9 of them — the 4 fleet commands (`queue`, `runner-preflight`, `runner-scale`, `runner-status`) plus the 5 placeholder no-ops (`check-frozen`, `mens-corpus-health`, `grpo-reward-baseline`, `collateral-damage-gate`, `constrained-gen-smoke`). The remaining ~40 the audit counted as "orphaned" are hand-run operator/diagnostic tools with no workflow caller **by design**; they were deliberately left in place. |
| Dead YAML inside live workflows (`nightly.yml` unreachable branches/guards, `cross-platform-check.yml` no-op `path-check`, `gui-cross-build.yml` dead PR branch) | Task 10, `44dd0340d` — stripped unreachable PR/merge-queue branches from `nightly.yml`, `cross-platform-check.yml`, `gui-cross-build.yml`. `compile-matrix.yml`'s stale `crates/vox-release-artifacts`/`crates/vox-assets` paths were dropped in Task 8, `3dc771a73`. |
| Stale CodeRabbit reminder in `pre_push.rs` | Task 3, `b39397d3b` — removed. |

**§3 Broken but needed**

| Workflow | Resolution |
|---|---|
| `docs-deploy.yml` (10/10 failures, docs not publishing) | **Not addressed.** No task in this plan touched `docs-deploy.yml`. Still the highest-value fix outstanding. |
| `release-gui.yml` (0/10 successes) | **Not addressed.** |
| `release-installers.yml` (0/5 successes) | **Not addressed.** |
| `setup-e2e.yml` (red nightly) | Task 1, `1d7e86db5` — fixed the root cause (MENS hub local-dir fields, duplicate `q_norm`/`k_norm` bindings) that was failing the onboarding script. |
| `mobile-e2e-ios.yml` (red nightly) | Task 8, `3dc771a73` — fixed the iOS pnpm cache path. |

**§4 Coverage gaps**

| # | Gap | Resolution |
|---|---|---|
| 1 | Toolchain-bump lint wave has no pre-merge check | Task 5, `75da26e78` — `ci.yml`'s `linux` leg now runs `cargo clippy`/`rustdoc -D warnings` on a detected toolchain bump. |
| 2 | No pre-merge Windows/macOS signal | Task 5, `75da26e78` — added an **advisory** Windows compile check (`windows-latest`, `merge_group` only). **Advisory only** — flipping it to a required/blocking check is a pending user-only decision (see below); macOS remains unaddressed. |
| 3 | `vox-gui` never tested | Task 6, `ec83dddc2` — `cargo test -p vox-gui` added to the nightly GUI cross-build. Nightly-only, not PR-gated. |
| 4 | GUI Playwright e2e nightly-only | Task 5, `75da26e78` — added a PR-required `ui` leg (typecheck + vitest + Playwright) gated on `crates/vox-gui/**` changes. |
| 5 | Golden `.vox` examples / doc lint only nightly | Task 4, `745430802` — examples-only diffs now select the crates whose tests read `examples/`, extending PR-time coverage to those paths. |
| 6 | No dead-man's switch for scheduled workflows | Task 12, `33e23aa31` — new `ci-liveness.yml`; opens `nightly-failure` issues when a scheduled workflow goes stale. |
| 7 | No automatic runtime-vs-cap measurement | **Not addressed.** `vox ci job-timings` still has no caller in any workflow; `nightly-report.yml` was extended (Task 12) for the dead-man's-switch only, not for timing/budget reporting. |
| 8 | `all-features-matrix` gated on affected set | Effectively resolved as a side effect: Task 5 (`75da26e78`) removed `all-features-matrix` from the PR-triggered path entirely, and Task 10 (`44dd0340d`) stripped the old label-gating comments from `nightly.yml`; the job now runs unconditionally on `nightly.yml`'s daily cron. No task targeted this gap directly. |
| 9 | `cargo-deny`/`cargo-audit` only nightly | Task 5, `75da26e78` — `linux` leg now runs `cargo-deny` licenses/bans/sources checks on dependency changes. |
| 10 | `workflow-permissions-guard` advisory, no `CiCmd` entry | **Partially addressed.** Task 13, `9b0718d18` — explicit least-privilege `permissions:` blocks added to every workflow and the guard flipped from advisory to strict. The `CiCmd` half is unchanged: there is still no standalone `vox ci workflow-permissions-guard` / `workflow-policy-guard` subcommand; the guard runs only as a stage inside `ssot-drift`. |

### Pending — user-only actions (not this plan's to do)

These items came up during the plan's review but require a decision or credential the user
holds, not an agent action:

- **`CF_API_TOKEN` rotation.** Flagged during review as needing rotation; pending the user.
- **Stale-cache delete.** A one-time manual delete of a stale GitHub Actions cache entry; pending the user.
- **Possible admin-bypass merge.** Merging this branch may require an admin bypass of branch
  protection for the first run (since the new required contexts haven't run on `main` yet);
  pending the user's call.
- **Windows-enforcement flip.** The advisory Windows compile check added in Task 5 (gap #2
  above) stays advisory until the user decides to flip it to a required, blocking context —
  that decision, and the branch-protection change it implies, is the user's to make.

### Post-merge verification (fill in after the branch merges and runs on GitHub)

| Item | Value |
|---|---|
| `linux` leg run time | TBD |
| `windows` leg run time / run ID | TBD |
| First UI-changing PR — `ui` leg run time | TBD |
| Rust cache size(s) after first `main` save | TBD |
| `setup-e2e.yml` — green confirmation | TBD |
| `scorecard.yml` — green confirmation | TBD |
| `docs-deploy.yml` — green confirmation | TBD |

### Documented uncertainties (from the Task 15 brief)

- `harness-eval-nightly.yml`'s "non-fast-forward" warning text may be masking a ruleset
  rejection rather than a genuine non-fast-forward push failure — **unverified**. Task 13 kept
  this workflow's job-level `contents: write` (an intentional deviation from the brief's
  original instruction — see the ledger's Task 13 ruling) precisely because the failure mode
  underneath that warning text was not fully characterized.
- External callers of the ~45–50 `vox ci` subcommands and fleet-tooling modules Task 14
  deleted are **unverified** — the search covered workflows, hooks, scripts, and
  `.claude/settings.json` in this repo, but not third-party or out-of-repo consumers.

### Expected noise until the Windows-enforcement flip

The advisory Windows leg added in Task 5 (gap #2) is expected to show failures and timeouts
as noise in `vox ci status` and PR checks until the user performs the Windows-enforcement flip
described above. This is expected behavior, not a regression — the leg is intentionally
non-blocking so it can accumulate signal before becoming a required check.

### Task-by-task outcome summary

Pulled verbatim in spirit from the plan ledger (`.superpowers/sdd/2026-09-22-ci-audit-remediation/progress.md`); see that file for full detail.

| Task | Commit(s) | Outcome |
|---|---|---|
| 1 | `1d7e86db5` | Complete, review clean. |
| 2 | `1d7e86db5..41afa7929` (amended from `5f6cc746e`) | 1 fix round (3 addressed: doc-accuracy mischaracterization of q_norm/k_norm shadowing direction [Important, code was already correct]; suppression pub-fn count off-by-one [Minor]; merge.rs report omission [Minor]). Complete, review clean after the fix round. |
| 3 | `41afa7929..b39397d3b` | Complete, review clean. |
| 4 | `b39397d3b..745430802` | Complete, review clean. |
| 5 | `745430802..75da26e78` | Complete, review clean. 2 minor findings deferred: toolchain-bump/budget-clock steps fail-open on a theoretically-impossible missing-file state (brief-verbatim, not an implementation defect); AGENTS.md's `act pull_request -j gate` references were already known stale, deferred to this task (Task 15). |
| 6 | `75da26e78..ec83dddc2` | Complete, review clean. |
| 7 | `ec83dddc2..203009a8c` | Complete, review clean, independent mutation check passed. 3 minor findings deferred: Swatinem step-level `if:` main-gate exempts save-if requirement (semantically sound); `cache_key_lint`'s PR-reachability rule has no `workflow_call` handling (theoretical, unused today); duplicated `with:` bodies in cache save/restore pairs can drift with no gate (perf-only risk). |
| 8 | `203009a8c..3dc771a73` | Complete, review clean. 2 minor findings deferred: a plan doc's parenthetical wrongly calls `graphify-out/` git-ignored (it's tracked); 87 `file:///c:/Users/Owner/vox/...` Windows-path links remain across 8 docs (lychee reports them "Unsupported", not "Errors" — out of this task's fixed-85 scope). |
| 9 | `3dc771a73..7b0f433c7` (amended from `26ceeedb1`) | 1 fix round (1 addressed: stale `docs/agents/doc-inventory.json` that would fail the real `vox ci doc-inventory verify` gate [Important]). Complete, review clean after the fix round. 2 minor findings deferred: `deploy-telemetry.yml`'s unreachable `workflow_run.conclusion == 'success'` disjunct left in place (harmless); `workflow-enumeration.md`'s deleted pm-provenance row has no replacement (judged sufficient since `binary-release-contract.md` already documents it). |
| 10 | `7b0f433c7..44dd0340d` | 1 fix round (1 addressed: report's hook-bypass rationale was factually false — `bom-check` isn't a pre-commit gate; nothing was actually skipped, all applicable gates independently re-verified passing; report-only correction, no code change). Complete, review clean after the fix round. 3 minor findings deferred: `cargo deny`/`audit` moved from parallel `guards-fast` into sequential `full` (a clippy regression could now mask a new RustSec advisory in the same run); deleted clippy allow-list's per-lint rationale has no documented home (the surviving `full` clippy is stricter, no coverage loss); `selective_ci_toestub_minimal_default_when_empty` deleted per explicit brief instruction while its subject still exists (a known brief self-conflict, followed correctly). |
| 11 | — | **Cut during plan review** — a proposed static guard for stale `paths:` filters was judged unnecessary (runtime already fails loudly; the guard had false positives). |
| 12 | `44dd0340d..33e23aa31` | Complete, review clean, exhaustive 20-workflow-name dry-run trace found zero anomalies. 3 minor findings deferred: `ci-liveness.yml` doesn't pre-create the `nightly-failure` label the way `nightly-report.yml` does (latent — label already exists on this repo); `ci-liveness.yml` interpolates `$title` into a jq query text directly rather than via `env.TITLE` (all 20 current workflow names are jq-safe today); the `disabled_manually` branch `continue`s before the close logic, so a stale-then-parked workflow's open issue is never auto-closed. |
| 13 | `33e23aa31..9b0718d18` (amended from `29c416c31`) | 1 fix round (1 addressed: `harness-eval-nightly.yml`'s job-level `contents: write` was restored after being removed, plus a corrected comment [Important, controller ruling — the brief's stated reason for removing it, "github.token is read-only by design," was factually wrong; this workflow's untouched twin `ci.yml`'s `ssot-autoregen` job keeps the same write scope for a documented graceful-degradation fallback]). Complete, review clean after the fix round. 1 minor finding deferred: the original review evidence for `ml_data_extraction.yml` (5/5 cancelled runs) was too shallow; corrected during the fix round to the stronger finding (11 historical successes, but the git-push step was skipped in all 11 on its `changed==true` gate — conclusion unchanged, leave read-only). |
| 14 | `9b0718d18..e8106f370` | Complete, review clean. 5 minor findings deferred, plus one non-minor item handled out-of-band: two plan docs reference the now-deleted `Dockerfile.ci-runner`/`ci-runner-local.sh` as forward-looking action items rather than historical record; `graphify-out/gui-coverage/cli-governance.json` (a generated, commit-keyed file, excluded from this task's scope) still lists a retired command; doc rewording left references to a fleet-container mechanism with no implementation (flagged as Task-15 doc-retirement scope — not acted on by this task); report prose said "six rows" removed from `doc-inventory.json` when the actual net was 9 removed/1 added (diff itself correct, only the prose count was off); C1's "exposed by this commit" was slightly overstated (one commit earlier also touched the file, so the false positive was latent one commit before). Separately, this task's implementer diagnosed a `secret-env-guard`/`crates-vox-cli-ci` allowlist false positive and correctly deferred fixing it (a security-allowlist change, out of task scope); the controller independently fixed it in a separate worktree (`.claude/worktrees/secret-guard-allowlist-fix`, branch `fix/secret-guard-vox-cli-ci-allowlist`), verified clean, and left it pending the user's explicit confirmation before committing, since it widens a security-relevant allowlist. |
| 15 | (this task) | Records this Resolution section and fixes two stale AGENTS.md CI-contract lines. |

**None of the above deferred items were re-opened or found to be more than minor during this
closing pass** — they are listed here for completeness, per the plan's own review discipline,
not because Task 15 re-verified each one independently.
