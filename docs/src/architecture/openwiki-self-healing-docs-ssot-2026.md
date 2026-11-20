---
title: "Self-Maintaining & Evidence-Cited Documentation Engine SSOT (2026)"
description: "Canonical architecture and operational contract for Vox's self-maintaining subsystem hubs, source-anchored crate API generators, zero-deficit freshness gates, evidence-cited LLM claim verification, and guarded self-healing draft-PR bot."
category: "Architecture SSOTs"
status: "current"
topic: "doc-pipeline"
domain: "documentation"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
code_anchors:
  - "crates/vox-doc-pipeline/src/pipeline/openwiki.rs"
  - "crates/vox-doc-pipeline/src/pipeline/openwiki_symbols.rs"
  - "crates/vox-doc-pipeline/src/claims/resolver.rs"
  - "crates/vox-doc-pipeline/src/claims/gate.rs"
  - "crates/vox-doc-pipeline/src/claims/llm_verifier.rs"
  - "crates/vox-doc-pipeline/src/claims/self_heal_bot.rs"
  - "crates/vox-doc-pipeline/src/claims/golden_mine.rs"
  - "crates/vox-doc-pipeline/src/pipeline/openwiki_polyglot.rs"
  - "crates/vox-doc-pipeline/src/pipeline/openwiki_heal.rs"
  - "crates/vox-cli-research/src/audit_claim_judge.rs"
  - "crates/vox-cli-ci/src/crate_api_docs.rs"
---

# Self-Maintaining & Evidence-Cited Documentation Engine SSOT (2026)

<!-- kb:summary -->
Vox's Self-Maintaining Documentation Engine treats `docs/src/` (the Starlight site on `voxlang.org`) as the single canonical OpenWiki documentation database on Vox and resolves external repositories from `<canonical_docs_root>/` (`docs/src/` -> `docs/` -> root Markdown). It couples deterministic AST/source extraction (`#Lstart-Lend` anchors, reverse-dependent tables, and `vox-diagram` topologies) with an honest ground-truth claim resolver (`vox-graph-reader` symbol checks, `catalog.toml` bundles, `vox-cli` registry groups, real path existence), a zero-deficit reader-facing baseline plus full-corpus shrink-only ratchet, an evidence-cited LLM claim verifier, and a guarded self-healing pipeline.
<!-- /kb:summary -->

## Architecture Overview

```vox-diagram
graph {
    direction: down
    node layers: "docs/src/architecture/layers.toml + Cargo.toml"
    node wiki: "vox ci wiki-sync"
    node api_sync: "vox ci crate-api-sync"
    node hubs: "docs/src/architecture/subsystems/*.generated.md"
    node api_pages: "docs/src/api/*.generated.md"
    node lint: "vox ci ssot-drift & vox-doc-pipeline --lint-only"
    node verify: "vox audit docs --verify-llm & --eval-golden-drift"
    node drafts: "vox run scripts/docs-fix-drafts.vox (Draft PRs <= 3)"
    layers -> wiki
    layers -> api_sync
    wiki -> hubs
    api_sync -> api_pages
    hubs -> lint
    api_pages -> lint
    lint -> verify
    verify -> drafts
}
```

## Five Pillars of Self-Maintaining & Evidence-Cited Documentation

### 1. Deep Source-Anchored Crate API Reference

`vox ci crate-api-sync [--write]` (`crates/vox-cli-ci/src/crate_api_docs.rs` + `crates/vox-doc-pipeline/src/pipeline/openwiki_symbols.rs`) walks each workspace crate's `src/lib.rs` (or `src/main.rs`) and its declared `pub mod` submodule files (`src/<mod>.rs`, `src/<mod>/mod.rs`) to emit:

- **`## Dependency Topology`**: Validated `vox-diagram` graph (`direction: right`) showing direct workspace dependencies and reverse dependents.
- **`## Reverse Dependents (Used By)`**: Cross-linked table of consumer crates computed via `compute_reverse_dependents`.
- **`### Deep Public Symbols (Source-Anchored)`**: Signatures, doc summaries, and 1-based `#L<start>-L<end>` source links.

### 2. L0–L5 Subsystem Architecture Atlas

`vox ci wiki-sync [--write]` (`crates/vox-doc-pipeline/src/pipeline/openwiki.rs`) parses `docs/src/architecture/layers.toml` and synthesizes 7 deterministic pages under `docs/src/architecture/subsystems/`:

- `index.generated.md` — Top-level L0–L5 dependency hierarchy and domain subsystem authority map.
- `l0-pure-types.generated.md` through `l5-binaries.generated.md` — Per-layer VoxDiagram topology diagrams and crate budget tables (`Fan-In Cap`, `LoC Baseline`).

`wiki_sync` runs inside `vox ci ssot-drift` with `VOX_AUTO_HEAL=1` support.

The polyglot five-layer pages (`vox ci wiki-sync --polyglot`) are written to the repository's own docs root, chosen by `openwiki_pages_rel` (`crates/vox-doc-pipeline/src/pipeline/openwiki_polyglot.rs`): an existing `docs/wiki/` is kept for back-compat, otherwise `docs/src/openwiki/` when `docs/src/` exists, otherwise `docs/openwiki/`. Nothing creates a `docs/wiki/` on Vox. The scan has no file or entry caps; a layer table over 400 rows becomes a rollup index plus one detail page per module group, and source anchors are depth-aware. The MCP `wiki_*` actions call this same engine (`crates/vox-orchestrator-mcp/src/repo_catalog_tools.rs`).

### 3. Honest Ground-Truth Resolution, Freshness Baseline & Full-Corpus Ratchet

- **Canonical Corpus (`docs/src/` on `voxlang.org`)**: Vox hosts all OpenWiki features in-place inside `docs/src/` (never a separate `docs/wiki/` tree); external repositories discover `<canonical_docs_root>/` (`docs/src/`, else `docs/`, else root Markdown).
- **Honest Ground-Truth Registry (`crates/vox-doc-pipeline/src/claims/resolver.rs`)**: Zero hardcoded crate alias lists. Resolves crates solely from `crates/*/Cargo.toml`, distribution bundles from `crates/vox-plugin-catalog/catalog.toml`, multi-repo entries from `.vox/repositories.yaml`, deleted crates via `RETIRED_CRATE_REPLACEMENTS`, CLI commands strictly against active `vox-cli` parent/leaf paths in `contracts/cli/command-registry.yaml`, file paths by exact disk existence, and `RustSymbol` paths via `vox-graph-reader` AST extraction and module/re-export scanning.
- **Role-Aware Extraction (`crates/vox-doc-pipeline/src/claims/extractor.rs`)**: Historical, archive (`/archive/`), `legacy`, `deprecated`, `archived`, `research`, and `roadmap` documents downgrade extracted tokens to `MentionRole::Mentions`; `status: "current"` active docs remain `Prescribes`.
- **Zero-Deficit Baseline + Shrink-Only Corpus Ratchet**: `contracts/reports/docs-freshness/baseline.v1.json` remains ratcheted to **0 grandfathered reader-facing deficits** (`[]`), while non-reader-facing active `docs/src/` pages (`architecture/`, `adr/`, `contributors/`, `ci/`, `api/`) and root guides (`README.md`, `AGENTS.md`) are gated by `contracts/reports/docs-freshness/corpus-ratchet.v1.json` (`evaluate_corpus_ratchet`).
- **Freshness Reports & Consolidation**: `vox audit docs --write-findings` emits live UTC timestamps to `contracts/reports/docs-freshness/{findings,metrics}.v1.json`, `vox audit docs --update-ledger` updates `contracts/reports/docs-freshness/ledger.v1.json`, and `vox audit docs --consolidate-duplicates` emits `301` redirect rules in `docs-astro/public/_redirects`.

### 4. Evidence-Cited Advisory Claim Verifier

`vox audit docs --verify-llm` and `vox audit docs --eval-golden-drift` (`crates/vox-doc-pipeline/src/claims/llm_verifier.rs`, `crates/vox-doc-pipeline/src/claims/self_heal_bot.rs`, `crates/vox-doc-pipeline/src/claims/golden_mine.rs`) enforce:

- **Grounded, offline by default**: `verify_claim_grounded` resolves each claim's anchors against ground truth. A prescriptive mention of a retired symbol is `Drifted`, cited to the retired-symbols contract line. Anchors that resolve to real files or crates are only `Uncertain` (existence is proven, entailment is not), and missing or ambiguous anchors are `Uncertain` with no evidence. Offline, the verifier never claims `Verified`.
- **Optional entailment judge**: `ClaimEntailmentJudge` is a seam defined in `vox-doc-pipeline`; `crates/vox-cli-research/src/audit_claim_judge.rs` implements it over `vox_actor_runtime::llm::llm_chat`. The judge only runs when every anchor resolved and real evidence exists, and its verdict still passes the citation rules below. Any failure leaves the deterministic verdict in place.
- **Citation Requirement**: Every `Verified` or `Drifted` verdict must carry a valid `[path:Lstart-Lend]` citation (`is_valid_line_range_citation`). A citation that points at the document being verified is rejected (`is_doc_self_citation`), and any uncited or self-cited verdict is downgraded to `Uncertain`.
- **Content-Addressed Caching**: Verdicts are cached by `(claim_hash, symbol_span_hash)` in `contracts/reports/docs-freshness/llm-verify-cache.v1.json`, where the span hash comes from the real anchor, under per-run token budgets (`DocVerifyBudget`).
- **Golden Drift Benchmark (v2)**: `vox audit docs --mine-golden-drift <patch>` mines unlabeled before/after pairs from `git log -p` of real doc fixes; a reviewer sets `label` and `reviewed_by`; `evaluate_golden_set_v2` scores them with the real resolver against `contracts/reports/docs-freshness/golden-drift-eval-set.v2.json`. Only `human:`-reviewed cases count toward promotion (`MIN_HUMAN_REVIEWED_CASES = 50`, `precision >= 80%`, `false_positive_rate <= 10%`); agent-labelled cases are reported but never license the bot.

### 5. Guarded Self-Healing Draft-PR Bot

`vox audit docs --self-heal-plan` and `vox run scripts/docs-fix-drafts.vox` (`crates/vox-doc-pipeline/src/claims/self_heal_bot.rs` and `.github/workflows/docs-self-heal.yml` read-only `verify-claims-readonly` job; write-side draft PR staging is handled by `.github/workflows/docs-drift-refresh.yml` on `bot/docs-drift-refresh`) enforce:

- **Benchmark gate**: `run_self_heal_bot_check` refuses to run unless the v2 benchmark licenses the bot (see above).
- **Surgical Tier-2 fixes**: `vox audit docs --heal-retired` (dry run; `--apply` writes) plans one-token, one-line fixes (`synthesize_line_scoped_fix`) for prescriptive mentions of retired crates, only for exact entries of the crate rename table (`exact_retired_crate_replacement`). Each fix is gated by `verify_tier2_candidate_four_gates`. Decision records, history, the changelog and ISO-dated pages are never rewritten (`is_autofix_eligible_path`).
- **Path Scope**: Only `docs/src/**` prose files may be patched; `*.generated.md`, `docs/src/archive/**`, `.rs`, `.vox`, `Cargo.toml`, and `contracts/**` are rejected (`validate_bot_target_path`).
- **Pre-PR Verification Gates**: Candidate patches must pass frontmatter checks, balanced code-fence checks, Mermaid validation, and line-range citation checks (`validate_bot_patch_scope_and_gates`).
- **Concurrency & Precision Auto-Pause**: Caps open bot branches at `MAX_CONCURRENT_BOT_DRAFT_PRS = 3`, updates existing subsystem branches in place, and auto-pauses if rolling acceptance precision in `contracts/reports/docs-freshness/bot-metrics.v1.json` drops below `80%`.
