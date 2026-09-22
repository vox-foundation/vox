//! `vox ci cache-key-lint` — two invariants over every cache step in
//! `.github/workflows/*.yml` **and** `.github/actions/*/action.yml`.
//!
//! ## Rule 1 — a `Cargo.lock`-hashing key must also key on the toolchain
//!
//! Before this lint, 0 of 26 `actions/cache` blocks in the workflow tree
//! referenced the toolchain in their key. A cache scoped only to
//! `hashFiles(...Cargo.lock...)` survives a toolchain bump untouched: after
//! `contracts/toolchain/workspace-toolchain.v1.yaml` moves to a new Rust
//! version, every job restores object files compiled by the *old* compiler
//! under the *new* toolchain's `target/`, corrupting the build silently
//! (stale `.rlib`/`.rmeta` metadata, mismatched ABI) instead of failing
//! loudly. Anchoring the key on the toolchain forces a clean cache on every
//! bump, exactly like the OS or lockfile hash already do.
//!
//! A violation is any `actions/cache` step whose `with.key` string mentions
//! `Cargo.lock` (i.e. hashes the lockfile) without also containing the
//! substring `toolchain` anywhere in the key expression. That substring
//! check is deliberately textual, not semantic, so it accepts any of the
//! ways a site in this repo satisfies it:
//!   - `${{ steps.<id>.outputs.toolchain }}` (setup-rust's own output), or
//!   - `hashFiles('rust-toolchain.toml', 'Cargo.lock')` (for a job with no
//!     setup-rust step to draw an output from).
//!
//! ## Rule 2 — only `main` (and scheduled runs) may WRITE a cache
//!
//! The repository's Actions cache is a hard 10 GB bucket with LRU eviction.
//! A PR-triggered run that saves gets a cache scoped to `refs/pull/N/merge`
//! that no other ref can ever read, so every PR both wastes a slot and evicts
//! the `main`-scope entries that every job actually restores from. At 9.8/10 GB
//! that churn is the whole problem: the caches are full of write-only garbage.
//!
//! So on a **PR-reachable** workflow — one whose `on:` contains
//! `pull_request`, `pull_request_target` or `merge_group`, or a `push` that is
//! not restricted to exactly `branches: [main]` — a cache step must not save:
//!   - `actions/cache@…` needs an `if:` mentioning `refs/heads/main`
//!     (`actions/cache/restore@…` never saves, so it is always fine);
//!   - `Swatinem/rust-cache@…` needs `save-if: false` or a `save-if` /
//!     step-level `if:` mentioning `refs/heads/main`.
//!
//! Composite actions under `.github/actions/*/action.yml` are checked with the
//! save rule applied **unconditionally**: a composite has no `on:` of its own
//! and is reachable from any caller, including every PR workflow. A nested
//! action's post-save does run under a composite (the pre-existing main-scope
//! `Linux-cargo-*` cache entries are the proof), so the gate has to live on the
//! step inside the composite.
//!
//! Unlike the advisory guards in this crate, this one takes no `--strict`
//! flag: it always fails on a violation, mirroring `release_draft_guard` and
//! `toolchain_workflow_lint`.

use std::path::Path;

use anyhow::{Context, Result, anyhow};

use crate::workflow_policy_guard::trigger_keys;

const CACHE_ACTION_PREFIX: &str = "actions/cache";
const CACHE_RESTORE_PREFIX: &str = "actions/cache/restore";
const RUST_CACHE_PREFIX: &str = "Swatinem/rust-cache";
const LOCKFILE_MARKER: &str = "Cargo.lock";
const TOOLCHAIN_MARKER: &str = "toolchain";
const MAIN_REF: &str = "refs/heads/main";

/// One Rule-1 violation: an `actions/cache` step in `file` named `step` whose
/// `key` hashes `Cargo.lock` but never references the toolchain.
#[derive(Debug)]
struct Violation {
    file: String,
    step: String,
    key: String,
}

/// One Rule-2 violation: a cache step in `file` named `step` that writes a
/// cache on a ref other than `main`. `why` names the missing gate.
#[derive(Debug)]
struct SaveViolation {
    file: String,
    step: String,
    why: String,
}

fn field<'a>(m: &'a serde_yaml::Mapping, k: &str) -> Option<&'a serde_yaml::Value> {
    m.get(serde_yaml::Value::String(k.into()))
}

fn str_field<'a>(m: &'a serde_yaml::Mapping, k: &str) -> Option<&'a str> {
    field(m, k).and_then(|v| v.as_str())
}

/// Step display name: prefers the `name:` field, falls back to `uses:`.
fn step_name(step: &serde_yaml::Mapping) -> String {
    str_field(step, "name")
        .or_else(|| str_field(step, "uses"))
        .map(str::to_string)
        .unwrap_or_else(|| "<unnamed step>".to_string())
}

/// True when `key` hashes `Cargo.lock` but never mentions the toolchain.
fn is_untoolchained_lockfile_key(key: &str) -> bool {
    key.contains(LOCKFILE_MARKER) && !key.contains(TOOLCHAIN_MARKER)
}

/// Every step mapping reachable from `doc`, in document order. Workflows keep
/// them under `jobs.<id>.steps`; composite actions under `runs.steps`. Both
/// walkers below run over this one iterator so the step loop exists once.
fn steps_of(doc: &serde_yaml::Value) -> Vec<&serde_yaml::Mapping> {
    let Some(root) = doc.as_mapping() else {
        return Vec::new();
    };
    let seqs: Vec<&serde_yaml::Sequence> = match field(root, "jobs").and_then(|j| j.as_mapping()) {
        Some(jobs) => jobs
            .iter()
            .filter_map(|(_, job)| job.as_mapping())
            .filter_map(|job| field(job, "steps"))
            .filter_map(|s| s.as_sequence())
            .collect(),
        None => field(root, "runs")
            .and_then(|r| r.as_mapping())
            .and_then(|r| field(r, "steps"))
            .and_then(|s| s.as_sequence())
            .into_iter()
            .collect(),
    };
    seqs.into_iter()
        .flat_map(|seq| seq.iter().filter_map(|s| s.as_mapping()))
        .collect()
}

/// Rule 1 over every step in `doc`.
fn check_doc(doc: &serde_yaml::Value, file: &str, out: &mut Vec<Violation>) {
    for step in steps_of(doc) {
        let is_cache_step =
            str_field(step, "uses").is_some_and(|s| s.starts_with(CACHE_ACTION_PREFIX));
        if !is_cache_step {
            continue;
        }
        let key = field(step, "with")
            .and_then(|w| w.as_mapping())
            .and_then(|w| str_field(w, "key"));
        if let Some(key) = key
            && is_untoolchained_lockfile_key(key)
        {
            out.push(Violation {
                file: file.to_string(),
                step: step_name(step),
                key: key.to_string(),
            });
        }
    }
}

/// True when `if:`/`save-if:` text gates the step on the default branch.
fn gates_on_main(cond: Option<&str>) -> bool {
    cond.is_some_and(|c| c.contains(MAIN_REF))
}

/// Rule 2 for one step: `Some(why)` when this step writes a cache off `main`.
fn save_violation(step: &serde_yaml::Mapping) -> Option<String> {
    let uses = str_field(step, "uses")?;
    // `actions/cache/restore` only reads; it has no post-save at all.
    if uses.starts_with(CACHE_RESTORE_PREFIX) {
        return None;
    }
    if gates_on_main(str_field(step, "if")) {
        return None;
    }
    if uses.starts_with(CACHE_ACTION_PREFIX) {
        return Some(
            "actions/cache saves on every ref; add `if: … github.ref == 'refs/heads/main'` \
             or switch this step to actions/cache/restore@<same sha>"
                .to_string(),
        );
    }
    if uses.starts_with(RUST_CACHE_PREFIX) {
        let save_if = field(step, "with")
            .and_then(|w| w.as_mapping())
            .and_then(|w| field(w, "save-if"));
        return match save_if {
            None => Some(
                "Swatinem/rust-cache saves by default; set \
                 `save-if: ${{ github.ref == 'refs/heads/main' }}` (or `save-if: false`)"
                    .to_string(),
            ),
            Some(v) if v.as_bool() == Some(false) => None,
            Some(v) if gates_on_main(v.as_str()) => None,
            Some(v) if v.as_str().is_some_and(|s| s.trim() == "false") => None,
            Some(_) => Some(
                "Swatinem/rust-cache `save-if` neither references refs/heads/main nor is false"
                    .to_string(),
            ),
        };
    }
    None
}

/// Rule 2 over every step in `doc`. `always_reachable` is true for composite
/// actions (no `on:` of their own; any PR workflow can call them).
fn check_save_scope(
    doc: &serde_yaml::Value,
    file: &str,
    always_reachable: bool,
    out: &mut Vec<SaveViolation>,
) {
    if !always_reachable && !is_pr_reachable(doc) {
        return;
    }
    for step in steps_of(doc) {
        if let Some(why) = save_violation(step) {
            out.push(SaveViolation {
                file: file.to_string(),
                step: step_name(step),
                why,
            });
        }
    }
}

/// True when a PR (or merge-queue, or a non-`main` branch push) can start this
/// workflow — i.e. when a run of it would write a ref-scoped, write-only cache.
fn is_pr_reachable(doc: &serde_yaml::Value) -> bool {
    trigger_keys(doc)
        .into_iter()
        .any(|(name, filters)| match name {
            "pull_request" | "pull_request_target" | "merge_group" => true,
            "push" => {
                let branches = filters
                    .and_then(|f| f.as_mapping())
                    .and_then(|f| field(f, "branches"))
                    .and_then(|b| b.as_sequence());
                match branches {
                    // `push:` with no branch filter fires on every branch.
                    None => true,
                    Some(b) => b.iter().any(|x| x.as_str() != Some("main")),
                }
            }
            _ => false,
        })
}

fn parse(path: &Path) -> Result<serde_yaml::Value> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn yaml_files_in(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .collect();
    entries.sort();
    Ok(entries)
}

pub fn run(repo_root: &Path) -> Result<()> {
    let gh = repo_root.join(".github");

    let mut violations = Vec::new();
    let mut saves = Vec::new();

    for path in yaml_files_in(&gh.join("workflows"))? {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let doc = parse(&path)?;
        check_doc(&doc, &name, &mut violations);
        check_save_scope(&doc, &name, false, &mut saves);
    }

    // Composite actions: `.github/actions/<name>/action.yml`.
    let actions_dir = gh.join("actions");
    if actions_dir.is_dir() {
        let mut dirs: Vec<_> = std::fs::read_dir(&actions_dir)
            .with_context(|| format!("read {}", actions_dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        for dir in dirs {
            for path in yaml_files_in(&dir)? {
                let name = format!(
                    "actions/{}/{}",
                    dir.file_name().unwrap_or_default().to_string_lossy(),
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
                let doc = parse(&path)?;
                check_doc(&doc, &name, &mut violations);
                check_save_scope(&doc, &name, true, &mut saves);
            }
        }
    }

    if violations.is_empty() && saves.is_empty() {
        println!(
            "cache-key-lint OK (every Cargo.lock-hashing cache key also keys on the toolchain; \
             no PR-reachable cache writes)"
        );
        return Ok(());
    }

    let mut msg = String::new();
    if !violations.is_empty() {
        let lines: Vec<String> = violations
            .iter()
            .map(|v| format!("  {}: step \"{}\" (key: {})", v.file, v.step, v.key))
            .collect();
        msg.push_str(&format!(
            "cache-key-lint: {} actions/cache key(s) hash Cargo.lock without keying on the \
             toolchain:\n{}\n\
             Fix: add the toolchain to the key, e.g. `${{{{ steps.<setup-rust-id>.outputs.toolchain }}}}` \
             when the job calls ./.github/actions/setup-rust with an `id:`, or \
             `hashFiles('rust-toolchain.toml', 'Cargo.lock')` when it doesn't.\n",
            violations.len(),
            lines.join("\n")
        ));
    }
    if !saves.is_empty() {
        let lines: Vec<String> = saves
            .iter()
            .map(|v| format!("  {}: step \"{}\" — {}", v.file, v.step, v.why))
            .collect();
        msg.push_str(&format!(
            "cache-key-lint: {} cache step(s) WRITE a cache from a PR-reachable run. The \
             repository cache is a 10 GB LRU bucket; a PR-scoped entry is write-only (no other \
             ref can read it) and evicts the main-scope entries every job restores from. Only \
             `main` and scheduled runs may save:\n{}\n",
            saves.len(),
            lines.join("\n")
        ));
    }
    Err(anyhow!(msg.trim_end().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations_for(yaml: &str, file: &str) -> Vec<Violation> {
        let doc: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let mut out = Vec::new();
        check_doc(&doc, file, &mut out);
        out
    }

    fn save_scope_violations_for(yaml: &str, file: &str) -> Vec<SaveViolation> {
        let doc: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let mut out = Vec::new();
        check_save_scope(&doc, file, false, &mut out);
        out
    }

    fn composite_violations_for(yaml: &str, file: &str) -> (Vec<Violation>, Vec<SaveViolation>) {
        let doc: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let (mut keys, mut saves) = (Vec::new(), Vec::new());
        check_doc(&doc, file, &mut keys);
        check_save_scope(&doc, file, true, &mut saves);
        (keys, saves)
    }

    #[test]
    fn toolchain_output_reference_passes() {
        let yaml = "jobs:\n  \
                      build:\n    steps:\n      - name: Cache cargo\n        \
                        uses: actions/cache@v5\n        with:\n          \
                        key: ${{ runner.os }}-cargo-${{ steps.rust.outputs.toolchain }}-${{ hashFiles('Cargo.lock') }}\n";
        let violations = violations_for(yaml, "ok.yml");
        assert!(
            violations.is_empty(),
            "a key referencing steps.*.outputs.toolchain must pass"
        );
    }

    #[test]
    fn rust_toolchain_toml_hash_passes() {
        let yaml = "jobs:\n  \
                      build:\n    steps:\n      - name: Cache cargo\n        \
                        uses: actions/cache@v5\n        with:\n          \
                        key: ${{ runner.os }}-cargo-${{ hashFiles('rust-toolchain.toml', 'Cargo.lock') }}\n";
        let violations = violations_for(yaml, "ok2.yml");
        assert!(
            violations.is_empty(),
            "hashing rust-toolchain.toml alongside Cargo.lock must pass"
        );
    }

    #[test]
    fn lockfile_only_key_fails() {
        let yaml = "jobs:\n  \
                      build:\n    steps:\n      - name: Cache cargo\n        \
                        uses: actions/cache@v5\n        with:\n          \
                        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}\n";
        let violations = violations_for(yaml, "bad.yml");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].file, "bad.yml");
    }

    #[test]
    fn key_with_no_lockfile_reference_passes() {
        // Not every cache is a cargo cache (e.g. a Playwright browser cache) --
        // this lint only fires when the key actually hashes Cargo.lock.
        let yaml = "jobs:\n  \
                      e2e:\n    steps:\n      - name: Cache Playwright browsers\n        \
                        uses: actions/cache@v5\n        with:\n          \
                        key: ms-playwright-${{ runner.os }}-${{ steps.pw_version.outputs.ver }}\n";
        let violations = violations_for(yaml, "playwright.yml");
        assert!(violations.is_empty());
    }

    #[test]
    fn non_cache_step_with_lockfile_mention_is_ignored() {
        let yaml = "jobs:\n  \
                      build:\n    steps:\n      - name: Not a cache step\n        \
                        run: echo Cargo.lock\n";
        let violations = violations_for(yaml, "not-cache.yml");
        assert!(violations.is_empty());
    }

    #[test]
    fn multiple_violations_in_one_file_are_all_reported() {
        let yaml = "jobs:\n  \
                      a:\n    steps:\n      - uses: actions/cache@v5\n        with:\n          \
                        key: ${{ hashFiles('**/Cargo.lock') }}\n  \
                      b:\n    steps:\n      - uses: actions/cache@v5\n        with:\n          \
                        key: ${{ hashFiles('Cargo.lock') }}\n";
        let violations = violations_for(yaml, "two.yml");
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn pr_workflow_cache_save_off_main_fails() {
        let yml = "on:\n  pull_request:\njobs:\n  a:\n    steps:\n      - name: c\n        uses: actions/cache@v5\n        with:\n          key: k\n      - name: s\n        uses: Swatinem/rust-cache@v2\n";
        let v = save_scope_violations_for(yml, "x.yml");
        assert_eq!(v.len(), 2, "{v:?}");
    }

    #[test]
    fn main_gated_saves_and_restores_pass() {
        let yml = "on:\n  merge_group:\njobs:\n  a:\n    steps:\n      - uses: actions/cache@v5\n        if: github.ref == 'refs/heads/main'\n        with:\n          key: k\n      - uses: actions/cache/restore@v5\n        with:\n          key: k\n      - uses: Swatinem/rust-cache@v2\n        with:\n          save-if: ${{ github.ref == 'refs/heads/main' }}\n      - uses: Swatinem/rust-cache@v2\n        with:\n          save-if: false\n";
        assert!(save_scope_violations_for(yml, "x.yml").is_empty());
    }

    #[test]
    fn main_only_and_scheduled_workflows_may_save() {
        let push_main = "on:\n  push:\n    branches: [main]\njobs:\n  a:\n    steps:\n      - uses: actions/cache@v5\n        with:\n          key: k\n";
        let sched = "on:\n  schedule:\n    - cron: '0 3 * * *'\njobs:\n  a:\n    steps:\n      - uses: Swatinem/rust-cache@v2\n";
        assert!(save_scope_violations_for(push_main, "a.yml").is_empty());
        assert!(save_scope_violations_for(sched, "b.yml").is_empty());
    }

    #[test]
    fn unfiltered_branch_push_is_pr_reachable() {
        // `on: push:` with no `branches:` fires on every feature branch, so a
        // save there is just as ref-scoped and write-only as a PR's.
        let yml = "on:\n  push:\njobs:\n  a:\n    steps:\n      - uses: Swatinem/rust-cache@v2\n";
        assert_eq!(save_scope_violations_for(yml, "x.yml").len(), 1);
    }

    #[test]
    fn composite_action_steps_are_checked() {
        // Composite actions run under every caller, including PR workflows.
        let action = "runs:\n  using: composite\n  steps:\n    - uses: actions/cache@abc\n      with:\n        key: ${{ runner.os }}-${{ hashFiles('Cargo.lock') }}\n";
        let (keys, saves) = composite_violations_for(action, "setup-rust/action.yml");
        assert_eq!(keys.len(), 1, "lockfile key without toolchain");
        assert_eq!(saves.len(), 1, "unconditional save");
    }
}
