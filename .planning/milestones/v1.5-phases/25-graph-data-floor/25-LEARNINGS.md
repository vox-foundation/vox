---
phase: 25
phase_name: "graph-data-floor"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 9
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 25 Learnings: graph-data-floor

## Decisions

### Added #[serde(default = "default_edge_kind")] pub kind
Added `#[serde(default = "default_edge_kind")] pub kind: String` to `ExtractedEdge` in `crates/vox-graph-reader/src/ast.rs`, defaulting to `"calls"` for resolved call edges and `"mentions"` for `confidence == "dangling"` boundary edges in `crates/vox-graph-reader/src/rebuild_resolve.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 25-01-SUMMARY.md

---

### Bumped EXTRACTOR_VERSION from "4" to "5" and split TS/JS/Py tree-sitter ...
Bumped `EXTRACTOR_VERSION` from `"4"` to `"5"` and split TS/JS/Py tree-sitter extraction into `crates/vox-graph-reader/src/ast_ts.rs` (`extract_ts_ast`) so `ast.rs` remains well below the 500-line TOESTUB cap.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 25-01-SUMMARY.md

---

### Persisted "kind"
Persisted `"kind": e.kind` in `links_val` and `"extractor_version": EXTRACTOR_VERSION` in `.graphify_manifest.v1.json` in `crates/vox-graph-reader/src/rebuild.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 25-01-SUMMARY.md

---

### Created crates/vox-graph-reader/src/ast_vox.rs (extract_vox_ast_in_modul...
Created `crates/vox-graph-reader/src/ast_vox.rs` (`extract_vox_ast_in_module`) extracting `.vox` declarations (`component`, `table`, `query`, `mutation`, `server`, `tool`, `resource`, `form`, `index`, `fn`, `type`, `struct`, `enum`, `trait`, `actor`, `workflow`, `activity`, `state_machine`, `module`, `routes`) into container-qualified `ExtractedNode`s and calls/JSX compositions/imports/trait impls into typed `ExtractedEdge`s without introducing a dependency on `vox-compiler`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 25-02-SUMMARY.md

---

### Included "vox" in walk_source_files in crates/vox-graph-reader/src/rebui...
Included `"vox"` in `walk_source_files` in `crates/vox-graph-reader/src/rebuild.rs` and dispatched `.vox` paths in `extract_ast_in_module_with_wrappers`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 25-02-SUMMARY.md

---

### Created crates/vox-graph-reader/src/lod.rs implementing AtlasLevel (Crat...
Created `crates/vox-graph-reader/src/lod.rs` implementing `AtlasLevel` (`Crate`, `Module`, `File`, `Symbol`), `AtlasQuery`, `AtlasIdentity`, `AtlasSubgraph`, and `query_atlas_subgraph`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 25-03-SUMMARY.md

---

### Derived crate -> file -> symbol contains hierarchy at query time from id...
Derived `crate -> file -> symbol` `contains` hierarchy at query time from `id.split("::")` and `crate_model::build_crate_map`, enforcing node budget caps (`DEFAULT_ATLAS_LIMIT = 400`, `MAX_ATLAS_LIMIT = 1000`) and rejecting stale `extractor_version` caches.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 25-03-SUMMARY.md

---

### Added GraphifyAtlasParams and graphify_atlas in crates/vox-orchestrator-...
Added `GraphifyAtlasParams` and `graphify_atlas` in `crates/vox-orchestrator-mcp/src/graph_tools.rs` and wired `"vox_search_atlas"` in `crates/vox-orchestrator-mcp/src/dispatch.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 25-04-SUMMARY.md

---

### Added integration test suite crates/vox-graph-reader/tests/atlas_floor_t...
Added integration test suite `crates/vox-graph-reader/tests/atlas_floor_tests.rs` and verified the full `vox-graph-reader` test suite (`cargo --config 'build.rustc-wrapper=""' test -p vox-graph-reader` — all 26 test binaries passed with 0 failures).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 25-04-SUMMARY.md

## Lessons

## Patterns

## Surprises

