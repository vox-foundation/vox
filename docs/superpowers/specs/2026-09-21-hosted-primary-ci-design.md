# Hosted-primary CI — design

Date: 2026-09-21. Supersedes the local-first/self-hosted-fleet CI contract
(`docs/superpowers/specs/2026-07-02-local-first-ci-queue-design.md`).

## Problem (measured 2026-09-21)

- `gh api repos/vox-foundation/vox/actions/runners` → `total_count: 0`. `ci.yml`
  and parts of ~30 other workflows run on `[self-hosted, linux]`. Of the last 200
  `ci.yml` runs: 146 cancelled (24h queue timeout), 54 failed, **0 succeeded**.
- The "hours-long GitHub runs" were queue wait, not execution. Per-job timings:
  vox-audit gates waited 1063 min / ran 16 min; GUI cross-build waited 399 min
  (self-hosted matrix job) / ran 76 min on `ubuntu-latest`; Mobile EAS waited
  375 min / ran 14 min.
- The hosted fallback (`ci-fallback-hosted.yml`) is red because `main` has real
  clippy errors nothing gated: `crates/vox-code-audit/src/review/providers.rs:222,225`
  (`unsafe` blocks under `-D unsafe-code`) and
  `crates/vox-plugin-browser/tests/ax_snapshot_probe_test.rs:5` (`useless_vec`).
- Health monitors (watchdog, dead-man) report green throughout.
- The repo is public → GitHub-hosted standard runners are free.

## Destination

1. **GitHub-hosted CI is primary.** The required context
   `Check, Build, and Test (Rust)` is published by a hosted job in `ci.yml`.
2. **Enforced timeouts.** Jobs in workflows triggered by `pull_request`,
   `merge_group`, or a branch `push`: `timeout-minutes` ≤ **30**. All other
   workflows (schedule, dispatch, tag release, workflow_run): ≤ **180**. Every job
   declares a literal integer. Enforced in `ssot-drift` (CI) and pre-push.
3. **Nightly on GitHub** for everything slower: full-workspace clippy/tests/
   doctests, deny/audit, compiler gates, audits, GUI/Docker/browser smokes,
   all-features. Nightly jobs also seed the shared Rust cache that PRs restore.
4. **Run any workflow job locally** with `nektos/act` (already wired as
   `vox ci pre-push --act`). Kept. No custom dispatcher.
5. **Push, don't pull, CI state to agents.** Agents won't remember a command, so
   status arrives through hooks every agent passes through:
   - git `pre-commit` and `pre-push` (lefthook): universal across Claude, Cursor,
     Codex, Copilot, humans;
   - Claude Code `SessionStart` (cached block) and `UserPromptSubmit`
     (only when the state changed since this session last saw it).
   The block names failed and **timed-out** jobs, the step that failed or was
   running at the timeout, and the three slowest steps — enough to self-correct
   (cache, shard, or move to nightly — never raise the cap).
6. **Nightly failures are GitHub issues** labelled `nightly-failure`, opened/
   updated by the failing scheduled workflow and closed on recovery. The hook
   block lists open ones. Issues are the durable, tool-agnostic record.
7. **Delete the fleet machinery**: `vox ci queue`, `runner_scale.rs`, the
   PreToolUse hook-guard, `runner-policy-check` + hosted-exceptions ledger, the
   `fleet-down` fallback workflow, health watchdog/dead-man, the CI runner image.

## Non-goals

- Blocking pushes on red CI (punishes the agent pushing the fix; the merge gate
  already enforces green).
- Mesh/Hetzner as CI capacity. Hetzner stays a deploy target.
- GPU lanes (qwen35, ML extraction): disabled until a GPU runner exists.
- Cursor/Gemini-specific hook files: git hooks already reach those agents.
