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
  - "crates/vox-doc-pipeline/src/claims/llm_verifier.rs"
  - "crates/vox-doc-pipeline/src/claims/self_heal_bot.rs"
  - "crates/vox-cli-ci/src/crate_api_docs.rs"
---

# Self-Maintaining & Evidence-Cited Documentation Engine SSOT (2026)

<!-- kb:summary -->
Vox's Self-Maintaining Documentation Engine keeps architectural and crate API documentation synchronized with live Rust source code, contracts, and layer boundaries. It combines deterministic AST/source symbol extraction (`#Lstart-Lend` anchors, reverse-dependent `Used By` tables, and lint-verified VoxDiagram topology diagrams) with a zero-deficit freshness baseline ratchet, an advisory content-addressed LLM claim verifier (`[path:Lstart-Lend]` citation enforcement), and a guarded draft-PR bot (`<= 3` concurrent draft PRs, strict path scope, and auto-pause below 80% precision).
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

### 3. Zero-Deficit Freshness Baseline & Duplicate Consolidation

- `contracts/reports/docs-freshness/baseline.v1.json` is ratcheted to **0 grandfathered deficits** (`[]`).
- Machine-generated reference pages (`*.generated.md`) and historical changelogs (`/changelog.md`) are excluded from reader-facing drift deficits (`is_reader_facing_page` in `crates/vox-doc-pipeline/src/claims/gate.rs`), matching the freshness ledger contract.
- Duplicate cluster consolidation (`vox audit docs --consolidate-duplicates`) selects the canonical `.generated.md` or `-ssot` page and synthesizes Cloudflare Pages / Starlight `301` redirect rules in `docs-astro/public/_redirects`.

### 4. Evidence-Cited Advisory LLM Claim Verifier

`vox audit docs --verify-llm` and `vox audit docs --eval-golden-drift` (`crates/vox-doc-pipeline/src/claims/llm_verifier.rs`) enforce:

- **Citation Requirement**: Every `Verified` or `Drifted` verdict must carry a valid `[path:Lstart-Lend]` citation (`is_valid_line_range_citation` / `is_valid_line_range_citation_on_disk`). Any uncited verdict is automatically downgraded to `Uncertain`.
- **Content-Addressed Caching**: Verdicts are cached by `(claim_hash, symbol_span_hash)` in `contracts/reports/docs-freshness/llm-verify-cache.v1.json` with strict per-run token budgets (`DocVerifyBudget`).
- **Golden Drift Benchmark**: Evaluated against `contracts/reports/docs-freshness/golden-drift-eval-set.v1.json` (`>= 50` pairs, requiring `precision >= 80%` and `false_positive_rate <= 10%`).

### 5. Guarded Self-Healing Draft-PR Bot

`vox audit docs --self-heal-plan` and `vox run scripts/docs-fix-drafts.vox` (`crates/vox-doc-pipeline/src/claims/self_heal_bot.rs` and `.github/workflows/docs-self-heal.yml`) enforce:

- **Path Scope**: Only `docs/src/**` prose files may be patched; `*.generated.md`, `docs/src/archive/**`, `.rs`, `.vox`, `Cargo.toml`, and `contracts/**` are rejected (`validate_bot_target_path`).
- **Pre-PR Verification Gates**: Candidate patches must pass frontmatter checks, balanced code-fence checks, Mermaid validation, and line-range citation checks (`validate_bot_patch_scope_and_gates`).
- **Concurrency & Precision Auto-Pause**: Caps open bot branches at `MAX_CONCURRENT_BOT_DRAFT_PRS = 3`, updates existing subsystem branches in place, and auto-pauses if rolling acceptance precision in `contracts/reports/docs-freshness/bot-metrics.v1.json` drops below `80%`.
