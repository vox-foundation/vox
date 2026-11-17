---
phase: 453-deep-research-reasoning-budget-separation-citation-scoped-au
plan: 01
subsystem: research-pipeline
tags: [llm-egress, reasoning-budget, claim-extractor, citation-audit, verification, cache]
requires:
  - phase: 452-after-audit-closure-inline-diff-and-next-lessons
    provides: Baseline deep-research pipeline, claim extractor, and citation audit harness
provides:
  - Separate `reasoning_max_tokens` (`4_000`) from synthesis `max_tokens` (`12_000`) and OpenRouter `reasoning` wire config
  - Quote-backed & `[N]`-scoped `audit_citations`, `evidence_context_for_claim` index preservation, and contrastive `not only` negation guard
  - Clean Markdown frontmatter/comment/code/table-row stripping in `vox-scientia` `claim_extractor`
  - Mode/wave/site-scope-aware `research_cache_key` excluding empty or `[INSUFFICIENT_EVIDENCE]` abstentions
affects: [vox-llm-egress, vox-scientia, vox-research-shim, vox-research-events]
tech-stack:
  added: []
  patterns:
    - "`OpenRouter` reasoning budget separation via `WireReasoningConfig` (`reasoning: { max_tokens: 4000 }`) alongside `max_tokens: 12000`"
    - "`extract_referenced_citation_indices` scoping `audit_citations` to actually cited `[N]` / `[lo-hi]` sources with `quote_overlaps` & lexical containment checks"
key-files:
  created: []
  modified:
    - crates/vox-llm-egress/src/lib.rs
    - crates/vox-llm-egress/src/wire.rs
    - crates/vox-actor-runtime/src/llm/types.rs
    - crates/vox-actor-runtime/src/llm/chat.rs
    - crates/vox-actor-runtime/src/llm/stream.rs
    - crates/vox-actor-runtime/src/llm/cascade.rs
    - crates/vox-research-shim/src/research/orchestrator/config.rs
    - crates/vox-research-shim/src/research/orchestrator/stages.rs
    - crates/vox-research-shim/src/research/orchestrator/pipeline.rs
    - crates/vox-research-shim/src/research/orchestrator/pipeline_cache.rs
    - crates/vox-research-shim/src/research/verifier.rs
    - crates/vox-scientia/src/claim_extractor/pipeline.rs
key-decisions:
  - "Cap OpenRouter synthesis reasoning tokens at `4_000` (`candidate.reasoning_max_tokens = Some(4_000)`) while allocating `synthesis_max_tokens >= 12_000` so reasoning tokens never starve visible synthesis output."
  - "Preserve full Markdown table rows as prose-safe masked spans and respect sentence boundaries before Markdown list/heading markers in `claim_extractor/pipeline.rs`."
  - "Key `research_cache_key` on normalized query, `scope`, `max_sources`, `verify_claims`, `lane`, `domain_mode`, `waves`, and `site_scope`, while refusing to store or serve empty or `[INSUFFICIENT_EVIDENCE]` responses."
patterns-established:
  - "Quote-backed citation audit checks referenced `[N]` indices first and falls back to full source range only when no inline citations exist."
requirements-completed: [LIVE-09, LIVE-10, LIVE-11]
duration: 45min
completed: 2026-11-16
---

# Phase 453 Plan 01: Reasoning Budget Separation, Citation-Scoped Audit & Cache Hardening Summary

**Phase 453 Plan 01 separated OpenRouter synthesis reasoning budgets from visible output budgets (`LIVE-09`), hardened `extract_claims` Markdown stripping plus contrastive/index-preserving claim verification (`LIVE-10`), and scoped `audit_citations` and `research_cache` against ungrounded citations and stale abstentions (`LIVE-11`).**

## Accomplishments

### `LIVE-09`: Reasoning Budget Separation
- In [`crates/vox-llm-egress/src/lib.rs`](crates/vox-llm-egress/src/lib.rs), [`crates/vox-llm-egress/src/wire.rs`](crates/vox-llm-egress/src/wire.rs), and [`crates/vox-actor-runtime/src/llm/cascade.rs`](crates/vox-actor-runtime/src/llm/cascade.rs), added `reasoning_max_tokens: Option<u64>`, `WireReasoningConfig` serialization (`reasoning: { max_tokens }`) restricted to OpenRouter requests, `ReasoningEffortWire` / `T3WireOptions` (`normalize_egress_request_for_t3_options`, `build_request_with_t3_options`), and `is_empty_reasoning_exhausted_response` candidate fallback in `chat_with_cascade`.
- In [`crates/vox-research-shim/src/research/orchestrator/config.rs`](crates/vox-research-shim/src/research/orchestrator/config.rs) and [`crates/vox-research-shim/src/research/orchestrator/stages.rs`](crates/vox-research-shim/src/research/orchestrator/stages.rs), raised `synthesis_max_tokens` to `12_000` (`judge_max_tokens` to `6_000`) and capped `candidate.reasoning_max_tokens = Some(4_000)` exclusively for `ResearchStage::Synthesis`.

### `LIVE-10`: Markdown-Safe Claim Extraction & Contrastive Evidence Selection
- Fixed borrow-checker `E0502`, sentence boundaries before `-`/`*`/`#`, and Markdown table-row masking in [`crates/vox-scientia/src/claim_extractor/pipeline.rs`](crates/vox-scientia/src/claim_extractor/pipeline.rs) so frontmatter, comments, fenced code blocks, and tables produce clean atomic claims.
- In [`crates/vox-research-shim/src/research/verifier.rs`](crates/vox-research-shim/src/research/verifier.rs), guarded contrastive `"not only"` / `"not just"` / `"not merely"` / `"not solely"` idioms against false negation contradictions and updated `evidence_context_for_claim` to select highest-scoring hits across top `16` sources while preserving original `[idx]` numbers.

### `LIVE-11`: Citation-Scoped Audit & Cache Key/Abstention Hardening
- In [`crates/vox-research-shim/src/research/orchestrator/pipeline.rs`](crates/vox-research-shim/src/research/orchestrator/pipeline.rs), `extract_referenced_citation_indices` and `audit_citations` scope auditing to cited `[N]`/`[lo-hi]` references with `quote_overlaps` and lexical containment (`>= 0.55`), and `compute_corroboration_counts` excludes `untrusted_synthesis` entries.
- In [`crates/vox-research-shim/src/research/orchestrator/pipeline_cache.rs`](crates/vox-research-shim/src/research/orchestrator/pipeline_cache.rs), updated `research_cache_key` to include `domain_mode`, `waves`, and `site_scope` and added `is_cacheable_research_result` so empty answers or answers beginning with `ABSTENTION_MARKER` (`[INSUFFICIENT_EVIDENCE]`) are neither stored nor served.

## Verification
- `cargo test -p vox-llm-egress -p vox-research-events`: `51` passed (`34` unit + `17` `wire_mock`) in `vox-llm-egress` and `37` passed in `vox-research-events` (`88 passed; 0 failed`).
- `cargo test -p vox-scientia --lib`: `380 passed; 0 failed; 1 ignored` (including `markdown_frontmatter_comments_code_and_tables_produce_clean_claims`).
- `cargo test -p vox-research-shim --lib`: `179 passed; 0 failed` (including `contrastive_not_only_idioms_do_not_trigger_negation_or_false_contradiction`, `evidence_context_for_claim_selects_relevant_hits_beyond_index_eight_preserving_indices`, and `cache_store_and_short_circuit_skip_empty_and_abstention_results`).

---
*Phase: 453-deep-research-reasoning-budget-separation-citation-scoped-au*
*Completed: 2026-11-16*
