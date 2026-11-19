---
phase: 37
phase_name: "kb-local-embeddings-ranking"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 37 Learnings: kb-local-embeddings-ranking

## Decisions

### Support provider
Support `provider: "local"` in `vox_actor_runtime::llm::llm_embed` (computing deterministic L2-normalized local feature-hashed embeddings with n-gram + token projections when offline/weights absent, or calling local endpoint when `base_url` is set), expose `default_local_embedding_config()` (`Alibaba-NLP/gte-modernbert-base@main`, dim 256) in `vox-search::embedding_env`, and enforce `VectorSpaceKey` (`model_id`, `revision`, `dim`) compatibility in `vox-search::embeddings` so vector spaces from different models or dimensions are never mixed.

**Rationale:** Primary phase objective established in plan.
**Source:** 37-01-PLAN.md

---

### Add contextual_prefix(title, heading_path, summary) in crates/vox-search...
Add `contextual_prefix(title, heading_path, summary)` in `crates/vox-search/src/library/chunk.rs` and hybrid RRF fusion (`search_hybrid_with_taxonomy`) in `crates/vox-search/src/library/mod.rs` combining Tantivy lexical BM25 with local dense cosine similarity + term-density cross-reranking when the query + chunk vectors share the same `VectorSpaceKey`.

**Rationale:** Primary phase objective established in plan.
**Source:** 37-02-PLAN.md

## Lessons

### Verified in [crates/vox-actor-runtime/src/llm/embed.rs](file:///Users/br...
Verified in [`crates/vox-actor-runtime/src/llm/embed.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-actor-runtime/src/llm/embed.rs), [`crates/vox-search/src/embedding_env.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-search/src/embedding_env.rs), and [`crates/vox-search/src/embeddings.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-search/src/embeddings.rs).

**Context:** Verified in phase acceptance checks.
**Source:** 37-VERIFICATION.md

---

### Verified in [crates/vox-search/src/library/chunk.rs](file:///Users/brbra...
Verified in [`crates/vox-search/src/library/chunk.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-search/src/library/chunk.rs) (`contextual_prefix`, `format_chunk_for_embedding`) and [`crates/vox-search/src/library/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-search/src/library/mod.rs) (`search_hybrid_with_taxonomy`).

**Context:** Verified in phase acceptance checks.
**Source:** 37-VERIFICATION.md

## Patterns

## Surprises

