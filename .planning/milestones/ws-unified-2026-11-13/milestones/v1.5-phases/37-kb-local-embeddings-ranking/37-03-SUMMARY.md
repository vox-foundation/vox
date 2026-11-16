# Summary 37-03: Hybrid RRF Fusion, Status Down-Weighting & Reranking

- **Status:** Complete (2026-10-15)
- **Requirements:** `EMB-04`, `EMB-06`, `RET-04`
- **Changes:**
  - Added `rank_multiplier_for_doc_frontmatter` (`RET-04`) and `search_hybrid_with_taxonomy` (`EMB-04`) in [`crates/vox-search/src/library/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-search/src/library/mod.rs), down-weighting `research`/`roadmap`/`legacy`/`deprecated` and superseded pages without excluding them and applying `TermDensityReranker` over the fused top-20.
  - Verified all 267 unit tests in `vox-search` pass (`0 failed`).
