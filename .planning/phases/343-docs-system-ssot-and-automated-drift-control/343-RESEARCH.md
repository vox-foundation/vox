# Phase 343: Docs System SSOT and Automated Drift Control - Research

**Researched:** 2026-11-06
**Domain:** Rust CI guards (`vox ci`), docs-drift detection, GitHub Actions scheduled automation, shrink-only baselines
**Confidence:** HIGH on in-repo mechanics (every claim below was read or measured this session); MEDIUM on sizing numbers (crude measurements, Wave 0 must re-measure); design recommendations needing a user call are listed in Assumptions Log / Open Questions.

<user_constraints>
## User Constraints (from CONTEXT.md)

No CONTEXT.md exists for this phase. The user chose to plan without discuss-phase, from the requirements, roadmap goal and `docs-system-ssot` §6, and chose "research first". Constraints carried from the invocation:

### Locked Decisions
- Scope = DOCS-DRIFT-01..06 exactly as worded in `.planning/ROADMAP.md` (Phase 343 block). `docs/src/architecture/docs-system-ssot.md` §6 "Known gaps" is the backlog; **each gap closed removes its §6 bullet in the same PR**.
- VoxLang-first glue: no new `.sh` / `.py` / `.ps1`. Test-first for every new `pub fn`.
- USER-AUTHORIZED-ONLY: a NEW shrink-only baseline file seeded by this phase needs an explicit user decision gate in the plan; never regenerate an existing baseline to admit a new item.
- Hosted CI is the gate. Workflow caps, `concurrency:` blocks and top-level `permissions:` are enforced by guards. Docs frontmatter + KB facets on any new `docs/src/` page.

### Claude's Discretion
- How each gate is implemented and where it runs; stage-table design; the ground-truth source for CLI mentions; the revive-vs-retire recommendation for the dormant docs-reality-audit program and the self-heal bot's draft-PR stage (recommend, user confirms).

### Deferred Ideas (OUT OF SCOPE)
- None stated. Do not add new docs-about-docs pages (SSOT §7: extend `docs-system-ssot.md` or governance instead).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DOCS-DRIFT-01 | regen-list parity (`scripts/ssot-regen.vox` vs `run_ssot_drift`) | `run_ssot_drift` has NO stage table (inline `ds!` macro calls, 40 stages, short-circuit `?`). Three hand-kept regen lists exist and have already drifted (see Finding F1). Recommend stage table + parity test. |
| DOCS-DRIFT-02 | overlay parity (AGENTS.md vs overlays) | `retired_symbol_check` already covers retired symbols for AGENTS/GEMINI/CLAUDE/.cursor, but NOT `copilot-instructions.md` and NOT frontmatter/banned-command claims. 4 real contradictions found (F2). |
| DOCS-DRIFT-03 | prose vs code (`vox <cmd> [--flag]`, "planned per") | A mention extractor + resolver already exist (`vox-doc-pipeline::claims`) but the resolver is permissive (first token wins) and `--check-drift` baseline is empty. Strict ground truth = clap tree (`build_catalog`) ∪ registry. ~87 real unresolved mentions on reader-facing pages (F3). |
| DOCS-DRIFT-04 | dead guards + orphan pages | Four silent-no-op sites in `check_docs_ssot`, more in `check_links`/`retired_symbol_check` (F4). Inbound-link graph already exists in `claims::prune`. ~229 orphans overall, ~48 reader-facing (rough). |
| DOCS-DRIFT-05 | scheduled auto-correct draft PR | `--update-ledger` is safe (never re-attests) but NOT idempotent (`updated_at: Utc::now()`); PR-opening precedents exist (`harness-eval-nightly.yml`, `release-prepare.yml`). Scheduled workflow must be listed in `nightly-report.yml` (guard-enforced). (F5) |
| DOCS-DRIFT-06 | meta-gate (§4 wiring, §6 gaps) | Precedents: `ci_workflow_contract.rs` source-probe tests; `frontmatter_governance_parity.rs`. Recommend an in-process `ssot-drift` stage so doc-only PRs are covered. (F6) |
</phase_requirements>

## Summary

The phase is almost entirely **in-repo Rust guard work plus one workflow**. Nothing needs a new external package or action: the extractor/resolver (`vox-doc-pipeline::claims`), link graph (`claims::prune`), clap tree (`vox-cli::command_catalog::build_catalog`), shrink-only baseline machinery (`kb::facet_backlog`), workflow guards (`workflow_policy_guard`) and PR-opening patterns all exist. The work is (a) turning inline code into a table, (b) tightening permissive/dead guards, (c) adding 3-4 new detectors as stages, (d) one scheduled workflow, (e) a meta-gate.

The most important planning facts: (1) `run_ssot_drift` is a 215-line chain of `ds!(...)?` — converting it to a stage table is the foundation for DOCS-DRIFT-01, for aggregate-all-failures reporting, and for DOCS-DRIFT-06's probes; every other plan adds a stage to it, so it must land first and the plans that add stages must be serialized (they share `docs.rs`, `pre_push.rs`, `docs-system-ssot.md` §4/§6, `detector-coverage-ledger.md`). (2) The "dead / permissive guard" class is bigger than the roadmap lists: the existing CLI-mention resolver accepts any command whose first token is known, which is why `baseline.v1.json` is empty — tightening it surfaces ~87 genuine doc defects that must be fixed (or baselined with user authorization). (3) The scheduled loop must be made idempotent first or it will open a PR every night.

**Primary recommendation:** Wave 1 = stage-table refactor (01); then 02/03/04 each add one stage and fix the first real findings; 05 after 04 (it consumes the ledger and link tooling); 06 last, asserting everything and removing the last §6 bullets. Add a `checkpoint:decision` for any new baseline and for the reality-audit/self-heal revive-or-retire choice.

## Project Constraints (from CLAUDE.md)

Extracted from `/Users/brbrainerd/dev/vox/CLAUDE.md` → `AGENTS.md` and `~/dev/CLAUDE.md`:
- No new `.sh`/`.py`/`.ps1`; automation is `.vox` run via `vox run`. Python only under `crates/vox-python-bridge/worker/**`. Research scratch scripts must stay outside the repo (used scratchpad only).
- Test-first: every new `pub fn` in `crates/*/src/**` (files >30 non-blank lines) needs a `#[test]` in the same file (`skeleton/untested-pub-api`). Verify security guards by **mutation** (break the guard, see the test fail, restore; guard with a before/after presence check).
- Do NOT hand-regenerate SSOT artifacts; `ssot-autoregen` does it. Never edit `*.generated.md`. Docs gates listed in AGENTS.md "Auto-generated documentation files".
- New `docs/src/` pages need frontmatter + `topic`/`domain`/`authorship`/`authorship_model` + 40-120 word `<!-- kb:summary -->`; never hand-add `last_updated`; never touch `kb-facet-backlog.v1.yaml` entries. Prefer editing existing pages (SSOT §7).
- `crate-edges` rule: no new workspace-crate dependency edges; `exceptions` ledger entries USER-AUTHORIZED-ONLY. (This research found none needed: `vox-cli-ci` already depends on `vox-doc-pipeline`; `vox-cli` has `command_catalog`.)
- Hosted CI is the gate: locally run `cargo check -p <crate>` + `cargo nextest run -p <crate>` only; never workspace clippy/tests; background every cargo invocation (never block a foreground tool call on cargo). New `Rust` caches in workflows save from `main` only (`cache-key-lint`).
- Workflows: top-level `permissions:`, `concurrency:` with `cancel-in-progress: true`, literal `timeout-minutes` (schedule cap 180), scheduled workflow names listed in `nightly-report.yml`.
- Retired surfaces: do not reference retired symbols (e.g. `vox-dei`, `architecture-index.md`, `research-index.md`). Archive dirs are not to be ingested.
- Build broker: plain `cargo` on PATH only.
- Never pattern-kill; never write live userdata from a worktree.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Stage table + aggregate failure reporting for `ssot-drift` | `vox-cli` (`commands/ci/run_body_helpers/docs.rs`) | `vox-cli-ci` (stage fns) | `run_ssot_drift` lives in vox-cli; stage bodies live in vox-cli-ci |
| Regen-list parity (01) | `vox-cli` unit test over `scripts/ssot-regen.vox` + table | `lefthook.yml` retry list | the table is the SSOT; `.vox` file and lefthook line are consumers |
| Overlay parity (02) | `vox-cli-ci` (next to `retired_symbol_check.rs`) | contract YAML under `contracts/documentation/` | pure text/regex, no clap needed |
| CLI/"planned" mention check (03) | `vox-cli` (`commands/ci/`) | `vox-doc-pipeline::claims` extractor | needs `build_catalog()` (clap tree, in vox-cli) |
| Dead-guard hardening (04a) | `vox-cli` `docs.rs`, `vox-cli-ci` `check_links.rs`, `retired_symbol_check.rs` | — | edit in place |
| Orphan-page report + baseline (04b) | `vox-cli-ci` or `vox-cli` stage | `claims::prune::build_inbound_link_graph`, `kb::facet_backlog` pattern | graph + shrink-only gate already exist |
| Scheduled refresh PR (05) | GitHub Actions (new workflow) | `vox audit docs --update-ledger`, `lychee` | CI orchestration only; logic stays in `vox` |
| Meta-gate (06) | `vox-cli` stage with unit tests | `ci_workflow_contract.rs`-style probes | needs pre-push labels + workflows + SSOT markdown |

## Standard Stack

### Core (all already in the tree — no installs)
| Library / component | Where | Purpose | Why |
|---|---|---|---|
| `regex`, `serde_yaml`, `walkdir`, `pulldown-cmark` | `vox-doc-pipeline`, `vox-cli-ci` Cargo.toml (workspace deps) | text scanning, YAML contracts, tree walk, Markdown AST | already used by every sibling guard; linear-time regex (no ReDoS) |
| `claims::extract_mentions` / `GroundTruthRegistry::resolve_mention` | `crates/vox-doc-pipeline/src/claims/` | mention extraction (`MentionKind::CliCommand`) | reuse; tighten resolver, do not write a second extractor |
| `command_catalog::build_catalog()` | `crates/vox-cli/src/command_catalog.rs` | full clap tree incl. `arguments[].long` | only exhaustive ground truth for subcommands AND flags |
| `claims::prune::build_inbound_link_graph` | `crates/vox-doc-pipeline/src/claims/prune.rs` | target -> sources link graph | reuse for orphans (needs fixes, see F4) |
| `kb::facet_backlog::run_facet_backlog_gate` pattern | `crates/vox-cli/src/commands/kb/facet_backlog.rs` | epoch-based shrink-only baseline vs base ref | reuse for the orphan baseline |
| `lycheeverse/lychee-action@v2` (`lycheeVersion: v0.24.2`) | `docs-quality.yml`, `link_checker.yml` | link checker | already pinned/used |
| `gh pr create` + `SSOT_AUTOREGEN_TOKEN \|\| github.token` | `harness-eval-nightly.yml` | bot PR opening | in-repo precedent; `peter-evans/create-pull-request@v8` (release-prepare.yml) is the alternative |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|---|---|---|
| Parity test for regen list | Derive `ssot-regen.vox` from table (`vox ci ssot-regen`) | derive is cleaner but 5 regen-only entries (`operations-sync`, `test-inventory`, `crate-build-map-parity`, `affected-crates --regen`, `config-gui-codegen --kb-facets`) have no `ssot-drift` stage; derive forces new stages. Roadmap allows either; recommend table + parity test, derive as stretch. |
| Registry-only CLI ground truth | clap tree ∪ registry | registry omits 14 real `vox ci` subcommands (F3) and has no flags; clap tree omits feature-gated groups. Union is the only correct source. |
| `peter-evans/create-pull-request` | `gh pr create --draft` | `gh` pattern already used for bot PRs and needs no extra action; `--draft` flag verified available in `gh` 2.103.3. |

**Installation:** none. **Package Legitimacy Audit:** no external packages added by this phase (all crates/actions above already resolved in the workspace/workflows). Packages removed (SLOP): none. Flagged (SUS): none. If the plan adds ANY new action or crate, run `gsd_run query package-legitimacy check` and add a `checkpoint:human-verify`.

## Architecture Patterns

### System Architecture Diagram

```
 edit docs / AGENTS.md / overlays / workflows / CLI code
          |
          v
 PR  --> ci.yml `linux` job: `./target/debug/vox ci pre-push` (unconditional, fresh build)  [ci.yml:64-65]
          |                      \
          |                       `-> pre_push.rs build_steps(): fmt, line-endings, ssot-drift, ..., check-links, retired-symbol-check, ...
          v
   run_ssot_drift  ==> STAGE TABLE (new)  [name, run(root, auto_heal), regen argv]
          |  existing stages ............ (40 today)
          |  NEW: overlay_parity (02), cli_claims (03), orphan_pages (04b), docs_system_parity (06)
          |  each stage returns Result; table runner COLLECTS all failures -> one aggregated error
          v
   fail (CI) / auto-heal rewrite (local) --> PR only: `ssot-autoregen` job runs scripts/ssot-regen.vox (list must mirror table: 01)
          ^
 nightly cron -- docs-drift-refresh.yml (new, 05):
          build vox -> `vox audit docs --update-ledger` -> lychee (external, fail:false) -> if ledger content changed or link failures:
          force-update branch bot/docs-drift-refresh -> ONE draft PR (create-or-update) ; job fails only if PR step fails -> nightly-report.yml issue
```

### Recommended Project Structure
```
crates/vox-cli/src/commands/ci/
  run_body_helpers/docs.rs        # run_ssot_drift -> stage table (01); dead branches removed (04a)
  docs_cli_claims.rs              # NEW (03): uses build_catalog() + claims extractor
  docs_system_parity.rs           # NEW (06): §4/§6 probes
crates/vox-cli-ci/src/
  overlay_parity.rs               # NEW (02): contract-driven overlay checks
  orphan_pages.rs                 # NEW (04b) (or extend check_links.rs, see Pitfall 6)
contracts/documentation/
  overlay-claims.v1.yaml          # NEW (02) claims table with AGENTS.md anchors
  orphan-pages-baseline.v1.yaml   # NEW (04b) — ONLY after user decision gate
.github/workflows/docs-drift-refresh.yml   # NEW (05)
```

### Pattern 1: Stage table with per-stage regen metadata and aggregated failures
**What:** replace `ds!("x", expr)?` x40 with `const`-like `Vec<Stage>`; the runner executes every stage, times it (keep the `ssot-drift  {:>6}ms  {}` eprintln), collects `(name, err)`, then bails once with all names. Keep `auto_heal` semantics (`CI` unset or `VOX_AUTO_HEAL` set).
**When:** DOCS-DRIFT-01/06 and every new stage.
**Example (shape only; values come from the quotes in F1):**
```rust
struct Stage { name: &'static str, run: fn(&Path, bool) -> Result<()>, regen: Option<&'static [&'static str]> }
// regen = the `vox` argv ssot-regen.vox runs for this stage, e.g. Some(&["ci","command-sync","--write"])
```
Replace the three source-text probe tests (`run_ssot_drift_dispatches_cache_key_lint`, `..._kb_facets_ts`, `..._capability_index`) with one test iterating the table; keep a mutation proof (delete a row, test fails).

### Pattern 2: Claims table with verified anchors (overlay parity)
**What:** AGENTS.md is prose; extracting claims by NLP is brittle. Use `contracts/documentation/overlay-claims.v1.yaml`: each claim has `id`, `anchor` (a verbatim AGENTS.md substring that MUST still occur — extraction = verification, so the table cannot silently rot), `forbid` (regexes that overlays must not match outside a negation/"retired" context), `overlays` (file globs). Fail if an anchor vanished from AGENTS.md or an overlay matches `forbid`.

### Pattern 3: Shrink-only baseline = `facet_backlog` epoch pattern
`epoch` + `epoch_note`, gate compares to the base ref (`prepush_base_ref()`), rejects same-epoch additions, allows deletions. Reuse `file_at`/`docs_at` helpers.

### Anti-Patterns to Avoid
- **A second mention extractor.** Extend `claims::extractor`; add `.mdc` and agent-file inputs rather than new regexes in a new crate.
- **Registry-only or installed-binary ground truth.** The installed `vox` is stale/feature-limited (it flagged real commands in my measurement); compute ground truth in-process from the tree being built.
- **`.vox`-from-Rust coupling for the derive.** `scripts/ssot-regen.vox` runs as a subprocess of `vox`; do not try to make it read Rust tables at runtime.
- **Adding gates only to `docs-quality.yml`.** Its path filter omits `.cursor/**`, `.github/workflows/*` (other than itself), `lefthook.yml`; the fast tier in `ci.yml` is unconditional. New gates belong in the fast tier (via the stage table).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| Markdown mention extraction | new regex scanner | `claims::extract_mentions` (+ `MentionKind::CliCommand`) | already fence/`vox:skip`/placeholder aware |
| clap tree walk | `vox commands` subprocess or `--help` parsing | `command_catalog::build_catalog()` in-process | exact, includes flags; stale-binary-proof; note clap's deep recursion needs the big-stack workaround used by the catalog tests |
| Inbound link graph | new walker | `prune::build_inbound_link_graph` (after fixes in F4) | already resolves `.md`/`.mdx`/`index.md` |
| Shrink-only baseline | ad hoc JSON | `kb::facet_backlog` epoch gate | rejects growth vs base ref |
| Draft PR create-or-update | hand-rolled git/curl | `gh pr list --head`/`gh pr create --draft`/`gh pr edit` | in-repo precedent, no new action |
| Workflow wiring assertions | grep in bash | `serde_yaml` parse in a Rust test like `ci_workflow_contract.rs` | existing pattern; YAML-structure not substring |

**Key insight:** every gap in §6 is "a guard exists nearby but is permissive, scoped too narrowly, or silently skips". Prefer tightening/extending the neighbor over adding a parallel mechanism.

## Findings (verified this session)

### F1 — DOCS-DRIFT-01: the "stage table" does not exist, and the lists have already drifted
- `run_ssot_drift` is inline: `[VERIFIED: crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs:580]` `ds!("check_docs_ssot", check_docs_ssot(root))?;` ... through `:781` `ds!(\n "kb_facets_ts",` — 40 stages, each `?`-short-circuited (this is why only one error surfaces per ~2 min run). Auto-heal flag: `[VERIFIED: docs.rs:570]` `let auto_heal = std::env::var_os("CI").is_none() || std::env::var_os("VOX_AUTO_HEAL").is_some();`.
- `scripts/ssot-regen.vox` self-describes `[VERIFIED: scripts/ssot-regen.vox:6-7]` `// SOURCE OF TRUTH for the generators: run_ssot_drift() in` / `// crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs. Keep in sync.` and runs 15 regenerators (lines 26-43).
- **Concrete drift today:** `ssot-drift` has auto-healing stages `crate_api_docs` (`vox ci crate-api-sync --write`) and `wiki_sync` (`vox ci wiki-sync --write`, clap name verified in `crates/vox-cli-ci/src/cmd_enums.rs:102` `#[command(name = "wiki-sync")]`) that **do not appear in ssot-regen.vox**, so the PR `ssot-autoregen` bot cannot fix drift in those artifacts.
- **Third hand-kept list:** `lefthook.yml:77` (pre-push retry) regenerates a different subset (`... crate-api-sync --write && ...`), omitting `command-sync`, `operations-sync`, `gui-version-sync`, `config-gui-codegen`, `test-inventory`, `affected-crates --regen`, `crate-build-map-parity`, `wiki-sync`. SSOT §4 prose describes this list. `[CITED: lefthook.yml:77]` — Plan must decide: include lefthook's list in the parity test (recommended) or shrink it to "run `vox run scripts/ssot-regen.vox`".
- `ci.yml` bot step: `[CITED: .github/workflows/ci.yml:417-421]` runs `./target/debug/vox run scripts/ssot-regen.vox || echo "::warning::..."` (best effort; gate re-verifies).
- `ssot-drift` tests that pin stage presence are brittle source-text probes (`docs.rs:983-1017`, three tests with split needles). A table replaces them.

### F2 — DOCS-DRIFT-02: overlay contradictions are broader than the one named
Verified contradictions (AGENTS.md is the authority per SSOT §5):
1. `[VERIFIED: .cursor/rules/documentation-policy.mdc:11]` `- New doc pages must include \`category\`, \`status\`, \`last_updated\`, and \`training_eligible\` frontmatter` vs AGENTS.md: "Do **not** hand-add `last_updated`" and the minimum-key list (`title, description, category, topic, domain, authorship`; no `status`/`training_eligible`). Lint treats hand-authored `last_updated` as a hard error (`crates/vox-doc-pipeline/src/pipeline/lint.rs:441` `} else if line.starts_with("last_updated:") {`; message at `pipeline/mod.rs:640`).
2. Same file, line 7: `use \`{{#include}}\` from \`examples/golden/\`` vs AGENTS.md "Always write inline \`\`\`vox blocks, do NOT use mdBook `{{#include}}` directives for new code."
3. `[CITED: docs/src/AGENTS.md:17]` `\`title\`, \`description\`, \`category\`, \`status\`, \`training_eligible\` are required (\`last_updated\` is optional metadata...` — **an overlay not in the roadmap's list**; include `docs/src/AGENTS.md` in the overlay set.
4. `[CITED: .cursor/rules/build-environment.mdc]` instructs `& "$env:USERPROFILE\.cargo\bin\cargo.exe" <args>` vs AGENTS.md Build Broker "never call the real cargo binary by an explicit resolved path (`~/.cargo/bin/cargo`...)". (Windows-flavoured; confirm intent with user — Assumption A5.)
- Existing coverage to build on: `retired_symbol_check::run` scans `["AGENTS.md", "GEMINI.md", "CLAUDE.md"]` `[VERIFIED: crates/vox-cli-ci/src/retired_symbol_check.rs:520]` and `.cursor/rules/*.mdc` (`:539-543`), each behind `if path.is_file()` / `is_dir()` silent skips, and **omits `.github/copilot-instructions.md` and `docs/src/AGENTS.md`**. `check_stale_doc_and_workflow_refs` (`docs.rs:338-346`) lists GEMINI/CLAUDE/copilot in `EXTRA_BANNED_SCAN_TARGETS` but only for fixed path substrings.
- `copilot-instructions.md` and `GEMINI.md` themselves carry `training_eligible: true` frontmatter (read this session) — fine, they are not docs/src pages, but any claim "`training_eligible` forbidden" would false-positive; do NOT claim that (the lint only forbids `true` without `training_rationale`/forced-ineligible pages, `facets.rs:104`).

### F3 — DOCS-DRIFT-03: extractor exists, resolver is too permissive, registry is not exhaustive
- Extractor emits `MentionKind::CliCommand` for backticked `vox ...`/`cargo run -p vox...` and fenced lines `[VERIFIED: crates/vox-doc-pipeline/src/claims/extractor.rs:208]` `if raw.starts_with("vox ") || raw.starts_with("cargo run -p vox") {` and `:300-302`.
- Resolver accepts a command if its FIRST token is known: `[VERIFIED: crates/vox-doc-pipeline/src/claims/resolver.rs:505-508]` `if subcmds.is_empty()\n || self.cli_commands.contains(&full)\n || self.cli_commands.contains(subcmds[0])` — so `vox audit anything` resolves. It also seeds ~65 hard-coded names (`:177-254`, e.g. `"oratio"`, `"ludus"`, `"generate-data"`), ignores registry `status`, and never checks flags.
- The drift gate scope is reader-facing `status: current` docs only `[CITED: crates/vox-doc-pipeline/src/claims/gate.rs is_reader_facing_page]` and its baseline is empty: `[CITED: contracts/reports/docs-freshness/baseline.v1.json]` `"deficits": []`. Agent files (AGENTS.md, overlays) are NOT scanned by `vox audit docs` (`scan_corpus` walks `docs/src` only, `claims/mod.rs:72`).
- **Registry is not an exhaustive CLI truth:** measured (script in scratchpad): clap names in `crates/vox-cli-ci/src/cmd_enums.rs` = 160, registry `ci` children = 154; 14 clap `vox ci` subcommands absent from `contracts/cli/command-registry.yaml` (`burn-rnd, check-changed, ci-changes, dom-test-env-ban, emit-reports, evidence-asset-ban, hit-every-surface, ingest-trust, knip-gui, preview-trust-audit, suppression-review-gate, transfer-budget-report, vouched-parse, wiki-sync`) [VERIFIED: ran `python3 -I .../ci_diff.py`]. The registry has no flag data (`grep -c -- "--write" command-registry.yaml` = 1) and only `active|deprecated|retired` statuses (1042/2/1). So the roadmap's "absent from command-registry.yaml" would false-positive on 14 real commands and cannot check flags.
- **Ground truth to use:** `[VERIFIED: crates/vox-cli/src/command_catalog.rs:102-103]` `pub fn build_catalog() -> CommandCatalog {` / `let root = crate::VoxCliRoot::command();` — walks the full tree with `CommandCatalogArgument.long`. Caveat verified in the same file (`:77-99`): feature-gated groups are invisible to a default build (`feature_gated_group_names()` returns `("dei","dei"), ("visus","dei"), ("safety","dei"), ("attention","dei")`), and my measurement with the default-feature installed binary also flagged `vox skill ...` — so union with registry paths (which carry `feature_gate`). For registry-only (not compiled) paths: check command existence only; skip flag checks.
- **Measured sizing** [VERIFIED: ran `cli_mentions2.py`, clap(617 paths, installed binary `vox commands --format json --include-nested`) ∪ registry(1028 paths)]: reader-facing `status: current` pages: 1104 backticked `vox ...` mentions, **87 unresolved** (e.g. `vox agent status`, `vox container init`, `vox training native`, `vox schola merge-qlora`, `vox populi config check`, `vox dashboard`). Agent files: 94 mentions, 9 unresolved on the default installed tree (some are feature-gated/stale-binary artifacts: `vox skill list`, `vox ci dev-loop-guard`; real candidates: `vox wasm run` (retired-table row — needs negation/retired-row skip), `vox schema generate` in `data-storage-policy.mdc:22`). Whole `docs/src` non-archive: 880 unresolved of 5046 (mostly ADR/architecture history) — **do not gate the whole tree**; keep the reader-facing + agent-file scope. Numbers are crude (inline-code only, installed binary); Wave 0 re-measures with the in-process catalog.
- "planned per" claims: `[CITED: AGENTS.md:205]` `- \`vox audit retirement\` — \`vox ci retirement-audit\` (planned per CR-L6)` and `:217` `\`vox audit effort\` (planned per [...]`; registry confirms both exist (`audit retirement`, `audit effort` under `audit`; `[CITED: command-registry.yaml:1283,1319]`). AGENTS.md has 4 "planned per" occurrences; lines 230/234 are about skills/roots, and line 234 also contains the real command `vox skill discover` — a naive "planned + existing command on the same line" rule would false-positive there. Use **adjacency**: a `vox ...` code span immediately followed (≤ ~8 chars, optionally `— `) by `(planned` / `planned per`. Fixtures for both lines.

### F4 — DOCS-DRIFT-04: dead guards (verified) and orphan sizing
Silent no-ops / "green while checking less":
1. `check_docs_ssot` orphan inventory: `[VERIFIED: docs.rs:60-61]` `let inv = root.join("docs/src/architecture/orphan-surface-inventory.md");\n if inv.is_file() {` with an `else` at `:104` that silently substitutes. File deleted (git: `a9d9641de`, 2026-04-23); `ls` confirms absent. Crate-roster parity is already covered by `vox-arch-check` Rule 12 `[CITED: crates/vox-arch-check/src/main.rs:32]` ("WTL / layers.toml / disk three-way parity"). **Recommended fix: delete the dead branch (retired surface), keep the `else` logic as the sole path** — failing loudly on a deliberately deleted file would break CI. The roadmap's "fail loudly" is satisfied by removal + a guard that lists its required inputs.
2. `docs.rs:155-156` `let arch_idx_path = root.join("docs/src/architecture/architecture-index.md");\n if arch_idx_path.is_file() {` — file retired (AGENTS.md: "do not revive `docs/src/architecture/architecture-index.md`"); branch is dead too (a second instance the roadmap doesn't name).
3. `docs.rs:210-213` stale-`training_eligible` warning depends on `md.find("\nlast_updated: ")`, a key the lint now forbids — can never fire legitimately.
4. `check_links::run`: `[VERIFIED: check_links.rs:135-142]` `if !docs_src.exists() {\n println!(... "SKIP" ...);\n return Ok(());` and `:127-133` (explicit target missing -> SKIP `Ok`); plus `[VERIFIED: check_links.rs:79-91]` expired allowlist entries print `WARN` `"allowlist entry expired (still skipping)"` and `return true` — expired entries keep suppressing. **Allowlist entries in `link-allowlist.v1.yaml` expire `2026-11-30` (24 days out)**; plan must decide: fail on expiry (loud) and refresh/fix entries.
5. `retired_symbol_check.rs:520-537` `if path.is_file()` skip for AGENTS/GEMINI/CLAUDE; `:539-540` `.cursor/rules` `is_dir()` skip; copilot-instructions never scanned.
6. `docs-self-heal.yml` `stage-draft-prs`: `[VERIFIED: .github/workflows/docs-self-heal.yml:59]` `run: cargo test -p vox-doc-pipeline --lib self_heal_bot` — a job named "stage-draft-prs" that only runs unit tests (gated on `workflow_dispatch`). `bot-metrics.v1.json` is static counters (`"accepted_prs": 5`).
7. `frontmatter_governance_parity.rs` uses `contains(...)` only (weak).
8. Deliberate (leave): `workflow_permissions_guard::run` returns `Ok` without a workflows dir (commented rationale).
- Pattern for the fix: repo-mode (no explicit target) requires its root inputs; mutation-proof test per site (delete/rename input, guard must fail). `vox-doc-pipeline --paths a b` first-path-only bug (fixed on this branch) is the same class; also check any other `--paths`/`target` handling when touching these.
- Orphan sizing [VERIFIED: ran `orphans.py` — regex link graph over `.md/.mdx` in `docs/src` (non-archive) with sources = docs/ + root `.md`]: 886 pages, **229 with zero inbound links**; by dir: architecture 149, reference 29, how-to 15, ci 8, contributors 5, explanation 4, operations 4, journeys 4…; **reader-facing non-generated: 48** (e.g. `how-to/index.md`, `reference/ref-stdlib.md`, `reference/changelog.md`). Rough: the real implementation's resolver rules decide the final count. SSOT claim "sidebar lists everything by frontmatter" holds (`docs-astro/src/utils/sidebar.mjs`; `page-status.mjs` only moves research/roadmap to an Internals section), so "no nav entry" is vacuous; orphan = **zero inbound links**.
- `prune::build_inbound_link_graph` limits: walks only the `docs_dir` it is given (`claims/prune.rs:65`), no archive exclusion, ignores links from root guides (AGENTS.md etc.) and `.mdx`-sourced links resolve but fenced/inline-code links are not skipped (check_links skips them). Either fix it or add inbound recording to `check_links::run`'s loop (it already resolves every link and skips fences/inline code; it must then also read `.mdx`).

### F5 — DOCS-DRIFT-05: scheduled loop
- `--update-ledger` never re-attests: `[VERIFIED: claims/ledger.rs:260-265]` `verified_hash` is carried from the previous ledger (`.map(|a| a.verified_hash.clone())`), only `content_hash`, `state`, metrics change. Safe to automate. Human re-attest stays `--attest <page>`.
- **Not idempotent:** `[VERIFIED: ledger.rs:389]` `updated_at: chrono::Utc::now().to_rfc3339(),` and `head_commit` change every run -> 7 MB `ledger.v1.json` (`contracts/reports/docs-freshness/`) would differ nightly. Fix first (test-first): in `audit.rs` `update_ledger` branch skip the write when `pages` and `metrics` equal the existing ledger (add `PartialEq` if absent). Without it the job opens a PR every night.
- Link checker today: `[VERIFIED: .github/workflows/link_checker.yml:45]` `fail: true`, cron `'0 6 * * *'`; failure -> `nightly-report.yml` (lists `Docs Link Checker`) opens the `nightly-failure` issue #578 (state OPEN, "Nightly failing: Docs Link Checker"). Lychee exclusions use regex in `.lycheeignore` (236 lines).
- Workflow guards a new scheduled workflow must satisfy: literal `timeout-minutes` (schedule cap `SLOW_CAP_MINS = 180`), name listed in `nightly-report.yml` `on.workflow_run.workflows` `[VERIFIED: crates/vox-cli-ci/src/workflow_policy_guard.rs:10-13, 28-30]`; top-level `permissions:` (`workflow_permissions_guard`); `concurrency:` with `cancel-in-progress: true` (`workflow_concurrency_guard`); `ci-liveness.yml` derives its watch list from `nightly-report.yml` via `yq` (so listing there covers liveness); Rust caches save from `main` only (`cache-key-lint`).
- PR-opening precedent: `[VERIFIED: .github/workflows/harness-eval-nightly.yml:~100-118]` (branch push with `https://x-access-token:${GH_TOKEN}@github.com/...`, `GH_TOKEN: ${{ secrets.SSOT_AUTOREGEN_TOKEN || github.token }}`, warning that GITHUB_TOKEN PRs do not trigger required checks); job-scoped `contents: write` / `pull-requests: write` with top-level `contents: read` as in `ci.yml:383-390` and `docs-self-heal.yml`. Pin privileged-job actions by SHA as `ssot-autoregen` does (`ci.yml:392-399`).
- Reality-audit state (measured): `findings.v1.json` is 44 bytes (empty), `inventory.v1.json` 5.7 KB (hand-written, dated 2026-09-04), `metrics.v1.json` 293 B; the governance page banner says "**Dormant (2026-10-08)**... Phase 21... reactivates it" (`docs/src/contributors/docs-reality-audit-program.md`), but `docs_reality_audit::run_verify` is a live `ssot-drift` stage that only schema-validates those artifacts, and the real live machinery is the claims engine (`vox audit docs --check-drift`, ledger). `vox audit docs --write-findings` exists (`audit.rs:68-70`). `self_heal_bot` (462 lines) needs an LLM-proposed patch to be useful; `bot-metrics` never moves.

### F6 — DOCS-DRIFT-06: meta-gate mechanics
- Fast-tier wiring is introspectable: `build_steps` returns `OwnedStep { label, ... }` with labels like `[VERIFIED: pre_push.rs:509]` `label: "vox ci check-links".into(),`, `"vox ci retired-symbol-check"`, `"vox ci canonical-map-verify"`, `"vox ci ssot-drift"`, `"vox-doc-pipeline facet backlog (tighten-only vs base)"`; `--dry-run` prints them. `build_steps` is private to `pre_push.rs`, so call it from a `#[cfg(test)]` inside that module or expose a `pub(crate) fn fast_tier_labels(root)`.
- `docs-quality.yml` jobs: steps verified by `Read` — `cargo run -p vox-doc-pipeline -- --lint-only`, `ci doctest-md --strict`, `audit docs --check-drift`, `ci check-links`, `ci canonical-map-verify`, lychee offline + selftest, build-output Playwright; job `generator-drift`: `ci sync-ignore-files --verify`, `ci command-sync`. `link_checker.yml` is the nightly external lychee.
- Precedent for workflow probes: `ci_workflow_contract.rs` (`include_str!` + `serde_yaml` e.g. `ci_affected_prefilter_regex_admits_contracts_and_dot_config`). Test crate placement caveat: a `vox-doc-pipeline` integration test (precedent `frontmatter_governance_parity.rs`) does **not** run on a docs-only PR (docs-quality never runs `cargo test`; affected-crates sees no crate change). An in-process `ssot-drift` stage runs on every PR via the unconditional fast tier (`ci.yml:64-65`) -> recommended.
- §6 machine-checking requires stable ids: tag each §6 bullet with `<!-- gap: <id> -->`; test maps `id -> closing-gate probe`; fails if a bullet's gate exists (gap should be removed) or a closed gap has no §4 row.
- Freshness: `vox ci *` refuses to run on a stale installed binary unless `VOX_SKIP_FRESHNESS_CHECK` is set (`[VERIFIED: crates/vox-cli/src/freshness.rs:30-31, 249-253]`), so locally a new stage is not enforced until reinstall; hosted CI builds fresh. Mention in the plan's verification notes; do not rely on pre-push locally to prove a new guard.

## Common Pitfalls

### Pitfall 1: Cascade failures once stages aggregate
**What goes wrong:** with collect-all, a failing generator stage can make a dependent verify stage fail too (e.g. `plugin_catalog_sync` -> `generate_plugin_catalog_docs`, crate graph -> `check_graph`).
**How to avoid:** keep stage order; print all failures but mark later ones "(may be a cascade of <earlier>)" only if cheap; do NOT reorder stages. **Warning sign:** a PR shows 5 errors, fixing the first removes 3.

### Pitfall 2: Strict CLI resolver floods the PR with 87 doc defects
**What goes wrong:** tightening resolver turns an empty baseline into ~87 failures on reader-facing pages.
**How to avoid:** separate plan tasks: (a) detector behind a not-yet-enforcing mode with a measured count, (b) fix the doc lines (many are retired commands -> point at replacements), (c) enable. Do NOT regenerate `baseline.v1.json` to admit them (growth = user-authorized only).

### Pitfall 3: Ground truth drift between binaries
**What goes wrong:** installed/default-feature `vox` omits feature-gated commands and may predate new ones -> false positives locally.
**How to avoid:** compute in-process from the tree under test; union registry for gated paths; skip flag checks for non-compiled paths.

### Pitfall 4: Negation context in overlays
**What goes wrong:** `copilot-instructions.md`/`retired-surfaces.mdc` legitimately list retired items ("NOT `vox-dei`"); `AGENTS.md` retired-surfaces table lists `vox wasm run`.
**How to avoid:** reuse `retired_symbol_check`'s skip logic (skips table rows, lines containing "retired"/"NOT"); fixtures for each.

### Pitfall 5: Non-idempotent bot -> daily PR spam
Fixed by F5's idempotence change + `create-or-update` single branch `bot/docs-drift-refresh`. Also GITHUB_TOKEN-opened PRs do not trigger required checks (warn, as `harness-eval-nightly.yml` does).

### Pitfall 6: Orphan false positives
Index pages (`index.md[x]`), `*.generated.md` hubs, `docs/src/AGENTS.md`, `.well-known/`, `archive/`, `ref/` redirect stubs. Links from `.mdx` sources and from repo-mounted root docs (AGENTS.md, README) must count as inbound. Count differences between my regex (229) and the implementation are expected; define exemptions explicitly in the test.

### Pitfall 7: Shared files serialize the plans
`docs.rs`, `pre_push.rs`, `docs-system-ssot.md` (§4 row + §6 bullet removal), `detector-coverage-ledger.md` (row per gate), `contracts/operations/catalog.v1.yaml` + `command-registry.yaml` (new `vox ci` subcommands need catalog rows + `operations-sync`, `docs/src/reference/cli.md` rows per `cli-command-registry.mdc`) are touched by 02/03/04/06. Run those plans in serial waves, or give each plan disjoint files and let only 06 touch the SSOT §4/§6 aggregate (but then "remove bullet in same PR" per gap is violated; prefer serial).

### Pitfall 8: New `vox ci` subcommands carry registry/catalog obligations
Adding a `CiCmd` variant requires: operations catalog row -> `vox ci operations-sync --target all --write` (regenerates registry, reference docs), `gate_policy_id` optional mapping, `docs/src/reference/cli.md`/generated surface. Prefer **no new subcommand**: add stages in the table and call the library fns in-process (pre-push precedent: `step_check_links`). Add a subcommand only if a standalone invocation is genuinely needed.

## Code Examples

### Parity test shape (01) — values quoted in F1
```rust
// regen argv in ssot-regen.vox looks like:
//   let c = regen(["--quiet", "ci", "generate-plugin-catalog-docs"], "ci generate-plugin-catalog-docs")
// [VERIFIED: scripts/ssot-regen.vox:28]
// Test: parse every `regen([ ... ], "label")` literal from include_str!("../../../../../../scripts/ssot-regen.vox"),
// drop the leading "--quiet", compare as a set against STAGES.iter().filter_map(|s| s.regen) ∪ REGEN_ONLY (each with a reason).
// Fail with the symmetric difference. Mutation proof: delete one regen(...) line / one table row, assert failure, restore.
```

### Scheduled workflow skeleton (05) — structure only
```yaml
# Source: patterns verified in harness-eval-nightly.yml, ci.yml (ssot-autoregen), link_checker.yml
name: Docs Drift Refresh          # MUST be added to nightly-report.yml on.workflow_run.workflows
on: { schedule: [{ cron: '30 6 * * *' }], workflow_dispatch: {} }
concurrency: { group: ${{ github.workflow }}, cancel-in-progress: true }
permissions: { contents: read }   # top-level required by workflow_permissions_guard
jobs:
  refresh:
    runs-on: ubuntu-latest
    timeout-minutes: 60           # literal; schedule cap is 180
    permissions: { contents: write, pull-requests: write }
    steps:
      # checkout (SHA-pinned like ssot-autoregen), setup-rust, build vox
      # `vox audit docs --update-ledger`  (idempotent after the F5 fix)
      # lychee external, fail: false, output markdown report
      # if git diff --quiet && no link failures: exit 0
      # else: force-push branch bot/docs-drift-refresh; `gh pr list --head` -> `gh pr create --draft` or `gh pr edit`
```
Decision (A4): only auto-propose `.lycheeignore` lines for bot-block statuses (403/429/5xx), list 404/DNS failures in the PR body for humans; never silently ignore 404s.

## State of the Art

| Old approach | Current approach | Impact |
|---|---|---|
| Hand-kept regen list ("Keep in sync") | table + parity test (this phase) | closes F1 drift (`crate-api-sync`, `wiki-sync` missing from the PR bot) |
| First-error-wins `ssot-drift` | aggregated failures | one run shows all gaps (user's learned fact verified: `?` at every stage) |
| "Dormant" reality-audit program | claims engine + ledger live; hand-curated inventory/findings stale | decision needed (A3) |

**Deprecated/outdated:** `docs/src/architecture/orphan-surface-inventory.md` and `architecture-index.md` (deleted/retired; guards still reference them); `docs-self-heal.yml` `stage-draft-prs` (tests only).

## Runtime State Inventory

Not a rename/refactor/migration phase — omitted. (One related note: `contracts/reports/docs-freshness/ledger.v1.json` is committed state that the new job rewrites; see F5.)

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Parity test (not derive) is the right 01 deliverable; `crate-build-map-parity` / `test-inventory` / `affected-crates --regen` are regen-only (no `ssot-drift` stage) | F1 | derive may be wanted; REGEN_ONLY list could hide a stage I didn't map |
| A2 | Orphan scope should be reader-facing only (≈48) with either linking-away or a user-authorized baseline; alternative is all of `docs/src` (≈229) | F4 | baseline size/decision; user may want full tree |
| A3 | Recommend REVIVE-minimal reality-audit (scheduled job adds `--write-findings`) and RETIRE the self-heal `stage-draft-prs` job (keep `verify-claims-readonly`); user confirms | F5 | retire-vs-revive blast radius (8+ surfaces if reality-audit retired) |
| A4 | Auto-propose `.lycheeignore` only for 403/429/5xx; 404/DNS go to PR body | F5 | policy call on suppressing link failures |
| A5 | `build-environment.mdc` explicit `cargo.exe` path contradicts the Build Broker rule (intent: Windows agent shells) | F2 | may be an intentional Windows exception; confirm before "fixing" |
| A6 | Overlay claims via a YAML claims table with AGENTS.md verbatim anchors (not NLP extraction, not marker blocks inside AGENTS.md) | Pattern 2 | new contract file; user may prefer markers in AGENTS.md |
| A7 | `--write-findings` output is deterministic and small enough for a PR | F5 | job could churn; measure in Wave 0 |
| A8 | Unresolved-mention counts (87 / 9 / 229 / 48) are crude (inline-code only, installed default-feature binary, regex link graph) | F3/F4 | sizing of fix tasks |
| A9 | Whether `vox ci pre-push` should also keep-going past the first failing step (outer loop `pre_push.rs:248-265` returns at first failure) | F1 | out of scope unless user wants it |

## Open Questions

1. **New baseline seeding (USER DECISION GATE).** Orphan-page baseline: reader-facing only (~48) vs whole tree (~229) vs "no baseline, add inbound links from section hubs". What we know: facet-backlog epoch pattern is reusable. Unclear: user preference. Recommendation: plan a `checkpoint:decision` before seeding; default to linking from hubs for reader-facing pages to avoid a baseline.
2. **Fix-or-baseline for ~87 CLI mention defects** (Pitfall 2). Recommendation: fix the doc lines in-phase (many are retired-command references); no baseline growth.
3. **`link-allowlist.v1.yaml` entries expire 2026-11-30.** Make expiry fatal now (needs refresh/fix of entries, user may need to approve extensions) or keep "still skipping"? Recommendation: fatal expiry + resolve entries in the DOCS-DRIFT-04 plan.
4. **Reality-audit / self-heal decision (A3).**
5. **`lefthook.yml:77` retry list:** include in parity or replace with a call to `scripts/ssot-regen.vox`? Needs a decision because the hook must not need a fresh build.
6. **Should `.github/copilot-instructions.md` and `docs/src/AGENTS.md` join the retired-symbol scan** (overlap with 02)? Recommendation: yes, fold into 02's overlay set.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| cargo / rustc | all Rust work | yes | cargo 1.98.1 (matches `rust-toolchain.toml` `channel = "1.98.1"`) | — |
| cargo-nextest | validation | yes | `~/.cargo/bin/cargo-nextest` | `cargo test -p` |
| gh CLI (authenticated) | workflow design/#578 lookup | yes | 2.103.3 | — |
| lychee | local link-check dry run | yes | `/opt/homebrew/bin/lychee` | CI-only |
| yq | `ci-liveness.yml` parity checks | yes | `/opt/homebrew/bin/yq` | — |
| act | optional local workflow run | yes | `/opt/homebrew/bin/act` | hosted CI is the gate |
| installed `vox` | measurement only | yes | 0.6.0+build.7512 (2953cfc99) — stale vs tree | build from tree (background) |
| `SSOT_AUTOREGEN_TOKEN` secret | PR that triggers CI | unknown (owner-only) | — | `github.token` fallback + warning (precedent) |

**Missing with no fallback:** none. Local CI is not needed: per `~/.claude/CLAUDE.md`, GitHub Actions may be billing-blocked — if a PR check shows "never started", run the touched crate's tests locally (wrap in `lockf -k ~/.cache/local-ci.lock`) and say so in the PR.

## Validation Architecture

### Test Framework
| Property | Value |
|---|---|
| Framework | Rust built-in `#[test]` run with `cargo nextest` (`cargo test -p` fallback); `serde_yaml`-based workflow probes |
| Config file | `.config/nextest.toml` (profiles `ci`, `ci-gate`) |
| Quick run command | `cargo nextest run -p vox-cli-ci <filter>` / `cargo nextest run -p vox-cli <filter>` / `cargo test -p vox-doc-pipeline --test <name>` (background it; never block foreground on cargo) |
| Full suite command | hosted CI (`vox ci pre-push` fast tier + `tests` shards); do not run workspace-wide locally |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|---|---|---|---|---|
| DOCS-DRIFT-01 | stage table ↔ `ssot-regen.vox` ↔ lefthook parity; aggregate failure collection; table replaces 3 source-probe tests | unit | `cargo nextest run -p vox-cli ssot_drift` | Wave 0 (new tests in `docs.rs`) |
| DOCS-DRIFT-02 | each contradiction fixture fails; each anchor present in AGENTS.md; negation fixtures pass; mutation: delete an overlay fix, gate fails | unit | `cargo nextest run -p vox-cli-ci overlay_parity` | Wave 0 |
| DOCS-DRIFT-03 | extractor→resolver: unknown subcommand, unknown flag, retired status, "planned" adjacency (AGENTS.md lines 205/217 fixtures + line 234 negative), feature-gated union | unit | `cargo nextest run -p vox-cli docs_cli_claims` | Wave 0 |
| DOCS-DRIFT-04 | each silent-skip site fails when input missing (temp-dir fixtures); orphan graph incl. exemptions; baseline rejects growth | unit | `cargo nextest run -p vox-cli-ci check_links orphan` | Wave 0 (extend `check_links.rs` tests) |
| DOCS-DRIFT-05 | ledger write skipped when content unchanged (idempotence); workflow passes `workflow_policy_guard`, permissions, concurrency, listed in `nightly-report.yml` | unit + workflow probe | `cargo nextest run -p vox-doc-pipeline ledger` ; `cargo nextest run -p vox-cli --test ci_workflow_contract` | Wave 0 |
| DOCS-DRIFT-06 | every §4 row has a probe and the probe's wiring exists; every §6 bullet has a gap id and no closing gate; mutation proofs | unit (fixture SSOT markdown + real tree smoke) | `cargo nextest run -p vox-cli docs_system_parity` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo check -p <touched crate>` + the new tests only (GSD lean verification).
- **Per wave merge:** `vox ci pre-push` (fast) from a **freshly built** binary, in background; `vox ci ssot-drift` on the branch.
- **Phase gate:** hosted CI green on the draft PR (`gate` job), then `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] Re-measure F3/F4 counts with the in-process catalog and the real resolver before choosing enforce/baseline.
- [ ] Mutation-proof harness notes per new guard (delete input / delete table row / revert overlay fix; before/after presence check as AGENTS.md requires).
- [ ] No test framework install needed.

## Security Domain

`security_enforcement` is not set to false in `.planning/config.json` (key absent = enabled).

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---|---|---|
| V2/V3 Authentication/Session | no | — |
| V4 Access Control | yes (CI privileges) | job-scoped `contents: write` / `pull-requests: write` only on the refresh job; top-level `permissions: contents: read`; scheduled on `main` only |
| V5 Input Validation | yes | docs/overlay/Markdown content is untrusted input to scanners: use `regex` (linear time), bounded reads (`vox_bounded_fs::read_utf8_path_capped`), YAML via `serde_yaml` typed structs |
| V6 Cryptography | no | — (do not touch `vox-crypto`; hashing in ledger already via existing hasher) |
| V14 Config / supply chain | yes | no new actions/crates; pin privileged actions by SHA as `ssot-autoregen` does; pass tokens and PR-derived values via `env:` (never inline `${{ }}` in `run:`) |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---|---|---|
| Expression injection via `${{ ... }}` in `run:` (branch names, PR titles) | Tampering/EoP | env-var indirection (precedent: `PUSH_TOKEN`, `PR_HEAD_REF` in `ci.yml`) |
| Bot PR auto-ignores real broken links (`.lycheeignore` growth hides defects) | Repudiation/Tampering | A4: only transient-status auto-proposals; draft PR; human review; never auto-merge |
| Bot token scope creep (`SSOT_AUTOREGEN_TOKEN` PAT) | EoP | use only for push/PR; fallback `github.token`; no secrets in logs |
| Guard bypass by deleting guard input (silent no-op) | Tampering | F4 fixes + mutation tests |
| ReDoS in claim `forbid` regexes from a YAML contract | DoS | `regex` crate only; contract entries reviewed; compile-time check test |

## Sources

### Primary (HIGH confidence — read or executed this session)
- `docs/src/architecture/docs-system-ssot.md`; `docs/src/contributors/documentation-governance.md` (forbidden keys, ledger notes); `AGENTS.md`; `CLAUDE.md`
- `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs` (stage chain, dead branches, tests); `scripts/ssot-regen.vox`; `crates/vox-cli/src/commands/ci/pre_push.rs` (`run`, `build_steps`, steps); `crates/vox-cli/tests/ci_workflow_contract.rs`; `crates/vox-cli/src/command_catalog.rs`; `crates/vox-cli/src/freshness.rs`
- `crates/vox-cli-ci/src/check_links.rs`, `retired_symbol_check.rs`, `docs_deprecated_command_guard.rs`, `docs_reality_audit.rs`, `workflow_policy_guard.rs`
- `crates/vox-doc-pipeline/src/claims/{mod,extractor,resolver,types,gate,prune,ledger,self_heal_bot}.rs`
- Workflows: `.github/workflows/{ci,docs-quality,link_checker,docs-self-heal,nightly-report,ci-liveness,harness-eval-nightly,release-prepare}.yml`; `lefthook.yml`; `.cursor/rules/*.mdc`; `GEMINI.md`; `.github/copilot-instructions.md`
- Contracts: `contracts/cli/command-registry.yaml`, `contracts/documentation/{retired-symbols,link-allowlist,kb-facet-backlog}.v1.yaml`, `contracts/reports/docs-freshness/*`, `contracts/reports/docs-reality-audit/*`
- Measurements: scratchpad scripts `orphans.py`, `ci_diff.py`, `cli_mentions.py`, `cli_mentions2.py` (outside repo); `gh issue view 578`; `vox commands --format json --include-nested`.

### Secondary (MEDIUM)
- `docs/src/contributors/docs-reality-audit-program.md` banner ("Dormant (2026-10-08)").

### Tertiary (LOW)
- None. No web research was needed (no external libraries involved).

## Recommended plan decomposition (for the planner)

| Plan | Reqs | Wave | Depends | Key tasks |
|---|---|---|---|---|
| 343-01 | 01 | 1 | — | stage table + aggregate runner (keep timing line, `auto_heal`); replace 3 source-probe tests; parity test vs `ssot-regen.vox` + lefthook decision; add the missing `crate-api-sync`/`wiki-sync` regenerators to `ssot-regen.vox`; remove §6 "Hand-kept sync lists" bullet; update §4 `ssot-drift` row |
| 343-02 | 02 | 2 | 01 | `overlay-claims.v1.yaml` + `overlay_parity` stage; scope = AGENTS.md vs `.cursor/rules/*.mdc`, GEMINI, CLAUDE, copilot, `docs/src/AGENTS.md`; fix the 3-4 contradictions (F2); extend retired-symbol scan to copilot; mutation proofs; §6/§4/ledger rows |
| 343-03 | 03 | 3 | 01 | `docs_cli_claims` stage (clap ∪ registry), extend extractor to agent files, "planned" adjacency rule, tighten resolver (drop first-token fallback/hard-coded list or keep behind union), fix AGENTS.md lines 205/217 and the ~87 reader-facing mentions; no baseline growth |
| 343-04 | 04 | 4 | 01 | remove dead branches (orphan inventory, architecture-index, stale-training warning), make `check_links` repo-mode inputs mandatory + fatal allowlist expiry, orphan stage; **checkpoint:decision** before any baseline seed |
| 343-05 | 05 | 5 | 03, 04 | ledger idempotence (test-first); `docs-drift-refresh.yml`; link_checker.yml `fail:false` + report; add to `nightly-report.yml`; decision A3 (reality-audit, self-heal) executed |
| 343-06 | 06 | 6 | all | `docs_system_parity` stage with §4 probes + §6 gap ids; remove final §6 bullets; final SSOT §4/§6 + governance cross-links |

Serialize waves (Pitfall 7). Each plan ends with: SSOT §4 row + §6 bullet removal in the same commit set, detector-coverage-ledger row, mutation proof, `cargo check -p` + new tests only.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all components exist in-tree and were read.
- Architecture: HIGH for placement/serialization; MEDIUM for A1/A6 design choices (user may differ).
- Pitfalls: HIGH for F1/F4/F5 (verified lines); MEDIUM for sizing numbers (A8).

**Research date:** 2026-11-06
**Valid until:** 2026-11-20 (the tree moves daily; `docs.rs`/`pre_push.rs` line numbers and the measured counts will shift; re-measure in Wave 0)
