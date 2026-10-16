---
phase: 25
plan: 1
subsystem: graph-data-floor
tags: [graph-reader, typed-edges, extractor-version]
completed: 2026-10-15
---

# Phase 25 Plan 01: Typed Edges & Extractor Version Bump (`EXTRACTOR_VERSION = "5"`) Summary

## Accomplishments
- Added `#[serde(default = "default_edge_kind")] pub kind: String` to `ExtractedEdge` in `crates/vox-graph-reader/src/ast.rs`, defaulting to `"calls"` for resolved call edges and `"mentions"` for `confidence == "dangling"` boundary edges in `crates/vox-graph-reader/src/rebuild_resolve.rs`.
- Bumped `EXTRACTOR_VERSION` from `"4"` to `"5"` and split TS/JS/Py tree-sitter extraction into `crates/vox-graph-reader/src/ast_ts.rs` (`extract_ts_ast`) so `ast.rs` remains well below the 500-line TOESTUB cap.
- Persisted `"kind": e.kind` in `links_val` and `"extractor_version": EXTRACTOR_VERSION` in `.graphify_manifest.v1.json` in `crates/vox-graph-reader/src/rebuild.rs`.
