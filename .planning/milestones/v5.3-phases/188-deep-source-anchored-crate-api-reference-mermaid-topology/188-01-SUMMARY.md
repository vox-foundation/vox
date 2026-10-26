---
phase: 188-deep-source-anchored-crate-api-reference-mermaid-topology
plan: 01
subsystem: docs-openwiki-api
tags: [openwiki, crate-api-sync, mermaid, reverse-dependents, source-anchors]
status: completed
---

# Phase 188 Summary: Deep Source-Anchored Crate API Reference & Mermaid Topology

## Requirements Completed (`OPENWIKI-API-01..04`)
- **`OPENWIKI-API-01`**: Upgraded crate API extraction (`crates/vox-doc-pipeline/src/pipeline/openwiki_symbols.rs` and `crates/vox-cli-ci/src/crate_api_docs.rs`) to walk `src/lib.rs` / `src/main.rs` and declared `pub mod` submodule files (`src/<mod>.rs`, `src/<mod>/mod.rs`).
- **`OPENWIKI-API-02`**: Emitted 1-based `#L<start>-L<end>` GitHub/repo relative source line links, signatures, and doc summaries in `### Deep Public Symbols (Source-Anchored)`.
- **`OPENWIKI-API-03`**: Computed workspace-wide `## Reverse Dependents (Used By)` via `compute_reverse_dependents` and cross-linked consumer crate API pages.
- **`OPENWIKI-API-04`**: Embedded `validate_and_repair_mermaid`-verified `flowchart LR` dependency diagrams across all 132 `docs/src/api/<crate>.generated.md` files.

## Verification
- `VOX_CRATE_API_SYNC_WRITE=1 cargo test -p vox-cli-ci --lib crate_api_docs` (`4 passed; 0 failed`).
- `cargo test -p vox-doc-pipeline --test openwiki_e2e_integration` (`phase_188_deep_crate_symbols_reverse_deps_and_mermaid ... ok`).
