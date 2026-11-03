# Hosted-Primary CI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make GitHub-hosted CI the working primary gate (≤30 min per PR job), move slow lanes to a GitHub-hosted nightly (≤180 min), enforce those timeouts, and push CI failures, timeouts and nightly breakage into every agent's context automatically.

**Architecture:** `ci.yml` is rewritten as one small hosted gate job that runs the local fast pre-push tier plus clippy/nextest on affected crates. The old 2000-line `ci.yml` becomes `nightly.yml`. One `nightly-report.yml` (`workflow_run` listener) opens/closes `nightly-failure` issues for every scheduled workflow. A new `workflow_policy_guard` (in `ssot-drift`) enforces timeout caps and that every scheduled workflow is listed in `nightly-report.yml`. A new `vox ci status` renders a per-branch CI block from a cached snapshot; git hooks and Claude Code hooks print it, so no agent has to remember to ask. The self-hosted fleet machinery is deleted last.

**Tech Stack:** GitHub Actions YAML, Rust (`vox-cli`, `vox-cli-ci`, `serde_yaml` = `serde_yaml_ng` 0.10, `serde_json`, `chrono`), `gh` CLI, lefthook, Claude Code hooks (`.claude/settings.json`), `nektos/act` (unchanged).

**Spec:** `docs/superpowers/specs/2026-09-21-hosted-primary-ci-design.md`

## Global Constraints

- PR/merge_group/branch-push job cap: `timeout-minutes` ≤ **30**. All other workflows: ≤ **180**. Literal integers only.
- Required branch-protection context name stays exactly `Check, Build, and Test (Rust)`, published only by `ci.yml` (enforced by the existing `required_context_guard`).
- New `runs-on` values: `ubuntu-latest` (or `macos-latest`/`windows-latest` where a job genuinely needs that OS). No `self-hosted` labels anywhere after Task 5, except the two GPU workflows (`qwen35-native-nightly.yml`, `ml_data_extraction.yml`), which stay disabled.
- Nightly-failure issue label: `nightly-failure`. Issue title: `Nightly failing: <workflow name>`. Reporter: `.github/workflows/nightly-report.yml`.
- Status cache: `vox_config::paths::dot_vox_user_dir()/ci-status/<sanitized-branch>.json`; freshness 120 s.
- Never run `cargo fmt --all`. Format with `vox run scripts/fmt.vox` (dirty files) or `cargo fmt -p <crate>`.
- Every `cargo` call goes through plain `cargo` on PATH (build broker).
- New `pub fn` in `crates/*/src/**` needs a test in the same file (tdd-guard pre-commit hook).
- After changing any `CiCmd` variant run, in order: `cargo run -p vox-cli -- ci command-sync`, `cargo run -p vox-cli -- ci gui-surface-coverage --write`, `cargo run -p vox-cli -- ci doc-inventory generate`. <!-- AMENDED: #8 — gui-surface-coverage.v1.json and doc-inventory.json embed CLI paths/doc files; ssot-drift fails without regen -->
- Never hand-edit generated artifacts (`*.generated.md`, `contracts/reports/*.v1.json`, `docs/agents/doc-inventory.json`, `graphify-out/**`, `command_catalog_paths_baseline.txt`) — regenerate them. <!-- AMENDED: #15 -->
- Workflow YAML edits: edit file by file; never mass-`sed` across workflows (matrix JSON / `fromJson` expressions break). After editing workflows run `cargo run -q -p vox-cli -- ci ssot-drift` (parses every workflow). <!-- AMENDED: #20 -->
- Do not push or open PRs unless the user asks. Tasks 1–9 are one branch/PR; Task 10 is a separate PR. Live GitHub state changes (disable/enable workflows, cancel runs, push, replacing the user's installed `vox`, editing `.claude/settings.json`) need explicit user confirmation.
- Commit messages end with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

### Task 1: Fix the clippy errors on `main`

These make the hosted gate red regardless of anything else in this plan.

**Files:**
- Modify: `crates/vox-code-audit/src/review/providers.rs` — test fn `ollama_default_url_resolves_through_the_config_ssot` (currently ~218-228)
- Modify: `crates/vox-plugin-browser/tests/ax_snapshot_probe_test.rs` — `let fixture = vec![` in `test_compact_ax_probe_bounds`

- [ ] **Step 1: Reproduce the failure**

Run: `cargo clippy -p vox-code-audit -p vox-plugin-browser --all-targets --locked -- -D warnings`
Expected: FAIL with `usage of an \`unsafe\` block` at `providers.rs:222` and `:225`, and `useless use of \`vec!\`` at `ax_snapshot_probe_test.rs:5`.

- [ ] **Step 2: Allow `unsafe_code` on the env-mutating test only**

The workspace sets `unsafe_code = "warn"` (`Cargo.toml` `[workspace.lints]`), which `-D warnings` promotes to an error; Rust 2024 makes `set_var` unsafe, and the existing SAFETY comment already justifies it — same pattern as `crates/vox-cli/src/commands/ci/pre_push.rs` `pub fn run`. <!-- AMENDED: #29 — crate does not "forbid" unsafe; it's a workspace warn lint -->

```rust
    #[test]
    #[allow(unsafe_code)] // Rust 2024 `set_var` is unsafe; see SAFETY below.
    fn ollama_default_url_resolves_through_the_config_ssot() {
```

- [ ] **Step 3: Replace `vec!` with an array**

```rust
    let fixture = [
        json!({ "role": { "type": "role", "value": "button" }, "name": { "value": "Submit" } }),
        json!({ "role": { "type": "role", "value": "paragraph" }, "name": { "value": "Static text" } }),
    ];
```

- [ ] **Step 4: Verify clean**

Run: `cargo clippy -p vox-code-audit -p vox-plugin-browser --all-targets --locked -- -D warnings`
Expected: PASS.
Run: `cargo test -p vox-code-audit ollama_default_url_resolves_through_the_config_ssot && cargo test -p vox-plugin-browser --test ax_snapshot_probe_test`
Expected: both PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-code-audit/src/review/providers.rs crates/vox-plugin-browser/tests/ax_snapshot_probe_test.rs
git commit -m "fix(clippy): clear the two errors keeping hosted CI red on main"
```

---

### Task 2: Disable the two GPU workflows

<!-- AMENDED: #14/#23 — original disabled 13 workflows and cancelled every queued run repo-wide (incl. merge_group/release runs, no rollback). 4 of the 13 are already hosted; self-hosted queued runs expire on their own after 24h; Task 5 migrates the rest. Only the GPU lanes need disabling (spec non-goal). -->

**Files:** none (repository settings via `gh`).

- [ ] **Step 1: Confirm with the user**

Ask: "Disable `qwen35-native-nightly.yml` and `ml_data_extraction.yml` (need a GPU runner that doesn't exist)? Re-enable later with `gh workflow enable <file>`." Wait for "yes".

- [ ] **Step 2: Disable**

```bash
gh workflow disable qwen35-native-nightly.yml
gh workflow disable ml_data_extraction.yml
```

- [ ] **Step 3: Verify**

Run: `gh workflow list --all --json path,state --jq '.[]|select(.state!="active")|.path'`
Expected: includes both paths.

Do **not** cancel queued runs; they expire after 24h and include merge-queue/release runs.

(No commit — no files changed.)

---

### Task 3: Retire `runner-policy-check`

It hard-fails `ssot-drift` whenever a workflow uses a GitHub-hosted runner without a ledger entry — the opposite of the new policy — so it must go before any workflow moves to `ubuntu-latest`.

**Files:**
- Delete: `crates/vox-cli-ci/src/runner_policy_check.rs`
- Delete: `docs/src/ci/github-hosted-exceptions.md`
- Modify: `crates/vox-cli-ci/src/lib.rs` (remove `pub mod runner_policy_check;`)
- Modify: `crates/vox-cli-ci/src/constants.rs` — remove `"docs/src/ci/github-hosted-exceptions.md",` from `DOCS_SSOT_FILES` <!-- AMENDED: #5 — check_docs_ssot is the first ssot-drift stage and hard-fails on a missing listed doc -->
- Modify: `crates/vox-cli-ci/src/cmd_enums.rs` (remove the `RunnerPolicyCheck` variant and its doc comment)
- Modify: `crates/vox-cli/src/commands/ci/run_body.rs` (remove the `CiCmd::RunnerPolicyCheck { strict } => …` arm)
- Modify: `crates/vox-cli/src/commands/ci/pre_push.rs` (remove the `OwnedStep` labelled `"vox ci runner-policy-check"` and `fn step_runner_policy_check`)
- Modify: `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs` (in `run_ssot_drift`: remove the comment block and `ds!("runner_policy_check", …)`)
- Modify: `crates/vox-cli-ci/Cargo.toml` (`description` mentions runner policy — reword) <!-- AMENDED: #16 — hits missing from file list -->
- Modify: `.cursor/rules/ci-runner-convention.mdc`, `docs/src/contributors/local-ci-pre-push.md`, `docs/src/ci/alternatives-and-local-mirroring.md`, `docs/src/ci/rcicd-coverage-cost-matrix-2026.md`, `docs/src/ci/runner-contract.md`, `docs/src/architecture/where-things-live.md`, `docs/src/reference/cli.md` — remove/reword sentences about the hosted-runner exception ledger
- Modify: `AGENTS.md` — §Local CI Gate Tiers: delete the sentence group from "**Local-first runner policy:** CI jobs default to" through "…hosted minutes are free." (Task 9 rewrites the CI contract section)
- Regenerate (never hand-edit): `docs/src/reference/cli-command-surface.generated.md`, `crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt`, `contracts/reports/gui-surface-coverage.v1.json`, `docs/agents/doc-inventory.json`

**Interfaces:** none produced.

- [ ] **Step 1: Find every reference**

Run: `git grep -n 'runner_policy_check\|runner-policy-check\|RunnerPolicyCheck\|github-hosted-exceptions' -- ':!docs/src/archive' ':!docs/superpowers' ':!graphify-out' ':!contracts/reports' ':!*.generated.md' ':!docs/agents/doc-inventory.json' ':!crates/vox-cli/tests/fixtures'`
Expected: the files listed above. `.github/workflows/*.yml` hits in comments: reword in Task 5 when those files are edited anyway; `publish-ci-runner.yml` is deleted in Task 5 — skip it here.

- [ ] **Step 2: Delete the module and its call sites** (edits listed under **Files**).

- [ ] **Step 3: Regenerate derived artifacts**

```bash
cargo run -p vox-cli -- ci command-sync
cargo run -p vox-cli -- ci gui-surface-coverage --write
cargo run -p vox-cli -- ci doc-inventory generate
```
Expected: the generated files lose `runner-policy-check` / `runner_policy_check.rs` / `github-hosted-exceptions.md`.

- [ ] **Step 4: Verify**

Run: `cargo check -p vox-cli -p vox-cli-ci --all-targets --locked`
Expected: PASS.
Run: `cargo run -q -p vox-cli -- ci ssot-drift && cargo run -q -p vox-cli -- ci check-links`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A crates docs AGENTS.md .cursor contracts
git commit -m "ci: retire runner-policy-check; hosted runners are the default now"
```

---

### Task 4: Hosted gate — move old `ci.yml` to `nightly.yml`, write a new small `ci.yml`

**Files:**
- Rename: `.github/workflows/ci.yml` → `.github/workflows/nightly.yml` (edited in Task 5)
- Create: `.github/workflows/ci.yml`
- Modify: `crates/vox-cli/tests/ci_workflow_contract.rs` <!-- AMENDED: #2 — ~10 include_str! tests assert old ci.yml content -->
- Delete: `crates/vox-cli/src/commands/ci/merge_group_fanout_guard.rs`; Modify `crates/vox-cli/src/commands/ci/mod.rs` (remove `#[cfg(test)] mod merge_group_fanout_guard;`) <!-- AMENDED: #4 — its test panics once ci.yml has no self-hosted jobs; it guards a fleet ceiling that no longer exists -->

**Interfaces:**
- Produces: job `gate` with `name: Check, Build, and Test (Rust)` in `ci.yml`; Rust cache `shared-key: workspace` restored (saved only by nightly on `main`, Task 5).

- [ ] **Step 1: Move the old workflow aside**

```bash
git mv .github/workflows/ci.yml .github/workflows/nightly.yml
```
Temporarily make it inert until Task 5 by replacing its `on:` block with:
```yaml
on:
  workflow_dispatch:
```

- [ ] **Step 2: Write the new `.github/workflows/ci.yml`**

```yaml
# PR / merge-queue gate. Hosted, ≤30 min (workflow_policy_guard enforces the cap).
# Slow lanes live in nightly.yml. Run this job locally in Docker:
#   act pull_request -j gate
name: CI

on:
  pull_request:
  merge_group:

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  CARGO_INCREMENTAL: "0"
  RUSTC_WRAPPER: sccache
  SCCACHE_GHA_ENABLED: "true"

jobs:
  gate:
    # Branch-protection required context — do not rename (required_context_guard).
    name: Check, Build, and Test (Rust)
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0
      - uses: ./.github/actions/install-linux-system-deps
      - uses: ./.github/actions/setup-rust
        with:
          components: rustfmt, clippy
          cache: "false"   # Swatinem below is the only Rust cache; PRs never write it
      - uses: mozilla-actions/sccache-action@v0.0.11
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: workspace
          save-if: false   # nightly.yml on main is the only writer (default-branch caches are shared)
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-nextest

      - name: Build vox CLI
        run: cargo build -p vox-cli --locked

      - name: Fast tier (same checks as local `vox ci pre-push`)
        run: ./target/debug/vox ci pre-push

      - name: Affected crates
        id: affected
        shell: bash
        env:
          BASE_SHA: ${{ github.event.pull_request.base.sha || github.event.merge_group.base_sha }}
        run: |
          set -euo pipefail
          git diff --name-only "$BASE_SHA...HEAD" > /tmp/changed.txt
          if ! grep -qE '^(crates/|Cargo\.(toml|lock)|\.cargo/|rust-toolchain\.toml|\.github/workflows/)' /tmp/changed.txt; then
            echo "No Rust changes — clippy/tests skipped."
            echo "p_args=" >> "$GITHUB_OUTPUT"
            exit 0
          fi
          ./target/debug/vox ci affected-crates --changed /tmp/changed.txt \
            --graph contracts/ci/crate-graph.v1.json --github-output /tmp/affected.out
          full=$(grep -E '^full=' /tmp/affected.out | tail -1 | cut -d= -f2 || true)
          args=$(grep -E '^affected_p_args=' /tmp/affected.out | tail -1 | cut -d= -f2- || true)
          # Fail closed: anything but an explicit non-empty affected set → whole workspace.
          if [ "$full" != "false" ] || [ -z "$(echo "$args" | xargs)" ]; then
            args="--workspace --exclude vox-gui"
          fi
          # vox-gui needs ui/dist + the release sidecar (tauri-build); it is covered by nightly.
          args=$(echo "$args" | sed 's/-p vox-gui//g' | xargs)
          echo "p_args=$args" >> "$GITHUB_OUTPUT"
          echo "### Affected: \`$args\`" >> "$GITHUB_STEP_SUMMARY"

      - name: Clippy (affected)
        if: steps.affected.outputs.p_args != ''
        run: cargo clippy ${{ steps.affected.outputs.p_args }} --all-targets --locked -- -D warnings

      - name: Tests (affected)
        if: steps.affected.outputs.p_args != ''
        run: cargo nextest run ${{ steps.affected.outputs.p_args }} --profile ci --locked --no-tests=pass
```
<!-- AMENDED: #1 — strip -p vox-gui from affected args (old ci.yml did); #6 — CARGO_INCREMENTAL quoted for sccache_workflow_guard; #12 — setup-rust cache "false"; #17 — .github/workflows/ counts as a Rust change; #18 — --act does not run ci.yml, use act pull_request -j gate -->

- [ ] **Step 3: Carry over `ssot-autoregen`**

Cut the `ssot-autoregen:` job block from `nightly.yml` and paste it under `jobs:` in `ci.yml`, then:
- delete its `needs: setup` line;
- set `runs-on: ubuntu-latest` and `timeout-minutes: 30` (cold build of vox-cli; 15 is too tight on a hosted runner);
- replace the `actions/download-artifact` step (artifact `vox-cli-bin`) and the following `Ensure vox-cli executable` step with the steps below. **This job holds `contents: write` and SHA-pins its actions** — pin every added action to a full commit SHA. Find the pins already used in the repo with `git grep -hoE '(Swatinem/rust-cache|mozilla-actions/sccache-action)@[0-9a-f]{40}' .github | sort -u`; if none exist, resolve with `gh api repos/Swatinem/rust-cache/commits/v2 --jq .sha`. <!-- AMENDED: #21 — supply-chain: tag refs ahead of a PAT push step -->
```yaml
      - uses: ./.github/actions/install-linux-system-deps
      - uses: ./.github/actions/setup-rust
        with:
          cache: "false"
      - uses: Swatinem/rust-cache@<40-char-sha> # v2
        with:
          shared-key: workspace
          save-if: false
      - name: Build vox CLI
        run: cargo build -p vox-cli --locked --features completion-toestub,extras-ludus,ars,coderabbit
```
(The features match the old prebuilt artifact — clap paths are feature-dependent, so generator output must not change.) <!-- AMENDED: #22 -->

- [ ] **Step 4: Update the workflow contract tests**

Run: `cargo test -p vox-cli --test ci_workflow_contract 2>&1 | grep -E '^test .* FAILED|panicked'`
Expected: several failures (tests `include_str!` the old `ci.yml`).
For each failing test:
- content that now lives in `nightly.yml` (llvm-cov, populi gate, TOESTUB, doc-inventory, fail-closed selective CI): change its `include_str!("…/.github/workflows/ci.yml")` to `nightly.yml`;
- assertions that only described the fleet (`runs-on: [self-hosted, linux, x64]`, shadow comparator, runner labels): delete the assertion (or the test if nothing else remains).
Add one test for the new gate:
```rust
#[test]
fn ci_gate_is_hosted_capped_and_owns_required_context() {
    let yml = include_str!("../../../.github/workflows/ci.yml");
    assert!(yml.contains("name: Check, Build, and Test (Rust)"));
    assert!(yml.contains("runs-on: ubuntu-latest"));
    assert!(yml.contains("timeout-minutes: 30"));
    assert!(!yml.contains("self-hosted"));
    assert!(yml.contains("sed 's/-p vox-gui//g'"), "affected args must never build vox-gui");
}
```
Verify the relative path matches the other `include_str!` calls in that file.

- [ ] **Step 5: Delete the fleet fan-out guard**

```bash
git rm crates/vox-cli/src/commands/ci/merge_group_fanout_guard.rs
```
Remove `#[cfg(test)] mod merge_group_fanout_guard;` from `crates/vox-cli/src/commands/ci/mod.rs`.

- [ ] **Step 6: Lint + tests**

Run: `cargo run -q -p vox-cli -- ci required-context-guard && cargo run -q -p vox-cli -- ci workflow-concurrency-guard --strict`
Expected: both PASS.
Run: `cargo test -p vox-cli --test ci_workflow_contract && cargo test -p vox-cli --lib commands::ci`
Expected: PASS (incl. `sccache_workflow_guard` tests).
Run: `act pull_request -j gate -n`
Expected: plan printed, no parse error.

- [ ] **Step 7: Run the gate locally in Docker**

Run: `act pull_request -j gate --container-architecture linux/amd64 -P ubuntu-latest=catthehacker/ubuntu:full-latest`
Expected: every step green. If `vox ci pre-push` fails for a missing tool (e.g. `pnpm`), add the matching setup step **before** the fast-tier step (for pnpm: copy the `pnpm/action-setup@v6` + `actions/setup-node@v7` steps from `nightly.yml`'s `guards-fast` job verbatim). Do not remove checks from the fast tier.

- [ ] **Step 8: Commit**

```bash
git add -A .github/workflows crates/vox-cli
git commit -m "ci: hosted 30-minute PR gate; old ci.yml parked as nightly.yml"
```

- [ ] **Step 9: Measure on GitHub (requires the user's go-ahead to push)**

Ask the user before pushing. **Cold-cache risk:** until a nightly on `main` has seeded the shared cache (impossible before this PR merges), the gate compiles cold and may exceed 30 min, and this PR's own required check comes from the new `ci.yml`. Mitigations in order: re-run the job (sccache writes PR-scoped entries, so the second run is warmer); if still over, tell the user an admin merge bypass is needed once. <!-- AMENDED: #24 — ordering risk: no fallback gate once ci-fallback-hosted is deleted -->
After a run completes:
Run: `gh run list --workflow ci.yml --branch "$(git branch --show-current)" --limit 1 --json databaseId --jq '.[0].databaseId' | xargs -I{} gh api repos/vox-foundation/vox/actions/runs/{}/jobs --jq '.jobs[].steps[]|select(.started_at)|"\(((.completed_at|fromdate)-(.started_at|fromdate))/60|floor)m \(.name)"'`
Expected: total under 30 min on a warm run. Record per-step minutes in the PR description.

---

### Task 5: Nightly on GitHub-hosted runners; delete fleet-only workflows

**Files:**
- Modify: `.github/workflows/nightly.yml`
- Delete: `.github/workflows/ci-fallback-hosted.yml`, `ci-health-watchdog.yml`, `ci-health-watchdog-test.yml`, `ci-health-deadman.yml`, `publish-ci-runner.yml`, `ci-timings.yml`, `.github/actions/ci-health-assess/` <!-- AMENDED: #25 — ci-timings.yml listens for the deleted fallback and enforces a 10-min budget that contradicts the 30-min cap -->
- Modify: `crates/vox-cli-ci/src/job_timings.rs` (module doc + `SLOW_JOB_THRESHOLD_SECS` doc mention `ci-timings.yml` — reword to "on demand"; code unchanged, `run_seconds` is reused by Task 8)
- Modify (runs-on only): every workflow still containing `self-hosted` (list with Step 4's command), except `qwen35-native-nightly.yml` and `ml_data_extraction.yml`
- Modify: `crates/vox-cli-ci/src/required_context_guard.rs` (doc paragraph only)

**Interfaces:**
- Produces: `nightly.yml` job `full` saves Rust cache `shared-key: workspace` on `main` (consumed by Task 4's gate).

- [ ] **Step 1: Re-trigger `nightly.yml` on schedule**

Replace its `on:` block with:
```yaml
on:
  schedule:
    - cron: '0 6 * * *'
  workflow_dispatch:
```
Change `name:` at the top of the file to `Nightly`.

- [ ] **Step 2: Make "nightly" actually run everything** <!-- AMENDED: #10 — setup's filter diffs HEAD~1 on non-merge_group events, and four jobs only run on push-to-main / full-ci label -->

- In the `setup` job's `filter` step, change `if [ "${{ github.event_name }}" = "merge_group" ]; then` to `if [ "${{ github.event_name }}" != "pull_request" ]; then` so schedule/dispatch force `rust=true` and `docs=true`.
- For jobs `docker-vox-image-smoke`, `vox-browser-cdp-smoke`, `gui-playwright-smoke`, `all-features-matrix`: append `|| github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'` to each job's `if:` expression (wrap the existing expression in parentheses first).

- [ ] **Step 3: Remove PR-only jobs from `nightly.yml`**

Delete job blocks `toolchain-lint-wave` and `ci-summary` (the one named `Check, Build, and Test (Rust)` — the required context must live only in `ci.yml`). Toolchain-bump lint waves are still caught: `rust-cache` keys include the rustc version, so the first nightly after a bump runs the `full` job's clippy + rustdoc cold (fresh), per AGENTS.md §Perennial Bug Patterns. <!-- AMENDED: #11 — re-home the toolchain-lint-wave guarantee -->

- [ ] **Step 4: Replace self-hosted labels with hosted runners**

Run: `git grep -n 'self-hosted' -- .github/ ':!**/qwen35-native-nightly.yml' ':!**/ml_data_extraction.yml'` <!-- AMENDED: #19 — pathspec after `--` -->
For each non-comment hit, edit that file by hand: replace `[self-hosted, linux]`, `[self-hosted, linux, x64]`, `[self-hosted, linux, x64, docker]`, `[self-hosted, linux, x64, browser]` with `ubuntu-latest`. For matrix-driven values (`${{ matrix.runner }}`, `${{ fromJson(matrix.runs_on) }}`, `${{ matrix.os }}`), edit the matrix entries: a plain string becomes `ubuntu-latest`; inside JSON, `["self-hosted","linux"]` becomes `["ubuntu-latest"]`. Reword comment hits that describe the fleet as current.
Re-run the grep; Expected: only comment lines (or none).

- [ ] **Step 5: Remove fleet-only assumptions inside moved jobs**

- In `nightly.yml` top-level `env:`, delete `RUSTC_WRAPPER: sccache` (and any sccache-only env). Only one job installs sccache-action; on hosted runners every other job would fail with "sccache not found". Delete any `sccache --show-stats` steps outside the job that installs it. <!-- AMENDED: #9 -->
- In every job that calls `./.github/actions/setup-rust` and also uses `Swatinem/rust-cache`, pass `cache: "false"` to setup-rust.
- Run: `git grep -n '/cache/\|colima\|docker volume' -- .github/workflows/` — replace `mkdir -p /cache/advisory-db && cargo deny fetch` with `cargo deny fetch`; delete steps whose whole purpose is a fleet shared volume. Keep the job's real checks.

- [ ] **Step 6: Add the full-workspace job that seeds the cache**

```yaml
  full:
    name: Full workspace (clippy, rustdoc, nextest, doctests, deny, audit)
    runs-on: ubuntu-latest
    timeout-minutes: 180
    steps:
      - uses: actions/checkout@v7
      - uses: ./.github/actions/install-linux-system-deps
      - uses: ./.github/actions/setup-rust
        with:
          components: rustfmt, clippy
          cache: "false"
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: workspace
          save-if: ${{ github.ref == 'refs/heads/main' }}
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-nextest,cargo-deny,cargo-audit
      - run: cargo build -p vox-cli --locked
      - run: cargo clippy --workspace --exclude vox-gui --all-targets --locked -- -D warnings
      - run: cargo doc --workspace --exclude vox-gui --no-deps --locked
        env:
          RUSTDOCFLAGS: -D warnings
      - run: cargo nextest run --workspace --exclude vox-gui --profile ci --locked
      - run: cargo test --workspace --exclude vox-gui --doc --locked
      - run: cargo deny check
      - run: cargo audit
```
Copy the `gui-windows-build-smoke` job from `ci-fallback-hosted.yml` into `nightly.yml` verbatim, then set `timeout-minutes: 180` and delete its `if:` line (the `fleet-down` condition).

- [ ] **Step 7: Delete fleet-only workflows**

```bash
git rm .github/workflows/ci-fallback-hosted.yml .github/workflows/ci-health-watchdog.yml .github/workflows/ci-health-watchdog-test.yml .github/workflows/ci-health-deadman.yml .github/workflows/publish-ci-runner.yml .github/workflows/ci-timings.yml
git rm -r .github/actions/ci-health-assess
```
Run: `git grep -n 'ci-fallback-hosted\|fleet-down\|ci-health-watchdog\|ci-health-deadman\|publish-ci-runner\|ci-health-assess\|ci-timings\|CI Fallback (GitHub-hosted)' -- ':!docs/src/archive' ':!docs/superpowers' ':!graphify-out' ':!contracts/reports'`
- `required_context_guard.rs`: tests use `"ci-fallback-hosted.yml"` as a fixture name — leave them; reword only the `//!` paragraph to past tense ("…named its `gate` job… (deleted 2026-09)").
- `job_timings.rs`: reword the two doc mentions of `ci-timings.yml`.
- Docs under `docs/src/ci/`: delete break-glass / fleet-down sections.
- `crates/vox-cli/src/commands/ci/{runner_scale,queue,oom_watch}.rs`: deleted in Task 10 — leave.

- [ ] **Step 8: Verify**

Run: `cargo run -q -p vox-cli -- ci required-context-guard && cargo run -q -p vox-cli -- ci workflow-concurrency-guard --strict && cargo run -q -p vox-cli -- ci ssot-drift && cargo run -p vox-cli -- ci doc-inventory generate`
Expected: all PASS.
Run: `cargo test -p vox-cli --test ci_workflow_contract && cargo test -p vox-cli-ci`
Expected: PASS (fix any assertion that referenced a deleted workflow the same way as Task 4 Step 4).
Run: `act workflow_dispatch -W .github/workflows/nightly.yml -n`
Expected: parses; lists jobs including `full`.

- [ ] **Step 9: Commit**

```bash
git add -A .github docs crates
git commit -m "ci: nightly on GitHub-hosted runners; delete fleet-only workflows"
```

---

### Task 6: `nightly-report.yml` — failing scheduled runs become `nightly-failure` issues

<!-- AMENDED: #C1 (simplicity, −~370 lines) — one workflow_run listener instead of a composite action + a report job copied into ~22 files with hand-maintained `needs:` lists. Also reordered before the guard so ssot-drift is never red between commits (#P5). -->

**Files:**
- Create: `.github/workflows/nightly-report.yml`

**Interfaces:**
- Produces: open GitHub issues labelled `nightly-failure`, titled `Nightly failing: <workflow name>` (read by Task 8). The `workflows:` list is the set Task 7's guard checks against.

- [ ] **Step 1: List the scheduled workflows' names**

Run: `for f in $(grep -lE '^\s+schedule:' .github/workflows/*.yml); do printf '%s\t%s\n' "$(basename "$f")" "$(grep -m1 -E '^name:' "$f" | sed -E 's/^name:[[:space:]]*//; s/^["'\'']//; s/["'\'']$//')"; done`
Expected: one line per scheduled workflow with its exact `name:` (e.g. `nightly.yml	Nightly`). Workflows without a top-level `name:` are listed by GitHub under their file path — give them a `name:` so they can be listed.

- [ ] **Step 2: Write `.github/workflows/nightly-report.yml`**

```yaml
# Opens (or comments on) a `nightly-failure` issue when a scheduled workflow
# fails, times out, or is cancelled; closes it when that workflow passes again.
# `vox ci status` (git + Claude hooks) lists open ones, so a broken nightly is
# surfaced the next time anyone works on vox.
# workflow_policy_guard fails ssot-drift if a scheduled workflow is missing here.
name: Nightly report

on:
  workflow_run:
    workflows:
      - Nightly
      # …one entry per line from Step 1, exact `name:` values…
    types: [completed]

permissions:
  contents: read

jobs:
  report:
    if: github.event.workflow_run.event == 'schedule'
    runs-on: ubuntu-latest
    timeout-minutes: 5
    permissions:
      issues: write
    steps:
      - shell: bash
        env:
          GH_TOKEN: ${{ github.token }}
          GH_REPO: ${{ github.repository }}
          CONCLUSION: ${{ github.event.workflow_run.conclusion }}
          TITLE: "Nightly failing: ${{ github.event.workflow_run.name }}"
          RUN_URL: ${{ github.event.workflow_run.html_url }}
        run: |
          set -euo pipefail
          gh label create nightly-failure --color B60205 \
            --description "A scheduled workflow is failing" --force >/dev/null
          num=$(gh issue list --label nightly-failure --state open --limit 100 \
            --json number,title --jq 'map(select(.title == env.TITLE))[0].number // empty')
          case "$CONCLUSION" in
            success|skipped|neutral)
              if [ -n "$num" ]; then gh issue close "$num" --comment "Recovered: $RUN_URL"; fi ;;
            *)
              if [ -n "$num" ]; then
                gh issue comment "$num" --body "Still failing ($CONCLUSION): $RUN_URL"
              else
                gh issue create --label nightly-failure --title "$TITLE" --body \
                  "Scheduled run ended \`$CONCLUSION\`: $RUN_URL. Closes itself when the workflow passes again. For timeouts check the slowest steps before touching any cap (workflow-policy-guard)."
              fi ;;
          esac
```
Replace the comment line with the full list from Step 1 (one `- <name>` per line). Values come in through `env:` only (no `${{ }}` inside `run:`), so workflow names cannot inject shell.

- [ ] **Step 3: Verify**

Run: `cargo run -q -p vox-cli -- ci ssot-drift && cargo run -q -p vox-cli -- ci workflow-concurrency-guard --strict`
Expected: PASS (`workflow_run` needs no concurrency block; top-level `permissions:` present).
Run: `act workflow_run -W .github/workflows/nightly-report.yml -n`
Expected: parses.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/nightly-report.yml
git commit -m "ci: failing nightlies open nightly-failure issues and close on recovery"
```

Note: `workflow_run` only fires from the default branch, so this goes live after merge.

---

### Task 7: `workflow_policy_guard` — enforce timeout caps and nightly reporting

**Files:**
- Create: `crates/vox-cli-ci/src/workflow_policy_guard.rs`
- Modify: `crates/vox-cli-ci/src/lib.rs` (add `pub mod workflow_policy_guard;` after `pub mod workflow_permissions_guard;`)
- Modify: `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs` (new `ds!` in `run_ssot_drift`, where `runner_policy_check` used to be)
- Modify: any workflow the guard flags (Step 7); `crates/vox-cli/tests/ci_workflow_contract.rs` if Step 7 moves `cross-platform-check.yml` / `gui-cross-build.yml` to nightly

<!-- AMENDED: #C2 — no separate pre-push step; ssot-drift already runs in the pre-push fast tier and in the ci.yml gate (−10 lines, no double run) -->

**Interfaces:**
- Produces: `pub fn check_doc(file: &str, doc: &serde_yaml::Value) -> Vec<String>`, `pub fn missing_from_report(scheduled: &[String], listed: &[String]) -> Vec<String>`, `pub fn run(repo_root: &std::path::Path) -> anyhow::Result<()>`, `pub const FAST_CAP_MINS: u64 = 30`, `pub const SLOW_CAP_MINS: u64 = 180`, `pub const REPORT_WORKFLOW: &str = "nightly-report.yml"`.

- [ ] **Step 1: Declare the module and write the failing tests**

Add `pub mod workflow_policy_guard;` to `crates/vox-cli-ci/src/lib.rs` **now** (otherwise the tests are never compiled and Step 2 passes falsely with 0 tests). <!-- AMENDED: #T6-false-green -->
Create `crates/vox-cli-ci/src/workflow_policy_guard.rs` containing only:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn doc(y: &str) -> serde_yaml::Value {
        serde_yaml::from_str(y).unwrap()
    }

    #[test]
    fn pr_workflow_job_over_30_is_flagged() {
        let v = check_doc(
            "ci.yml",
            &doc("on: { pull_request: {} }\njobs:\n  a:\n    runs-on: ubuntu-latest\n    timeout-minutes: 31\n    steps: []"),
        );
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("exceeds cap 30"), "{v:?}");
    }

    #[test]
    fn sequence_trigger_caps_at_30() {
        let at = doc("on: [pull_request]\njobs:\n  a:\n    timeout-minutes: 30\n    steps: []");
        assert!(check_doc("ci.yml", &at).is_empty());
        let over = doc("on: [pull_request]\njobs:\n  a:\n    timeout-minutes: 31\n    steps: []");
        assert_eq!(check_doc("ci.yml", &over).len(), 1);
    }

    #[test]
    fn missing_expression_or_quoted_timeout_is_flagged() {
        let v = check_doc(
            "x.yml",
            &doc("on: workflow_dispatch\njobs:\n  a:\n    steps: []\n  b:\n    timeout-minutes: ${{ matrix.t }}\n    steps: []\n  c:\n    timeout-minutes: \"30\"\n    steps: []"),
        );
        assert_eq!(v.len(), 3, "{v:?}");
        assert!(v.iter().any(|m| m.contains("`a` has no timeout-minutes")));
        assert!(v.iter().any(|m| m.contains("`b` timeout-minutes must be a literal integer")));
        assert!(v.iter().any(|m| m.contains("`c` timeout-minutes must be a literal integer")));
    }

    #[test]
    fn trigger_classes() {
        let job = |on: &str, mins: u32| doc(&format!("on: {on}\njobs:\n  a:\n    timeout-minutes: {mins}\n    steps: []"));
        // fast (cap 30)
        assert_eq!(check_doc("w", &job("push", 31)).len(), 1);
        assert_eq!(check_doc("w", &job("{ pull_request_target: {} }", 31)).len(), 1);
        assert_eq!(check_doc("w", &job("{ merge_group: {} }", 31)).len(), 1);
        assert_eq!(check_doc("w", &job("{ push: { branches: [main], tags: ['v*'] } }", 31)).len(), 1);
        // slow (cap 180)
        assert!(check_doc("w", &job("{ push: { tags: ['v*'] } }", 180)).is_empty());
        assert!(check_doc("w", &job("{ workflow_run: { workflows: [CI] } }", 180)).is_empty());
        assert!(check_doc("w", &job("{ schedule: [ { cron: '0 6 * * *' } ] }", 180)).is_empty());
        assert_eq!(check_doc("w", &job("{ schedule: [ { cron: '0 6 * * *' } ] }", 181)).len(), 1);
    }

    #[test]
    fn reusable_workflow_call_jobs_are_skipped() {
        let v = check_doc("x.yml", &doc("on: [pull_request]\njobs:\n  a:\n    uses: ./.github/workflows/y.yml"));
        assert!(v.is_empty(), "{v:?}");
    }

    #[test]
    fn scheduled_names_missing_from_report_are_named() {
        let scheduled = vec!["Nightly".to_string(), "CodeQL".to_string()];
        let listed = vec!["Nightly".to_string()];
        let v = missing_from_report(&scheduled, &listed);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("CodeQL") && v[0].contains("nightly-report.yml"), "{v:?}");
        assert!(missing_from_report(&scheduled, &scheduled).is_empty());
    }

    #[test]
    fn repo_workflows_satisfy_policy() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        run(&root).unwrap();
    }
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cargo test -p vox-cli-ci workflow_policy_guard`
Expected: FAIL to compile — `cannot find function check_doc` / `missing_from_report` / `run`. (If it reports "0 tests", the `mod` line is missing — fix Step 1.)

- [ ] **Step 3: Implement**

Prepend to the same file:

```rust
//! `workflow_policy_guard` — the hosted-CI contract for workflow YAML
//! (docs/superpowers/specs/2026-09-21-hosted-primary-ci-design.md):
//!
//! 1. Every job declares a literal integer `timeout-minutes`.
//! 2. Jobs in a *fast* workflow — triggered by `pull_request`,
//!    `pull_request_target`, `merge_group`, or a branch `push` — cap at
//!    [`FAST_CAP_MINS`]; everything else (schedule, dispatch, tag-only push,
//!    workflow_run) caps at [`SLOW_CAP_MINS`]. A job that needs more belongs in
//!    nightly, or needs caching/sharding — not a higher cap.
//! 3. Every `schedule`-triggered workflow's `name:` is listed in
//!    [`REPORT_WORKFLOW`]'s `on.workflow_run.workflows`, so a failing nightly
//!    becomes an open `nightly-failure` issue agents are shown.
//!
//! Always fails on a violation. Runs inside `ssot-drift` (pre-push fast tier + ci.yml gate).

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde_yaml::Value;

pub const FAST_CAP_MINS: u64 = 30;
pub const SLOW_CAP_MINS: u64 = 180;
pub const REPORT_WORKFLOW: &str = "nightly-report.yml";

/// `serde_yaml_ng` parses `on:` as a string key; the `Bool(true)` fallback
/// covers YAML-1.1 parsers (mirrors workflow_concurrency_guard).
fn triggers(doc: &Value) -> Option<&Value> {
    let m = doc.as_mapping()?;
    m.get(Value::String("on".into()))
        .or_else(|| m.get(Value::Bool(true)))
}

fn trigger_keys(doc: &Value) -> Vec<(&str, Option<&Value>)> {
    match triggers(doc) {
        Some(Value::String(s)) => vec![(s.as_str(), None)],
        Some(Value::Sequence(seq)) => seq.iter().filter_map(Value::as_str).map(|s| (s, None)).collect(),
        Some(Value::Mapping(m)) => m
            .iter()
            .filter_map(|(k, v)| k.as_str().map(|k| (k, Some(v))))
            .collect(),
        _ => Vec::new(),
    }
}

/// A push filtered to tags only is a release, not the dev loop.
fn is_tag_only_push(filters: Option<&Value>) -> bool {
    let Some(m) = filters.and_then(Value::as_mapping) else {
        return false;
    };
    let has = |k: &str| m.contains_key(Value::String(k.into()));
    (has("tags") || has("tags-ignore")) && !has("branches") && !has("branches-ignore")
}

fn is_fast(doc: &Value) -> bool {
    trigger_keys(doc).into_iter().any(|(k, v)| match k {
        "pull_request" | "pull_request_target" | "merge_group" => true,
        "push" => !is_tag_only_push(v),
        _ => false,
    })
}

fn has_schedule(doc: &Value) -> bool {
    trigger_keys(doc).iter().any(|(k, _)| *k == "schedule")
}

fn workflow_name(file: &str, doc: &Value) -> String {
    doc.get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| file.to_string())
}

fn report_listed(report: &Value) -> Vec<String> {
    trigger_keys(report)
        .into_iter()
        .find(|(k, _)| *k == "workflow_run")
        .and_then(|(_, v)| v?.get("workflows")?.as_sequence())
        .map(|s| s.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

/// Timeout violations for one parsed workflow, each prefixed with `file`.
pub fn check_doc(file: &str, doc: &Value) -> Vec<String> {
    let cap = if is_fast(doc) { FAST_CAP_MINS } else { SLOW_CAP_MINS };
    let jobs = doc.get("jobs").and_then(Value::as_mapping);
    let mut out = Vec::new();
    for (name, job) in jobs.into_iter().flatten() {
        let name = name.as_str().unwrap_or("?");
        // Reusable-workflow calls inherit the callee's job timeouts.
        if job.get("uses").is_some() {
            continue;
        }
        match job.get("timeout-minutes") {
            None => out.push(format!("{file}: job `{name}` has no timeout-minutes (cap {cap})")),
            Some(t) => match t.as_u64() {
                Some(m) if m <= cap => {}
                Some(m) => out.push(format!(
                    "{file}: job `{name}` timeout-minutes {m} exceeds cap {cap}"
                )),
                None => out.push(format!(
                    "{file}: job `{name}` timeout-minutes must be a literal integer (got {t:?})"
                )),
            },
        }
    }
    out
}

/// Scheduled workflow names absent from the report workflow's list.
pub fn missing_from_report(scheduled: &[String], listed: &[String]) -> Vec<String> {
    scheduled
        .iter()
        .filter(|n| !listed.contains(n))
        .map(|n| format!("scheduled workflow `{n}` is not listed in {REPORT_WORKFLOW} on.workflow_run.workflows"))
        .collect()
}

pub fn run(repo_root: &Path) -> Result<()> {
    let dir = repo_root.join(".github").join("workflows");
    if !dir.is_dir() {
        return Ok(());
    }
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .with_context(|| format!("read {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .collect();
    paths.sort();
    let mut violations = Vec::new();
    let mut scheduled = Vec::new();
    let mut listed = Vec::new();
    for path in paths {
        let file = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let doc: Value = serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        violations.extend(check_doc(&file, &doc));
        if has_schedule(&doc) {
            scheduled.push(workflow_name(&file, &doc));
        }
        if file == REPORT_WORKFLOW {
            listed = report_listed(&doc);
        }
    }
    violations.extend(missing_from_report(&scheduled, &listed));
    if violations.is_empty() {
        println!("workflow-policy-guard OK");
        return Ok(());
    }
    Err(anyhow!(
        "workflow-policy-guard: {} violation(s):\n  {}\n\
         Fast workflows (PR / merge_group / branch push) cap jobs at {FAST_CAP_MINS} min; \
         others at {SLOW_CAP_MINS}. Over budget? Cache, shard, or move the job to nightly.yml. \
         Every scheduled workflow must be listed in {REPORT_WORKFLOW}.",
        violations.len(),
        violations.join("\n  ")
    ))
}
```

- [ ] **Step 4: Run the unit tests**

Run: `cargo test -p vox-cli-ci workflow_policy_guard -- --skip repo_workflows_satisfy_policy`
Expected: 6 PASS.

- [ ] **Step 5: See the real violations**

Run: `cargo test -p vox-cli-ci workflow_policy_guard::tests::repo_workflows_satisfy_policy`
Expected: FAIL listing violations. From the 2026-09-21 scan expect at least: `nightly.yml` job with `${{ matrix.timeout }}`; fast-class jobs above 30 (`cross-platform-check.yml` 90/60, `gui-cross-build.yml` 90, `mobile-e2e-android.yml` 45, `docker-telemetry.yml` 45, `setup-e2e.yml` 90 if branch-push-triggered); jobs with no timeout (e.g. `coolify-eval-sync.yml`); slow-class jobs above 180 (`nightly-artifacts.yml` 240, `release-binaries.yml`/`release-gui.yml` 240 — tag-only?).

- [ ] **Step 6: Fix timeout violations**

For each violation, pick exactly one, in this order of preference: <!-- AMENDED: #26 — rule for missing timeouts -->
1. Missing `timeout-minutes` → add the smallest literal that fits the job's recent run time (look it up with `gh run list --workflow <file> --limit 5`), never above the class cap.
2. A slow lane on a fast trigger → remove the `pull_request`/`merge_group`/branch-`push` trigger, add `schedule: [{cron: '30 6 * * *'}]` + `workflow_dispatch:`, give it a `name:` if missing, and add that name to `nightly-report.yml`'s list. **If the file is `cross-platform-check.yml` or `gui-cross-build.yml`**, update `crates/vox-cli/tests/ci_workflow_contract.rs` tests `cross_platform_gate_is_required_three_os_matrix`, `cross_platform_pr_is_path_filtered`, `gui_cross_build_covers_three_os_with_webkit` to assert `schedule:` instead of `pull_request:`/`merge_group:` (delete the path-filter assertion). <!-- AMENDED: #3 -->
3. Legitimately fast → lower `timeout-minutes` to ≤ 30.
4. Slow-class job over 180 (release builds) → `timeout-minutes: 180`.
5. `${{ … }}` timeout → the largest literal the matrix used, clamped to the cap.

Re-run until green:
Run: `cargo test -p vox-cli-ci workflow_policy_guard && cargo test -p vox-cli --test ci_workflow_contract`
Expected: all PASS.

- [ ] **Step 7: Wire into ssot-drift**

In `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs` `run_ssot_drift`, where the `runner_policy_check` `ds!` was removed in Task 3:
```rust
    ds!(
        "workflow_policy_guard",
        vox_cli_ci::workflow_policy_guard::run(root)
    )?;
```
Run: `cargo run -q -p vox-cli -- ci ssot-drift`
Expected: PASS, prints `workflow-policy-guard OK`.
Run: `cargo run -q -p vox-cli -- ci workflow-concurrency-guard --strict && cargo run -q -p vox-cli -- ci required-context-guard`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-cli-ci crates/vox-cli .github/workflows
git commit -m "ci: enforce 30/180-minute job caps and nightly reporting via workflow-policy-guard"
```

---

### Task 8: `vox ci status` — per-branch CI block from a cached snapshot

**Files:**
- Create: `crates/vox-cli/src/commands/ci/status.rs`
- Modify: `crates/vox-cli/src/commands/ci/mod.rs` (add `mod status;` after `mod sccache_workflow_guard;` / before `mod unexpected_exit_watch;`) <!-- AMENDED: #28 — alphabetical -->
- Modify: `crates/vox-cli-ci/src/cmd_enums.rs` (add `Status` variant after `Queue`)
- Modify: `crates/vox-cli/src/commands/ci/run_body.rs` (dispatch arm after `CiCmd::Queue`; **mandatory** freshness exemption in `should_enforce_freshness` + test)
- Modify: `crates/vox-cli/src/commands/ci/pre_push.rs` (`pub fn run`: print live status) <!-- AMENDED: #13 — moved from Task 9 so print_live_for_push is not dead code at this commit -->

<!-- AMENDED: #C3/#C4 — dropped --json and --refresh (cache file already is the JSON; background refresh = plain `vox ci status` with stdout discarded) -->

**Interfaces:**
- Consumes: `vox_cli_ci::job_timings::run_seconds(Option<&str>, Option<&str>) -> Option<i64>`, `vox_cli_ci::constants::REPO_SLUG`, `vox_config::paths::dot_vox_user_dir() -> PathBuf`.
- Produces (used by Task 9):
  - `vox ci status` — live fetch, write cache, print block (or an all-clear line).
  - `vox ci status --hook` — print cached block (spawn background refresh if stale); never blocks, never fails; ignores stdin.
  - `vox ci status --changed-only` — reads Claude hook JSON on stdin (skipped when stdin is a terminal); prints only if the block changed since this session last saw it (first call in a session prints the current block if non-empty).
  - `pub(crate) fn print_live_for_push()`.

- [ ] **Step 1: Write the failing tests**

Create `crates/vox-cli/src/commands/ci/status.rs` with only this test module, and add `mod status;` to `mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn step(name: &str, status: &str, conclusion: Option<&str>, start: &str, end: Option<&str>) -> Step {
        Step {
            name: name.into(),
            status: status.into(),
            conclusion: conclusion.map(Into::into),
            started_at: Some(start.into()),
            completed_at: end.map(Into::into),
        }
    }

    #[test]
    fn classify_distinguishes_timeout_failure_and_supersede() {
        let timeout = vec!["The job running on runner X has exceeded the maximum execution time of 30 minutes.".to_string()];
        assert_eq!(classify(Some("cancelled"), &timeout), Some(ProblemKind::TimedOut));
        assert_eq!(classify(Some("failure"), &timeout), Some(ProblemKind::TimedOut));
        assert_eq!(classify(Some("timed_out"), &[]), Some(ProblemKind::TimedOut));
        assert_eq!(classify(Some("failure"), &[]), Some(ProblemKind::Failed));
        // A concurrency cancel (newer push) is not a problem.
        assert_eq!(classify(Some("cancelled"), &[]), None);
        assert_eq!(classify(Some("success"), &[]), None);
        assert_eq!(classify(None, &[]), None);
    }

    #[test]
    fn job_problem_names_running_step_and_slowest_three() {
        let job = Job {
            id: 42,
            name: "Check, Build, and Test (Rust)".into(),
            conclusion: Some("cancelled".into()),
            html_url: Some("https://github.com/x/y/actions/runs/1/job/42".into()),
            steps: vec![
                step("Checkout", "completed", Some("success"), "2026-09-21T10:00:00Z", Some("2026-09-21T10:00:10Z")),
                step("Build vox CLI", "completed", Some("success"), "2026-09-21T10:00:10Z", Some("2026-09-21T10:12:10Z")),
                step("Clippy (affected)", "completed", Some("success"), "2026-09-21T10:12:10Z", Some("2026-09-21T10:17:10Z")),
                step("Tests (affected)", "completed", Some("cancelled"), "2026-09-21T10:17:10Z", Some("2026-09-21T10:30:00Z")),
            ],
        };
        let p = job_problem("CI", &job, ProblemKind::TimedOut);
        assert_eq!(p.step.as_deref(), Some("Tests (affected)"));
        assert_eq!(p.job_id, 42);
        let names: Vec<&str> = p.slowest.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["Tests (affected)", "Build vox CLI", "Clippy (affected)"]);
        assert_eq!(p.slowest[1].1, 720);
    }

    #[test]
    fn job_problem_prefers_the_failed_step() {
        let job = Job {
            id: 7,
            name: "gate".into(),
            conclusion: Some("failure".into()),
            html_url: None,
            steps: vec![
                step("Clippy (affected)", "completed", Some("failure"), "2026-09-21T10:00:00Z", Some("2026-09-21T10:01:00Z")),
                step("Tests (affected)", "completed", Some("skipped"), "2026-09-21T10:01:00Z", Some("2026-09-21T10:01:00Z")),
            ],
        };
        let p = job_problem("CI", &job, ProblemKind::Failed);
        assert_eq!(p.step.as_deref(), Some("Clippy (affected)"));
        assert_eq!(p.url, "");
    }

    #[test]
    fn real_gh_json_shapes_parse() {
        let jobs: JobsResponse = serde_json::from_str(
            r#"{"total_count":1,"jobs":[{"id":1,"name":"g","conclusion":null,"html_url":null,
                "steps":[{"name":"s","status":"in_progress","conclusion":null,"number":1,
                "started_at":"2026-09-21T10:00:00Z","completed_at":null}]}]}"#,
        )
        .unwrap();
        assert_eq!(jobs.jobs[0].steps.len(), 1);
        let runs: Vec<RunRow> = serde_json::from_str(
            r#"[{"databaseId":5,"workflowName":"CI","status":"in_progress","conclusion":"","headSha":"abc"}]"#,
        )
        .unwrap();
        assert_eq!(runs[0].database_id, 5);
        let issues: Vec<NightlyIssue> =
            serde_json::from_str(r#"[{"number":9,"title":"Nightly failing: Nightly","url":"https://i"}]"#).unwrap();
        assert_eq!(issues[0].number, 9);
    }

    #[test]
    fn render_is_empty_when_all_clear_and_explains_timeouts() {
        let clear = CiStatus { branch: "b".into(), ..Default::default() };
        assert_eq!(render(&clear), "");

        let s = CiStatus {
            branch: "feat/x".into(),
            problems: vec![JobProblem {
                workflow: "CI".into(),
                job: "gate".into(),
                job_id: 42,
                kind: ProblemKind::TimedOut,
                step: Some("Tests (affected)".into()),
                slowest: vec![("Tests (affected)".into(), 773), ("Build vox CLI".into(), 720)],
                url: "https://u".into(),
            }],
            nightly: vec![NightlyIssue { number: 9, title: "Nightly failing: Nightly".into(), url: "https://i".into() }],
            ..Default::default()
        };
        let r = render(&s);
        assert!(r.contains("TIMED OUT on feat/x: CI / gate at step `Tests (affected)`"), "{r}");
        assert!(r.contains("Tests (affected) 12m53s, Build vox CLI 12m0s"), "{r}");
        assert!(r.contains("move the job to nightly"), "{r}");
        assert!(r.contains("NIGHTLY FAILING: Nightly failing: Nightly (#9) -> https://i"), "{r}");
        assert!(r.lines().next().unwrap().starts_with("GitHub CI"), "{r}");
    }

    #[test]
    fn timeout_without_step_timings_says_unknown() {
        let s = CiStatus {
            branch: "b".into(),
            problems: vec![JobProblem {
                workflow: "CI".into(),
                job: "gate".into(),
                job_id: 1,
                kind: ProblemKind::TimedOut,
                step: None,
                slowest: vec![],
                url: String::new(),
            }],
            ..Default::default()
        };
        assert!(render(&s).contains("slowest steps: unknown"), "{}", render(&s));
    }

    #[test]
    fn failed_job_render_points_at_the_log_command() {
        let s = CiStatus {
            branch: "b".into(),
            problems: vec![JobProblem {
                workflow: "CI".into(),
                job: "gate".into(),
                job_id: 7,
                kind: ProblemKind::Failed,
                step: Some("Clippy (affected)".into()),
                slowest: vec![],
                url: "https://u".into(),
            }],
            ..Default::default()
        };
        assert!(render(&s).contains("gh run view --job 7 --log-failed"), "{}", render(&s));
    }

    #[test]
    fn change_message_only_on_change_and_announces_recovery() {
        assert_eq!(change_message(None, ""), None);
        assert_eq!(change_message(Some("X"), "X"), None);
        assert_eq!(change_message(None, "X").as_deref(), Some("X"));
        assert_eq!(change_message(Some("X"), "Y").as_deref(), Some("Y"));
        assert!(change_message(Some("X"), "").unwrap().contains("resolved"));
    }

    #[test]
    fn session_id_is_read_from_hook_json_and_sanitized() {
        assert_eq!(session_id(r#"{"session_id":"abc-123","prompt":"hi"}"#), "abc-123");
        assert_eq!(session_id(r#"{"session_id":"../../etc"}"#), "etc");
        assert_eq!(session_id("not json"), "default");
        assert_eq!(sanitize("feat/x y"), "feat_x_y");
    }
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cargo test -p vox-cli --lib commands::ci::status`
Expected: FAIL to compile — `cannot find type Step` / `classify` etc.

- [ ] **Step 3: Implement the module**

Prepend to `status.rs`:

```rust
//! `vox ci status` — GitHub CI state pushed into agent context by hooks
//! (git pre-commit/pre-push via lefthook; Claude Code SessionStart and
//! UserPromptSubmit), so no agent has to remember to ask. Two signals:
//!
//! 1. failed or timed-out jobs on the current branch's latest head commit,
//!    with the failing step (or the step running when the timeout hit) and the
//!    three slowest steps — enough to cache, shard, or move a job to nightly;
//! 2. open `nightly-failure` issues (opened by `.github/workflows/nightly-report.yml`).
//!
//! Hook modes read a per-branch cache and refresh it in a detached background
//! process, so they never block and never fail the hook.
//! Spec: docs/superpowers/specs/2026-09-21-hosted-primary-ci-design.md

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use vox_cli_ci::constants::REPO_SLUG;
use vox_cli_ci::job_timings::run_seconds;

const CACHE_FRESH_SECS: i64 = 120;
/// Bound on the live fetch in pre-push so an offline push never stalls.
const PUSH_FETCH_TIMEOUT: Duration = Duration::from_secs(10);
/// GitHub's annotation text on a job killed by `timeout-minutes`.
const TIMEOUT_MARKER: &str = "exceeded the maximum execution time";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProblemKind {
    Failed,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobProblem {
    pub workflow: String,
    pub job: String,
    pub job_id: u64,
    pub kind: ProblemKind,
    /// The failed step, or the step still running when the timeout hit.
    pub step: Option<String>,
    /// Up to three `(step, seconds)`, slowest first.
    pub slowest: Vec<(String, i64)>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NightlyIssue {
    pub number: u64,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CiStatus {
    pub generated_at: i64,
    pub branch: String,
    pub head_sha: Option<String>,
    pub problems: Vec<JobProblem>,
    pub nightly: Vec<NightlyIssue>,
}

#[derive(Debug, Deserialize)]
struct JobsResponse {
    jobs: Vec<Job>,
}

#[derive(Debug, Deserialize)]
struct Job {
    id: u64,
    name: String,
    #[serde(default)]
    conclusion: Option<String>,
    /// The API schema allows `null`.
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
struct Step {
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunRow {
    database_id: u64,
    workflow_name: String,
    status: String,
    /// `""` while a run is in progress.
    conclusion: Option<String>,
    head_sha: String,
}

/// `cancelled` without a timeout annotation is a concurrency cancel (a newer
/// push superseded the run) — not a problem worth an agent's attention.
fn classify(conclusion: Option<&str>, annotations: &[String]) -> Option<ProblemKind> {
    let timed_out = annotations.iter().any(|a| a.contains(TIMEOUT_MARKER));
    match conclusion {
        Some("timed_out") => Some(ProblemKind::TimedOut),
        Some("failure" | "cancelled") if timed_out => Some(ProblemKind::TimedOut),
        Some("failure" | "startup_failure") => Some(ProblemKind::Failed),
        _ => None,
    }
}

fn job_problem(workflow: &str, job: &Job, kind: ProblemKind) -> JobProblem {
    let step = job
        .steps
        .iter()
        .find(|s| s.conclusion.as_deref() == Some("failure"))
        .or_else(|| {
            job.steps
                .iter()
                .find(|s| s.status == "in_progress" || s.conclusion.as_deref() == Some("cancelled"))
        })
        .map(|s| s.name.clone());
    let mut slowest: Vec<(String, i64)> = job
        .steps
        .iter()
        .filter_map(|s| {
            let secs = run_seconds(s.started_at.as_deref(), s.completed_at.as_deref())?;
            Some((s.name.clone(), secs))
        })
        .collect();
    slowest.sort_by_key(|(_, secs)| std::cmp::Reverse(*secs));
    slowest.truncate(3);
    JobProblem {
        workflow: workflow.into(),
        job: job.name.clone(),
        job_id: job.id,
        kind,
        step,
        slowest,
        url: job.html_url.clone().unwrap_or_default(),
    }
}

/// Empty string when there is nothing to report (hooks then print nothing).
fn render(s: &CiStatus) -> String {
    let mut out = Vec::new();
    for p in &s.problems {
        let what = match p.kind {
            ProblemKind::TimedOut => "TIMED OUT",
            ProblemKind::Failed => "FAILED",
        };
        out.push(format!(
            "{what} on {}: {} / {} at step `{}` -> {}",
            s.branch,
            p.workflow,
            p.job,
            p.step.as_deref().unwrap_or("?"),
            p.url
        ));
        match p.kind {
            ProblemKind::TimedOut => {
                let slow: Vec<String> = p
                    .slowest
                    .iter()
                    .map(|(n, secs)| format!("{n} {}m{}s", secs / 60, secs % 60))
                    .collect();
                let slow = if slow.is_empty() { "unknown".to_string() } else { slow.join(", ") };
                out.push(format!(
                    "  slowest steps: {slow}. Fix: cache or shard the slow step, or move the job to \
                     nightly.yml — do not raise the cap (workflow-policy-guard)."
                ));
            }
            ProblemKind::Failed => {
                out.push(format!("  log: gh run view --job {} --log-failed", p.job_id));
            }
        }
    }
    for i in &s.nightly {
        out.push(format!("NIGHTLY FAILING: {} (#{}) -> {}", i.title, i.number, i.url));
    }
    if out.is_empty() {
        return String::new();
    }
    out.insert(0, "GitHub CI (auto-injected by vox hooks):".into());
    out.join("\n")
}

/// What the per-prompt hook prints, given what this session last saw.
fn change_message(last_shown: Option<&str>, current: &str) -> Option<String> {
    let prev = last_shown.unwrap_or("");
    if prev == current {
        None
    } else if current.is_empty() {
        Some("GitHub CI: previously reported problems are resolved.".into())
    } else {
        Some(current.to_string())
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

/// Claude Code hooks receive JSON on stdin with a `session_id`.
fn session_id(stdin: &str) -> String {
    serde_json::from_str::<serde_json::Value>(stdin)
        .ok()
        .and_then(|v| v.get("session_id")?.as_str().map(|s| s.to_string()))
        .map(|s| sanitize(s.rsplit('/').next().unwrap_or("")))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default".into())
}

fn cache_dir() -> PathBuf {
    vox_config::paths::dot_vox_user_dir().join("ci-status")
}

fn cache_path(branch: &str) -> PathBuf {
    cache_dir().join(format!("{}.json", sanitize(branch)))
}

fn write_cache(s: &CiStatus) -> Result<()> {
    let path = cache_path(&s.branch);
    std::fs::create_dir_all(cache_dir())?;
    // Temp + rename: parallel sessions refresh concurrently; no torn reads.
    let tmp = path.with_extension(format!("json.{}", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec(s)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

fn read_cache(branch: &str) -> Option<CiStatus> {
    serde_json::from_slice(&std::fs::read(cache_path(branch)).ok()?).ok()
}

fn gh(args: &[&str]) -> Result<String> {
    let out = Command::new("gh")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .context("spawn gh")?;
    if !out.status.success() {
        return Err(anyhow!(
            "gh {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn annotations(job_id: u64) -> Vec<String> {
    gh(&[
        "api",
        &format!("repos/{REPO_SLUG}/check-runs/{job_id}/annotations"),
        "--jq",
        ".[].message",
    ])
    .map(|o| o.lines().map(str::to_string).collect())
    .unwrap_or_default()
}

fn fetch(branch: &str, now: i64) -> Result<CiStatus> {
    let runs: Vec<RunRow> = serde_json::from_str(&gh(&[
        "run", "list", "--repo", REPO_SLUG, "--branch", branch, "--limit", "30", "--json",
        "databaseId,workflowName,status,conclusion,headSha",
    ])?)?;
    let head_sha = runs.first().map(|r| r.head_sha.clone());
    let mut problems = Vec::new();
    for r in runs
        .iter()
        .filter(|r| Some(&r.head_sha) == head_sha.as_ref() && r.status == "completed")
    {
        if matches!(r.conclusion.as_deref(), Some("success" | "skipped" | "neutral")) {
            continue;
        }
        // One unreadable run must not hide the others.
        let Ok(text) = gh(&[
            "api",
            &format!("repos/{REPO_SLUG}/actions/runs/{}/jobs?per_page=100", r.database_id),
        ]) else {
            continue;
        };
        let Ok(jobs) = serde_json::from_str::<JobsResponse>(&text) else {
            continue;
        };
        for job in &jobs.jobs {
            let ann = match job.conclusion.as_deref() {
                Some("failure" | "cancelled") => annotations(job.id),
                _ => Vec::new(),
            };
            if let Some(kind) = classify(job.conclusion.as_deref(), &ann) {
                problems.push(job_problem(&r.workflow_name, job, kind));
            }
        }
    }
    let nightly: Vec<NightlyIssue> = serde_json::from_str(&gh(&[
        "issue", "list", "--repo", REPO_SLUG, "--label", "nightly-failure", "--state", "open",
        "--json", "number,title,url",
    ])?)?;
    Ok(CiStatus { generated_at: now, branch: branch.into(), head_sha, problems, nightly })
}

fn current_branch() -> Option<String> {
    // vox-arch-check: allow git-exec
    let out = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Detached `vox ci status` (stdout discarded). A marker file suppresses a
/// stampede of refreshes from parallel sessions while one is in flight.
// ponytail: on Windows the child holds target/debug/vox.exe open for a few
// seconds; add creation_flags(DETACHED_PROCESS) if that blocks rebuilds.
fn spawn_refresh(branch: &str) {
    let marker = cache_dir().join(format!("refreshing-{}", sanitize(branch)));
    let in_flight = std::fs::metadata(&marker)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age.as_secs() < CACHE_FRESH_SECS as u64);
    if in_flight {
        return;
    }
    let _ = std::fs::create_dir_all(cache_dir());
    let _ = std::fs::write(&marker, b"");
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe)
            .args(["ci", "status"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

pub struct StatusArgs {
    pub hook: bool,
    pub changed_only: bool,
}

pub fn run(args: StatusArgs) -> Result<()> {
    let branch = current_branch().unwrap_or_else(|| "HEAD".into());
    let now = chrono::Utc::now().timestamp();
    if !(args.hook || args.changed_only) {
        let s = fetch(&branch, now)?;
        write_cache(&s)?;
        let _ = std::fs::remove_file(cache_dir().join(format!("refreshing-{}", sanitize(&branch))));
        let r = render(&s);
        if r.is_empty() {
            println!("GitHub CI: nothing failing on {branch}; no open nightly failures.");
        } else {
            println!("{r}");
        }
        return Ok(());
    }
    // Hook modes: never block, never fail the hook.
    let cached = read_cache(&branch);
    if cached.as_ref().is_none_or(|c| now - c.generated_at > CACHE_FRESH_SECS) {
        spawn_refresh(&branch);
    }
    let current = cached.as_ref().map(render).unwrap_or_default();
    if args.changed_only {
        let mut stdin = String::new();
        if !std::io::stdin().is_terminal() {
            let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut stdin);
        }
        let shown = cache_dir().join(format!("shown-{}.txt", session_id(&stdin)));
        if let Some(msg) = change_message(std::fs::read_to_string(&shown).ok().as_deref(), &current) {
            println!("{msg}");
        }
        let _ = std::fs::create_dir_all(cache_dir());
        let _ = std::fs::write(&shown, &current);
    } else if !current.is_empty() {
        println!("{current}");
    }
    Ok(())
}

/// Live fetch for `vox ci pre-push`, bounded by [`PUSH_FETCH_TIMEOUT`]; falls
/// back to the cached block. Silent inside GitHub Actions.
pub(crate) fn print_live_for_push() {
    if std::env::var_os("GITHUB_ACTIONS").is_some() {
        return;
    }
    let Some(branch) = current_branch() else {
        return;
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let b = branch.clone();
    std::thread::spawn(move || {
        let _ = tx.send(fetch(&b, chrono::Utc::now().timestamp()));
    });
    let s = match rx.recv_timeout(PUSH_FETCH_TIMEOUT) {
        Ok(Ok(s)) => {
            let _ = write_cache(&s);
            s
        }
        Ok(Err(e)) => {
            println!("pre-push: GitHub CI status unavailable ({e}); showing cached");
            match read_cache(&branch) {
                Some(c) => c,
                None => return,
            }
        }
        Err(_) => match read_cache(&branch) {
            Some(c) => c,
            None => return,
        },
    };
    let r = render(&s);
    if !r.is_empty() {
        println!("{r}");
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p vox-cli --lib commands::ci::status`
Expected: 9 PASS.

- [ ] **Step 5: Add the CLI variant, dispatch, and freshness exemption**

In `crates/vox-cli-ci/src/cmd_enums.rs`, directly after the `Queue { … }` variant:
```rust
    /// GitHub CI state for the current branch (failed/timed-out jobs, slowest
    /// steps) plus open `nightly-failure` issues. Hooks call `--hook` /
    /// `--changed-only`; agents never need to run it by hand.
    #[command(name = "status")]
    Status {
        /// Hook mode: print the cached block (refresh in background if stale); never fails.
        #[arg(long)]
        hook: bool,
        /// Per-session hook mode: read hook JSON on stdin; print only if changed for this session.
        #[arg(long)]
        changed_only: bool,
    },
```
In `crates/vox-cli/src/commands/ci/run_body.rs`, directly after the `CiCmd::Queue { … } => …` arm:
```rust
        CiCmd::Status { hook, changed_only } => {
            super::status::run(super::status::StatusArgs { hook, changed_only })
        }
```
In `should_enforce_freshness` (same file), add `| CiCmd::Status { .. }` to the pattern that lists `CiCmd::RunnerScale { .. } | CiCmd::RunnerPreflight` — **required**: without it a stale installed `vox` refuses and `|| true` hides it forever. <!-- AMENDED: #13-freshness — was "If…", now mandatory -->
In that file's existing test that asserts `!should_enforce_freshness(&CiCmd::RunnerPreflight)`, add:
```rust
        assert!(!should_enforce_freshness(&CiCmd::Status {
            hook: true,
            changed_only: false
        }));
```

- [ ] **Step 6: Pre-push prints live status**

In `crates/vox-cli/src/commands/ci/pre_push.rs` `pub fn run`, directly after the existing `if opts.dry_run { … }` handling returns (i.e. only on real runs; dry-run tests must not call `gh`), before the steps execute: <!-- AMENDED: #27 — no live gh calls in --dry-run tests -->
```rust
    // Surface GitHub CI failures/timeouts and open nightly-failure issues to
    // whoever is pushing — agents see this in the push output without asking.
    super::status::print_live_for_push();
```
Read `pub fn run` first to find the exact point after the dry-run branch.

- [ ] **Step 7: Regenerate and smoke-test live**

```bash
cargo run -p vox-cli -- ci command-sync
cargo run -p vox-cli -- ci gui-surface-coverage --write
cargo run -p vox-cli -- ci doc-inventory generate
```
Run: `cargo run -q -p vox-cli -- ci status`
Expected: `GitHub CI: nothing failing on <branch>…` or a block; exit 0 with `gh` authenticated.
Run: `echo '{"session_id":"t1"}' | cargo run -q -p vox-cli -- ci status --changed-only; echo '{"session_id":"t1"}' | cargo run -q -p vox-cli -- ci status --changed-only`
Expected: second invocation prints nothing (the live run above warmed the cache, so no background refresh races).
Run: `cargo clippy -p vox-cli -p vox-cli-ci --all-targets --locked -- -D warnings && cargo test -p vox-cli --lib commands::ci && cargo run -q -p vox-cli -- ci ssot-drift`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
vox run scripts/fmt.vox
git add -A crates docs contracts
git commit -m "feat(ci): vox ci status — CI failures, timeouts, nightly issues for hooks"
```

---

### Task 9: Deliver the block automatically (git hooks + Claude hooks)

**Files:**
- Modify: `lefthook.yml` (pre-commit commands)
- Modify: `.claude/settings.json` (**ask the user first** — persistent harness config)
- Modify: `AGENTS.md` (replace §Local-First CI Verification Contract)

**Interfaces:**
- Consumes: `vox ci status --hook`, `vox ci status --changed-only` from Task 8.

- [ ] **Step 1: Pre-commit prints the cached block**

In `lefthook.yml` under `pre-commit: commands:`, add (last entry). Uses `vox` from PATH so docs-only commits don't compile vox-cli:
```yaml
    ci-status:
      # Never fails the commit; prints GitHub CI problems + open nightly failures.
      run: vox ci status --hook || true
```

- [ ] **Step 2: Claude Code hooks (after user confirmation)**

Replace `.claude/settings.json` with the following. It drops the PreToolUse remote-watch block (hosted runs are capped at 30 min, so watching one is fine) and uses `--changed-only` for SessionStart too, so the first prompt doesn't repeat the session-start block: <!-- AMENDED: #B12 — duplicate block at session start -->
```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "vox ci status --changed-only 2>/dev/null || true"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "vox ci status --changed-only 2>/dev/null || true"
          }
        ]
      }
    ]
  }
}
```

- [ ] **Step 3: Update the AGENTS.md contract**

Replace the whole `## Local-First CI Verification Contract (Required, SSOT)` section with:
```markdown
## CI Contract (Required, SSOT)

- **GitHub-hosted CI is the gate.** `ci.yml` (required context
  `Check, Build, and Test (Rust)`) runs the local fast tier plus clippy/nextest
  on affected crates; jobs are capped at 30 min. `nightly.yml` and other
  scheduled workflows run the slow lanes, capped at 180 min. Caps are enforced
  by `workflow-policy-guard` (in `ssot-drift`). Over budget? Cache, shard, or
  move the job to nightly — never raise the cap.
- **Run CI locally first:** `vox ci pre-push` (fast), `--complete`/`--full`
  for code changes, or run the PR gate in Docker with `act pull_request -j gate`.
- **CI state comes to you.** Failed/timed-out jobs on your branch and open
  `nightly-failure` issues are printed by the git pre-commit/pre-push hooks
  and injected by Claude Code hooks. When you see a block, fix it before
  continuing. `vox ci status` prints the same block on demand.

Spec: `docs/superpowers/specs/2026-09-21-hosted-primary-ci-design.md`.
```
Also in `## Local CI Gate Tiers (SSOT)`, delete the remaining fleet text (the `vox ci pre-push --act` bullet's claim that it reproduces "the actual GitHub jobs", and the bullet about "a red GitHub check whose local equivalent passes").

- [ ] **Step 4: Verify each surface**

The installed `vox` lacks `ci status` until rebuilt — **ask the user** before replacing it (e.g. `cargo install --path crates/vox-cli --locked`, or however they install it). Until then verify via cargo:
Run: `cargo run -q -p vox-cli -- ci status && cargo run -q -p vox-cli -- ci status --hook`
Expected: the `--hook` output equals the block from the first command (or both are all-clear/empty).
Run (after the user reinstalls): `git commit --allow-empty -m "test: hook output"` then `git reset --soft HEAD~1`
Expected: lefthook output includes `ci-status`; commit succeeds.
Run: `cargo run -q -p vox-cli -- ci pre-push`
Expected: status block (if any) printed before the first gate step.

- [ ] **Step 5: Commit**

```bash
git add lefthook.yml .claude/settings.json AGENTS.md
git commit -m "ci: push CI status into every agent via git and Claude hooks"
```

---

### Task 10 (separate PR, after 1–9 merge): Delete the self-hosted fleet machinery

**Files:**
- Delete: `crates/vox-cli/src/commands/ci/queue.rs`, `runner_scale.rs`, `oom_watch.rs`, `unexpected_exit_watch.rs`
- Modify: `crates/vox-cli/src/commands/ci/mod.rs`, `run_body.rs` (keep `CiCmd::Status` in `should_enforce_freshness`), `providers.rs`
- Modify: `crates/vox-cli-ci/src/cmd_enums.rs` (remove `Queue`, `RunnerScale`, `RunnerPreflight`, `RunnerStatus`, and the OOM/unexpected-exit watch variants)
- Modify: `crates/vox-cli-ci/src/constants.rs` — remove `"docs/src/ci/local-first-ci.md",` from `DOCS_SSOT_FILES` <!-- AMENDED: #5 -->
- Delete: `infra/ci-runner/`, `docs/src/ci/runner-autoscaling.md`, `docs/src/ci/local-first-ci.md`
- Modify: `docs/src/ci/runner-contract.md` (reduce to: hosted runners, caps, nightly issues, `act`), `docs/src/ci/compute-placement.md` (drop fleet invariants), any other grep hit (e.g. `crates/vox-gui/src/commands/harness_town.rs`)
- Regenerate: command-sync artifacts, `gui-surface-coverage.v1.json`, `doc-inventory.json`

- [ ] **Step 1: Find the surface**

Run: `git grep -n 'runner_scale\|queue::\|oom_watch\|unexpected_exit_watch\|RunnerScale\|RunnerPreflight\|RunnerStatus\|CiCmd::Queue\|ci queue\|ci-queue-snapshot\|local-first-ci\|runner-autoscaling' -- ':!docs/src/archive' ':!docs/superpowers' ':!graphify-out' ':!contracts/reports' ':!*.generated.md'`
Expected: a finite list; every hit is removed or reworded in this task.

- [ ] **Step 2: Delete modules and variants; fix what breaks** (edits under **Files**).

- [ ] **Step 3: Verify**

```bash
cargo run -p vox-cli -- ci command-sync
cargo run -p vox-cli -- ci gui-surface-coverage --write
cargo run -p vox-cli -- ci doc-inventory generate
```
Run: `cargo clippy -p vox-cli -p vox-cli-ci -p vox-gui --all-targets --locked -- -D warnings` (vox-gui only if its sidecar/dist exist in this worktree; otherwise `cargo check -p vox-gui` is skipped and noted)
Expected: PASS.
Run: `cargo test -p vox-cli --lib commands::ci && cargo test -p vox-cli-ci && cargo run -q -p vox-cli -- ci ssot-drift && cargo run -q -p vox-cli -- ci check-links`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add -A crates infra docs contracts
git commit -m "ci: delete self-hosted fleet machinery (queue, autoscaler, watchers)"
```

---

## Notes for the executor

- **Timeout detection is unverified against a live timed-out job.** `classify` accepts either conclusion `timed_out` or GitHub's annotation text `exceeded the maximum execution time`. The first real timeout after Task 4 validates it: read `~/.vox/ci-status/<branch>.json` after `vox ci status` and confirm `"kind": "timed_out"`. If it reports nothing, dump `gh api repos/vox-foundation/vox/check-runs/<job id>/annotations` and adjust `TIMEOUT_MARKER`.
- **Cold cache.** PR gates only restore the cache; `nightly.yml`'s `full` job on `main` is the only writer. Until the first nightly completes after merge, PR gates compile cold and may time out — expected once; the status block will say so.
- **`act` limits.** Linux jobs only; macOS/Windows jobs (GUI Windows smoke, cross-platform) need GitHub.

## Execution Order

Sequential Constraints (Shared-File Collisions — CANNOT parallelize):
- Task 3 → Task 7: both modify `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs` (`run_ssot_drift`) and `crates/vox-cli-ci/src/lib.rs`
- Task 3 → Task 8 → Task 10: all modify `crates/vox-cli-ci/src/cmd_enums.rs`, `crates/vox-cli/src/commands/ci/run_body.rs`, and regenerate `cli-command-surface.generated.md`, `command_catalog_paths_baseline.txt`, `gui-surface-coverage.v1.json`, `doc-inventory.json`
- Task 3 → Task 8: both modify `crates/vox-cli/src/commands/ci/pre_push.rs`
- Task 4 → Task 8 → Task 10: all modify `crates/vox-cli/src/commands/ci/mod.rs`
- Task 4 → Task 5 → Task 6 → Task 7: all modify `.github/workflows/nightly.yml` / scheduled workflows
- Task 4 → Task 5 → Task 7: all may modify `crates/vox-cli/tests/ci_workflow_contract.rs`
- Task 3 → Task 5 → Task 10: all modify `docs/src/ci/runner-contract.md`
- Task 3 → Task 10: both modify `crates/vox-cli-ci/src/constants.rs`
- Task 3 → Task 9: both modify `AGENTS.md`
- Task 9 → Task 10: `.claude/settings.json` must stop calling `vox ci queue` before `queue.rs` is deleted

Pre-Flight Checklist:
- [x] Worktree isolated (`.claude/worktrees/cicd-complexity-tradeoffs-b62a67`)
- [ ] Target git history confirmed: branch `claude/cicd-complexity-tradeoffs-b62a67` from `main` @ `877406d84` (`git log -1 main`)
- [x] Next migration sequence — N/A (no DB schema)
- [x] Local test database — N/A
- [ ] `gh auth status` OK (Tasks 2, 4 Step 9, 8 Step 7)
- [ ] Docker + `act` available (Tasks 4, 5, 6)

Recommended Task Sequence:
Task 1 → Task 2 (user-gated, no files; can run any time) → Task 3 → Task 4 → Task 5 → Task 6 → Task 7 → Task 8 → Task 9 → [merge] → Task 10 (new PR).
Batch candidates: Task 1 and Task 2 are independent of each other and of everything else.

SDD Ledger Pre-Population (copy into progress.md):
- Conflict on `run_body_helpers/docs.rs`, `cmd_enums.rs`, `run_body.rs`, `pre_push.rs`, `mod.rs`, `nightly.yml`, `ci_workflow_contract.rs`, `AGENTS.md`, generated artifacts: sequential execution enforced — ruling: settled
- #1: PR gate strips `-p vox-gui` from affected args — ruling: settled
- #2: `ci_workflow_contract.rs` tests are retargeted to `nightly.yml` or deleted if fleet-only; new gate test added — ruling: settled
- #4: `merge_group_fanout_guard.rs` is deleted in Task 4 (fleet-only) — ruling: settled
- #5: `DOCS_SSOT_FILES` entries removed alongside the doc deletions (Task 3, Task 10) — ruling: settled
- #8: every CLI change regenerates command-sync + gui-surface-coverage + doc-inventory — ruling: settled
- #10: nightly `setup` treats all non-PR events as full; four push-main-gated jobs also run on schedule/dispatch — ruling: settled
- #11: toolchain-lint-wave not re-homed; nightly `full` (cold after a toolchain bump via rust-cache key) + rustdoc covers it — ruling: settled
- #14: no repo-wide `gh run cancel`; only the two GPU workflows are disabled — ruling: settled
- #C1: one `nightly-report.yml` (workflow_run) replaces per-workflow report jobs; guard checks the name list — ruling: settled
- #C2: guard runs only via `ssot-drift` (no separate pre-push step) — ruling: settled
- #C3/#C4: `vox ci status` has no `--json`/`--refresh` — ruling: settled
- Timeout caps 30/180 are fixed; never raise a cap to make a job fit — ruling: settled

## Deferred Minor Issues

- `nightly.yml` `full` job repeats clippy/nextest/deny/audit that the parked old `lints`/`tests`/`guards-fast` jobs also run (`ci.yml:760`, `:1133`, `:491-497` pre-rename). After the first green nightly, delete the duplicate steps from whichever job is slower (saves ~1 workspace compile per night).
- `shown-<session>.txt` and `refreshing-*` files under `~/.vox/ci-status/` are never pruned; add a prune of files older than 7 days to `run()` if the directory grows.
- Windows: the detached refresh child briefly holds `target/debug/vox.exe` (see `ponytail:` comment in `spawn_refresh`).
- Consider having `vox doctor` warn when the installed `vox` lacks `ci status` (hooks are silent until reinstall).
- Optional: move `status.rs` into `vox-cli-ci` next to `job_timings.rs` and reuse its paginated `gh_json` (~−15 lines).
- `workflow_concurrency_guard.rs` comment says serde_yaml is YAML 1.1; the workspace uses `serde_yaml_ng` 0.10 (YAML 1.2). Harmless dead `Bool(true)` branch.
- The guard doesn't verify `nightly-report.yml`'s job condition / conclusion handling; a human edit could break reporting silently.
