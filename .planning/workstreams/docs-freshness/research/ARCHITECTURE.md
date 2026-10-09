# Architecture: Self-Maintaining Docs (docs-freshness)

**Dimension:** How to extend in-repo machinery into a self-maintaining docs system
**Researched:** 2026-10-08 (codebase-grounded; every claim cites a path)
**Confidence:** HIGH for "what exists" (read directly), MEDIUM for proposed placement (layer/edge rules read, not dry-run)

---

## 1. What exists today (ground truth)

| Component | What it actually computes | Gap for freshness |
|---|---|---|
| `crates/vox-doc-pipeline` (L3, 1.9k LoC) | `pipeline/lint.rs`: frontmatter keys/categories/status, code fences, `{{#include}}` anchors, README↔`index.mdx` sync blocks, hand-authored `last_updated` ban. `pipeline/doctest.rs`: compiles ```` ```vox ```` fences via `vox-compiler`. `pipeline/anchors.rs`: ANCHOR-block extraction (25 lines). Errors are a closed `LintKind` enum (`pipeline/types.rs`). | Checks doc *form*, never doc *content vs code*. No notion of a mention, claim, or anchor to code. |
| `crates/vox-doc-inventory` (L2) | Despite the name, it inventories **Rust doc comments**, not Markdown: `hints.rs` extracts `///`/`//!` blocks, `relevance.rs` scores code files by doc-line density × hotspot tier → `docs/agents/doc-inventory.json`. | Wrong direction (code→comments). Useful only as a "which code is hot" signal for impact scoring. |
| Docs Reality Audit (`contracts/documentation/docs-reality-audit.program.v1.yaml`, `contracts/reports/docs-reality-audit/*`) | `crates/vox-cli-ci/src/docs_reality_audit.rs` (429 lines) only **validates**: JSON-schema checks, inventory glob paths resolve, findings reference real claim ids, metrics recomputed. Bundled into `vox ci ssot-drift` (`run_body_helpers/docs.rs:652`). | **Dormant because nothing produces findings.** 10 hand-seeded claims in `inventory.v1.json`, `findings: []`. Taxonomy + PriorityScore formula are good and reusable as-is. |
| `vox ci retired-symbol-check` (`crates/vox-cli-ci/src/retired_symbol_check.rs`, `contracts/documentation/retired-symbols.v1.yaml`) | Regex denylist over docs. | Only catches symbols someone remembered to list. Precedent for deterministic doc scanning. |
| `vox-code-audit::stdlib_parity` (`crates/vox-code-audit/src/stdlib_parity.rs`) wrapped by `vox audit stdlib-coverage` (`crates/vox-audit/src/subcommands/stdlib_coverage.rs`) | Three-way drift: `eval/builtins.rs` ↔ `reference/ref-builtins-stdlib.md` ↔ `scripts/`. | **The only existing doc-claim↔code checker.** It is the template for the deterministic layer, generalized beyond one page. |
| `vox graph` (`crates/vox-graph-reader` L1; registry `contracts/retrieval/vox-graph-corpora.v1.yaml`, loader `crates/vox-config/src/graphify.rs`) | `repo-code-graph` corpus: 47k nodes, ids like `crates/vox-actor-runtime/src/llm_result.rs::LlmResult::map`; `coverage.rs` already classifies `missing:true`/`dangling` edges as **DeadEnd**. | No docs corpus. Graph is a **Tier D local cache** (`.vox/cache/graphify/…`, 20 MB), not built in CI. |
| `vox audit` umbrella (`crates/vox-audit`, L5) | Registry of `Subcommand`s keyed by `CrlGate` (`lib.rs:259`); canonical `AuditReport` is *pass-rate shaped* (`report.rs:131`), not a findings list. `ToolingStdlibCoverage` = non-GA tooling gate. | Fits a `vox audit docs` *score*; findings JSONL must be a sidecar (as `vox audit effort` does: `vox-effort-audit/src/output/jsonl.rs`, `schema_version:"1.0"`). |
| LLM judge precedent: `vox audit effort` / `effort-route` (`crates/vox-effort-audit`, `crates/vox-effort-route`, both L3) | Calls only `vox_actor_runtime::llm::{infer_with_retry, llm_embed}` (`vox-effort-route/src/route/mod.rs:252`, `embed.rs:55`). Judge model: `--model` → `vox.toml [audit.route.judge].model_preference` → `vox_orchestrator::models::select` with `TaskCategory::CodeEffortJudge` (`crates/vox-cli/src/commands/audit_route.rs:121`). Drafted artifacts go to a **staging dir, never the build tree** (`vox-effort-route/src/emit/artifacts.rs:1`). Has `pricing.rs` budget caps. | **Exact pattern for claim verification + fix drafting.** `TaskCategory` is generated from `contracts/orchestration/model-routing.v1.yaml` (`vox-orchestrator-types/build.rs:152`), so a new judge category is a contract edit. |
| Bot-that-commits: `ssot-autoregen` (`.github/workflows/ci.yml:320`) | Same-repo PRs only; runs generators, commits to PR head with a PAT (`SSOT_AUTOREGEN_TOKEN`) so follow-up workflows trigger. | Deterministic regen only. LLM prose must instead go to a **separate draft PR** (brief: never auto-merge prose). |
| Site (`docs-astro/astro.config.mjs`) | Starlight 0.38, `routeMiddleware: ./src/routeData.ts` (injects `noindex` for archive), `lastUpdated: true`, `src/utils/git-dates.mjs` (last-commit per doc), `starlight-llms-txt@0.10.0` with `llmsFullTxt: true`. Frontmatter schema in `src/content.config.ts`. | `lastUpdated` = "prose last touched", not "verified against code". |
| llms.txt | Three surfaces: plugin-generated `/llms.txt`, `/llms-full.txt`, `/llms-small.txt`; plus **hand-curated** `docs/src/.well-known/{llms.txt,llms-full.txt,vox-docs.json}` (the committed `llms-full.txt` is 2.4 KB — clearly not "full"). | Split-brain. Plugin `exclude` applies to **llms-small only** (`node_modules/starlight-llms-txt/types.ts:189`); llms-full filters only `draft` (`generator.ts:30`). |

**Constraints discovered:** `vox-cli-ci` is already 28,960 LoC vs `max_loc = 15_000` (`layers.toml:273`) — do not grow it. `contracts/ci/crate-edges.allow.v1.json` is an exact edge ratchet; `vox-doc-pipeline` today depends only on `vox-bounded-fs`, `vox-compiler`. New workspace edges need user authorization (AGENTS.md §Dependency Discipline).

---

## 2. Core model: Mention → Anchor → Claim

Everything hangs on one data model, owned by **`vox-doc-pipeline`** (new module `src/claims/`):

- **Mention** (deterministic, extracted from Markdown with `pulldown-cmark`, already a workspace dep `Cargo.toml:409`): an inline-code span or fence line classified as one of `path` (`crates/x/src/y.rs`), `crate` (`vox-foo`), `symbol` (`Foo::bar`, `fn baz`), `cli` (`vox ci ssot-drift --write`), `env` (`VOX_DB_URL`), `config_key` (`[audit.route.judge]`), `contract` (`contracts/…yaml`). Each carries `doc_path`, line, byte span.
- **Anchor**: a resolved mention → concrete repo location (file path + optional symbol id). Optional explicit anchors via new frontmatter key `code_anchors: [glob…]` for curated pages (extend `lint.rs` allowed keys + `docs-astro/src/content.config.ts`).
- **Claim**: a prose sentence/paragraph plus its anchors. Only the LLM layer materializes claims; the deterministic layer works on mentions/anchors.

Resolvers (all deterministic, no network):

| Mention kind | Resolves against | SSOT |
|---|---|---|
| path / contract | filesystem (+ `git ls-files`) | repo |
| crate | workspace members | root `Cargo.toml`, `layers.toml [crates]` |
| cli | command tree | `contracts/cli/command-registry.yaml`, `contracts/operations/catalog.v1.yaml` |
| env | env registry | `contracts/config/env-vars.v1.yaml` |
| retired | denylist (hit = finding) | `contracts/documentation/retired-symbols.v1.yaml` |
| symbol | `repo-code-graph` graph.json suffix-match **if present**, else `git grep -w` fallback | `vox graph` cache |

Symbol resolution must not *require* the graph in PR CI (Tier D cache, not built there); graph is an accuracy upgrade for local/nightly runs.

---

## 3. Component architecture

```
             docs/src/**/*.md                    code + contracts
                    │                                   │
   (a) DRIFT  ┌─────▼──────────────────────────────────▼──────┐
   every PR   │ vox-doc-pipeline::claims  extract → resolve    │
              │  → anchors ledger + broken-mention findings    │
              └─────┬───────────────────────────┬─────────────┘
                    │ anchor index (path→docs)  │ findings JSONL
   (b) VERIFY ┌─────▼──────────────┐            │
   PR advisory│ vox-doc-verify (L3)│  LLM via vox_actor_runtime::llm
   + nightly  │ changed files →    │  model via models::select(DocClaimJudge)
              │ affected docs →    │────────────┤ verdicts JSONL
              │ claim verdicts     │            │
              └─────┬──────────────┘            │
   (c) DRAFT        │ staged patches            │
   nightly    ┌─────▼──────────────┐            │
              │ .vox script: branch│→ gh pr create --draft
              │ + apply + PR       │            │
              └────────────────────┘            │
   (d) PUBLISH ┌──────────────────────────────── ▼───────────┐
   on build    │ contracts/reports/docs-freshness/ledger.v1.json │→ docs-astro badges,
               └──────────────────────────────────────────────┘   sidebar, llms sets
   (e) AUDIT   → contracts/reports/docs-reality-audit/auto-findings.v1.json
                 + vox audit docs (AuditReport score) → case-by-case prune
```

### (a) Deterministic drift layer — cheap, every PR, blocking for broken mentions
- **Owner:** `vox-doc-pipeline::claims` (L3). Exposed as `vox-doc-pipeline --drift [--paths …] [--json]` (its own bin, `src/main.rs`) and wrapped as `vox ci docs-drift` through the existing `vox-cli-ci → vox-doc-pipeline` dependency (`crates/vox-cli-ci/Cargo.toml:21`) — a thin wrapper only, given vox-cli-ci's LoC overrun.
- **Outputs:** (1) `LintKind::BrokenMention { kind, text }` folded into normal lint so it shows in `docs-quality.yml`; (2) **anchor index** `path → [doc_path]` (inverted ledger) used by (b) and by a PR comment: *"This PR changes `crates/x/src/y.rs`; these docs reference it: …"* — the minimum viable "drift warning surfaced to LLMs/humans."
- **Generated-section staleness:** reuse existing generators (`command-sync`, plugin catalog, etc.) — already covered by `ssot-drift`/`ssot-autoregen`. Add only: every `*.generated.md` must have a registered generator (detects orphaned generated pages).
- **Gating policy:** blocking only on `status: current` pages under `reference/`, `tutorials/`, `how-to/` (reader-facing); advisory elsewhere. `research`/`roadmap` pages are exempt (they are `IntentionalHistorical` by definition). Ratchet with a baseline file like other repo gates, so day-one noise doesn't block.

### (b) LLM claim-verification layer — advisory on PR, full sweep nightly
- **Owner:** new L3 library **`vox-doc-verify`**, cloned in shape from `vox-effort-route` (prompt module, `infer_with_retry`, pricing/budget caps, JSONL emitter). CLI entry in `vox-cli` as `vox audit docs verify` beside `audit_effort.rs`/`audit_route.rs`; model resolution identical to `audit_route.rs:121` with a new `TaskCategory::DocClaimJudge` added to `contracts/orchestration/model-routing.v1.yaml`.
- **Input:** `git diff --name-only base...head` → anchor index → affected docs → for each, extract claims (paragraphs containing anchored mentions) + the anchored code slices (bounded bytes) → judge prompt with JSON `response_format`.
- **Verdict schema** (maps 1:1 onto the dormant taxonomy): `supported | contradicted | unverifiable`, `classification ∈ {DocDeficit, CodeDeficit, AmbiguousNeedsDecision}`, `evidence: [{path, line}]`, `suggested_fix` (prose). Require evidence lines that exist; drop verdicts citing nonexistent lines (hallucination guard).
- **Budget:** PR job caps claims (e.g. ≤40) and tokens via `pricing.rs`-style ceilings; skips silently (exit 0, `incomplete:true`) when no model/secret — fork PRs and offline runs stay green. Nightly runs full corpus under the 180-min cap, sharded by section.
- Secrets via `vox_secrets::resolve_secret`; no vendor hostnames (enforced by `llm_provider_call` detector).

### (c) Fix-drafting bot — draft PRs only
- `vox-doc-verify` emits **patch files to a staging dir** (copy `effort-route/emit/artifacts.rs` discipline: never write the tree).
- Orchestration in a committed **`.vox` script** (`scripts/docs-fix-drafts.vox`, per VoxScript-first policy): group patches by doc section, create `docs-bot/<date>-<section>` branch, apply, `vox ci doc lint` + `doctest-md` on the result (drop patches that fail), `gh pr create --draft` with the verdict evidence table in the body.
- New scheduled workflow `docs-freshness.yml` (`concurrency:` block, ≤180 min, `ubuntu-latest`). Needs a PAT like `SSOT_AUTOREGEN_TOKEN` so the draft PR triggers `docs-quality.yml` (GITHUB_TOKEN-opened PRs don't trigger workflows). Max N open bot PRs; skip docs that already have an open bot PR (dedupe by finding fingerprint).
- Deterministic fixes (e.g. renamed path where `git log --follow` gives a unique new name) can ride the existing `ssot-autoregen` path instead — they're not prose.

### (d) Freshness metadata, badges, llms.txt
- **SSOT:** `contracts/reports/docs-freshness/ledger.v1.json` (+ schema), generated, `linguist-generated` in `.gitattributes`. Per doc: `anchors[]` with `blob_sha` at last verification, `last_verified_commit`, `verdict_summary`, derived `state ∈ {verified, drifting, broken, unanchored, historical}`. Don't put this in frontmatter — the repo already bans hand-maintained dates (`HandAuthoredLastUpdated`), and bot churn in 668 files would swamp diffs.
- **State is computable without an LLM:** `drifting` = any anchor's current blob sha ≠ ledger sha (Swimm-style). This makes badges cheap and honest even when (b) hasn't run.
- **Site:** `docs-astro/src/utils/freshness.mjs` reads the ledger (same pattern as `git-dates.mjs`). `routeData.ts` middleware sets `starlightRoute.entry.data.banner` for `drifting`/`broken` pages (no component override needed) and "Verified against `<sha>` on `<date>`" text; `sidebar.mjs` adds Starlight `badge` to drifting items. `status: research|roadmap` pages get a "Historical/Research" banner from frontmatter alone.
- **llms:** retire the hand-curated `docs/src/.well-known/llms-full.txt` (stale split-brain); keep `.well-known/llms.txt` as curated index but have `vox ci` verify every URL in it maps to a `verified` page. In `astro.config.mjs`: `exclude` (llms-small) + `demote` from the ledger's non-verified slugs, and a `customSets` entry "current" containing only verified pages. llms-full has no exclude hook in 0.10.0 — either upstream an `exclude`-for-full option (plugin is small, MIT) or point agents at the "current" set. Flag as open item.

### (e) Audit report for case-by-case pruning
- Reactivate Docs Reality Audit by **generating** findings: drift layer + verify layer write `contracts/reports/docs-reality-audit/auto-findings.v1.json` (new file; keeps `findings.v1.json` for human triage overrides keyed by finding `fingerprint`). Extend findings schema (bump `x-vox-version`): `source: auto|manual`, `fingerprint`, `detector`. Auto-generate matching claim ids (`claim.auto.<doc>.<n>`) or relax `verify_findings_consistency` (`docs_reality_audit.rs:178`) for `source:auto`.
- **Scores auto-filled** per the existing formula (`program.v1.yaml` §priority_scoring): Impact from section (reference/tutorial high), BlastRadius from inbound links + sidebar/llms.txt inclusion, Staleness from days since anchor change, EnforcementGap = 3 if no anchors, Tractability from verdict confidence.
- **Pruning candidates** (report only, never auto-archive): `historical` pages with zero inbound links, unanchored `current` pages, near-duplicate pages (embed via `llm_embed`, cluster like `effort-route/cluster.rs`).
- `vox audit docs` = new `CrlGate::ToolingDocsFreshness` in `crates/vox-audit` (non-GA, like `ToolingStdlibCoverage`): `overall_pass_rate` = verified ÷ anchored reader-facing pages; findings JSONL sidecar with `schema_version`. If the `vox-audit → vox-doc-pipeline` edge is refused, register it in `vox-cli` like `audit effort` instead.
- Optional later: a `docs-mentions` corpus in `vox-graph-corpora.v1.yaml` emitting `doc:` nodes with edges to code nodes, so `vox graph coverage` reports vanished references as **DeadEnd** for free (`coverage.rs`).

---

## 4. Ownership and layer compliance

| Piece | Crate (layer) | New workspace edges |
|---|---|---|
| Mention extraction, resolvers, anchor ledger, freshness ledger writer | `vox-doc-pipeline` (L3) | none (external deps only: `pulldown-cmark`, `serde_yaml`/`regex`); graph.json read via `serde_json`, avoiding a `vox-graph-reader` edge |
| `vox ci docs-drift` wrapper | `vox-cli-ci` (L5) | none (dep exists) |
| LLM verify + patch drafting | **new** `vox-doc-verify` (L3, `max_loc ≈ 4_000`) | `vox-doc-verify → vox-actor-runtime, vox-doc-pipeline, vox-config`; `vox-cli → vox-doc-verify` — **propose to user** (mirrors existing effort-route edges) |
| Judge category | `contracts/orchestration/model-routing.v1.yaml` | contract edit |
| `vox audit docs` score | `vox-audit` (L5) | `vox-audit → vox-doc-pipeline` — **propose** (or host in vox-cli) |
| PR drafting, branch/PR ops | `scripts/docs-fix-drafts.vox` | n/a |
| Badges / banners / llms sets | `docs-astro/src/{routeData.ts,utils/*.mjs}`, `astro.config.mjs` | n/a |
| Workflows | `docs-quality.yml` (+drift step), new `docs-freshness.yml` (nightly verify+draft) | needs PAT secret (user-provisioned) |

Add rows to `docs/src/architecture/where-things-live.md` and `layers.toml [crates]` in the same PR as the new crate.

## 5. New contracts

1. `contracts/documentation/doc-mentions.v1.yaml` — mention kinds, extraction regexes, which sections/statuses are gated, ignore rules (illustrative fences, `// vox:skip`).
2. `contracts/reports/docs-freshness/ledger.v1.{json,schema.json}` — generated freshness ledger.
3. `contracts/reports/docs-reality-audit/auto-findings.v1.json` + findings schema bump (`source`, `fingerprint`, `detector`).
4. `contracts/documentation/doc-verify-verdict.v1.schema.json` — LLM verdict JSONL line.
5. `model-routing.v1.yaml`: `DocClaimJudge` task category. `vox.toml [audit.docs]` budget/judge config.
6. Baseline: `contracts/documentation/docs-drift-baseline.v1.json` (ratchet; tighten-only).

## 6. Build order (with dependencies)

1. **Mention extractor + resolvers (deterministic, report-only)** in `vox-doc-pipeline::claims`, tests first. No deps. Produces the first real drift numbers (measure before designing gates).
2. **Anchor index + PR comment** ("docs referencing changed files"). Depends on 1. Delivers outcome (1) immediately, zero LLM cost.
3. **Freshness ledger (blob-sha based) + baseline ratchet + blocking gate for reader-facing `current` pages.** Depends on 1–2. Wire into `docs-quality.yml`.
4. **Site surfacing**: `freshness.mjs`, `routeData.ts` banners, sidebar badges, llms `demote`/`exclude`/`customSets`; retire hand `llms-full.txt`. Depends on 3 only — can run parallel with 5.
5. **Auto-findings into Docs Reality Audit** + schema bump + `vox audit docs` score. Depends on 1, 3. Produces the pruning report (outcome 4) — run the human prune pass here, *before* LLM work, to shrink what (6) must verify.
6. **`vox-doc-verify` LLM layer** (needs user-approved crate edges + `DocClaimJudge`). Depends on 2 (affected-docs), 5 (finding sink). Advisory on PR, nightly sweep.
7. **Draft-PR bot** (`.vox` script + `docs-freshness.yml` + PAT). Depends on 6. Start with ≤3 open PRs/night.
8. **Generate more reference from contracts** (env-vars, config keys, error codes) — parallelizable after 1; each converts a hand page into a `*.generated.md` and removes it from (6)'s scope.
9. **Optional:** `docs-mentions` graph corpus for DeadEnd reporting.

## 7. Risks specific to this architecture

- **False positives from illustrative code** (example paths, hypothetical commands) will kill trust in the gate. Mitigate: only gate reader-facing `current` pages, honor `// vox:skip`/`text` fences, baseline ratchet, measure FP rate in step 1 before turning on blocking.
- **Anchor sha churn**: rustfmt-only or unrelated edits to a big file flip pages to `drifting`. Mitigate: anchor to symbols where possible (graph id → item span hash) rather than whole-file blob sha; nightly verify re-stamps unchanged-meaning pages.
- **LLM rubber-stamping**: require cited evidence lines that exist; sample-audit verdicts; `supported` verdicts on unchanged anchors should not re-cost tokens (cache by claim hash + anchor shas).
- **Edge approvals block step 6** — sequence so steps 1–5 deliver value with zero new workspace edges.
