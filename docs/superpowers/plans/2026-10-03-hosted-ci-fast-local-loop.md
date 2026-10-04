# Hosted CI Gate and a Fast Local Loop — Implementation Plan

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI**, one task per headless
> run (`node .agents/scripts/drive.mjs docs/superpowers/plans/2026-10-03-hosted-ci-fast-local-loop.md <N>`), per
> [`antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md). Tasks marked **(Claude)**
> or **(Owner)** are not driven. The agent never stages or commits; Claude re-runs every check, reviews the diff and
> commits. Workflow changes are verified on a draft PR, never by pushing to `main`.

**Goal:** Hosted CI is the only gate and it is fast and green; the laptop runs only `cargo check` and the tests of the
crate being edited; no git hook builds anything.

**Evidence:** [`ci-and-build-loop-findings-2026.md`](../../src/architecture/ci-and-build-loop-findings-2026.md). Every
task below cites the finding it acts on.

**Targets (measured on hosted runs, recorded in the findings doc when met):**

| Metric | Today | Target |
|---|---|---|
| `git commit` hook time | minutes (cold `cargo run`) | under 5 s, no cargo |
| `git push` hook time | minutes to hours | under 30 s, no cargo |
| PR CI, small affected set | 4 to 6 min compile per shard × 3 | under 8 min total, one shard |
| PR CI, full workspace | 22 to 30 min, 10/24 shards hit the cap | under 18 min, none at the cap |
| Nightly | red every day since 2026-09-28 | green 3 nights running |
| Actions cache | 14.48 GB | under 9 GB |

## Global constraints

- **Nothing here lowers coverage.** A test moved off the PR gate runs nightly; a test deleted is either dead (expired
  sunset, retired behaviour) or a duplicate, and the commit body says which.
- **Never add a test to the #569 quarantine** to make a PR green (`AGENTS.md`, CI Contract). Moving a test to the
  nightly tier is a named, reviewed tier change in `.config/nextest.toml`, not a quarantine entry.
- **User-authorized only (STOP and ask):** crate-edge exceptions, the fan-in snapshot, `layers.toml` budgets, the
  transport-crypto ledger (any rustls/ring/aws-lc/h2/hyper bump in T0.3), raising any workflow time cap, deleting
  Actions caches (T2.4), pushing or opening PRs (T0.1 on the first run), and the macOS Developer Tools setting.
- Workflow caps stay: 30 min per PR job, 180 min per scheduled job (`workflow-policy-guard`). Over budget means shard,
  cache or move to nightly, never raise the cap.
- Rust caches save from `main` only (`vox ci cache-key-lint`). The cache-seed job in T2.1 obeys this.
- Test-first for every behaviour change: the failing test or the failing CI check is captured before the fix.
- Speed rules for agents: no `cargo clippy --workspace`, no `cargo test --workspace` locally. Run the named crate's
  tests only; hosted CI runs the rest.

## Phase 0 — Unblock (do first; everything else is measured against a green baseline)

### Task 0.1 (Claude + Owner): Land local `main` through a PR

**Acts on:** nightly `--locked` failure; 57 unpushed commits.

- [ ] **Step 1:** `git rev-list --count origin/main..main` and `cargo metadata --locked --offline --format-version 1 >/dev/null`
  (must exit 0; local `main` carries the one-line `Cargo.lock` fix `origin/main` lacks).
- [ ] **Step 2 (Owner):** approve pushing. Then push a branch from `main` and open a **draft** PR:
  `git push -u origin main:land/2026-10-03-main && gh pr create --draft --base main --head land/2026-10-03-main`.
- [ ] **Step 3:** read its CI. Failures that predate this work go to Task 0.2; anything this branch caused is fixed on
  the branch. Mark ready and merge through the queue.
- [ ] **Done when:** the next scheduled nightly's `setup` job passes (`gh run list --workflow nightly.yml -L 1`).

### Task 0.2: Fix the all-features compile errors

**Files:** the crates the nightly log names: `crates/vox-actor-runtime` (E0599 `DbConfig::resolve_canonical`),
`crates/vox-codegen` (E0432, E0277), `crates/vox-cli` (E0275 recursion through netlink `Tcf: Send`), and the
`objc2` target gating in `crates/vox-ml-cli` and `crates/vox-populi`.

- [ ] **Step 1:** reproduce each with the exact nightly command, scoped: `cargo check -p <crate> --all-features`
  (on Linux-only errors, read the nightly log instead of reproducing on macOS). Save output to `target/ci-t02-<crate>.txt`.
- [ ] **Step 2:** fix at the root: a missing `cfg(feature)` on an import, a method gated on a feature its caller does
  not enable, `objc2` behind `cfg(target_os = "macos")` in `[target.'cfg(...)'.dependencies]`. E0275 is usually a
  boxed future or an explicit `Send` bound that ends the recursion; name the type in the commit body.
- [ ] **Step 3:** `cargo check -p <crate> --all-features` is clean for each; one commit per crate.

### Task 0.3 (Claude, Owner for crypto): Clear the `cargo audit` advisories

- [ ] **Step 1:** list them from the latest nightly log (7 to 11: wasmtime, h2, a TLS 1.3 issue).
- [ ] **Step 2:** non-transport bumps (wasmtime) go ahead with `cargo update -p <crate> --precise <ver>`.
- [ ] **Step 3: STOP** for h2, rustls, ring, aws-lc or hyper: draft the `contracts/crypto/transport-providers.v1.json`
  ledger entry text in the PR description; the owner applies it.
- [ ] **Done when:** `cargo audit` in nightly reports zero unignored advisories.

### Task 0.4: Make nightly's own jobs pass

- [ ] CDP smoke ("Browser::connect failed: Received no response"): give Chromium `--no-sandbox` / a longer connect
  timeout as the harness intends, or move it behind the browser feature's own job; record which in the commit body.
- [ ] Playwright at its 25-minute cap: run with `--workers=4` sharded `--shard=1/2`, `2/2` (two jobs) instead of raising
  the cap.
- [ ] Windows GUI smoke (93 to 124 min): move to its own scheduled workflow (`gui-cross-build.yml` already exists) so
  it no longer sets nightly's wall time.
- [ ] **Done when:** three consecutive scheduled nightlies are green; close the matching `nightly-failure` issues
  (#574 to #593) with the run link.

## Phase 1 — The local loop stops building

### Task 1.1: Hooks run installed binaries, never `cargo run`

**Acts on:** `lefthook.yml:30-48,58-59,79-80`.
**Files:** `lefthook.yml`, `scripts/install-hooks.vox`, `docs/src/contributors/local-ci-pre-push.md`.

- [ ] **Step 1 (test-first):** add a guard test (in `crates/vox-cli-ci`, next to the existing workflow guards) that parses
  `lefthook.yml` and fails if any command contains `cargo run`, `cargo build`, `cargo clippy` or `cargo test`. Run it;
  save the red output.
- [ ] **Step 2:** replace each `cargo run -p vox-cli -- <args>` with `vox <args>` and the toestub call with the installed
  `toestub` binary. A missing or stale binary prints one line naming the install command and **skips** (exit 0); it never
  builds. Hosted CI re-runs every one of these checks (`ci.yml:53-54`), so a skip cannot let a defect through.
- [ ] **Step 3:** `pre-push` keeps only `fmt --check` on dirty files and the in-process guards that need no build.
  Remove the local clippy step entirely (hosted CI gates clippy).
- [ ] **Step 4:** fix the dead `ci-status` hook: `vox ci status` exists in source but not in older installed binaries;
  the hook calls it with `|| true` and the install docs say how to refresh.
- [ ] **Done when:** the guard test passes; a commit touching a `.rs` file and a push both finish in under 5 s and 30 s
  on a cold `target/` (time them; record in the findings doc).

### Task 1.2: Where the installed `vox` comes from

- [ ] `voxup install --tag nightly` (see `nightly-builds-ssot.md`) installs a CI-built `vox` and `toestub`, so no laptop
  build is needed to get them. Make `nightly-artifacts.yml` publish `toestub` alongside `vox` if it does not.
- [ ] `scripts/install-hooks.vox` checks for both binaries and prints the one `voxup` command when either is missing.
- [ ] Fallback for offline work: `cargo install --locked --path crates/vox-cli` once, documented, never run by a hook.

### Task 1.3: Stop oversubscribing the machine

**Files:** `.cargo/config.toml`, `crates/vox-cargo-shim/src/main.rs`, `docs/src/contributors/build-broker-usage.md`.

- [ ] Delete `[build] jobs = 24` (cargo's default is the core count). Default `VOX_BROKER_MAX_CONCURRENT` to
  `max(1, cores / 6)` (3 on an 18-core machine) and document it.
- [ ] Shim: release the slot when `cargo run`'s build step ends, not when the program exits (test-first: a unit test on
  the shim's subcommand classification). Reap `inflight/` entries whose PID is gone on startup.

### Task 1.4: One `target/` per worktree, no symlinks

- [ ] `vox doctor` warns when `target` is a symlink into another checkout and names the fix (per-worktree targets are
  already the intent of `.cargo/config.toml:71-86`; the symlink habit came from agent sessions).
- [ ] The agy driver kit and `antigravity-driven-execution.md` say: never symlink `target`; a fresh worktree builds only
  the crate it edits.

### Task 1.5 (Owner): macOS Developer Tools exemption

- [ ] System Settings → Privacy & Security → Developer Tools: add and enable **Terminal** (or the terminal in use) **and
  the Claude app**, which is what launches `cargo` for agent runs. Restart those apps.
- [ ] **(Claude) measure:** repeat the Part A loop of `rust-build-loop-ssot-2026.md`; record user vs sys seconds. If sys
  time drops below half of total, note it as the confirmed cause.

## Phase 2 — Hosted CI builds once and warms its own cache

### Task 2.1: Build test binaries once, run them in shards

**Acts on:** shards compiling the same binaries; `save-if: false`.
**Files:** `.github/workflows/ci.yml`, `crates/vox-cli/src/commands/ci/run_body_helpers/*` if the affected logic emits the
nextest arguments.

- [ ] A `build-tests` job runs `cargo nextest archive --archive-file tests.tar.zst <same -p args>` and uploads it.
  The three `tests` shards `needs: build-tests`, download it, and run `cargo nextest run --archive-file tests.tar.zst
  --partition hash:K/3` (no compile). Keep the gate's required context name unchanged.
- [ ] Add a `push: branches: [main]` trigger used **only** by a cache-seed job that builds the workspace test binaries
  and saves the `workspace` rust-cache (main scope, allowed by the cache rule). It is not a gate.
- [ ] **Done when:** on a draft PR, shard jobs show no `Compiling` lines, and the full-workspace run finishes under 18 min.

### Task 2.2: One shard for small affected sets

- [ ] When the affected set is not the full workspace and holds fewer than ~30 crates, emit a single partition (`1/1`);
  shards 2 and 3 skip and count as success in `gate` (the gate already treats an unplanned `tests` as success).

### Task 2.3: Narrow the "contracts forces a full run" rule

**Files:** `crates/vox-cli/src/commands/ci/affected.rs` (lines 5-12, 33-44, 143-151).

- [ ] Test-first: a case where only `contracts/reports/*.v1.json` changes must not force a full run; a case where
  `contracts/ci/crate-edges.allow.v1.json` changes still must.
- [ ] Map contract directories to the crates that read them (grep `include_str!`/path constants); unknown contract paths
  still force a full run (fail safe).

### Task 2.4 (Owner approves deletion): Get the Actions cache under budget

- [ ] Shrink the sccache footprint (7.0 GB, 7,232 entries): give its key a version suffix so a fresh, bounded set
  starts, and set `SCCACHE_CACHE_SIZE` on the four workflows that use it.
- [ ] **STOP** before deleting existing entries; list them by key prefix and size for the owner. Deletion is
  `gh cache delete --all --key <prefix>` once approved.

## Phase 3 — The test suite pays only for what the gate needs

### Task 3.1: Nested-cargo tests run nightly, serialized

**Files:** `.config/nextest.toml`.

- [ ] Define a `nested-cargo` test-group (`max-threads = 1`) and assign by filter: `emit_compile_harness`,
  `emission_ladder_test *_compiles_as_rust_script`, `build_e2e`, trybuild `compile_fail`, `workflow_runtime_no_db`.
- [ ] The `ci-gate` profile excludes the group; the nightly `ci` profile runs it. Assign the defined-but-empty `slow`
  group too, so `nightly.yml:814`'s claim becomes true.
- [ ] Remove the duplicated ladder targets (`crud_api`, `auth_patterns`, `db_native_ir` compiled by two binaries).

### Task 3.2: Merge tiny integration-test binaries

- [ ] Per crate, starting with `vox-cli` (58 files with two tests or fewer), then `vox-integration-tests`, `vox-compiler`,
  `vox-codegen`: move `tests/*.rs` into `tests/it/<name>.rs` with a `tests/it/main.rs` that declares them as modules.
  Test names change only by the `it::<name>::` prefix; nextest filters in `.config/nextest.toml` are updated in the
  same commit.
- [ ] **Done when:** the test count per crate is unchanged (`cargo nextest list -p <crate> | wc -l` before and after).

### Task 3.3: Resolve the quarantine and the expired ignores

- [ ] For each of the 32 #569 entries: fix it, or delete it with the reason (behaviour retired, duplicate). Nightly
  stops being red by construction.
- [ ] The 71 `#[ignore]` with the passed 2026-08-01 sunset and the 12 asserting retired `activity`/`@v0` behaviour:
  un-ignore and fix, or delete. One commit per crate.

### Task 3.4: Nightly does each thing once

- [ ] The plain workspace run in `full` (`nightly.yml:1561`) becomes `cargo nextest run --no-run` (still seeds the cache);
  the llvm-cov run (`nightly.yml:809`) is the one execution.
- [ ] PR Playwright `workers: CI ? 2 : undefined`; regenerate `contracts/budgets/test-tier-budgets.v1.yaml` from a
  green nightly's JUnit so the budgets reflect the real ~minutes, not 27 s.

## Phase 4 — How work reaches CI

### Task 4.1: Short-lived draft PR per batch

**Files:** `AGENTS.md` (CI Contract, PR & Review Discipline), `docs/src/contributors/antigravity-driven-execution.md`,
`docs/agents/agy-driver-kit/`.

- [ ] Document the loop: branch → batch of commits → `git push -u origin <branch>` → `gh pr create --draft` → hosted CI
  → self-review → ready → merge queue. Direct commits to `main` stop.
- [ ] Local verification for a batch is `cargo check -p <touched>` and `cargo nextest run -p <touched>` only; the rest is
  the PR's job.

### Task 4.2: Local runners are backup only

- [ ] Move `ml_data_extraction.yml`'s `extract` job to `ubuntu-latest`; leave `train` on its GPU label with a comment that
  it is optional and nothing gates on it.
- [ ] `docs/src/ci/runner-contract.md` and the Mac fleet notes say: self-hosted runners are an optional backup; no
  required check targets them.

## Execution order

1. **Phase 0 first.** Task 0.1 needs the owner's push approval; 0.2 to 0.4 can run while it waits, on the same branch.
2. **Phase 1** in parallel with Phase 0 (it touches only hooks, `.cargo/`, the shim and docs). 1.5 is owner-only.
3. **Phase 2** after nightly is green (its cache seed then has something to save).
4. **Phase 3** after Phase 2, so the PR timing improvement of each step is measurable on its own.
5. **Phase 4** last, once PR CI is fast enough that a PR per batch is not a tax.

Agents: Tasks 0.2, 1.1, 1.3, 2.3, 3.1 and 3.2 are well suited to agy (mechanical, test-first). Tasks 0.1, 0.3, 0.4, 2.1,
2.2 and 3.3 need judgment on CI logs and run as Claude's own work.

## Handoff prompt (paste into a new Claude Code tab)

```text
Work in the worktree ~/dev/vox/.claude/worktrees/ci-speed (branch work/ci-speed). Read
docs/src/architecture/ci-and-build-loop-findings-2026.md, then execute
docs/superpowers/plans/2026-10-03-hosted-ci-fast-local-loop.md phase by phase in the stated order, driving
agy for the tasks the plan marks as agy-suited (node .agents/scripts/drive.mjs <plan> <task>; install the kit
from docs/agents/agy-driver-kit first) and doing the rest yourself. Do not symlink target/. Never run
cargo clippy --workspace or cargo test --workspace locally; verify on a draft PR. Stop and ask before: pushing
the first time, any crypto-ledger change, deleting Actions caches, raising a workflow cap, or any
crate-edge/fan-in/layers change. Commit by pathspec with the repo's attribution trailer, update each task's
checkbox and the findings doc's targets table as numbers come in, and report progress per phase.
```
