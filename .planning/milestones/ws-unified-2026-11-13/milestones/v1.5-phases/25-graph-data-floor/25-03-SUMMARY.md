---
phase: 25
plan: 3
subsystem: graph-data-floor
tags: [graph-reader, lod, containment, subgraph]
completed: 2026-10-15
---

# Phase 25 Plan 03: Budgeted LOD Subgraph Engine & Derived Containment Hierarchy (`lod.rs`) Summary

## Accomplishments
- Created `crates/vox-graph-reader/src/lod.rs` implementing `AtlasLevel` (`Crate`, `Module`, `File`, `Symbol`), `AtlasQuery`, `AtlasIdentity`, `AtlasSubgraph`, and `query_atlas_subgraph`.
- Derived `crate -> file -> symbol` `contains` hierarchy at query time from `id.split("::")` and `crate_model::build_crate_map`, enforcing node budget caps (`DEFAULT_ATLAS_LIMIT = 400`, `MAX_ATLAS_LIMIT = 1000`) and rejecting stale `extractor_version` caches.
