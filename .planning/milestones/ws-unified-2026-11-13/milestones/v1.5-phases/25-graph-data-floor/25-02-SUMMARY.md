---
phase: 25
plan: 2
subsystem: graph-data-floor
tags: [graph-reader, vox-ast, declarations]
completed: 2026-10-15
---

# Phase 25 Plan 02: First-Class `.vox` AST Declaration & Call Extractor (`ast_vox.rs`) Summary

## Accomplishments
- Created `crates/vox-graph-reader/src/ast_vox.rs` (`extract_vox_ast_in_module`) extracting `.vox` declarations (`component`, `table`, `query`, `mutation`, `server`, `tool`, `resource`, `form`, `index`, `fn`, `type`, `struct`, `enum`, `trait`, `actor`, `workflow`, `activity`, `state_machine`, `module`, `routes`) into container-qualified `ExtractedNode`s and calls/JSX compositions/imports/trait impls into typed `ExtractedEdge`s without introducing a dependency on `vox-compiler`.
- Included `"vox"` in `walk_source_files` in `crates/vox-graph-reader/src/rebuild.rs` and dispatched `.vox` paths in `extract_ast_in_module_with_wrappers`.
