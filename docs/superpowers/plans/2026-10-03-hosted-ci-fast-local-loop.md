# Hosted CI Gate and a Fast Local Loop — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
> Executed **in the owner's working tab**: Claude runs Track A itself and drives Gemini Flash through `agy` for Track B
> (`node .agents/scripts/drive.mjs docs/superpowers/plans/2026-10-03-hosted-ci-fast-local-loop.md <N>`, one task per
> run, per [`antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md)). The agent never
> stages or commits; Claude re-runs each task's checks, reviews the diff and commits by pathspec. Steps use `- [ ]`.

**Goal:** Hosted CI is the only gate and it is fast and green; the laptop runs only `cargo check` and the edited crate's
tests; no git hook builds anything; the two failure classes that got us here are guarded so they cannot quietly return.

**Architecture:** One new failing guard (`vox ci dev-loop-guard`, in-process, in the fast tier) turns the self-inflicted
slowness into CI errors. Hooks call installed, CI-built binaries. The build broker admits builds by measured load instead
of a fixed count. PR CI builds test binaries once and shards only execution, with a warm cache written from `main`.
Work reaches `main` only through a PR, enforced by branch protection and a pre-push refusal.

**Tech Stack:** Rust 2024 (`vox-cli-ci`, `vox-build-queue`, `vox-cargo-shim`, `vox-cli`), lefthook, GitHub Actions,
cargo-nextest (`archive` / `--archive-file`), `gh`.

**Spec:** [`docs/src/architecture/ci-and-build-loop-findings-2026.md`](../../src/architecture/ci-and-build-loop-findings-2026.md)
(every finding cited below is in it with its source).

## Global Constraints

- **No hardcoded CPU or job counts anywhere** (`.cargo/config.toml`, the broker, workflows, scripts). Concurrency derives
  from `std::thread::available_parallelism()` and measured load; an env var may override, never a literal default.
- **No coverage loss.** A test leaves the PR gate only into a nightly tier; a deleted test is dead (expired sunset,
  retired behaviour) or a duplicate, and the commit body says which.
- **Never add a test to the #569 quarantine** to make a PR green. Nightly tiers are separate, named nextest groups.
- **STOP and ask the owner before:** crate-edge exceptions, the fan-in snapshot, `layers.toml`, the transport-crypto
  ledger (any h2/rustls/ring/aws-lc/hyper bump), raising a workflow time cap, deleting Actions caches, changing GitHub
  repository settings (branch protection), the macOS Developer Tools setting.
- Workflow caps stay: 30 min for push/PR jobs, 180 min for scheduled (`workflow_policy_guard.rs`).
- Rust caches save from `main` only (`vox ci cache-key-lint`).
- **Local verification never runs workspace-wide clippy or tests.** Run the named crate only:
  `cargo test -p <crate> --lib <filter>`, `cargo clippy -p <crate> --all-targets -- -D warnings`. Hosted CI runs the rest.
- Test-first: every behaviour change captures its failing test output to `target/ci-t<N>-red.txt` first.
- Commit trailer: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`; agy-driven commits add
  `Assisted-by: Gemini 3.8 Flash (Antigravity)`.

## File Structure

| File | Status | Task | Responsibility |
|---|---|---|---|
| `crates/vox-cli-ci/src/dev_loop_guard.rs` | create | 1.1 | Failing checks: hooks build nothing, hooks call registered commands, no hardcoded `jobs` |
| `crates/vox-cli-ci/src/lib.rs` | modify | 1.1, 1.4 | `pub mod dev_loop_guard; pub mod pre_push_refs;` |
| `crates/vox-cli/src/commands/ci/pre_push.rs` | modify | 1.1, 1.4 | fast-tier step; refuse direct pushes to `main` |
| `crates/vox-cli-ci/src/pre_push_refs.rs` | create | 1.4 | parse git pre-push stdin; decide refusal |
| `lefthook.yml` | modify | 1.2, 1.4 | installed binaries only; `use_stdin` for pre-push |
| `.cargo/config.toml` | modify | 1.2 | drop `[build] jobs = 24` |
| `contracts/operations/catalog.v1.yaml` (+ generated registry) | modify | 1.3 | register `ci status` |
| `crates/vox-build-queue/src/global.rs`, `Cargo.toml` | modify | 1.5 | load-aware admission, no clamp |
| `crates/vox-cargo-shim/src/main.rs` | modify | 1.5, 1.6 | admission loop; `cargo run` = build under slot, run outside it |
| `crates/vox-cli/src/commands/ci/status.rs` | modify | 1.7 | report unpushed `main` and red nightly loudly |
| `.github/workflows/ci.yml` | modify | 2.1, 2.2 | `build-tests` archive job; shards run the archive; one shard for small sets |
| `.github/workflows/cache-seed.yml` | create | 2.1 | push-to-`main` warm-cache writer (not a gate) |
| `crates/vox-cli-ci/src/affected.rs` | modify | 2.3 | contracts → crates mapping |
| `.config/nextest.toml` | modify | 3.1, 3.3 | `nested-cargo` group, `slow` assignments, quarantine burn-down |
| `crates/<crate>/tests/it/**` | move | 3.2 | merged integration-test binaries |
| `.github/workflows/nightly.yml` | modify | 0.4, 3.4 | own failures; dedupe the plain run |
| `AGENTS.md`, `docs/src/contributors/*.md`, `docs/agents/agy-driver-kit/` | modify | 4.1, 4.2 | PR-per-batch, runners-as-backup, no-hardcoded-counts rule |

## Tracks (run in parallel in this tab)

- **Track A — Claude (judgment, CI logs):** 0.1 → 0.2 → 0.3 → 0.4, then 2.1 → 2.2 → 2.3, then 3.3, 3.4, 4.1, 4.2.
- **Track B — agy (mechanical, test-first):** 1.1 → 1.2 → 1.3 → 1.4 → 1.5 → 1.6 → 1.7, then 3.1 → 3.2.
- Tracks touch disjoint files except `pre_push.rs` (1.1 and 1.4, both Track B, sequential) and `ci.yml` (Track A only).
- Each track works in its own worktree with **its own `target/`** (never a symlink); Track B: `.claude/worktrees/ci-speed`.
- Owner tasks (0.1 push approval — given 2026-10-03, 1.8, 2.5, 4.3) are surfaced in chat when reached.

---

## Phase 0 — Unblock

### Task 0.1: Land local `main` through a draft PR (Claude)

**Status 2026-10-03:** owner approved; pushed `main` as `land/2026-10-03-main`, draft PR opened.

- [ ] **Step 1:** read the PR's CI. Failures this branch caused are fixed on the branch; pre-existing ones go to 0.2–0.4.
- [ ] **Step 2:** self-review the full range (`/code-review high`), mark ready, merge through the queue.
- [ ] **Done when:** `gh run list --workflow nightly.yml -L 1` shows `setup` passing on the next scheduled run.

### Task 0.2: Fix the all-features compile errors (Claude)

**Files:** `crates/vox-actor-runtime` (E0599 `DbConfig::resolve_canonical`), `crates/vox-codegen` (E0432, E0277),
`crates/vox-cli` (E0275 via netlink `Tcf: Send`), `objc2` in `crates/vox-ml-cli`, `crates/vox-populi`.

- [ ] **Step 1:** copy each error and its command from the latest nightly log into `target/ci-t02-<crate>-red.txt`
  (`gh run view <id> --log-failed`). Reproduce with `cargo check -p <crate> --all-features` where the target OS allows.
- [ ] **Step 2:** fix at the root: a feature-gated item used without its `cfg`, or a platform crate missing
  `[target.'cfg(target_os = "macos")'.dependencies]`. For E0275, box the recursive future or add the explicit `Send`
  bound that ends the cycle; name the type in the commit body.
- [ ] **Step 3:** `cargo check -p <crate> --all-features` clean (Linux-only errors: verified on the PR's CI). One commit per crate.

### Task 0.3: Clear the `cargo audit` advisories (Claude; Owner for crypto)

- [ ] **Step 1:** list them from the nightly log (wasmtime, h2, a TLS 1.3 issue).
- [ ] **Step 2:** non-transport: `cargo update -p <crate> --precise <fixed-version>`; commit with the advisory IDs.
- [ ] **Step 3: STOP** for h2/rustls/ring/aws-lc/hyper: put the drafted `contracts/crypto/transport-providers.v1.json`
  entry in the PR description; the owner applies it.

### Task 0.4: Nightly's own failures (Claude)

**Files:** `.github/workflows/nightly.yml`, `.github/workflows/gui-cross-build.yml`.

- [ ] CDP smoke ("Browser::connect failed: Received no response"): read the harness; pass the Chromium flags it expects
  for a container (`--no-sandbox`, a connect timeout from its own constant). Record the root cause in the commit body.
- [ ] Playwright at its 25-min cap: split into two jobs with `--shard=1/2` and `--shard=2/2` (`--workers` left to
  Playwright's default, which follows the runner's cores). No cap change.
- [ ] Windows GUI smoke (93–124 min): move to `gui-cross-build.yml`'s schedule so it stops setting nightly's wall time.
- [ ] **Done when:** three consecutive scheduled nightlies are green; close #574–#593 with the run link.

---

## Phase 1 — Prevention and a fast local loop

### Task 1.1: `vox ci dev-loop-guard` (agy)

**Acts on:** hooks running `cargo run` (`lefthook.yml:31,36,41,46,58,80`); a hook calling an unregistered command
(`ci status` is absent from `contracts/cli/command-registry.yaml`); `[build] jobs = 24` (`.cargo/config.toml:23`).

**Files:**
- Create: `crates/vox-cli-ci/src/dev_loop_guard.rs`
- Modify: `crates/vox-cli-ci/src/lib.rs` (add `pub mod dev_loop_guard;` beside `pub mod dev_loop_audit;`)
- Modify: `crates/vox-cli/src/commands/ci/pre_push.rs` (fast-tier step)

**Interfaces:**
- Produces: `vox_cli_ci::dev_loop_guard::{hook_build_offenders(&str) -> Vec<String>, unregistered_hook_commands(&str, &HashSet<String>) -> Vec<String>, hardcoded_build_jobs(&str) -> bool, run(&Path) -> anyhow::Result<()>}`.

- [ ] **Step 1: Write the failing tests** — create `dev_loop_guard.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn flags_hooks_that_build() {
        let y = "pre-commit:\n  commands:\n    a:\n      run: cargo run -p vox-cli --quiet -- ci command-sync\n    b:\n      run: rustfmt {staged_files}\n    # cargo build in a comment is fine\n";
        assert_eq!(hook_build_offenders(y), vec!["cargo run -p vox-cli --quiet -- ci command-sync".to_string()]);
    }

    #[test]
    fn passes_hooks_that_only_call_installed_binaries() {
        let y = "pre-push:\n  commands:\n    p:\n      run: vox ci pre-push\n";
        assert!(hook_build_offenders(y).is_empty());
    }

    #[test]
    fn flags_vox_subcommands_missing_from_the_registry() {
        let reg: HashSet<String> = ["ci pre-push", "run"].iter().map(|s| s.to_string()).collect();
        let y = "x:\n  run: vox ci status --hook || true\ny:\n  run: vox ci pre-push\nz:\n  run: vox run scripts/fmt.vox -- --all\n";
        assert_eq!(unregistered_hook_commands(y, &reg), vec!["ci status".to_string()]);
    }

    #[test]
    fn flags_a_hardcoded_build_jobs_count() {
        assert!(hardcoded_build_jobs("[build]\njobs = 24\nrustdocflags = []\n"));
        assert!(!hardcoded_build_jobs("[build]\nrustdocflags = []\n[net]\njobs = 3\n"));
        assert!(!hardcoded_build_jobs("# jobs = 24\n[build]\n"));
    }
}
```

  Add `pub mod dev_loop_guard;` to `lib.rs` so it compiles.
- [ ] **Step 2: Run, expect failure:** `timeout 1500s cargo test -p vox-cli-ci --lib dev_loop_guard > target/ci-t11-red.txt 2>&1; tail -15 target/ci-t11-red.txt`
  — Expected: `error[E0425]: cannot find function` for the four functions.
- [ ] **Step 3: Implement** above the test module:

```rust
//! `vox ci dev-loop-guard` — fails when the local loop is made slow or broken by configuration:
//! a git hook that builds, a hook that calls an unregistered `vox` command, or a hardcoded cargo
//! `jobs` count. In-process and build-free, so it runs in the fast tier locally and in CI.

use anyhow::{Result, bail};
use std::collections::HashSet;
use std::path::Path;

const BUILD_COMMANDS: [&str; 5] = ["cargo run", "cargo build", "cargo clippy", "cargo test", "cargo nextest"];

/// `run:` values in a lefthook file that invoke a cargo build.
pub fn hook_build_offenders(lefthook: &str) -> Vec<String> {
    lefthook
        .lines()
        .filter_map(|l| l.trim().strip_prefix("run:").map(str::trim))
        .filter(|cmd| BUILD_COMMANDS.iter().any(|b| cmd.contains(b)))
        .map(str::to_string)
        .collect()
}

/// `vox <words>` invocations in a lefthook file whose command path is not in `registry`. The path is
/// the words after `vox` up to the first flag, placeholder, path or shell operator.
pub fn unregistered_hook_commands(lefthook: &str, registry: &HashSet<String>) -> Vec<String> {
    let mut out = Vec::new();
    for cmd in lefthook.lines().filter_map(|l| l.trim().strip_prefix("run:").map(str::trim)) {
        let mut words = cmd.split_whitespace();
        if words.next() != Some("vox") {
            continue;
        }
        let path: Vec<&str> = words
            .take_while(|w| !w.starts_with('-') && !w.starts_with('{') && !w.contains('/') && !w.contains('.') && !w.starts_with('|'))
            .collect();
        let joined = path.join(" ");
        if !joined.is_empty() && !registry.contains(&joined) {
            out.push(joined);
        }
    }
    out
}

/// True when the `[build]` table of a cargo config sets `jobs`.
pub fn hardcoded_build_jobs(cargo_config: &str) -> bool {
    let mut in_build = false;
    for line in cargo_config.lines().map(str::trim) {
        if line.starts_with('[') {
            in_build = line == "[build]";
        } else if in_build && line.split('=').next().map(str::trim) == Some("jobs") {
            return true;
        }
    }
    false
}

fn registry_paths(root: &Path) -> Result<HashSet<String>> {
    #[derive(serde::Deserialize)]
    struct Op { surface: String, path: Vec<String> }
    #[derive(serde::Deserialize)]
    struct Reg { operations: Vec<Op> }
    let reg: Reg = serde_yaml::from_str(&std::fs::read_to_string(root.join("contracts/cli/command-registry.yaml"))?)?;
    Ok(reg.operations.into_iter().filter(|o| o.surface == "vox-cli").map(|o| o.path.join(" ")).collect())
}

pub fn run(root: &Path) -> Result<()> {
    let hooks = std::fs::read_to_string(root.join("lefthook.yml"))?;
    let cargo = std::fs::read_to_string(root.join(".cargo/config.toml"))?;
    let mut errors = Vec::new();
    for c in hook_build_offenders(&hooks) {
        errors.push(format!("lefthook.yml builds in a hook: `{c}` — call the installed binary instead"));
    }
    for c in unregistered_hook_commands(&hooks, &registry_paths(root)?) {
        errors.push(format!("lefthook.yml calls `vox {c}`, which is not in contracts/cli/command-registry.yaml"));
    }
    if hardcoded_build_jobs(&cargo) {
        errors.push(".cargo/config.toml sets [build] jobs — let cargo use the machine's parallelism".into());
    }
    if errors.is_empty() {
        println!("dev-loop-guard: ok");
        return Ok(());
    }
    for e in &errors {
        eprintln!("dev-loop-guard: {e}");
    }
    bail!("dev-loop-guard: {} problem(s)", errors.len())
}
```

- [ ] **Step 4: Run, expect pass:** `timeout 1500s cargo test -p vox-cli-ci --lib dev_loop_guard 2>&1 | tail -6` — 4 passed.
- [ ] **Step 5: Wire into the fast tier.** In `pre_push.rs`, after the `workflow-permissions-guard` `OwnedStep`, add

```rust
        OwnedStep {
            label: "vox ci dev-loop-guard".into(),
            scope: None,
            run: Box::new(step_dev_loop_guard),
        },
```

  and beside `fn step_workflow_permissions_guard`:

```rust
fn step_dev_loop_guard(root: &Path) -> Result<()> {
    vox_cli_ci::dev_loop_guard::run(root)
}
```

  The step **will fail on today's tree**; that is intended. Task 1.2 and 1.3 make it pass, and they land in the same PR.
- [ ] **Step 6 (Claude):** mutation proofs — delete `"cargo run"` from `BUILD_COMMANDS` (fails `flags_hooks_that_build`);
  make `hardcoded_build_jobs` ignore `in_build` (fails the `[net]` case). Clippy `-p vox-cli-ci`. Commit
  "feat(ci): dev-loop-guard fails on hooks that build, unregistered hook commands and hardcoded jobs".

### Task 1.2: Hooks run installed binaries; no hardcoded jobs (agy)

**Files:** `lefthook.yml`, `.cargo/config.toml`, `scripts/install-hooks.vox`, `docs/src/contributors/local-ci-pre-push.md`.

- [ ] **Step 1:** in `lefthook.yml` replace `cargo run -p vox-cli --quiet -- ` with `vox ` in `sync-ignore-files`,
  `command-sync`, `gui-version-sync`, `plugin-catalog-docs`, and the `pre-push` command (lines 31, 36, 41, 46, 80).
- [ ] **Step 2:** `tdd-guard` (line 58): `run: toestub {staged_files} --rules skeleton --min-severity warning --mode enforce-strict --format terminal --suppressions contracts/toestub/suppressions.v1.json`.
- [ ] **Step 3:** add at the top of `lefthook.yml` a comment block: hooks never build; binaries come from
  `voxup install --tag nightly` (Task 1.7) or `cargo install --locked --path crates/vox-cli` / `--path crates/vox-code-audit --bin toestub`;
  a missing binary makes lefthook report "command not found" and the hosted fast tier still runs every check.
- [ ] **Step 4:** delete `jobs = 24` from `[build]` in `.cargo/config.toml` and add the comment
  `# No jobs count: cargo defaults to the machine's available parallelism. dev-loop-guard rejects a hardcoded value.`
- [ ] **Step 5:** `scripts/install-hooks.vox` checks `vox` and `toestub` are on PATH and prints the `voxup` command if not.
- [ ] **Step 6 (Claude):** `vox ci dev-loop-guard` now reports only the `ci status` registry problem (fixed in 1.3).
  Time `git commit` on a staged `.rs` change and `git push --dry-run` on a cold `target/`; record both in the findings
  doc targets table. Commit.

### Task 1.3: Register `ci status` (agy)

**Files:** `contracts/operations/catalog.v1.yaml`, regenerated `contracts/cli/command-registry.yaml` and
`docs/src/reference/cli-command-surface.generated.md`.

- [ ] **Step 1:** add the `ci status` operation to the catalog, copying the shape of the `ci pre-push` row
  (`handler_rust: commands::ci::status`).
- [ ] **Step 2:** `vox ci operations-sync --target cli --write` then `vox ci command-sync`.
- [ ] **Step 3 (Claude):** `vox ci dev-loop-guard` prints `ok`; `vox ci ssot-drift` is clean. Commit.

### Task 1.4: Refuse direct pushes to `main` (agy)

**Acts on:** 57 commits reached `main` without CI; nightly broke on a lockfile no PR checked.

**Files:**
- Create: `crates/vox-cli-ci/src/pre_push_refs.rs`
- Modify: `crates/vox-cli-ci/src/lib.rs` (`pub mod pre_push_refs;`), `crates/vox-cli/src/commands/ci/pre_push.rs`, `lefthook.yml`

**Interfaces:** Produces `vox_cli_ci::pre_push_refs::refused_main_push(stdin: &str, allow: bool) -> Option<String>`.

- [ ] **Step 1: Failing tests** in `pre_push_refs.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    const Z: &str = "0000000000000000000000000000000000000000";

    #[test]
    fn refuses_a_push_that_updates_remote_main() {
        let s = format!("refs/heads/main abc refs/heads/main {Z}\n");
        assert!(refused_main_push(&s, false).is_some());
    }

    #[test]
    fn allows_a_branch_push_and_the_explicit_override() {
        let s = format!("refs/heads/main abc refs/heads/land/x {Z}\n");
        assert!(refused_main_push(&s, false).is_none());
        let m = format!("refs/heads/main abc refs/heads/main {Z}\n");
        assert!(refused_main_push(&m, true).is_none());
    }

    #[test]
    fn ignores_empty_stdin() {
        assert!(refused_main_push("", false).is_none());
    }
}
```

- [ ] **Step 2:** red run to `target/ci-t14-red.txt`.
- [ ] **Step 3: Implement:**

```rust
//! Git pre-push stdin: `<local ref> <local sha> <remote ref> <remote sha>` per line.
//! Work reaches `main` through a PR (AGENTS.md, PR & Review Discipline), so a push that
//! updates remote `main` is refused unless `VOX_ALLOW_MAIN_PUSH=1`.

pub fn refused_main_push(stdin: &str, allow: bool) -> Option<String> {
    if allow {
        return None;
    }
    stdin
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .find(|remote_ref| *remote_ref == "refs/heads/main")
        .map(|_| "refusing to push to main: open a draft PR (git push -u origin HEAD:<branch> && gh pr create --draft), or set VOX_ALLOW_MAIN_PUSH=1".to_string())
}
```

- [ ] **Step 4:** in `pre_push.rs`'s entry point, before any step runs: when stdin is not a terminal
  (`!std::io::stdin().is_terminal()`), read it to a string and, if
  `refused_main_push(&s, std::env::var_os("VOX_ALLOW_MAIN_PUSH").is_some())` is `Some(msg)`, `bail!(msg)`.
  CI runs the fast tier with empty stdin, so nothing changes there.
- [ ] **Step 5:** in `lefthook.yml` add `use_stdin: true` under the `pre-push` command.
- [ ] **Step 6 (Claude):** tests pass; mutation: change `"refs/heads/main"` to `"refs/heads/mainx"` (fails the first test).
  Verify end to end: `git push --dry-run origin HEAD:main` is refused; `HEAD:land/test` proceeds. Commit.

### Task 1.5: The broker admits by load, not a fixed count (agy)

**Acts on:** `max_concurrent_from` = `(parallelism / 3).clamp(2, 8)` (`crates/vox-build-queue/src/global.rs:43-51`) with
`jobs = 24` per build → about 144 rustc on 18 cores; load 15–40; 80–92% kernel time.

**Files:** `crates/vox-build-queue/src/global.rs`, `crates/vox-build-queue/Cargo.toml` (add `libc = { workspace = true }`
under `[target.'cfg(unix)'.dependencies]` — already a workspace dependency, no new crate), `crates/vox-cargo-shim/src/main.rs`,
`docs/src/contributors/build-broker-usage.md`.

**Interfaces:**
- Produces: `vox_build_queue::global::{should_admit(load_1m: Option<f64>, parallelism: usize, held: usize, cap: usize) -> bool, load_average_1m() -> Option<f64>, acquire_slot_adaptive(root: &Path, cap: usize) -> anyhow::Result<(Slot, u64, usize)>}`.
- `max_concurrent_from(raw, parallelism)` default changes from `(parallelism / 3).clamp(2, 8)` to `parallelism.max(1)` —
  the slot count becomes an upper bound only; admission is decided by load.

- [ ] **Step 1: Failing tests** (append to the existing `mod tests` in `global.rs`):

```rust
    #[test]
    fn admits_the_first_build_regardless_of_load() {
        assert!(should_admit(Some(1000.0), 8, 0, 8));
    }

    #[test]
    fn admits_another_build_only_while_the_machine_has_idle_cores() {
        assert!(should_admit(Some(5.0), 8, 1, 8));
        assert!(!should_admit(Some(8.0), 8, 1, 8));
        assert!(!should_admit(Some(12.5), 8, 2, 8));
    }

    #[test]
    fn never_exceeds_the_slot_cap() {
        assert!(!should_admit(Some(0.0), 8, 8, 8));
    }

    #[test]
    fn without_a_load_reading_admits_one_build_at_a_time() {
        assert!(should_admit(None, 8, 0, 8));
        assert!(!should_admit(None, 8, 1, 8));
    }

    #[test]
    fn default_cap_follows_the_machine_with_no_literal_bounds() {
        assert_eq!(max_concurrent_from(None, 18), 18);
        assert_eq!(max_concurrent_from(None, 1), 1);
        assert_eq!(max_concurrent_from(Some("3"), 18), 3);
    }
```

  Delete any existing test asserting the old `clamp(2, 8)` default and say so in the commit body (it pins the behaviour
  this task removes).
- [ ] **Step 2:** red run to `target/ci-t15-red.txt`.
- [ ] **Step 3: Implement** in `global.rs`:

```rust
/// Whether a build may start now. The first build always runs. Another runs only while the
/// 1-minute load average is below the machine's parallelism (idle cores exist), and never past
/// `cap` slots. Without a load reading (non-unix), builds run one at a time.
pub fn should_admit(load_1m: Option<f64>, parallelism: usize, held: usize, cap: usize) -> bool {
    if held == 0 {
        return true;
    }
    if held >= cap {
        return false;
    }
    match load_1m {
        Some(load) => load < parallelism as f64,
        None => false,
    }
}

/// 1-minute load average, or `None` where the OS does not report one.
pub fn load_average_1m() -> Option<f64> {
    #[cfg(unix)]
    {
        let mut v = [0f64; 3];
        // SAFETY: getloadavg writes at most `nelem` doubles into the provided buffer.
        let n = unsafe { libc::getloadavg(v.as_mut_ptr(), 3) };
        (n >= 1).then_some(v[0])
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Like [`acquire_slot`], but a free slot is taken only when [`should_admit`] agrees.
pub fn acquire_slot_adaptive(root: &Path, cap: usize) -> Result<(Slot, u64, usize)> {
    let start = now_ms();
    let parallelism = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    loop {
        let held = probe_busy_slots(root, cap)?;
        if should_admit(load_average_1m(), parallelism, held, cap)
            && let Some((slot, busy)) = try_acquire_slot(root, cap)?
        {
            return Ok((slot, now_ms().saturating_sub(start) as u64, busy));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
```

  (`probe_busy_slots` is the existing non-perturbing probe at `global.rs:144`.) Change `max_concurrent_from`'s fallback to `parallelism.max(1)` and its doc comment to match. In the shim,
  replace `global::acquire_slot(&root, n)` with `global::acquire_slot_adaptive(&root, n)`.
- [ ] **Step 4:** `timeout 1500s cargo test -p vox-build-queue 2>&1 | tail -6` passes; build the shim with
  `cargo build --manifest-path crates/vox-cargo-shim/Cargo.toml` (never `-p`).
- [ ] **Step 5 (Claude):** mutations — drop the `held == 0` early return (fails the first test); compare `<=` instead of `<`
  (fails the second). Update `build-broker-usage.md`: admission by load; `VOX_BROKER_MAX_CONCURRENT` is an upper bound.
  Commit.

### Task 1.6: `cargo run` holds a slot only while building (agy)

**Acts on:** the shim holds a slot for the whole runtime of `cargo run` (`crates/vox-cargo-shim/src/main.rs:162-181`).

**Files:** `crates/vox-cargo-shim/src/main.rs`, `crates/vox-build-queue/src/resolve.rs` (test home if the shim has none).

**Interfaces:** Produces `vox_build_queue::resolve::build_args_for_run(args: &[String]) -> Option<Vec<String>>` — the
`cargo build` argv equivalent to a `cargo run` argv (drops everything from `--` on; `None` when `args[0] != "run"`).

- [ ] **Step 1: Failing tests** in `resolve.rs`:

```rust
    #[test]
    fn run_becomes_build_without_program_args() {
        let a: Vec<String> = ["run", "-p", "vox-cli", "--quiet", "--", "ci", "pre-push"].iter().map(|s| s.to_string()).collect();
        assert_eq!(build_args_for_run(&a).unwrap(), vec!["build", "-p", "vox-cli", "--quiet"]);
    }

    #[test]
    fn non_run_subcommands_are_left_alone() {
        let a: Vec<String> = ["test", "-p", "x"].iter().map(|s| s.to_string()).collect();
        assert!(build_args_for_run(&a).is_none());
    }
```

- [ ] **Step 2:** red run; **Step 3:** implement:

```rust
pub fn build_args_for_run(args: &[String]) -> Option<Vec<String>> {
    if args.first().map(String::as_str) != Some("run") {
        return None;
    }
    let mut out = vec!["build".to_string()];
    out.extend(args[1..].iter().take_while(|a| a.as_str() != "--").cloned());
    Some(out)
}
```

  In `run_global`: when `build_args_for_run(args)` is `Some(b)`, run the real cargo with `b` while holding the slot, drop
  the slot, then `exec_real(real, args, depth, toolchain)` (the second `cargo run` finds the artifact fresh and starts the
  program without rebuilding). A failed build returns its exit code without running.
- [ ] **Step 4:** stale `inflight/` entries (145 found): `register_inflight` (`global.rs:182`) holds an exclusive `fs2` lock on
  its own entry for the build's lifetime, and before registering, removes any entry it can lock (its owner is gone). Test: create
  an unlocked entry in a temp `VOX_BROKER_HOME`, call `register_inflight`, assert the stale file is gone and a locked one survives.
- [ ] **Step 5 (Claude):** tests pass; commit.

### Task 1.7: Make the warnings reach the owner (agy)

**Acts on:** 13 open `nightly-failure` issues nobody saw because the `ci-status` hook called a subcommand the installed
`vox` lacked; `main` 57 commits ahead of `origin/main` with no signal.

**Files:** `crates/vox-cli/src/commands/ci/status.rs`, `.github/workflows/nightly-artifacts.yml`, `scripts/install-hooks.vox`.

- [ ] **Step 1: Failing test** in `status.rs`: a pure `fn unpushed_main_line(ahead: u32) -> Option<String>` returns
  `Some("main is 57 commits ahead of origin/main — land it through a PR")` for 57 and `None` for 0.
- [ ] **Step 2:** implement; `vox ci status --hook` calls it with `git rev-list --count origin/main..main` and prints the
  open `nightly-failure` issue count in the same block (already fetched by `status.rs`).
- [ ] **Step 3:** `nightly-artifacts.yml` publishes `toestub` with `vox`; `install-hooks.vox` installs both via `voxup`.
- [ ] **Step 4 (Claude):** commit; confirm `vox ci status --hook` prints both lines on this machine.

### Task 1.8: macOS Developer Tools (Owner)

- [ ] System Settings → Privacy & Security → Developer Tools: enable the terminal in use and the Claude app; restart both.
- [ ] **(Claude):** repeat Part A of `rust-build-loop-ssot-2026.md`; record user vs sys seconds in the findings doc.

---

## Phase 2 — Hosted CI builds once and keeps its cache warm

### Task 2.1: Build test binaries once; a cache writer on `main` (Claude)

**Files:** `.github/workflows/ci.yml` (jobs `tests`, new `build-tests`), create `.github/workflows/cache-seed.yml`.

- [ ] **Step 1:** new job `build-tests` (`needs: linux`, `if: needs.linux.outputs.run_tests == 'true'`, same setup steps
  as `tests`) runs `cargo nextest archive $P_ARGS --profile ci-gate --locked --archive-file tests.tar.zst` and
  `actions/upload-artifact` with `retention-days: 1`.
- [ ] **Step 2:** `tests` gets `needs: [linux, build-tests]`, downloads the artifact, drops the Rust cache step, and runs
  `cargo nextest run --archive-file tests.tar.zst --workspace-remap . --profile ci-gate --no-tests=pass --partition "hash:${SHARD}/${SHARDS}"`.
- [ ] **Step 3:** `cache-seed.yml`: `on: push: branches: [main]`, top-level `concurrency` (cancel-in-progress) and
  `permissions: contents: read`, one job (`timeout-minutes: 30`) running `cargo nextest archive --workspace --exclude vox-gui --locked --archive-file /tmp/a.tar.zst`
  with `Swatinem/rust-cache` `shared-key: workspace`, `save-if: ${{ github.ref == 'refs/heads/main' }}`.
- [ ] **Step 4:** `vox ci workflow-concurrency-guard`, `vox ci workflow-permissions-guard`, `vox ci cache-key-lint` pass;
  the `gate` job's required context name is unchanged (`vox ci required-context-guard`).
- [ ] **Done when:** on a draft PR, shard logs contain no `Compiling`; record the full-run wall time in the findings doc.

### Task 2.2: Shard count follows the work (Claude)

- [ ] `linux`'s `Plan tests` step emits `shards=1` when `p_args` is not `--workspace…` and names fewer than 30 crates,
  else `shards=3`; the `tests` matrix becomes `shard: ${{ fromJSON(needs.linux.outputs.shard_list) }}` with
  `shard_list` = `[1]` or `[1,2,3]`. No literal parallelism anywhere else.

### Task 2.3: Narrow "contracts forces a full run" (agy)

**Files:** `crates/vox-cli-ci/src/affected.rs` (rules at lines 5-12, 33-44, 143-151).

- [ ] **Step 1: Failing tests:** only `contracts/reports/gui-surface-coverage.v1.json` changed → not full; only
  `contracts/ci/crate-edges.allow.v1.json` changed → full; an unknown `contracts/new/x.yaml` → full (fail safe).
- [ ] **Step 2:** implement a table `CONTRACT_READERS: &[(&str prefix, &[&str] crates)]` built by grepping
  `include_str!`/path constants for each `contracts/<dir>`; `contracts/reports/` maps to no crates (generated output).
- [ ] **Step 3 (Claude):** mutation: remove the fail-safe branch (fails the unknown-path test). Commit.

### Task 2.4: Pin the cache budget in code (Claude)

- [ ] Give the sccache key a version suffix and set `SCCACHE_CACHE_SIZE` from the job's disk headroom in the four workflows
  that use it. Extend `vox ci cache-key-lint` to fail a workflow that uses sccache without `SCCACHE_CACHE_SIZE`.

### Task 2.5: Delete over-budget caches (Owner)

- [ ] Claude lists cache entries by key prefix and size (`gh cache list -L 100 --sort size_in_bytes`); the owner approves;
  `gh cache delete --all --key <prefix>` per approved prefix. Done when usage is under 9 GB.

---

## Phase 3 — The suite pays only for what the gate needs

### Task 3.1: Nested-cargo tests move to a serialized nightly group (agy)

**Files:** `.config/nextest.toml`.

- [ ] Add `[test-groups.nested-cargo] max-threads = 1` and `[[profile.default.overrides]]` assigning
  `filter = 'test(/emit_compile_harness|_compiles_as_rust_script|build_e2e|compile_fail|workflow_runtime_no_db/)'` to it; assign
  the existing `slow` group by its intended filter.
- [ ] `[profile.ci-gate]` default-filter excludes `group(nested-cargo)`; `[profile.ci]` (nightly) runs it.
- [ ] Remove the duplicated ladder targets compiled by two binaries (`crud_api`, `auth_patterns`, `db_native_ir`).
- [ ] **(Claude):** `cargo nextest list -p vox-codegen --profile ci-gate` no longer lists them and `--profile ci` does.

### Task 3.2: Merge tiny integration-test binaries (agy)

- [ ] Per crate, in order vox-cli, vox-integration-tests, vox-compiler, vox-codegen: `git mv tests/<name>.rs tests/it/<name>.rs`,
  create `tests/it/main.rs` with `mod <name>;` per file; update `.config/nextest.toml` filters to the `it::<name>::` prefix.
- [ ] **Done when** `cargo nextest list -p <crate> | wc -l` is unchanged before and after (record both numbers in the commit).

### Task 3.3: Burn down the quarantine and expired ignores (Claude)

- [ ] Each of the 32 #569 entries: fix, or delete with the reason. Each `#[ignore]` past its 2026-08-01 sunset and the 12
  asserting retired `activity`/`@v0` behaviour: un-ignore and fix, or delete. One commit per crate.

### Task 3.4: Nightly does each thing once (Claude)

- [ ] `nightly.yml:1561` plain run → `cargo nextest run --no-run` (still seeds the cache); llvm-cov (`:809`) is the one execution.
- [ ] Regenerate `contracts/budgets/test-tier-budgets.v1.yaml` from a green nightly's JUnit.

---

## Phase 4 — How work reaches CI

### Task 4.1: Document the loop (Claude)

**Files:** `AGENTS.md` (CI Contract; PR & Review Discipline; a new line under Structural Limits: "no hardcoded CPU or job
counts — derive from the machine"), `docs/src/contributors/antigravity-driven-execution.md`, `docs/agents/agy-driver-kit/`.

- [ ] Branch → batch → `git push -u origin HEAD:<branch>` → `gh pr create --draft` → hosted CI → self-review → ready →
  merge queue. Local verification is `cargo check -p <touched>` and that crate's tests. Never symlink `target/`.

### Task 4.2: Runners are backup only (Claude)

- [ ] `ml_data_extraction.yml` `extract` → `ubuntu-latest`; `train` keeps its GPU label with a comment that nothing gates on it.
- [ ] `docs/src/ci/runner-contract.md`: self-hosted runners are optional backup; no required check may target them.
  Add that rule to `workflow_policy_guard` (a required-context job with a `self-hosted` label fails).

### Task 4.3: Branch protection on `main` (Owner)

- [ ] Require a PR and the `Check, Build, and Test (Rust)` context; block direct pushes. Claude prepares the exact
  `gh api -X PUT repos/{owner}/{repo}/branches/main/protection` payload; the owner runs or approves it.

## Self-review against the spec

- Findings → tasks: hooks building (1.1, 1.2), unregistered hook command (1.1, 1.3), `jobs = 24` (1.1, 1.2), broker
  oversubscription and `cargo run` slot (1.5, 1.6), target symlinks (Tracks note, 4.1), syspolicyd (1.8), PR-only CI and
  unpushed main (0.1, 1.4, 4.3, 1.7), red nightly (0.1–0.4, 3.3), cold shard compiles (2.1, 2.2), contracts full runs (2.3),
  cache budget (2.4, 2.5), nested-cargo and tiny binaries (3.1, 3.2), duplicate nightly run and stale budgets (3.4),
  self-hosted (4.2).
- Recurrence guards: dev-loop-guard (build-free hooks, registered commands, no hardcoded jobs) in the fast tier locally
  and on CI; pre-push refusal plus branch protection (no CI-less `main`); `vox ci status` names unpushed `main` and
  nightly failures; cache-key-lint grows the sccache size rule; workflow_policy_guard grows the self-hosted rule.
