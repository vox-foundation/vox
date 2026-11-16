# Phase 343: Docs System SSOT and Automated Drift Control - Pattern Map

**Mapped:** 2026-11-06
**Files analyzed:** 14 new/modified
**Analogs found:** 14 / 14 (all analog paths verified git-tracked)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs` (stage table, 01/04a) | service/orchestrator | batch | itself (`run_ssot_drift`, `ds!` chain) | exact (refactor) |
| `scripts/ssot-regen.vox` (add crate-api-sync, wiki-sync) | config/script | batch | itself, lines 26-43 | exact |
| `crates/vox-cli-ci/src/overlay_parity.rs` (02) | utility/guard | transform | `crates/vox-cli-ci/src/retired_symbol_check.rs` | exact |
| `contracts/documentation/overlay-claims.v1.yaml` (02) | config | transform | `contracts/documentation/retired-symbols.v1.yaml` | role-match |
| `crates/vox-cli/src/commands/ci/docs_cli_claims.rs` (03) | guard | transform | `command_catalog::build_catalog` + `claims/resolver.rs` | role-match |
| `crates/vox-doc-pipeline/src/claims/resolver.rs` (03 tighten) | utility | transform | itself (`:505-508`) | exact |
| `crates/vox-cli-ci/src/check_links.rs` (04a, orphans) | guard | file-I/O | itself (`:120-145`) | exact |
| `crates/vox-cli-ci/src/orphan_pages.rs` (04b) | guard | batch | `claims/prune.rs` (`build_inbound_link_graph`) | role-match |
| `contracts/documentation/orphan-pages-baseline.v1.yaml` (04b, user gate) | config | CRUD | `kb-facet-backlog.v1.yaml` + `commands/kb/facet_backlog.rs` | exact |
| `crates/vox-doc-pipeline/src/claims/ledger.rs` + `audit.rs` (05 idempotence) | service | CRUD | itself (`:389`) | exact |
| `.github/workflows/docs-drift-refresh.yml` (05) | config/workflow | event-driven (cron) | `.github/workflows/harness-eval-nightly.yml` | exact |
| `.github/workflows/nightly-report.yml`, `link_checker.yml` (05) | config | event-driven | themselves | exact |
| `crates/vox-cli/src/commands/ci/docs_system_parity.rs` (06) | guard/test | transform | `crates/vox-cli/tests/ci_workflow_contract.rs`, `crates/vox-doc-pipeline/tests/frontmatter_governance_parity.rs` | role-match |
| `docs/src/architecture/docs-system-ssot.md`, `docs/src/contributors/detector-coverage-ledger.md` | doc | n/a | themselves (§4 row, §6 bullet removal) | exact |

## Pattern Assignments

### `docs.rs` stage table (01)

**Current shape** (`run_ssot_drift`, ~lines 570-600): inline `ds!(label, expr)?` with timing macro.
```rust
let auto_heal = std::env::var_os("CI").is_none() || std::env::var_os("VOX_AUTO_HEAL").is_some();
macro_rules! ds {
    ($label:literal, $expr:expr) => {{
        let _t = std::time::Instant::now();
        let _r = $expr;
        eprintln!("ssot-drift  {:>6}ms  {}", _t.elapsed().as_millis(), $label);
        _r
    }};
}
ds!("check_docs_ssot", check_docs_ssot(root))?;
ds!("run_query_all_guard", super::guards::run_query_all_guard(root, true))?;
ds!("db_schema_coverage", vox_cli_ci::db_schema_coverage::run(root))?;
```
Keep the eprintln timing line and `auto_heal` flag; table entry = `{name, run: fn(&Path,bool)->Result<()>, regen: Option<&[&str]>}`; runner collects failures then bails once. Do not reorder stages.

**Tests to replace** (~lines 975-1017): three source-text probes (`run_ssot_drift_dispatches_cache_key_lint`, `_kb_facets_ts`, `_capability_index`) using `include_str!("docs.rs")` + split-needle. Replace with one table-iterating test plus a mutation proof.

**Parity test source** (`scripts/ssot-regen.vox:28-42`): each regenerator is `regen(["--quiet", "ci", "<sub>", ...], "<label>")`. Test parses these literals via `include_str!("../../../../../../scripts/ssot-regen.vox")`, strips `--quiet`, and compares as a set to table `regen` values plus a `REGEN_ONLY` list with reasons. `crate-api-sync --write` and `wiki-sync --write` are missing from the .vox file today. Also add `lefthook.yml:77` retry list to the comparison, or replace it (open decision).

### `overlay_parity.rs` (02)

**Analog:** `crates/vox-cli-ci/src/retired_symbol_check.rs` (lines ~510-550): scan loop with per-file `ScanCfg`.
```rust
for extra in ["AGENTS.md", "GEMINI.md", "CLAUDE.md"] {
    let path = root.join(extra);
    if path.is_file() {          // silent skip: make mandatory in the new guard
        let body = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        failures.extend(scan_source_lines(&path, root, &body, &regexes,
            ScanCfg { is_md: true, skip_md_table_rows: true, is_rust: false }));
    }
}
let cursor_rules = root.join(".cursor/rules");
if cursor_rules.is_dir() { collect_cursor_rule_files(&cursor_rules, &mut mdc_files); /* same scan */ }
```
Reuse its negation logic (skip table rows, lines containing "retired"/"NOT"). Add `.github/copilot-instructions.md` and `docs/src/AGENTS.md` to the scanned set. Claims come from the YAML table with verbatim AGENTS.md anchors (anchor must still occur, else fail). Use `regex` only (linear time).

### `docs_cli_claims.rs` (03)

**Ground truth:** `crates/vox-cli/src/command_catalog.rs:102-125`, `build_catalog()` walks `VoxCliRoot::command()` iteratively; entries have `path` and `arguments[].long`. Union with registry paths (`contracts/cli/command-registry.yaml`) for feature-gated groups; skip flag checks for registry-only paths. Tests that touch the clap tree need the big-stack helper (`run_on_big_stack`, test-only, same file).

**Extractor/resolver to tighten:** `crates/vox-doc-pipeline/src/claims/resolver.rs:505-508`:
```rust
if subcmds.is_empty()
    || self.cli_commands.contains(&full)
    || self.cli_commands.contains(subcmds[0])   // first-token fallback: the permissiveness to remove
```
"planned" rule: adjacency only (a `vox ...` code span followed within ~8 chars by `(planned`/`planned per`). Fixtures: AGENTS.md lines 205 and 217 positive, line 234 negative.

### `check_links.rs` (04a)

Silent no-ops to turn fatal in repo mode (`:127-142`):
```rust
} else {
    println!("{} Target not found or invalid: {:?}", "SKIP".yellow(), abs_t);
    return Ok(());
}
...
if !docs_src.exists() {
    println!("{} Docs source not found at {:?}", "SKIP".yellow(), docs_src);
    return Ok(());
}
```
Also `:79-91`, expired allowlist entries `WARN ... still skipping` then `return true`. Allowlist entries expire 2026-11-30. Same class in `docs.rs`: dead `orphan-surface-inventory.md` branch (`:60-61`, else `:104`), dead `architecture-index.md` branch (`:155-156`), stale-`last_updated` warning (`:210-213`). Pattern: delete dead branches (retired files), make live inputs mandatory, add a temp-dir mutation test per site.

### `orphan_pages.rs` and baseline (04b)

**Analog graph:** `crates/vox-doc-pipeline/src/claims/prune.rs` `build_inbound_link_graph` (walks only the given `docs_dir`; no archive exclusion; ignores root-guide links). Fix those or record inbound links in the `check_links::run` loop, which already skips fences and inline code. Exempt `index.md[x]`, `*.generated.md`, `docs/src/AGENTS.md`, `.well-known/`, archive, redirect stubs.

**Baseline analog:** `crates/vox-cli/src/commands/kb/facet_backlog.rs` (lines 1-30): `file_at(git, base, rel)` returns `Ok(None)` only on bootstrap and fails closed on git errors; the gate compares against the base ref and rejects same-epoch additions. Baseline seeding needs a user decision `checkpoint:decision` first. Default to linking from section hubs instead.

### `ledger.rs` / `audit.rs` idempotence (05)

`claims/ledger.rs:~385-390`:
```rust
FreshnessLedger {
    schema_version: 1,
    generated_by: "vox audit docs (vox-doc-pipeline)".to_string(),
    updated_at: chrono::Utc::now().to_rfc3339(),   // non-idempotent
    head_commit: head_commit.to_string(),           // also churns
    pages, metrics: LedgerMetrics { ... },
```
Test first: in the `update_ledger` branch of `audit.rs`, skip the write when `pages` and `metrics` equal the existing ledger (derive `PartialEq` if missing).

### `.github/workflows/docs-drift-refresh.yml` (05)

**Analog:** `.github/workflows/harness-eval-nightly.yml` (PR step, lines 88-116) and `link_checker.yml` header (schedule, `concurrency`, top-level `permissions: contents: read`, literal `timeout-minutes`).
```yaml
env:
  GH_TOKEN: ${{ secrets.SSOT_AUTOREGEN_TOKEN || github.token }}
  HAS_PAT: ${{ secrets.SSOT_AUTOREGEN_TOKEN != '' }}
run: |
  set -euo pipefail
  if [ -z "$(git status --porcelain -- <paths>)" ]; then echo "nothing new"; exit 0; fi
  if [ "$HAS_PAT" != "true" ]; then echo "::warning::... GITHUB_TOKEN PR does not trigger required checks"; fi
  git config user.name "github-actions[bot]"
  git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
  git switch -c "$branch"; git add ...; git commit -m "..."
  git push <authenticated remote URL built from GH_TOKEN> "HEAD:refs/heads/${branch}"
  gh pr create --base main --head "$branch" --title ... --body ...
```
Differences: use one fixed branch `bot/docs-drift-refresh` (force-push), `gh pr list --head` then `gh pr create --draft` or `gh pr edit`, and no `gh pr merge --auto`. Pass values via `env:`, never inline `${{ }}` in `run:`. Job-scoped `contents: write` and `pull-requests: write`. Add the workflow `name:` to `nightly-report.yml` `on.workflow_run.workflows` (list at lines 10-22, e.g. `- Docs Link Checker`). For `link_checker.yml` change `fail: true` to `fail: false` plus a report (A4).

### `docs_system_parity.rs` (06)

**Analogs:**
- `crates/vox-cli/tests/ci_workflow_contract.rs`: `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.github/workflows/<f>.yml"))` plus `assert!(yml.contains(..), "msg")`; prefer `serde_yaml` parsing for structure.
- `crates/vox-doc-pipeline/tests/frontmatter_governance_parity.rs`: a parity test, but it only uses `contains` and does not run on docs-only PRs, so make 06 an in-process `ssot-drift` stage.
- Wiring probes: `pre_push.rs` `build_steps` returns `OwnedStep { label, .. }` (e.g. `"vox ci check-links"`). It is private, so add a `#[cfg(test)]` inside the module or a `pub(crate) fn fast_tier_labels`.
- Tag each §6 bullet in `docs-system-ssot.md` with `<!-- gap: id -->`; test maps id to its closing-gate probe.

## Shared Patterns

### Test-first and mutation proofs
Every new `pub fn` needs a `#[test]` in the same file (`skeleton/untested-pub-api`). Each guard gets a mutation proof (delete input, row, or fix; confirm failure; restore) with a before/after presence check.

### Stage registration
Add the stage to the stage table in `docs.rs`, not a new `vox ci` subcommand. Precedent is `pre_push.rs` `step_check_links`; a new subcommand would require operations catalog, registry, and cli.md rows.

### Same-commit doc updates
Each gap closed removes its §6 bullet and updates its §4 row in `docs-system-ssot.md`, and adds a row in `detector-coverage-ledger.md`. Do not create new docs-about-docs pages.

### Workflow guards
Top-level `permissions:`, `concurrency:` with `cancel-in-progress: true`, literal `timeout-minutes` (schedule cap 180), scheduled name listed in `nightly-report.yml`, rust caches saved from `main` only.

## No Analog Found

| File | Role | Reason |
|---|---|---|
| `contracts/documentation/overlay-claims.v1.yaml` | config | Only partial analog (`retired-symbols.v1.yaml`); the anchor-verification schema is new |

## Metadata

**Analog search scope:** `crates/vox-cli`, `crates/vox-cli-ci`, `crates/vox-doc-pipeline`, `.github/workflows`, `scripts`, `contracts/documentation`
**Files read (targeted ranges):** about 10
**Pattern extraction date:** 2026-11-06
