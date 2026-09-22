# CI Audit Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix every error and address every finding in the 2026-09 CI/CD audit: broken builds and red workflows, the pre-push time waste, the coverage gaps (toolchain-bump rustdoc, pre-merge Windows, `vox-gui` tests, GUI Playwright, examples, liveness), cache pressure, and the dead surface (fleet tooling, stale workflows, dead YAML).

**Architecture:** All changes land on the hosted-primary CI branch `claude/cicd-complexity-tradeoffs-b62a67`, on top of the 9-task migration already there. Rust fixes are test-first. Workflow changes are verified locally by `vox ci ssot-drift` (parses every workflow and runs `workflow_policy_guard`), `required-context-guard`, `workflow-concurrency-guard --strict`, and `crates/vox-cli/tests/ci_workflow_contract.rs`. The required context `Check, Build, and Test (Rust)` becomes an aggregator over a `linux` leg and a path-conditioned `ui` leg; a merge-queue Windows leg ships warn-only. Anything only GitHub can prove is listed under **Bootstrap & post-merge verification**, not claimed done.

**Tech Stack:** Rust (`vox-cli`, `vox-cli-ci`, `vox-doc-pipeline`, `vox-db`, `vox-populi`, `vox-quantize`, …), GitHub Actions YAML, `gh` CLI, `yq`, pnpm/Playwright (`crates/vox-gui/ui`).

**Spec:** `docs/src/architecture/ci-cd-audit-findings-2026.md` (the audit). Background: `docs/superpowers/specs/2026-09-21-hosted-primary-ci-design.md`. Grill record (binding rulings): `.superpowers/review/2026-09-22-ci-audit-remediation-grill.md`.

## Global Constraints

- Branch: `claude/cicd-complexity-tradeoffs-b62a67` (worktree `.claude/worktrees/cicd-complexity-tradeoffs-b62a67`). Do not push or open a PR without the user's go-ahead.
- Timeout caps are fixed: PR / merge_group / branch-push jobs ≤ **30** min, everything else ≤ **180** — enforced by `workflow_policy_guard`. Never raise a cap to make a job fit.
- Required branch-protection context name stays exactly `Check, Build, and Test (Rust)`, owned only by `ci.yml` (`required_context_guard`). Only jobs in `ci.yml` can block a merge — a check that must block goes in `ci.yml` under the aggregator. <!-- GRILL Ex 10, 14 -->
- Every scheduled workflow's `name:` must be listed in `nightly-report.yml` `on.workflow_run.workflows` (guard-enforced). Removing a schedule means removing the list entry in the same commit.
- No `runs-on: self-hosted` outside `SELF_HOSTED_ALLOWLIST` in `crates/vox-cli-ci/src/workflow_policy_guard.rs`.
- Permissions: grant only the scopes a job **currently uses successfully**; never widen a job to make a currently-failing write succeed. <!-- GRILL Ex 12 -->
- Never `cargo fmt --all`; format with `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`.
- New `pub fn` needs a same-file test (tdd-guard pre-commit hook).
- After any `CiCmd` change, regenerate in order: `cargo run -p vox-cli -- ci command-sync`, `cargo run -p vox-cli -- ci gui-surface-coverage --write`, `cargo run -p vox-cli -- ci doc-inventory generate`, then `UPDATE_CLI_CATALOG_BASELINE=1 cargo test -p vox-cli --test command_catalog_paths_baseline`. Never hand-edit generated artifacts.
- Credentials are the user's: this plan never sets, reads, or rotates a secret.
- Live repo-state changes — `gh workflow disable/enable`, `gh cache delete`, pushes, PR creation, admin/bypass merges, dispatching workflows on `main`, branch-protection edits — need explicit user confirmation, each time. <!-- GRILL Ex 3, 17 -->
- Commit messages end with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

### Task 1: Fix the feature-gated compile errors (vox-populi, mens-candle-cuda)

`setup-e2e.yml` is red every night on these; default-feature builds never compile them.

**Files:**
- Modify: `crates/vox-populi/src/mens/hub.rs` — the local-directory branch of the download function (the `return Ok(DownloadedModelFiles { cache_dir: local_path, config, weights, tokenizer })` literal)
- Modify: `crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs` — the struct literal ending `head_dim, q_norm, k_norm, }` that lists `q_norm`/`k_norm` twice

- [ ] **Step 1: Reproduce** — `cargo check -p vox-populi --features mens-hf-hub` → FAIL `E0063 missing fields chat_template and tokenizer_config in initializer of DownloadedModelFiles`.
- [ ] **Step 2: Fill the two fields** in the local-directory branch (mirrors how that branch finds `tokenizer`):

```rust
        let tokenizer_config = Some(local_path.join("tokenizer_config.json")).filter(|p| p.is_file());
        let chat_template = Some(local_path.join("chat_template.jinja")).filter(|p| p.is_file());
        return Ok(DownloadedModelFiles {
            cache_dir: local_path,
            config,
            weights,
            tokenizer,
            tokenizer_config,
            chat_template,
        });
```

- [ ] **Step 3: Undo the union-resolved merge in the cuda layer loader** <!-- AMENDED: #2 — the duplicate fields are the visible half of a move+merge; the second `let` shadows the load-bearing F32 `load_norm` bindings -->
  - Delete the **second** pair of bindings `let q_norm = vb_mmap.get(head_dim, &format!("{layer_prefix}.self_attn.q_norm.weight"))… / let k_norm = …` (just before `let attn = crate::model::Qwen2Attention {`). Keep `let q_norm = load_norm(&q_key); let k_norm = load_norm(&k_key);` (the "confirmed load-bearing" block that converts to `DType::F32`).
  - Delete the duplicated `q_norm,` / `k_norm,` after `head_dim,` in the `Qwen2Attention` literal (keep the first occurrence).
  - Verify with `grep -c 'let q_norm' crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs` → `1`.
- [ ] **Step 4: Verify** — `cargo check -p vox-populi --features mens-hf-hub && cargo check -p vox-populi --features mens-train` → PASS. `cargo check -p vox-plugin-mens-candle-cuda` → PASS, or a build-script failure for missing `nvcc`; if `nvcc` blocks it, confirm by reading that the duplicate lines are gone and record "cuda plugin verified post-merge by setup-e2e.yml".
- [ ] **Step 5: Commit** `fix(mens): fill hub local-dir fields and drop duplicate q/k_norm (setup-e2e red)`.

---

### Task 2: Make the full-workspace clippy clean

The nightly `full` job runs `cargo clippy --workspace --exclude vox-gui --all-targets --locked -- -D warnings`; today it fails. Inventory log: `.superpowers/review/2026-09-22-ci-audit-remediation-clippy.log`.

| Crate | File:line | Lint |
|---|---|---|
| vox-quantize | `src/read.rs:181,182,183,186,198,199,200,201` | 8 × `collapsible_if` |
| vox-db | `src/research_doc_io.rs:43` | `never_loop` (**check for a logic bug**) |
| vox-db | `src/research_doc_io.rs:109,110,139` | `collapsible_if` |
| vox-db | `src/research_doc_io.rs:112` | `io_other_error` |
| vox-db | `src/research_doc_io.rs:219` | `unnecessary_lazy_evaluations` |
| vox-db | `src/research_pipeline.rs:473,480` | `collapsible_if` |
| vox-db | `src/research_pipeline.rs:993,996` | `manual_clamp` |
| vox-db | `src/temporal_claims.rs:72,73,80,81` | `collapsible_if` |

Caveats: crates depending on `vox-db` couldn't be linted while it failed, so the table is a floor — Step 3's full re-run is the acceptance check. `vox-db`'s `research_*` files are under active development elsewhere; keep edits lint-only unless `never_loop` is a real bug.

- [ ] **Step 1: Reproduce per crate** — `cargo clippy -p vox-quantize -p vox-db --all-targets --locked -- -D warnings`.
- [ ] **Step 2: Fix each lint at its source** (clippy's suggested form). For `never_loop` at `research_doc_io.rs:43`: the loop is a `#[cfg(windows)]` sharing-violation rename retry that only "never loops" off-Windows — do **not** remove it. Put `#[cfg_attr(not(windows), allow(clippy::never_loop))]` on the `loop` (or on the enclosing fn if attributes on the expression are rejected), with a comment naming the Windows retry. <!-- AMENDED: #9 — both original options broke or couldn't test the Windows retry -->
- [ ] **Step 3: Verify** — `cargo clippy --keep-going --workspace --exclude vox-gui --all-targets --locked -- -D warnings` → exit 0 (fix anything newly surfaced the same way); `cargo test -p vox-quantize -p vox-db` → PASS.
- [ ] **Step 4: Commit** `fix(clippy): clear the workspace lint wave (nightly full job)`.

---

### Task 3: Pre-push — kill the 3-second step padding, drop the CodeRabbit reminder, lint docs in-process

**Files:**
- Modify: `crates/vox-cli/src/commands/ci/pre_push.rs` — `fn run_step_with_heartbeat`; the re-push advisory printing `@coderabbitai review`; `fn step_doc_frontmatter_full`, `fn step_doc_frontmatter_scoped`
- Modify: `crates/vox-doc-pipeline/src/pipeline/mod.rs` — `pub fn run`
- Modify: `crates/vox-doc-pipeline/src/pipeline/lint.rs` — `fn lint_readme_sync_paths` visibility

- [ ] **Step 1: Failing heartbeat test** (in `pre_push.rs` tests):

```rust
    #[test]
    fn heartbeat_does_not_pad_fast_steps() {
        let t0 = std::time::Instant::now();
        run_step_with_heartbeat("noop", t0, || Ok(())).unwrap();
        assert!(t0.elapsed() < std::time::Duration::from_millis(500), "{:?}", t0.elapsed());
    }
```
`cargo test -p vox-cli --lib heartbeat_does_not_pad_fast_steps` → FAIL (~3 s).

- [ ] **Step 2: Replace the sleep loop with a channel wait** so stop wakes the thread immediately:

```rust
    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
    let label_owned = label.to_string();
    let bg = thread::spawn(move || {
        let t0 = Instant::now();
        while let Err(std::sync::mpsc::RecvTimeoutError::Timeout) =
            stop_rx.recv_timeout(vox_config::timeouts::D_3S)
        {
            let step_s = t0.elapsed().as_secs();
            let total_s = push_start.elapsed().as_secs();
            eprintln!(
                "pre-push: still running `{}` — step {:02}:{:02} | total {:02}:{:02}",
                label_owned, step_s / 60, step_s % 60, total_s / 60, total_s % 60,
            );
        }
    });
    let out = f();
    let _ = stop_tx.send(());
    let _ = bg.join();
    out
```
Delete the `use std::sync::atomic::{AtomicBool, Ordering};` line (used only by the old heartbeat); keep `Arc` (still used elsewhere in the file). Test → PASS. <!-- AMENDED: #8 — unused import fails clippy -D warnings -->

- [ ] **Step 3: Remove the CodeRabbit re-push advisory** — delete `fn print_pr_review_discipline_hint` and its call site (it only prints the CodeRabbit reminder). AGENTS.md's "`vox ci pre-push` prints an **advisory** reminder on re-push" sentence is fixed in Task 15. <!-- AMENDED: #17 -->

- [ ] **Step 4: Failing in-process lint tests** (in `crates/vox-doc-pipeline/src/pipeline/mod.rs` tests). `lint_in` must read only under `root`, never the process cwd: <!-- GRILL Ex 9 -->

```rust
    fn fixture_root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("vox-doc-lint-{}-{:?}", std::process::id(), std::thread::current().id()));
        std::fs::create_dir_all(root.join("docs/src")).unwrap();
        root
    }

    #[test]
    fn lint_in_reports_missing_paths_without_exiting() {
        let root = fixture_root();
        assert_eq!(lint_in(&root, &["--paths=nope.md".to_string()]), 1);
    }

    #[test]
    fn lint_in_reads_readme_sync_under_root_not_cwd() {
        // In-sync pair under `root` must lint clean (a cwd-reading lint_in fails here:
        // cargo's cwd is crates/vox-doc-pipeline, which has no docs/src).
        let root = fixture_root();
        write_readme_sync_fixture(&root, "Same text.", "Same text.");
        assert_eq!(lint_in(&root, &["--paths=index.mdx".to_string()]), 0);
        // Drift under `root` must be reported.
        write_readme_sync_fixture(&root, "Same text.", "Different text.");
        assert_ne!(lint_in(&root, &["--paths=index.mdx".to_string()]), 0);
    }
```
`write_readme_sync_fixture(root, readme_body, mdx_body)` is a test helper that writes `README.md` with the `<!-- ANCHOR: why_vox -->` … block and `docs/src/index.mdx` (valid frontmatter + the matching `SYNC-FROM-README` block) exactly in the marker shapes `lint_readme_sync_paths` / `ReadmeSyncMissingAnchor` expect (lint.rs:636-694 — read them and copy the marker syntax verbatim). Choose the lint args so the README-sync check runs for this fixture (check `lint_in`'s dispatch; if README sync only runs in full `--lint-only` mode, use that and make the fixture otherwise lint-clean). Mutation check: temporarily make `lint_in` pass `Path::new("README.md")` instead of `root.join("README.md")` and confirm the first assertion fails, then restore. <!-- AMENDED: #8 — original fixture lacked anchors, so the test passed for root- and cwd-reading implementations alike -->

- [ ] **Step 5: Extract `pub fn lint_in(root: &Path, args: &[String]) -> i32`** from `run()`:
  - `docs_src = root.join("docs/src")`;
  - every `std::process::exit(1)` → `return 1`; success → `0`;
  - lint calls go through the root-taking functions: `lint::collect_lint_errors_target_with_root(target, &mut errors, root)` (never the cwd-reading `collect_lint_errors*` wrappers) and `lint::lint_readme_sync_paths(&root.join("README.md"), &root.join("docs/src/index.mdx"), &mut errors)` — raise `lint_readme_sync_paths` to `pub(crate)`;
  - never call `std::env::set_current_dir` (process-global; the heartbeat thread runs concurrently).
  `run()` becomes `std::process::exit(lint_in(Path::new("."), &std::env::args().collect::<Vec<_>>()))`; corpus mode keeps its behavior. Tests → PASS.

- [ ] **Step 6: Call it in-process from pre-push** (vox-cli already depends on vox-doc-pipeline):

```rust
fn step_doc_frontmatter_full(root: &Path) -> Result<()> {
    match vox_doc_pipeline::pipeline::lint_in(root, &["--lint-only".to_string()]) {
        0 => Ok(()),
        c => bail!("vox-doc-pipeline lint failed (exit {c})"),
    }
}
```
and the scoped variant passing `format!("--paths={}", rel_paths.join(","))` (keep its empty-list early return).

- [ ] **Step 7: Verify** — `cargo test -p vox-doc-pipeline && cargo test -p vox-cli --lib commands::ci`; `cargo build -p vox-cli && time ./target/debug/vox ci pre-push`; record the doc-lint step time before/after. Expected: doc-lint drops from ~69 s to a few seconds; no flat `OK (3000ms)` floors.
- [ ] **Step 8: Commit** `perf(pre-push): wake heartbeat immediately, lint docs in-process under root, drop CodeRabbit notice`.

---

### Task 4: Make `compute_affected` seed the crates that read `examples/`

Examples-only diffs are skipped by `ci.yml`'s Rust prefilter today (no Rust job runs at all), and `compute_affected` returns `Affected::None` for them. <!-- GRILL Ex 5 --> <!-- AMENDED: #10 — corrected: today they are skipped, not run full --> <!-- AMENDED: D3 — list is 8 crates; vox-cli-tests / vox-populi mention examples/ only in comments -->

**Files:** Modify `crates/vox-cli-ci/src/affected.rs` (`pub fn compute_affected`, test `golden_only_none_affected`)

- [ ] **Step 1: Failing tests** — replace `golden_only_none_affected` with:

```rust
    #[test]
    fn examples_only_seeds_examples_consumers() {
        let got = compute_affected(&["examples/golden/foo.vox".into()], &BTreeMap::new());
        let want: BTreeSet<String> = EXAMPLES_CONSUMERS.iter().map(|s| s.to_string()).collect();
        assert_eq!(got, Affected::Crates(want));
    }

    fn rs_files_mention_examples(dir: &std::path::Path) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else { return false };
        entries.flatten().any(|e| {
            let p = e.path();
            if p.is_dir() {
                rs_files_mention_examples(&p)
            } else {
                // Comment-only mentions don't count (vox-cli-tests and vox-populi name
                // `examples/` in doc comments but read their own local fixtures).
                p.extension().is_some_and(|x| x == "rs")
                    && std::fs::read_to_string(&p)
                        .unwrap_or_default()
                        .lines()
                        .any(|l| !l.trim_start().starts_with("//") && l.contains("examples/"))
            }
        })
    }

    /// Every crate whose integration tests (`tests/**/*.rs`) read `examples/` must be in
    /// EXAMPLES_CONSUMERS, or an examples-only PR silently skips its tests. In-`src`
    /// `#[cfg(test)]` readers can't be told apart from non-test mentions by a scan, so
    /// they are listed by hand in SRC_TEST_READERS.
    #[test]
    fn examples_consumers_matches_the_tree() {
        const SRC_TEST_READERS: &[&str] = &["vox-ml-cli"]; // eval_local_prompt.rs golden smoke test
        let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut expected: BTreeSet<String> = SRC_TEST_READERS.iter().map(|s| s.to_string()).collect();
        for krate in std::fs::read_dir(&crates_dir).unwrap().flatten() {
            if rs_files_mention_examples(&krate.path().join("tests")) {
                expected.insert(krate.file_name().to_string_lossy().to_string());
            }
        }
        let listed: BTreeSet<String> = EXAMPLES_CONSUMERS.iter().map(|s| s.to_string()).collect();
        assert_eq!(expected, listed, "update EXAMPLES_CONSUMERS");
    }
```
`cargo test -p vox-cli-ci affected` → FAIL (no `EXAMPLES_CONSUMERS`).

- [ ] **Step 2: Implement**

```rust
/// Crates whose tests read `examples/` (kept honest by `examples_consumers_matches_the_tree`).
pub const EXAMPLES_CONSUMERS: &[&str] = &[
    "vox-audit",
    "vox-cli",
    "vox-codegen",
    "vox-compiler",
    "vox-integration-tests",
    "vox-ml-cli",
    "vox-orchestrator-mcp",
    "vox-workflow-runtime",
];
```
In `compute_affected`, make `seeds` mutable and, before the empty check:
```rust
    if changed_files.iter().any(|f| f.starts_with("examples/")) {
        seeds.extend(EXAMPLES_CONSUMERS.iter().map(|s| s.to_string()));
    }
```
- [ ] **Step 3: Verify** — `cargo test -p vox-cli-ci affected` PASS (existing affected tests unchanged).
- [ ] **Step 4: Commit** `ci(affected): examples-only diffs select the crates whose tests read examples/`.

---

### Task 5: Restructure the required gate — `linux` + `ui` legs under an aggregator, warn-only Windows

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/nightly.yml` (Windows cache seed)
- Modify: `crates/vox-cli/tests/ci_workflow_contract.rs` (`ci_gate_is_hosted_capped_and_owns_required_context`)

Decisions (grill): <!-- GRILL Ex 1, 2, 4, 10, 11, 14 -->
- Windows is **warn-only**: it runs on `merge_group` but is **not** in `gate.needs`. The user flips it to enforcing (see Bootstrap & post-merge verification).
- The GUI `ui` leg **blocks**: it always runs; its steps skip when no file under `crates/vox-gui/` (or `crates/vox-orchestrator/src/orch_daemon/mod.rs`, which a vitest reads) changed, so the job result is `success` either way. Detection **fails closed** (any error → run the UI tests). <!-- AMENDED: #11 — UI tests read `crates/vox-gui/tauri.conf.json` and the orch_daemon file; a narrower filter lets a Rust-only PR break vitest unseen -->
- Every diff-dependent decision reads `/tmp/changed.txt` written by one `git diff` whose failure fails the step — never `git diff | grep -q` (under Actions' `bash -eo pipefail`, an early `grep -q` exit SIGPIPEs `git diff` and the condition silently reads false). <!-- AMENDED: #4 -->
- **Rollback:** if the new aggregator blocks every PR (e.g. a pre-existing red vitest), the revert PR cannot pass it either. Recovery is user-gated: an admin bypass merge of the revert. Step 6's local UI runs exist to make this unlikely. <!-- AMENDED: #13 -->
- On a toolchain bump (`rust-toolchain.toml` in the diff) the `linux` leg runs clippy **and** rustdoc `-D warnings`, and skips nextest (nightly runs it after merge). No separate toolchain workflow.
- cargo-deny in the required leg checks only `licenses bans sources` (diff-dependent); advisories stay nightly-only.

- [ ] **Step 1: Update the contract test first:**

```rust
#[test]
fn ci_gate_is_hosted_capped_and_owns_required_context() {
    let yml = include_str!("../../../.github/workflows/ci.yml");
    assert!(yml.contains("name: Check, Build, and Test (Rust)"));
    assert!(yml.contains("runs-on: ubuntu-latest"));
    assert!(yml.contains("runs-on: windows-latest"));
    assert!(yml.contains("timeout-minutes: 30"));
    assert!(!yml.contains("self-hosted"));
    assert!(yml.contains("sed 's/-p vox-gui//g'"), "affected args must never build vox-gui");
    assert!(yml.contains("needs: [linux, ui]"), "required context aggregates linux + ui; windows is warn-only");
    assert!(yml.contains("cargo deny check licenses bans sources"), "no date-dependent advisories in the required leg");
    assert!(yml.contains("RUSTDOCFLAGS"), "toolchain bumps must run rustdoc -D warnings in the required leg");
    assert!(yml.contains("playwright test --project=chromium"), "UI changes must pass Playwright before merge");
    assert!(yml.contains("examples/|\\.github/workflows/)"), "examples-only diffs must reach the affected step");
}
```
Run → FAIL.

- [ ] **Step 2: `ci.yml` — rename the current `gate` job id to `linux`** (`name: Linux (fmt, guards, clippy, tests — affected)`) and inside it:
  - first step: `- name: Start budget clock` / `run: echo "GATE_START=$(date +%s)" >> "$GITHUB_ENV"` (shell: bash);
  - in `Affected crates`: add `[ -n "$BASE_SHA" ] || { echo "::error::no base SHA"; exit 1; }` before the `git diff`, and widen the prefilter regex to `'^(crates/|Cargo\.(toml|lock)|\.cargo/|rust-toolchain\.toml|examples/|\.github/workflows/)'` — `examples/` goes **before** `\.github/workflows/` so the substring `\.github/workflows/)` that `selective_ci_workflow_changes_force_rust_gate` (ci_workflow_contract.rs) asserts survives (Task 4 makes examples-only select real crates); <!-- AMENDED: #5 -->
  - after `Affected crates`, add:
```yaml
      - name: Toolchain bump?
        id: bump
        shell: bash
        run: |
          if grep -qx 'rust-toolchain.toml' /tmp/changed.txt; then
            echo "toolchain=true" >> "$GITHUB_OUTPUT"
          else
            echo "toolchain=false" >> "$GITHUB_OUTPUT"
          fi
      - name: cargo-deny licenses/bans/sources (dependency changes only)
        shell: bash
        run: |
          if grep -qE '(^|/)Cargo\.(toml|lock)$|^deny\.toml$' /tmp/changed.txt; then
            cargo deny check licenses bans sources
          else
            echo "no dependency changes"
          fi
```
  (add `cargo-deny` to the existing `taiki-e/install-action` `tool:` list: `tool: cargo-nextest,cargo-deny`);
  - after `Clippy (affected)`:
```yaml
      - name: Rustdoc -D warnings (toolchain bump)
        if: steps.bump.outputs.toolchain == 'true'
        run: cargo doc --workspace --exclude vox-gui --no-deps --locked
        env:
          RUSTDOCFLAGS: -D warnings
```
  - change `Tests (affected)`'s `if:` to `steps.affected.outputs.p_args != '' && steps.bump.outputs.toolchain != 'true'`;
  - last step:
```yaml
      - name: Warn when the leg nears its 30-min cap
        if: always()
        shell: bash
        run: |
          mins=$(( ($(date +%s) - GATE_START) / 60 ))
          echo "linux leg ran ${mins} min"
          if [ "$mins" -ge 24 ]; then
            echo "::warning::linux leg took ${mins} min (>80% of the 30-min cap) — cache, shard, or move work to nightly.yml"
          fi
```

- [ ] **Step 3: Add the `ui` leg:**
```yaml
  ui:
    name: GUI UI (typecheck, vitest, Playwright — runs only when crates/vox-gui changes)
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0
      - name: UI changed?
        id: ui
        shell: bash
        env:
          BASE_SHA: ${{ github.event.pull_request.base.sha || github.event.merge_group.base_sha }}
        run: |
          # Fail closed: an empty base or a git error runs the UI tests.
          if [ -n "$BASE_SHA" ] && git diff --name-only "$BASE_SHA...HEAD" > /tmp/changed.txt \
             && ! grep -qE '^crates/vox-gui/|^crates/vox-orchestrator/src/orch_daemon/mod\.rs$' /tmp/changed.txt; then
            echo "run=false" >> "$GITHUB_OUTPUT"
          else
            echo "run=true" >> "$GITHUB_OUTPUT"
          fi
      - uses: pnpm/action-setup@v6
        if: steps.ui.outputs.run == 'true'
        with:
          version: 11
      - uses: actions/setup-node@v7
        if: steps.ui.outputs.run == 'true'
        with:
          node-version: '24'
          package-manager-cache: false   # no PR-scoped cache writes (cache is at 9.8/10 GB)
      - name: Install, typecheck, vitest, Playwright
        if: steps.ui.outputs.run == 'true'
        working-directory: crates/vox-gui/ui
        run: |
          pnpm install --frozen-lockfile
          pnpm typecheck
          pnpm test
          pnpm exec playwright install --with-deps chromium
          pnpm exec playwright test --project=chromium
```
(Only chromium is installed; `pnpm test:e2e` would also select `firefox-review` — match nightly's `--project=chromium`.) <!-- AMENDED: #12 -->
(Confirm pnpm/node pins with `vox ci node-pnpm-ssot-guard`; adjust to its expected values.)

- [ ] **Step 4: Add the warn-only Windows leg and the aggregator:**
```yaml
  windows:
    # Warn-only until a seeded run proves it fits (see plan "Bootstrap & post-merge
    # verification"); deliberately NOT in gate.needs.
    name: Windows compile check (merge queue, advisory)
    if: github.event_name == 'merge_group'
    runs-on: windows-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v7
      - uses: ./.github/actions/setup-rust
        with:
          cache: "false"
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: workspace-windows
          save-if: false   # nightly.yml windows-cache-seed on main is the only writer
      - run: cargo check --workspace --exclude vox-gui --all-targets --locked

  gate:
    # Branch-protection required context — do not rename (required_context_guard).
    name: Check, Build, and Test (Rust)
    needs: [linux, ui]
    if: ${{ !cancelled() }}   # a superseded (cancel-in-progress) run stays "cancelled", not "failed"
    runs-on: ubuntu-latest
    timeout-minutes: 5
    steps:
      - name: linux and ui must both succeed
        shell: bash
        env:
          LINUX: ${{ needs.linux.result }}
          UI: ${{ needs.ui.result }}
        run: |
          [ "$LINUX" = success ] || { echo "::error::linux=$LINUX"; exit 1; }
          [ "$UI" = success ] || { echo "::error::ui=$UI"; exit 1; }
```
`ssot-autoregen` is unaffected.

- [ ] **Step 5: Seed the Windows cache nightly** — add to `nightly.yml`:
```yaml
  windows-cache-seed:
    name: Windows workspace check (seeds merge-queue cache)
    runs-on: windows-latest
    timeout-minutes: 120
    steps:
      - uses: actions/checkout@v7
      - uses: ./.github/actions/setup-rust
        with:
          cache: "false"
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: workspace-windows
          save-if: ${{ github.ref == 'refs/heads/main' }}
      - run: cargo check --workspace --exclude vox-gui --all-targets --locked
```
- [ ] **Step 6: Verify** — contract test PASS; `vox ci required-context-guard`, `workflow-concurrency-guard --strict`, `ssot-drift`, `node-pnpm-ssot-guard` PASS; `act pull_request -W .github/workflows/ci.yml -n` parses; locally `cd crates/vox-gui/ui && pnpm install && pnpm typecheck && pnpm test && pnpm exec playwright test --project=chromium` **all PASS** on this branch (required, not optional — a red here means the new blocking leg would lock every UI PR; fix or stop and report). Run each new `run:` block locally under `bash -eo pipefail` with `BASE_SHA` set to the branch's merge-base, including an empty-`BASE_SHA` case for the `ui` step (expect `run=true`). Update the `ci.yml` header comment and the ssot-autoregen comment that say `act pull_request -j gate` / "only `gate`" to name the `linux` job. <!-- AMENDED: #13, #4, #17 -->
- [ ] **Step 7: Commit** `ci: required gate = linux + ui legs; toolchain rustdoc; deny licenses/bans/sources; advisory Windows leg`.

---

### Task 6: Test `vox-gui` nightly

**Files:** Modify `.github/workflows/gui-cross-build.yml`; `crates/vox-cli/tests/ci_workflow_contract.rs`

- [ ] **Step 1: Failing contract test** <!-- AMENDED: #14 — the original assertion already passed against the existing prereqs-only test -->

```rust
#[test]
fn vox_gui_is_tested_somewhere() {
    let yml = include_str!("../../../.github/workflows/gui-cross-build.yml");
    // `cargo test -p vox-gui --test gui_tauri_prereqs` already exists; require the full suite.
    let full = yml.find("cargo test -p vox-gui --locked").expect("full vox-gui test suite");
    let sidecar = yml.find("Stage Tauri external sidecar").expect("sidecar staging step");
    assert!(full > sidecar, "tests need the staged sidecar");
}
```
- [ ] **Step 2:** In the build job, after sidecar staging and before `cargo build -p vox-gui`, add `- name: vox-gui tests` / `run: cargo test -p vox-gui --locked`.
- [ ] **Step 3: Verify** — test PASS; `ssot-drift` PASS.
- [ ] **Step 4: Commit** `ci: run vox-gui tests in the nightly GUI cross-build`.

---

### Task 7: Stop per-PR cache writes (Actions cache at 9.8 / 10 GB) <!-- AMENDED: #6 — composite actions DO run a nested action's post-save (the existing main `Linux-cargo-*` key proves it); the save-cache input design would have saved an empty target/ at job start -->

**Files:** Modify `.github/actions/setup-rust/action.yml` (the `Cache cargo registry, git, and target directories` step); `.github/workflows/compile-matrix.yml`, `ts-emit-noemit.yml`, `mobile-eas-build.yml`, `cr-l-gates.yml`, `differential-gate-nightly.yml`, `docker-eval.yml`, `docker-telemetry.yml`; `crates/vox-cli-ci/src/cache_key_lint.rs`; `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs` (`run_ssot_drift`) <!-- GRILL Ex 3 --> <!-- AMENDED: D5, D6 -->

`cache-key-lint` exists but no gate runs it, and it has already drifted: today it fails on `differential-gate-nightly.yml` ("Cache Cargo Registry & Target" hashes `**/Cargo.lock` with no toolchain). This task makes the cache invariants machine-checked **first**, then fixes the tree to satisfy them.

- [ ] **Step 0a: Failing tests in `cache_key_lint.rs`** (the existing `violations_for(yaml, file)` helper runs `check_doc`; add sibling helpers for the new rule):

```rust
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
    fn composite_action_steps_are_checked() {
        // Composite actions run under every caller, including PR workflows.
        let action = "runs:\n  using: composite\n  steps:\n    - uses: actions/cache@abc\n      with:\n        key: ${{ runner.os }}-${{ hashFiles('Cargo.lock') }}\n";
        let (keys, saves) = composite_violations_for(action, "setup-rust/action.yml");
        assert_eq!(keys.len(), 1, "lockfile key without toolchain");
        assert_eq!(saves.len(), 1, "unconditional save");
    }
```
(Note: YAML parses `on:` as the string key `on` with serde_yaml_ng; confirm in the first test run and handle a boolean `true` key too if it appears.) Run `cargo test -p vox-cli-ci cache_key_lint` → FAIL (helpers don't exist).

- [ ] **Step 0b: Implement.**
  - **PR-reachable workflow:** one whose `on` contains `pull_request`, `pull_request_target` or `merge_group`, or `push` without `branches` being exactly `[main]`.
  - **Save rule for PR-reachable workflows:** flag every `actions/cache@…` step (not `actions/cache/restore@…`) whose `if:` doesn't contain `refs/heads/main`. Also flag every `Swatinem/rust-cache@…` step whose `with.save-if` is absent, or neither contains `refs/heads/main` nor equals `false`.
  - **Composite actions:** walk `runs.steps` instead of `jobs.*.steps`. Apply the existing lockfile-key rule, and apply the save rule unconditionally, because a composite is reachable from PR callers.
  - Extend `run()` to also read `.github/actions/*/action.yml`.
  - Factor the step iteration so both walkers share it, with no copy-paste of the step loop. The helper doing so is private, so the tdd-guard is not triggered.
  - Update the module doc comment to describe both rules.
  - Tests → PASS.
- [ ] **Step 0c: Wire into `run_ssot_drift`** — `ds!("cache_key_lint", vox_cli_ci::cache_key_lint::run(root))?;` next to `workflow_policy_guard`. `ssot-drift` fails until Steps 1–2 land; commit Steps 0–3 together (P2).


- [ ] **Step 1: setup-rust** — replace the single `actions/cache` step with two mutually exclusive steps, same pinned SHA (`actions/cache@0400d5f644dc74513175e3cd8d07132dd4860809`, and `actions/cache/restore@<same sha>`), same `path`/`key`/`restore-keys`, keeping the existing comment about anchoring to the top-level `Cargo.lock`:
```yaml
    - name: Cache cargo registry, git, and target directories (main saves)
      if: ${{ inputs.cache == 'true' && github.ref == 'refs/heads/main' }}
      uses: actions/cache@0400d5f644dc74513175e3cd8d07132dd4860809
      with: { …unchanged path/key/restore-keys… }
    - name: Restore cargo registry, git, and target directories (no save off main)
      if: ${{ inputs.cache == 'true' && github.ref != 'refs/heads/main' }}
      uses: actions/cache/restore@0400d5f644dc74513175e3cd8d07132dd4860809
      with: { …unchanged path/key/restore-keys… }
```
  Literal `refs/heads/main` (repo idiom; `github.event.repository.default_branch` is absent from `schedule` payloads). No new input, no caller edits for callers that use only setup-rust. (No caller reads this step's `cache-hit`; the only `cache-hit` reference in `.github/` is `vox-mental-tracker.yml`'s own Playwright cache.)
- [ ] **Step 2: Swatinem callers on PR/merge_group/branch-push triggers** — in `compile-matrix.yml` (3 jobs), `ts-emit-noemit.yml`, `mobile-eas-build.yml` (2 jobs): pass `with: { cache: "false" }` to setup-rust (Swatinem already caches `target/`; today both cache it) and set Swatinem `save-if: ${{ github.ref == 'refs/heads/main' }}`. In `cr-l-gates.yml`, change its two `actions/cache@v5` steps to the same main-saves / restore-elsewhere pair. In `differential-gate-nightly.yml`, key the cache on the toolchain: `${{ runner.os }}-cargo-differential-gate-${{ hashFiles('rust-toolchain.toml', '**/Cargo.lock') }}`. In `docker-eval.yml` and `docker-telemetry.yml`, set the trivy step's `cache: 'false'`. Its default daily-keyed `cache-trivy-<date>` entries hold about 7 × 85 MB on main, and one DB download per daily run is cheaper. Fix every other violation the new lint reports the same way; never add an exemption.
- [ ] **Step 3: Verify** — `cargo test -p vox-cli-ci cache_key_lint` PASS; `./target/debug/vox ci cache-key-lint` exits 0; `ssot-drift` PASS (now including `cache_key_lint`); `act -n` on `compile-matrix.yml` and `ci.yml` parses. Mutation check: temporarily delete the `if:` from setup-rust's main-saves step and confirm `ci cache-key-lint` fails, then restore.
- [ ] **Step 4: Commit** `ci: save Rust caches only from main (cache at 9.8/10 GB)`.

**User action (gated):** a one-time delete, after showing the user the list and getting explicit confirmation:
- stale PR-scoped caches (~4.15 GB today): `gh cache list --limit 1000 --json id,key,ref,sizeInBytes --jq '.[] | select(.ref|startswith("refs/pull/"))'`;
- stale main-scope Rust caches (~1.3 GB): every `Linux-cargo-*` key on `refs/heads/main` except the newest (`gh cache list --ref refs/heads/main --key Linux-cargo- --sort created_at --order desc --json id,key,sizeInBytes`), plus the `cache-trivy-*` entries that Step 2 stops creating.
Then run `gh cache delete <id>` per entry.

---

### Task 8: Fix the red workflows that are fixable in the repo

**Files:** `.github/workflows/scorecard.yml`, `mobile-e2e-ios.yml`, `nightly.yml` (`all-features-matrix`), `compile-matrix.yml`; docs with dead links + `.lycheeignore`

- [ ] **Step 1: scorecard** — the job needs OIDC to sign with `publish_results: true`. Give the scorecard job `permissions: { id-token: write, contents: read, actions: read }` (no `security-events: write` — the workflow uploads SARIF only as an artifact, `scorecard.yml:30-35`). Keep top-level `read-all`. <!-- GRILL CONFIRM addendum -->
- [ ] **Step 2: mobile-e2e-ios** — add `cache-dependency-path: apps/vox-mental-tracker/pnpm-lock.yaml` to its `actions/setup-node` step.
- [ ] **Step 3: all-features-matrix** — `vox-oratio` → `vox-speech`, `vox-scientia-ingest` → `vox-scientia`; delete `vox-primitives`.
- [ ] **Step 4: compile-matrix** — delete the `crates/vox-release-artifacts/**` and `crates/vox-assets/**` path entries.
- [ ] **Step 5: link_checker (85 real dead links)** — list: `gh run list --workflow link_checker.yml --limit 1 --json databaseId -q '.[0].databaseId' | xargs -I{} gh run view {} --log | grep -E '\[(404|403|ERR|TIMEOUT)\]'`. 404 → fix or remove the link; 403 from a bot-walled site → add that host to `.lycheeignore` with a comment; `your-domain.com` placeholder → backticks. Never ignore whole TLDs or `github.com`. Then prove the external links locally with the workflow's own command (the exact `args:` from `link_checker.yml`): `lychee --root-dir . --no-progress --max-retries 5 --retry-wait-time 5 --timeout 40 --exclude-path docs/src/archive --exclude-path '\.claude/' --exclude-path 'assets/skills/' --exclude-path 'node_modules/' '**/*.md'`. Expect 0 errors. lychee is not installed here: `brew install lychee` is a download, so ask the user first. If they decline, record "external links verified by the next scheduled link_checker run". <!-- AMENDED: D2 -->
- [ ] **Step 6: Verify** — `ssot-drift`, `workflow-concurrency-guard --strict`, `ci_workflow_contract`, `vox ci check-links`, doc lint on touched docs → PASS.
- [ ] **Step 7: Commit** `ci: fix scorecard signing, ios pnpm cache path, stale crate names, dead links`.

**User action:** `docs-deploy.yml` fails 10/10 with Cloudflare `Authentication error [code: 10000]` — rotate the `CF_API_TOKEN` repo secret.

---

### Task 9: Retire dead and dormant workflows

**Files:** delete `.github/workflows/qwen35-native-nightly.yml`, `.github/workflows/pm-provenance-verify.yml`; modify `docker-telemetry.yml`, `deploy-telemetry.yml`, `vox-visus-audit.yml`, `nightly-report.yml`, `nightly.yml` (`audits` job), `nightly-artifacts.yml` (comment naming a deleted workflow); `crates/vox-cli-ci/src/workflow_policy_guard.rs` (`SELF_HOSTED_ALLOWLIST` and its doc comment "the two allowlisted workflows"); docs naming the deleted/parked workflows — at least `docs/src/ci/workflow-enumeration.md`, `binary-release-contract.md`, `deploy-contract.md`, `rcicd-coverage-cost-matrix-2026.md`, `compute-placement.md` (find all with `git grep -nE 'qwen35-native-nightly|pm-provenance-verify' -- docs ':!docs/src/archive' ':!docs/superpowers'`). <!-- AMENDED: #15 -->

- [ ] **Step 1:** `git rm .github/workflows/qwen35-native-nightly.yml`; remove `"qwen35-native-nightly.yml"` from `SELF_HOSTED_ALLOWLIST` and `Qwen3.5 Native Nightly` from `nightly-report.yml`.
- [ ] **Step 2: Telemetry chain** — both files: keep only `workflow_dispatch:` under `on:` (keeping `deploy-telemetry.yml`'s existing `workflow_dispatch.inputs`), header comment `# Dormant until the telemetry stack is revived (never deployed successfully; see docs/src/architecture/ci-cd-audit-findings-2026.md).`
- [ ] **Step 3: visus audit** — `workflow_dispatch:` only, comment `# no-op without VOX_VISUS_STAGING_URL`; remove `Vox Visus Audit` from `nightly-report.yml`.
- [ ] **Step 4: pm-provenance** — add to `nightly.yml`'s `audits` job (fixture outside the checkout so later audit steps see a clean tree): <!-- GRILL Ex 15 -->
```yaml
      - name: PM provenance strict gate (fixture)
        shell: bash
        run: |
          set -euo pipefail
          fx="$RUNNER_TEMP/pm-fixture/.vox_modules/provenance"
          mkdir -p "$fx"
          node='{"schema":"vox.pm.provenance/1","package":"fixture-pkg","version":"0.0.1","content_hash":"ab","built_at_epoch":99,"tool":"ci-fixture","registry":"http://127.0.0.1:9"}'
          printf '%s\n' "$node" > "$fx/fixture-pkg@0.0.1.json"
          ./target/debug/vox --quiet ci pm-provenance --strict --root "$RUNNER_TEMP/pm-fixture"
```
  then `git rm .github/workflows/pm-provenance-verify.yml`.
- [ ] **Step 5: Verify** — `cargo test -p vox-cli-ci workflow_policy_guard` (incl. `repo_workflows_satisfy_policy`), `ssot-drift`, `ci_workflow_contract`, `vox ci check-links` PASS; locally reproduce Step 4's commands with a temp dir and `./target/debug/vox ci pm-provenance --strict --root <tmp>` → PASS.
- [ ] **Step 6: Commit** `ci: retire qwen35 lane, park telemetry + visus, fold pm-provenance into nightly`.

---

### Task 10: Strip dead YAML and duplicate nightly work

**Files:** `.github/workflows/nightly.yml`, `cross-platform-check.yml`, `gui-cross-build.yml`, `crates/vox-cli/tests/ci_workflow_contract.rs`

- [ ] **Step 1: nightly.yml is schedule/dispatch only** — remove every branch needing `pull_request`, `merge_group`, or `push`: setup `filter` → keep only `rust=true`/`docs=true`; setup `affected` → keep only the `full=true` body; delete always-true job/step `if:` clauses (`!= 'merge_group'`, the four `(push main || full-ci label) || schedule || dispatch` guards); delete never-run steps (`== 'merge_group'` "Shadow — junit vs affected set", `== 'pull_request'` "Architecture / Code Drift Guard (TOESTUB)", any other); rewrite comments describing PR/merge-queue tiering.
- [ ] **Step 2: Deduplicate against `full`** — `full` runs clippy, rustdoc, `cargo test --doc`, `cargo deny check` and `cargo audit`. Delete: the `Rustdoc` and `Lint Check (Clippy)` steps from `lints`; `cargo deny check` / `cargo audit` and their install/warm steps from `guards-fast`; the "Doc tests" step from `tests`. If `lints` is then left with only cheap `vox` guard steps sharing `guards-fast`'s setup, move them into `guards-fast` and delete the `lints` job (update any `needs: lints`). <!-- AMENDED: Track C cut 5 -->
- [ ] **Step 3: cross-platform-check.yml** — delete the no-op `path-check` job and every `needs: path-check` / `needs.path-check…` / `github.event_name != 'pull_request'` guard; reduce `github.event_name == 'merge_group' || github.event_name == 'schedule'` guards (e.g. lines ~123, ~127) to schedule/dispatch semantics. <!-- GRILL Ex 13 -->
- [ ] **Step 4: gui-cross-build.yml** — collapse the matrix-compute `pull_request` branch to the non-PR branch.
- [ ] **Step 5: Contract tests (fixed classification):** <!-- GRILL Ex 13 -->
  - delete `selective_ci_setup_exports_affected_outputs`;
  - retarget `selective_ci_fail_closed_on_empty_affected` to `ci.yml`: assert it contains `if [ "$full" != "false" ] || [ -z "$(echo "$args" | xargs)" ]` and `args="--workspace --exclude vox-gui"`; fix its messages;
  - delete `selective_ci_fail_closed_on_docs_only_empty_affected` (docs-only PRs skip Rust by design);
  - delete `selective_ci_toestub_minimal_default_when_empty` (scoped TOESTUB lives in pre-push);
  - in `cross_platform_gate_is_required_three_os_matrix`: delete the `merge_group` assertion and "merge_group leg" wording; keep the clippy/nextest assertions.
  Write the retargeted assertion as a raw string (`r#"if [ "$full" != "false" ] || [ -z "$(echo "$args" | xargs)" ]"#`).
  Any other assertion broken by Steps 1–4: retarget if the behavior moved, delete only if it's gone by design — never weaken an assertion about something still present.
- [ ] **Step 6: Verify** — `grep -nE "pull_request|merge_group" .github/workflows/nightly.yml .github/workflows/cross-platform-check.yml .github/workflows/gui-cross-build.yml` → only comments, or `on:` triggers where a workflow genuinely keeps them; review every remaining hit. This catches the shell-form guards (`[ "${{ github.event_name }}" != "pull_request" ]`) and the `github.event.pull_request.labels` guards that the narrower pattern missed. A step whose condition is `merge_group … || push to main` (e.g. gui-playwright-smoke's "Commit visual-review cache") never runs on schedule: delete the step, don't just trim its `if:`. Then run <!-- AMENDED: #16 --> `ssot-drift`, `ci_workflow_contract`, `act workflow_dispatch -W .github/workflows/nightly.yml -n` PASS.
- [ ] **Step 7: Commit** `ci: strip unreachable PR/merge-queue branches and duplicate lints from scheduled workflows`.

---

### Task 11: (cut) stale-crate-reference guard <!-- AMENDED: #7 — reviewed and cut -->

Not implemented. Rule-1 and rule-3 staleness (`cargo … -p <gone>`, matrix `crate:` entries) already fails loudly at runtime, and `nightly-report` turns that into a `nightly-failure` issue; that is how the audit found them. A dead `paths:` entry is a harmless no-op filter. Task 8 fixes the actual stale names. The designed scanner also had three false-positive classes against live workflows (`mkdir -p dist`, `--package all`, `-p ${{ matrix.crate }}`), which would have pushed implementers to edit correct workflows. For the refined rules, if one is ever needed, see Deferred Minor Issues.

---

### Task 12: Liveness — a dead-man's switch for scheduled workflows <!-- AMENDED: #1 — `gh workflow view --json` does not exist, so every workflow was skipped and the job exited 0; seeded-cache step cut (false negatives via substring match and --limit) -->

**Files:** Create `.github/workflows/ci-liveness.yml`; modify `.github/workflows/nightly-report.yml` (add `CI liveness`); modify `crates/vox-cli/src/commands/ci/status.rs` (`fn render`)

- [ ] **Step 1: Failing test in `status.rs`**

```rust
    #[test]
    fn stale_nightly_issue_renders_without_double_prefix() {
        let s = CiStatus {
            branch: "b".into(),
            nightly: vec![NightlyIssue { number: 3, title: "Nightly stale: Benchmarks".into(), url: "https://i".into() }],
            ..Default::default()
        };
        assert!(render(&s).contains("NIGHTLY STALE: Benchmarks (#3)"), "{}", render(&s));
    }
```
- [ ] **Step 2: In `render`**, map titles: `Nightly failing: X` → `NIGHTLY FAILING: X`, `Nightly stale: X` → `NIGHTLY STALE: X`, else `NIGHTLY: <title>`. All status tests PASS.

- [ ] **Step 3: Create `.github/workflows/ci-liveness.yml`** — schedule-event runs only, per-cron cadence, fails loudly on an empty/unmapped list: <!-- GRILL Ex 6, 16 -->
```yaml
# Dead-man's switch: nightly-report.yml only reacts to runs that happen. This opens
# a `nightly-failure` issue when a listed scheduled workflow has no *scheduled* run
# within 2x its cron cadence (daily → 2 days, weekly → 14), and closes it when one lands.
# A workflow GitHub auto-disabled (disabled_inactivity) is an error, not a skip.
name: CI liveness

on:
  schedule:
    - cron: '0 12 * * *'
  workflow_dispatch:

concurrency:
  group: ${{ github.workflow }}
  cancel-in-progress: true

permissions:
  contents: read

jobs:
  liveness:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    permissions:
      contents: read
      actions: read
      issues: write
    steps:
      - uses: actions/checkout@v7
        with:
          sparse-checkout: .github/workflows
      - name: Every reported workflow ran on schedule recently
        shell: bash
        env:
          GH_TOKEN: ${{ github.token }}
          GH_REPO: ${{ github.repository }}
        run: |
          set -euo pipefail
          names=$(yq -r '.on.workflow_run.workflows[]' .github/workflows/nightly-report.yml)
          [ -n "$names" ] || { echo "::error::no names parsed from nightly-report.yml"; exit 1; }
          # `gh workflow view` has no --json; list states once. Keyed by path.
          gh workflow list --all --limit 500 --json path,state > /tmp/wf-states.json
          fail=0
          while IFS= read -r name; do
            file=""
            for f in .github/workflows/*.yml; do
              [ "$(yq -r '.name' "$f")" = "$name" ] && { file="$f"; break; }
            done
            [ -n "$file" ] || { echo "::error::'$name' listed but no workflow file has that name"; fail=1; continue; }
            cron=$(yq -r '.on.schedule[0].cron // ""' "$file")
            [ -n "$cron" ] || { echo "::error::'$name' ($file) has no schedule"; fail=1; continue; }
            dow=$(awk '{print $5}' <<< "$cron")
            case "$dow" in
              '*') days=2 ;;
              [0-7]) days=14 ;;
              *) echo "::error::'$name' cron '$cron' has an unsupported day-of-week"; fail=1; continue ;;
            esac
            state=$(jq -r --arg p "$file" '.[] | select(.path == $p) | .state' /tmp/wf-states.json)
            case "$state" in
              active) ;;
              disabled_manually) echo "skip $name (disabled_manually)"; continue ;;
              *) echo "::error::'$name' state is '${state:-unknown}' (GitHub disables crons after 60 days of inactivity)"; fail=1; continue ;;
            esac
            last=$(gh run list --workflow "$(basename "$file")" --event schedule --limit 1 \
                   --json createdAt -q '.[0].createdAt // empty')
            cutoff=$(date -u -d "$days days ago" +%s)
            title="Nightly stale: $name"
            num=$(gh issue list --label nightly-failure --state open --limit 100 --json number,title \
                  --jq "map(select(.title == \"$title\"))[0].number // empty")
            if [ -z "$last" ] || [ "$(date -u -d "$last" +%s)" -lt "$cutoff" ]; then
              [ -n "$num" ] || gh issue create --label nightly-failure --title "$title" \
                --body "No scheduled run of \`$name\` in ${days}+ days (last: ${last:-never}). Check the cron and whether GitHub disabled the schedule."
            elif [ -n "$num" ]; then
              gh issue close "$num" --comment "Scheduled run landed at $last."
            fi
          done <<< "$names"
          exit $fail
```
  Add `CI liveness` to `nightly-report.yml`'s list.
- [ ] **Step 4: Verify** — `cargo test -p vox-cli --lib commands::ci::status` PASS; `ssot-drift` PASS; locally `yq -r '.on.workflow_run.workflows[]' .github/workflows/nightly-report.yml | wc -l` equals the number of listed names; run the whole step body locally under `bash -eo pipefail` against the real repo with `gh issue create`/`gh issue close` stubbed as `echo` shell functions (read-only `gh workflow list`/`gh run list` stay live; on macOS put GNU `gdate` first on PATH as `date` — never rewrite the YAML to BSD `date`). Expect: every name maps; workflows not yet on `main` (e.g. `nightly.yml` before merge) print a would-create line — that is correct pre-merge. `act workflow_dispatch -W .github/workflows/ci-liveness.yml -n` parses.
- [ ] **Step 5: Commit** `ci: dead-man's switch for scheduled workflows; render stale issues`.

---

### Task 13: Explicit `permissions:` on every workflow; make the guard strict

18 workflows lack a top-level block once Task 9 has deleted `qwen35-native-nightly.yml` (`for f in .github/workflows/*.yml; do grep -q '^permissions:' $f || echo $f; done`).

**Files:** those workflows; `crates/vox-cli/src/commands/ci/pre_push.rs` — `fn step_workflow_permissions_guard`

- [ ] **Step 1:** Add top-level `permissions:\n  contents: read` to each. Where a job **currently succeeds** at a `GITHUB_TOKEN` write (check its recent runs: `gh run list --workflow <file> --limit 5`), add the narrowest job-level scope for it. Where a job's write only works via a PAT or currently fails with `github.token` (e.g. `harness-eval-nightly.yml`, push via `SSOT_AUTOREGEN_TOKEN || github.token`), keep it read-only and add `# push requires the SSOT_AUTOREGEN_TOKEN PAT; github.token is read-only by design`. <!-- GRILL Ex 12 -->
- [ ] **Step 2:** Flip `step_workflow_permissions_guard` to `vox_cli_ci::workflow_permissions_guard::run(root, true)`; delete the "backlog" comment.
- [ ] **Step 3: Verify** — `./target/debug/vox ci pre-push` (strict guard passes); `ssot-drift`.
- [ ] **Step 4: Commit** `ci: explicit least-privilege permissions on every workflow; guard strict`.

---

### Task 14: Delete the self-hosted fleet tooling and no-op subcommands

**Keep `watch-run`** (a live tool, `crates/vox-cli-ci/src/watch_run.rs`, documented in `gui-native-roadmap-status-2026.md`). **Delete `check-frozen`** (no-op). <!-- GRILL Ex 7 -->

**Files (delete):** `crates/vox-cli/src/commands/ci/queue.rs`, `runner_scale.rs`, `oom_watch.rs`, `unexpected_exit_watch.rs`; `infra/ci-runner/`; `Dockerfile.ci-runner`; `scripts/ci-runners-up.vox`; `scripts/ci-runner-local.sh`; `scripts/ci/install-runner-schedule.vox`; `scripts/ci/voxcirunnerscale-task.cmd`; `docs/src/ci/runner-autoscaling.md`; `docs/src/ci/local-first-ci.md`.
**Files (modify):** `crates/vox-cli/src/commands/ci/mod.rs`, `run_body.rs` (dispatch + `should_enforce_freshness`), `providers.rs`; `crates/vox-cli-ci/src/cmd_enums.rs` (`Queue`, `RunnerScale`, `RunnerPreflight`, `RunnerStatus`, placeholders `mens-corpus-health`, `grpo-reward-baseline`, `collateral-damage-gate`, `constrained-gen-smoke`, `check-frozen`); `crates/vox-cli-ci/src/frozen_crates.rs` and its `lib.rs` module line; `crates/vox-cli-ci/src/constants.rs` (`DOCS_SSOT_FILES`: drop `local-first-ci.md`); `crates/vox-cli/src/commands/diagnostics/doctor/checks_standard/build_health.rs` (`DiagCheckKind::HookGuard`, `hook_guard_check`, `hook_guard_verdict`, the `ci.hook_guard_stale_binary` registry entry and tests); `contracts/config/env-vars.v1.yaml`, `contracts/config/registry.v1.yaml` and the config-hygiene baseline (`VOX_HOOK_GUARD_DISABLE` entries); `crates/vox-cli-ci/src/toolchain_ssot.rs` (the `Dockerfile.ci-runner` / `infra/ci-runner/Dockerfile` ROWS and their tests); `.gitignore` (`.ci-runner-logs/`); comments in `release-binaries.yml`, `vox-foundation/src/tracing.rs`, `vox-gui/src/commands/harness_town.rs` that name the fleet; `docs/src/ci/alternatives-and-local-mirroring.md`; <!-- AMENDED: #3 --> docs: `docs/src/ci/runner-contract.md`, `compute-placement.md`, `concurrency-exceptions.md`, `shared-compile-cache.md`, `docs/src/architecture/where-things-live.md`, `docs/src/reference/cli.md`, `docs/src/architecture/data-storage-lint-and-ci-spec-2026.md` (lines ~90, ~282: M-68 becomes a **new** guard, not an extension of `check-frozen`).

- [ ] **Step 1: Inventory (both spellings, precise patterns)** — `git grep -nE 'runner_scale|ci::queue|commands::ci::queue|oom_watch|unexpected_exit_watch|hook_guard|HookGuard|HOOK_GUARD|RunnerScale|RunnerPreflight|RunnerStatus|CiCmd::Queue|MensCorpusHealth|GrpoRewardBaseline|CollateralDamageGate|ConstrainedGenSmoke|CheckFrozen|frozen_crates|runner-scale|runner-preflight|runner-status|check-frozen|\bci queue\b|hook-guard|mens-corpus-health|grpo-reward-baseline|collateral-damage-gate|constrained-gen-smoke|local-first-ci|runner-autoscaling|\bci-runner\b|Dockerfile\.ci-runner|voxcirunnerscale' -- ':!docs/src/archive' ':!docs/superpowers' ':!graphify-out' ':!contracts/reports' ':!*.generated.md' ':!.superpowers'`. **Classify each hit before editing; edit only references to the deleted fleet surface.** Unrelated identifiers (the `vox-orchestrator` `queue` module, `task_queue::`, `vox_build_queue::`, UI test strings like `'sci-runner'`) are left alone. Any genuine fleet hit not in the Files list is added to it and fixed. If `docs/src/ci/runner-contract.md`'s `§Local-first CI` heading changes, fix the AGENTS.md link to it in Task 15. <!-- GRILL Ex 7 --> <!-- AMENDED: #3 — `queue::` / `ci-runner` matched ~40 unrelated live files, and "every hit is removed" ordered wrong edits -->
- [ ] **Step 2: Delete and fix what breaks** — `cargo check -p vox-cli -p vox-cli-ci --all-targets` until clean, then again with the feature set ssot-autoregen builds (`cargo check -p vox-cli --features completion-toestub,extras-ludus,ars,coderabbit`) so feature-gated references are seen. <!-- AMENDED: #18 -->
- [ ] **Step 3: Regenerate** — command-sync → gui-surface-coverage --write → doc-inventory generate → `UPDATE_CLI_CATALOG_BASELINE=1 cargo test -p vox-cli --test command_catalog_paths_baseline`.
- [ ] **Step 4: Verify** — `cargo clippy -p vox-cli -p vox-cli-ci --all-targets --locked -- -D warnings`; `cargo test -p vox-cli --lib commands && cargo test -p vox-cli-ci && cargo test -p vox-cli --test command_catalog_paths_baseline --test ci_workflow_contract`; `ssot-drift`; `check-links`; `./target/debug/vox doctor` runs without the removed check; re-run Step 1's grep → only hits classified as unrelated in Step 1 remain (list them in the commit body).
- [ ] **Step 5: Commit** `ci: delete self-hosted fleet tooling and no-op ci subcommands`.

External callers of the deleted subcommands outside this repo can't be verified here — note it in the Task 15 resolution.

---

### Task 15: Update the audit doc and AGENTS.md CI contract

**Files:** `docs/src/architecture/ci-cd-audit-findings-2026.md`, `AGENTS.md` (`## CI Contract (Required, SSOT)`)

- [ ] **Step 1:** Add a "Resolution (2026-09-22)" section mapping each finding to its task/commit, plus: user-only items (`CF_API_TOKEN`, stale-cache delete, bypass merge if needed, Windows enforcement flip); post-merge verification items (see below) with a placeholder table for run IDs and minutes; `harness-eval-nightly.yml`'s warning (`:66`) blaming every push rejection on "non-fast-forward" may hide a ruleset rejection (unverified: whether the PAT owner can bypass the merge-queue ruleset); external callers of deleted subcommands unverified; the advisory Windows leg's failures/timeouts will show in `vox ci status` blocks while it is warn-only (expected noise until the enforcement flip).
- [ ] **Step 2:** AGENTS.md: delete the "`vox ci pre-push` prints an **advisory** reminder on re-push…" bullet (removed by Task 3); change `act pull_request -j gate` → `act pull_request -j linux` everywhere it appears (Local CI Gate Tiers + CI Contract: `gate` is now a step-less aggregator); keep the `runner-contract.md §Local-first CI` link valid (retarget it if Task 14 renamed the heading). In the CI Contract section: the required context aggregates <!-- AMENDED: #17 --> a `linux` leg and a `ui` leg (UI PRs must pass typecheck + vitest + Playwright); toolchain bumps run rustdoc `-D warnings` in the linux leg and defer nextest to nightly; the merge queue also runs an advisory Windows compile check; scheduled workflows that stop running open `Nightly stale:` issues; Rust caches are saved only from `main`.
- [ ] **Step 3: Verify** — doc lint on the audit doc; `check-links`.
- [ ] **Step 4: Commit** `docs: record CI audit resolution; AGENTS.md CI contract`.

---

## Bootstrap & post-merge verification <!-- GRILL Ex 11, 17, 18 -->

This PR can only be measured by GitHub, and its own required `linux` leg runs **cold** (no `workspace` cache exists on `main`; `.github/workflows/` changes force a full run). Each step below needs the user's explicit go-ahead.

1. **Push + open the PR.** The cold `linux` leg is the first real measurement of the 30-min budget. Record per-step minutes in the PR and in the Task 15 resolution.
   - If clippy alone takes > ~24 min, the toolchain-bump design (clippy + rustdoc in the required leg) must be revisited **before merge**.
2. **If the `linux` leg passes in < 30 min**, merge normally through the queue.
3. **If it times out**, there is no CI proof (a timeout cancels the remaining steps, and no pre-merge job may exceed 30 min). The implementer runs, at the PR head SHA:
   - `cargo clippy --workspace --exclude vox-gui --all-targets --locked -- -D warnings`
   - `cargo nextest run --workspace --exclude vox-gui --profile ci --locked`
   and posts SHA, commands, pass/fail/skip counts and durations as a PR comment (and in Task 15). The user decides on a one-time admin bypass merge. Known gap: that proof runs on macOS, not Linux; the backstop is step 4.
4. **Right after merge:** dispatch `nightly.yml` on `main` (seeds `workspace` and `workspace-windows`, runs full nextest on Linux; a red run opens a `nightly-failure` issue to fix first).
5. **Windows flip to enforcing:** after the first `merge_group` run following a successful `windows-cache-seed` on `main`, if its Windows leg succeeded in ≤ 24 min (`gh run view <id> --json jobs`), the user approves a one-line change adding `windows` to `gate.needs` and the check `[ "$WINDOWS" = success ] || [ "$EVENT" != merge_group ]`. Record run ID + minutes in the Task 15 resolution.
6. Also record: `workspace` and `workspace-windows` cache sizes after the first nightly (`gh cache list --ref refs/heads/main --key v0-rust-workspace --limit 5`) against the 10 GB line; that `release-gui.yml` / `release-installers.yml` failures are the stale pre-fix ones (audit §3: verify only); first UI PR's `ui` leg minutes; `setup-e2e.yml` green (validates the cuda plugin fix); `scorecard.yml` green; `docs-deploy.yml` green after the token rotation.

## Not in scope (decided)

- `ci.yml`'s `linux` and `ssot-autoregen` compiling vox-cli twice: different feature sets; autoregen is non-blocking and hosted minutes are free for this public repo.
- The ~40 CLI-only `vox ci` subcommands that are *not* placeholders/no-ops (e.g. `commit-lint`, `build-cache-doctor`, `ssot-audit`, `watch-run`): manual tools by design; pruning needs per-command owner judgment.
- Nightly `full` nextest vs `tests` llvm-cov nextest: different artifacts.

## Deferred Minor Issues

Deferred items addressed in the plan (2026-09-22):
- D1 `!cancelled()` → Task 5 Step 4.
- D2 local lychee check of external links → Task 8 Step 5.
- D3 comment-only examples consumers → Task 4.
- D5 main-scope cache waste → Task 7 Step 2 and the user action.
- D6 cache-invariant lint wired into ssot-drift, fixing one live drift → Task 7 Step 0.

Remaining items are not defects:
1. Task 11 stays cut (review ruling #7). If a stale crate reference ever ships silently, these are the refined rules to revive it with:
   - rule 1 applies only on lines containing `cargo`, to tokens matching `^[a-z][a-z0-9-]*$`, skipping `${{`;
   - rule 2 skips dirs containing `*`, `$` or `{`;
   - rule 3 stays as designed;
   - negative tests: `mkdir -p dist`, `--package all`, `-p ${{ matrix.crate }}`, `crates/*/Cargo.toml`, `crates/${c}/`.
2. `nightly-report.yml` re-runs `gh label create --force` every run. This is intended: ci-liveness depends on the label existing, and the call is idempotent.

## Execution Order

Sequential Constraints (Shared-File Collisions — CANNOT parallelize):
- Task 5 → Task 8 → Task 9 → Task 10: all modify `.github/workflows/nightly.yml`
- Task 9 → Task 12: both modify `.github/workflows/nightly-report.yml` (liveness assumes the qwen35/visus entries are gone)
- Task 5 → Task 6 → Task 10: all modify `crates/vox-cli/tests/ci_workflow_contract.rs`
- Task 6 → Task 10 → Task 13: `gui-cross-build.yml`; Task 10 → Task 13: `cross-platform-check.yml`; Task 8 → Task 13: `mobile-e2e-ios.yml`, `link_checker.yml`; Task 9 → Task 13: `vox-visus-audit.yml`, deleted `qwen35-native-nightly.yml`
- Task 7 → Task 13: `compile-matrix.yml`, `ts-emit-noemit.yml`, `mobile-eas-build.yml`, `cr-l-gates.yml`, `differential-gate-nightly.yml`, `docker-eval.yml`, `docker-telemetry.yml`
- Task 7 → Task 9: `docker-telemetry.yml` (trivy cache, then dispatch-only)
- Task 7 → Task 12: `run_ssot_drift` stage list is touched by Task 7 only; no conflict
- Task 3 → Task 13: `crates/vox-cli/src/commands/ci/pre_push.rs`
- Task 9 → Task 14: `docs/src/ci/compute-placement.md` and other ci docs; Task 14 regenerates artifacts once, after all CiCmd edits
- Task 3, 5, 14 → Task 15: `AGENTS.md` stale lines fixed last

Pre-Flight Checklist:
- [ ] Worktree isolated via superpowers:using-git-worktrees (`.claude/worktrees/cicd-complexity-tradeoffs-b62a67`)
- [ ] Target git history confirmed: branch `claude/cicd-complexity-tradeoffs-b62a67`, merge-base `877406d84`; `main` is 10 commits ahead and touches no plan file
- [ ] Next migration sequence verified from live directory: N/A (no DB schema changes)
- [ ] Local test database verified: N/A (no DB tests added)
- [ ] Commit the audit doc and this plan before Task 1 (both untracked)

Recommended Task Sequence:
Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 6 → Task 7 → Task 8 → Task 9 → Task 10 → Task 12 → Task 13 → Task 14 → Task 15 (Task 11 cut).
Batch candidates: Tasks 1 + 2 (independent lint/compile fixes in different crates, one dispatch); Task 6 is a one-step YAML change and may ride with Task 5's reviewer context.

SDD Ledger Pre-Population (copy into progress.md):
```
Conflict on .github/workflows/nightly.yml (Tasks 5, 8, 9, 10): sequential execution enforced — ruling: settled
Conflict on nightly-report.yml (Tasks 9, 12): sequential — ruling: settled
Conflict on ci_workflow_contract.rs (Tasks 5, 6, 10): sequential; always run the whole `--test ci_workflow_contract` — ruling: settled
Conflict on pre_push.rs (Tasks 3, 13): sequential — ruling: settled
[Critical #1] Task 12: use `gh workflow list --all --json path,state` once + `--workflow <basename>`; disabled_inactivity is an error — ruling: settled
[Critical #2] Task 1: delete the second q_norm/k_norm `let` bindings as well as the duplicate fields; keep load_norm — ruling: settled
[Critical #3] Task 14: precise inventory regex; classify hits; never edit unrelated `queue` / `sci-runner` code — ruling: settled
[Critical #4] Task 5: diff checks read /tmp/changed.txt; ui detection fails closed; BASE_SHA asserted non-empty — ruling: settled
[Important #5] Task 5: prefilter alternation `examples/|\.github/workflows/)` to keep the contract substring — ruling: settled
[Important #6] Task 7: two mutually exclusive cache steps gated on literal refs/heads/main; no save-cache input — ruling: settled
[D6] Task 7: cache_key_lint gains the PR save-scope rule + composite scan and is wired into ssot-drift; fix the tree, never exempt — ruling: settled
[Important #7] Task 11 cut (runtime already fails loudly; guard had false positives) — ruling: settled; cost if wrong: a stale paths: entry lingers silently
[Important #9] Task 2 never_loop: cfg_attr allow, keep the Windows retry — ruling: settled
[Important #13] Task 5: local vitest + chromium Playwright must pass before commit; rollback = user-gated admin bypass of the revert — ruling: settled
[Important #11/#12] ui leg path filter `^crates/vox-gui/` + orch_daemon/mod.rs; `playwright test --project=chromium` — ruling: settled
Never weaken repo_workflows_satisfy_policy or edit correct workflows to satisfy a guard — ruling: settled
Never build vox-gui locally (missing sidecar/ui dist); act only with -n; never cargo fmt --all — ruling: settled
```
