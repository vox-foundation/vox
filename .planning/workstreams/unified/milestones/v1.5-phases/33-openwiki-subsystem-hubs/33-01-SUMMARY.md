---
phase: 33-openwiki-subsystem-hubs
plan: 01
subsystem: vox-doc-pipeline
tags: [openwiki, subsystem-hubs, mermaid, symbol-anchors]
provides:
  - OpenWiki subsystem layer hubs (`L0..L5`) from `docs/src/architecture/layers.toml` (`WIKI-01`, `WIKI-04`)
  - Deep crate symbol extraction (`pub struct`, `pub enum`, `pub trait`, `pub fn`, `pub type`) with `#Lstart-Lend` anchors and `Used By` reverse dependents (`WIKI-02`)
  - Deterministic Mermaid-lint-validated crate & subsystem dependency flowcharts (`WIKI-03`)
key-files:
  created:
    - crates/vox-doc-pipeline/src/pipeline/openwiki.rs
  modified:
    - crates/vox-doc-pipeline/src/pipeline/mod.rs
requirements-completed: [WIKI-01, WIKI-02, WIKI-03, WIKI-04]
completed: 2026-10-15
---

# Phase 33 Plan 01: OpenWiki Subsystem Hubs Summary

## Accomplishments
- Implemented `parse_layers_toml` and `SUBSYSTEM_LAYERS` (`L0`–`L5`) in [`crates/vox-doc-pipeline/src/pipeline/openwiki.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/pipeline/openwiki.rs) (`WIKI-01`, `WIKI-04`).
- Implemented `extract_deep_crate_symbols` and `compute_reverse_dependents` with 1-based `#L<start>-L<end>` GitHub/local line-range anchors and signatures (`WIKI-02`).
- Implemented `render_crate_dependency_mermaid` and `render_subsystem_layer_mermaid` validated through `validate_and_repair_mermaid` (`WIKI-03`).

## Verification
- `timeout 900s cargo --config 'build.rustc-wrapper=""' test -p vox-doc-pipeline --lib` — **84 passed, 0 failed**.
