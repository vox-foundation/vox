# Summary 37-02: Deterministic Contextual Chunk Prefix

- **Status:** Complete (2026-10-15)
- **Requirements:** `EMB-03`
- **Changes:**
  - Implemented `contextual_prefix(title, heading_path, summary)` and `format_chunk_for_embedding` in [`crates/vox-search/src/library/chunk.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-search/src/library/chunk.rs) so every chunk is embedded and lexically indexed with `title › heading_path — summary`.
