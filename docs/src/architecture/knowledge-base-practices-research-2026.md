---
title: "Knowledge Base Practices for Agent Context (2026)"
description: "Evidence-graded practices, technique verdicts, pitfall-to-test matrix and locked decisions for the Vox curated knowledge base that feeds model context."
category: "Architecture SSOTs"
status: "research"
supersedes: kb-systems-sota-research-2026.md
training_eligible: false
training_rationale: "LLM-synthesized research. Under this page's own rule, it is ineligible for MENS until a human attests it."
---

# Knowledge Base Practices for Agent Context (2026)

Vox's curated knowledge base is plain git markdown. That covers `docs/src/**` (excluding `archive/`) and `docs/src/library/` for general knowledge. It is the only curated source of truth. The database is a derived, content-hash-keyed index plus an inbox (`kb_findings`) for unreviewed captures. A turn gets context in two tiers. First, a small preamble that is relevance-gated and spotlighted, using a per-model budget as a ceiling. Second, `vox_kb_*` tools the model can call for more depth. Nothing reaches the curated tier unless an agent drafts a PR and a human merges it. Every claim on this page has a confidence grade and a named gate. Retrieval quality is measured on a ratcheted, ID-based eval before anything is tuned.

This page supersedes [kb-systems-sota-research-2026.md](kb-systems-sota-research-2026.md) (2026-06-17). That page is still right about the RAG-vs-memory split, hybrid BM25+dense+RRF, reranking, provenance, staleness, and the context-poisoning risk. It is wrong or stale on ColBERT as a "third pillar", on conversational-memory benchmarks (LoCoMo/LongMemEval) for doc retrieval, and on AMP verbs. It also says nothing about curated-doc practice: taxonomy, page-one summaries, Diátaxis.

Research date: 2026-10-03. Confidence grading: **HIGH** means verified against a primary source or read from this repository's code. **MEDIUM** means peer-reviewed or corroborated by at least two sources. **LOW** means a single secondary source.

## 1. Principles of KB management for agent context

1. **Git is the curated SSOT, and the index is derived.** Review, provenance and rollback come free with git. A committed catalog or index file is forbidden because it drifts. Frontmatter is the metadata SSOT. (HIGH, repo policy)
2. **Measure before changing.** Retrieval metrics are computed on document and chunk **IDs**, run deterministically in PR CI, and are compared to a baseline file that may only tighten. Generation metrics run nightly. A technique without an eval delta on *this* corpus isn't adopted. (MEDIUM: practitioner reports show "best practices" losing to naive baselines on real corpora.)
3. **Provenance before generation.** Record authorship and trust tier before any LLM-written text lands. Done in the other order, the result can't be recovered: human and model text become inseparable for training and bias analysis. (MEDIUM: model-collapse literature; HIGH: repo state.)
4. **Accumulate, don't replace.** LLM rewrites are limited to a delimited summary block. The human body stays verbatim and is the training-eligible part. Synthetic text that *replaces* real text is the regime in which model collapse occurs. (MEDIUM, Gerstgrasser et al.; Shumailov et al.)
5. **The budget is a ceiling, not a target.** Inject only what clears a relevance floor. Inject nothing when nothing clears it. Long or distractor-laden context lowers accuracy in every model tested. (MEDIUM, Lost in the Middle; Context Rot; Power of Noise)
6. **One write path into the curated tier.** Web captures, agent findings, "save answer", flags and re-verification drafts all go to the inbox and then to a PR. A human merges it. Inbox content is never auto-injected. (MEDIUM, product convergence: Cursor removed auto-Memories, and NotebookLM only suggests sources.)
7. **Verification is a badge, an owner and an interval, not a gate.** Stale and unverified pages stay visible and searchable. They are labelled and down-ranked, never deleted. Logical archive is `status: legacy`. (MEDIUM, Guru, Notion, KCS v6)
8. **Freshness means verified-true time, not edit time.** Use reviewed `verified_at`, never git mtime. Reformat commits, the SSOT autoregen bot and bulk rewrites all reset mtime. (HIGH, repo behavior; MEDIUM, Dated Data)
9. **Similarity is not truth, and retrieved text is data, not instructions.** Every row carries a trust tier. Non-curated text is spotlighted, and exfiltration-capable tools are gated when it is in context. (MEDIUM, Greshake et al.; Spotlighting; PoisonedRAG)
10. **Summary-first pages.** Each page opens with a self-contained 40–120-word "page one" block that answers "what is this and when do I need it", with no forward references. That block is both the per-doc summary card and the contextual prefix for every chunk. (MEDIUM, AWS RAG writing guide; kapa.ai; Anthropic contextual retrieval)
11. **One topic answers one question.** Sections should be self-contained (the DITA discipline, without DITA XML). `doc_type` comes from the Diátaxis directory, not from a field authors must keep in sync. (HIGH, diataxis.fr; DITA 2.0)
12. **Controlled vocabulary.** Facet values come from one SKOS-shaped contract with `altLabel`s, which double as BM25 synonyms. Classifiers may only pick from it. (HIGH, W3C SKOS; LOW, drift reports)
13. **Show the payload.** Users and agents must be able to see exactly which chunks were injected, why, at what cost, and what was dropped. No surveyed product does this per chunk. (MEDIUM, product survey)
14. **Benchmark data never enters the corpus.** Golden sets live outside the indexer roots, carry a canary string, and are excluded from MENS. (MEDIUM, contamination literature)

## 2. Evaluated techniques

Corpus measured on 2026-10-03: 669 non-archive markdown files, about 9.8 MB, about 2.4M tokens. That is roughly 12x over the "under 200k tokens, stuff the prompt" threshold, and far below the scale where ANN indexes, vector databases or graph RAG pay off.

| Technique | Verdict | Evidence | Confidence |
| --- | --- | --- | --- |
| Content-hash-keyed incremental indexing (file hash skip, chunk hash identity, embeddings keyed `(chunk_hash, model)`) | **Adopt** | Continue, Cursor and LlamaIndex all key on content. The current Vox reindex DELETEs and re-INSERTs AUTOINCREMENT rows, which orphans `embeddings.source_id`. | HIGH |
| Heading-bounded chunking with `heading_path`, plus a token-bounded split of oversize sections (`text-splitter` `MarkdownSplitter`, `markdown` feature only) | **Adopt** | The current chunkers are line-based or fixed 4096-byte. They split code fences and embed raw frontmatter. | HIGH |
| Deterministic contextual prefix (title › heading path + page-one summary) on both the embedding and BM25 input | **Adopt** | Anthropic: contextual embeddings and contextual BM25 cut top-20 retrieval failures by 35% and 49%. | HIGH |
| LLM-generated per-chunk context (Anthropic style) | **Gate** on eval delta | About US$1 per M doc tokens with prompt caching, and caching is not uniform across OpenRouter providers. | HIGH (cost), MEDIUM (gain here) |
| Hybrid BM25 + dense + RRF | **Adopt** (reuse `rrf.rs`) | Already implemented. Add field boosts and taxonomy synonyms. RRF has no absolute relevance floor, so it needs a calibrated floor on top. | HIGH |
| Local dense embeddings: Candle 0.10.2 `modernbert` with `Alibaba-NLP/gte-modernbert-base` (149M, Apache-2.0, 8k context) | **Adopt** as default | Pure Rust, already in the lockfile. Self-reported BEIR 55.3 and CoIR 79.3. Placed behind a feature or plugin boundary, because candle is not allowed unconditionally in L0–L3. | HIGH (licence/API), MEDIUM (quality) |
| Cross-encoder rerank, `gte-reranker-modernbert-base`, on the fused top 20 | **Adopt**, latency-gated | Largest single step in Anthropic's ablation (−67% failures with rerank). CPU latency for 20 pairs needs measuring before it goes on the auto-preamble path. | HIGH (method), LOW (latency) |
| Qwen3-Embedding-0.6B quality tier | **Gate**: GPU only, and only at ≥ +2 nDCG@10 on the Vox golden set | Strong small model, but candle `qwen3` has no padding mask, so batch size is 1. | HIGH (specs), MEDIUM (quality) |
| fastembed-rs / ONNX Runtime | **Skip** | It needs a non-optional pre-release `ort` with a build-time binary download and a TLS feature (a transport-crypto ledger change). It also brings candle 0.11, tokenizers 0.23 with the onig C library, and hf-hub 0.5, all of which conflict with workspace pins. | HIGH |
| EmbeddingGemma, CC-BY-NC weights (jina-reranker-v2, NV-Embed-v2) | **Skip** | EmbeddingGemma has a non-OSI licence, a gated download and a 2k context. The others are non-commercial, which is incompatible with FOSS redistribution. | HIGH |
| Late chunking | **Skip** | +2.7–3.6% nDCG and designed for mean-pooled models. The defaults are CLS-pooled, and the gain overlaps with the contextual prefix. | HIGH |
| RAPTOR multi-level trees | **Skip** (per-doc summary card = level 1) | Heavy offline summarization. Authored summaries beat LLM cluster summaries on a curated corpus. | MEDIUM |
| GraphRAG / LightRAG | **Skip** | GraphRAG-Bench (ICLR 2026): vanilla RAG matches or beats GraphRAG on fact retrieval, at 10–100x lower token cost. The corpus already has an authored graph (`related:`, links, taxonomy, `vox graph`). | HIGH |
| HippoRAG-style PageRank over the existing link and taxonomy graph | **Defer** | Only if the eval shows multi-hop failures. Roughly 100 lines, with no LLM triple extraction. | MEDIUM |
| ColBERT, SPLADE | **Skip** | Multi-vector storage is about 100x the size of dense. A reranker plus taxonomy synonyms covers the same gap at this scale. | MEDIUM |
| Vector database / ANN / Qdrant sidecar | **Skip** below ~200k chunks | Linear-scan cosine over ~25k × 768 f32 takes milliseconds. The optional Qdrant feature stays off. | HIGH |
| Two-tier injection (budgeted preamble + pinned tools), mirroring the MCP resource/tool split and Vox's skill catalog plus `vox_skill_use` | **Adopt** | MCP 2025-06-18: resources are application-driven and tools are model-controlled. Anthropic recommends just-in-time retrieval. | HIGH |
| Model-cutoff-aware ranking from OpenRouter `knowledge_cutoff` | **Adopt**, narrowly | Only for `scope: external` pages, using `verified_at`, with a conservative margin. Reported cutoffs differ from effective ones, and Vox-internal facts have no model prior. | HIGH (API), MEDIUM (method) |
| Spotlighting (per-turn boundary tokens) for non-curated tiers | **Adopt** as one layer | Attack success fell from over 50% to under 2% on GPT models in the paper. This is not a guarantee, so it is paired with tool gating and sanitization. | MEDIUM |
| Prompt-injection classifier as the defense | **Skip** as a defense (signal only) | A 95% detection rate is a failing grade against adaptive attackers. | MEDIUM |
| CRAG web fallback, free providers first, paid spend capped | **Adopt** (reuse `crag.rs`, `tavily_budget.rs`) | Locked decision. OpenRouter web via Exa costs about $0.02 per request at defaults. | MEDIUM |
| RAG eval frameworks (RAGAS, DeepEval, TruLens, ARES) as Python glue | **Skip**. Port the metric *definitions* to Rust plus judge prompts through `vox_actor_runtime::llm`. | The VoxScript-first rule. They would also bypass the LLM facade, secrets and cost telemetry. | HIGH |
| llms.txt | **Keep** (already generated by `starlight-llms-txt` 0.10) | The spec requires only an H1, and `llms-full.txt` is a vendor convention. It serves external agents. Internal agents use `vox_kb_*`. | HIGH |

## 3. Current-state defects this practice must fix

These were read from code on 2026-10-03 (HIGH). They are not hypothetical.

| ID | Defect | Location | Remedy |
| --- | --- | --- | --- |
| R1 | Web hits are persisted into the searchable corpus by default (`persist_web_hits = true`), with no review and errors ignored | `crates/vox-search/src/execution.rs`, `policy.rs` | Reroute into the inbox tier, and require a trust tier on every write |
| R2 | Research syntheses are written back with confidence floored at 0.95. `signal_chat` writes assistant turns into `kb_entries` with `accepted=1` | `crates/vox-orchestrator-mcp/src/kb_tools/signal_research.rs`, `signal_chat.rs` | Remove the floor and the writeback. Agent-derived text goes to the inbox and never outranks curated text |
| R3 | The injected preamble is bullet lines with no delimiting, no per-item provenance and no sanitization. Six sources are concatenated, each with its own top-k | `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` | One ranked, budgeted list with spotlighting and provenance |
| R4 | `calculate_recall_at_5` is word overlap between the answer and the gold answer. `calculate_groundedness` is nearly always true | `crates/vox-search/src/evaluation.rs` | Delete or rename these. Use ID-based IR metrics |
| R5 | `TrustScorer` fails open | `crates/vox-search/src/trust.rs` | Fine for ranking. Promotion eligibility must fail closed |
| R6 | 395 of 670 docs are `training_eligible: true`, and `vox-gamify` awards XP for setting it | `docs/src/**`, `crates/vox-gamify/src/quest/templates.rs` | An `authorship` field, `training_eligible: false` for LLM or web text until attested, and a neutralized incentive |
| R7 | The legacy VoxKB surface (2026-06-17) uses `LIKE '%prompt%'` search outside `vox-search`. Two of three signal adapters have no callers. A Tantivy docs mirror has no writer | `KbStore`, `KbRouter`, `kb_tools/*`, `lexical_tantivy.rs` | Retire it into `vox-search`-backed retrieval, and replace the colliding `vox_kb_*` tool names |

## 4. Pitfall to test matrix

Gate types:
- **BLOCK** is a deterministic PR-CI failure.
- **RATCHET** is a baseline in `contracts/` that may only tighten.
- **NIGHTLY** is a live-model or statistical eval in `nightly.yml`. It files an issue and never blocks a PR.

| Pitfall | Invariant / test | Layer | Gate |
| --- | --- | --- | --- |
| Stored prompt injection | `sanitize(s)` removes the Unicode tag block (U+E0000–E007F), zero-width (U+200B–U+200F, U+2060–U+2064) and bidi (U+202A–U+202E, U+2066–U+2069) characters, and is idempotent (proptest) | ingestion + injection | BLOCK |
| Stored prompt injection | Every indexed write carries a non-null `trust_tier`. The tier is an enum argument with no default | indexer | BLOCK |
| Stored prompt injection | Non-curated items are wrapped in delimiters that item text containing the boundary token can't break | injection | BLOCK |
| Stored prompt injection | A `web-ingest:` row is never returned as curated | retrieval | BLOCK |
| Stored prompt injection | A turn with non-curated context can't call an exfiltration-capable tool without an approval event | injection | BLOCK |
| Stored prompt injection | Red-team fixture corpus (`contracts/kb/redteam/`, about 30 pages, AgentDojo-style pairs): attack success rate per model family stays ≤ baseline, with a ≤ 2% target | eval | NIGHTLY + RATCHET |
| Promotion laundering / poisoning | Lint: `origin: web-promoted` requires `sources[]` with url, retrieved date and hash, and `training_eligible: false` unless `human_attested_by` is set | doc-lint | BLOCK |
| Promotion laundering / poisoning | Promotion eligibility is `Ineligible` when the trust lookup errors | ingestion | BLOCK |
| Promotion laundering / poisoning | PoisonedRAG-style k = 1, 3, 5 adversarial pages: target wrong-answer rate | eval | NIGHTLY + RATCHET |
| Self-consuming loop | MENS corpus builder emits zero rows for `authorship: llm-generated` and strips summary blocks | training-corpus | BLOCK |
| Self-consuming loop | Agent-derived entries score below curated entries at equal similarity | retrieval | BLOCK |
| Self-consuming loop | `llm-assisted` token share and distinct-2/3 of the training corpus | training-corpus | RATCHET |
| Self-consuming loop | Source bias: top-1 rate of `llm-*` pages vs their corpus base rate (alert above 1.5x) | eval | NIGHTLY |
| Rewrite meaning loss | Summary identifiers, paths and CLI commands resolve. Summaries name no retired surface as current. Negations and numbers are preserved. Rewrites never touch code fences or add `vox:skip` | doc-lint (tool) | BLOCK |
| Rewrite meaning loss | Mean pairwise similarity of summaries ≤ that of bodies + ε | indexer | NIGHTLY + RATCHET |
| Context dilution | Injected tokens ≤ `budget(model)` for every model in the live registry | injection | BLOCK |
| Context dilution | All scores below the floor → zero items and the "no context" marker set | injection | BLOCK |
| Context dilution | Per-doc chunk cap holds. No superseded or expired items are injected. Rank 1 is at position 0 | retrieval + injection | BLOCK |
| Context dilution | Accuracy drop with gold + 1/3/5 hard distractors | eval | NIGHTLY + RATCHET |
| Wrong freshness clock | Cutoff demotion is a no-op for `scope: vox-internal` and monotone in `verified_at` for `external`. A whitespace-only commit leaves ranking byte-identical | retrieval | BLOCK |
| Wrong freshness clock | KB-on accuracy ≥ KB-off − ε on an external-facts subset, per model | eval | NIGHTLY + RATCHET |
| Citation hallucination | Cited IDs not in the turn's injected set are marked `unverified` and never upgraded | injection | BLOCK |
| Citation hallucination | An out-of-set citation renders the unverified badge (mock IPC) | GUI (Playwright) | BLOCK |
| Citation hallucination | Citation precision and recall (cross-family judge, swap-checked) | eval | NIGHTLY + RATCHET |
| Evals that cannot fail | Nothing under `contracts/kb/eval/` is reachable from indexer roots. The canary string never appears in the index or the MENS corpus | eval + training-corpus | BLOCK |
| Evals that cannot fail | Retrieval eval reports are byte-identical across two runs on the same commit | eval | BLOCK |
| Evals that cannot fail | Recall@5 and nDCG@10 on IDs vs `contracts/kb/retrieval-baseline.v1.json` | eval | RATCHET |
| Evals that cannot fail | Judge swap-consistency ≥ 80%. Generation evals use n ≥ 3 runs with confidence intervals | eval | NIGHTLY |
| Taxonomy drift | Every facet value is in the vocabulary, with a "did you mean" for deprecated aliases. Blocks new and changed files. The legacy backlog is an allowlist that only shrinks | doc-lint | BLOCK + RATCHET |
| Taxonomy drift | Classifier output is always in the vocabulary (constrained decode) | doc-lint (tool) | BLOCK |
| Duplicates / contradictions | `superseded_by` targets exist and have no cycles. Superseded pages are absent from injection | doc-lint + indexer | BLOCK |
| Duplicates / contradictions | Near-duplicate and contradiction-candidate counts | indexer | NIGHTLY + RATCHET |
| Staleness avalanche / rubber-stamp | `verified_at` can't advance without at least one passing mechanical evidence check (path exists, symbol resolves, CLI command present, link returns 200) | ingestion (re-verify) | BLOCK |
| Staleness avalanche / rubber-stamp | No more than X% of pages share a `review_by` week (hash-jittered) | doc-lint | BLOCK |
| Staleness avalanche / rubber-stamp | Share of injected items that are expired | retrieval | NIGHTLY + RATCHET |
| Index / embedding drift | Inserting a vector whose `embed_model_id` differs from the manifest errors. A deleted or renamed doc is gone after reindex | indexer | BLOCK |
| Index / embedding drift | `vox kb status --strict` reports drift against HEAD | indexer | advisory (`vox doctor`) |
| Private-data egress | A fake API key ingested from chat is absent from a remote-model payload. `sensitivity: private` never goes to a remote provider class | injection | BLOCK |
| Opaque context (UX) | Inspector item count equals the backend payload item count. A badge renders for every tier. No remote images auto-load in the Inspector or assistant output | GUI (Playwright) | BLOCK |

## 5. Decisions

| Decision | Choice | Rationale |
| --- | --- | --- |
| Curated SSOT | Git markdown, `docs/src/**` (excluding `archive/`) + `docs/src/library/` | Review, history and rollback for free. `archive/` is never indexed |
| Captured web findings | DB inbox `kb_findings` with provenance: summary, quotes, URL, date, content hash. Full text is a local gitignored snapshot | Keeps unreviewed text out of git and the curated tier, and limits copyright exposure |
| Promotion | Agent drafts the PR (isolated worktree → doc-lint → draft PR via `vox-cli-review`). A human merges | Single write path. PR bodies carry per-claim source excerpts |
| Trust | Trust tiers with a ranking penalty, mapped onto the existing `ContextTrustTier` enum (`Untrusted`, `Advisory`, `Trusted`, `SystemVerified`) | One vocabulary. Categorical, not a fake-precise score |
| Taxonomy | Controlled vocabulary contract plus facet frontmatter, kept separate from `category` (which drives the Astro sidebar) | Prevents drift and keeps navigation stable |
| Page structure | Summary-first "page one" block on every page | It is the summary card and the contextual prefix |
| General knowledge home | `docs/src/library/` | Separates curated non-Vox knowledge from architecture docs |
| Embeddings | Local by default: Candle `gte-modernbert-base` + `gte-reranker-modernbert-base` behind a feature or plugin boundary that respects `layers.toml`, reached via `llm_embed` `provider: "local"`. fastembed rejected | Zero-config, pure Rust, no ledger change. Lexical-only degradation when weights are absent |
| Injection | Hybrid. A per-model budget used as a **ceiling**, with a relevance floor, an inject-nothing path, and pinned `vox_kb_*` tools for depth | Context-rot evidence and the MCP resource/tool split |
| Cutoff-aware ranking | Only for `scope: external` pages, on reviewed `verified_at`, never git mtime | Vox-internal facts have no model prior, and mtime is reset by formatting and bots |
| Web fallback | CRAG, free providers first, paid spend capped. The web toggle is merged into research mode "off" as one control | Avoids two contradicting switches |
| Legacy write paths | `persist_web_hits` rerouted into the inbox tier. The `signal_research` 0.95 floor and the post-turn `accepted=1` writeback removed or rerouted | Closes existing poisoning paths before adding new ones |
| Legacy VoxKB surface | `KbStore` (`LIKE` search), the signal adapters and the dead Tantivy mirror are retired into `vox-search`-backed retrieval. Colliding `vox_kb_*` tool names are replaced, not aliased | One retrieval SSOT |
| Per-chat scope | `@kb` mentions are kept as a locked v1 control and remapped onto the Library scope | Manual @-scope is a universal expectation |
| Authorship | The `authorship:` field lands before any LLM rewrite. LLM-rewritten and web-promoted pages default to `training_eligible: false` until a human attests | Prevents a self-consuming training loop |
| Staleness | `review_by` plus scheduled agent re-verification that needs mechanical evidence. Demote, never delete | Guru/KCS model, plus anti-rubber-stamp |
| Eval | About 40 human-authored seed questions plus an LLM-generated, human-pruned expansion, in BEIR layout under `contracts/kb/eval/`, excluded from the indexer and MENS, with a held-out split | Deterministic, portable and leak-free |
| Gates | Blocking lints + a ratcheted retrieval eval + Playwright for the GUI. Generation evals nightly only | Live-model gates in PR CI become flaky and then get muted |
| Crate edges | None taken. Hashing is routed through `vox-orchestrator-mcp` (already depends on `vox-crypto`). PR drafting goes through `vox-cli-review` (already reaches `vox-forge`). `vox-search → vox-crypto` and `vox-cli → vox-forge` are optional proposals that need user authorization | Crate-edge exceptions are user-authorized only |
| GUI | Library (absorbs Memory and DocReader), Inbox, and a per-turn per-chunk Inspector, all over `callTool` plus one Tauri command | Single writer (the daemon) and the same HITL gates as agents |

**Open questions.** These are recorded for planning, not decided here:
- **Lexical leg.** The VoxDb FTS5 probe probably fails on the Turso engine, whose FTS is Tantivy-backed `USING fts`, and then degrades to `LIKE`. A runtime probe decides between that and a Tantivy Library index with an owning writer.
- **Relevance floor and ceiling.** Calibrating the floor, and the ceiling formula. One suggested starting point is `min(1500 tokens, 2% of context_length)`.
- **Copyright posture** for promoted web knowledge. This needs a legal call.
- **Faithfulness judge** selection under the no-pinned-models policy.
- **Long-term home of `vox-cli-review`**, given that CodeRabbit is retired.

## 6. Sources

| Source | URL | Used for | Confidence |
| --- | --- | --- | --- |
| Anthropic, Contextual Retrieval (2024-09-19) | https://www.anthropic.com/engineering/contextual-retrieval | Contextual prefix, rerank ablation, cost, 200k threshold | HIGH |
| OpenRouter models API | https://openrouter.ai/docs/api/api-reference/models/get-models | `knowledge_cutoff`, `context_length` | HIGH |
| MCP spec 2025-06-18, resources | https://modelcontextprotocol.io/specification/2025-06-18/server/resources | Application-driven resources | HIGH |
| MCP spec 2025-06-18, tools | https://modelcontextprotocol.io/specification/2025-06-18/server/tools | Model-controlled tools | HIGH |
| gte-modernbert-base model card | https://huggingface.co/Alibaba-NLP/gte-modernbert-base | Default embedder | HIGH |
| gte-reranker-modernbert-base model card | https://huggingface.co/Alibaba-NLP/gte-reranker-modernbert-base | Default reranker | HIGH |
| Qwen3-Embedding-0.6B model card | https://huggingface.co/Qwen/Qwen3-Embedding-0.6B | Gated quality tier | HIGH |
| granite-embedding-english-r2 model card | https://huggingface.co/ibm-granite/granite-embedding-english-r2 | Alternative embedder | HIGH |
| EmbeddingGemma model card | https://huggingface.co/google/embeddinggemma-300m | Rejected (licence, gating) | HIGH |
| Candle ModernBERT source | https://github.com/huggingface/candle/blob/main/candle-transformers/src/models/modernbert.rs | Mask-aware forward, sequence classification | HIGH |
| text-splitter | https://crates.io/crates/text-splitter | Markdown splitter | HIGH |
| fastembed-rs | https://github.com/Anush008/fastembed-rs | Rejected (ort, version conflicts) | HIGH |
| Turso FTS docs | https://docs.turso.tech/sql-reference/functions/fts | Lexical-leg open question | MEDIUM |
| llms.txt spec | https://llmstxt.org/ | External agent exposure | HIGH |
| W3C SKOS Reference | https://www.w3.org/TR/skos-reference/ | Taxonomy shape | HIGH |
| Diátaxis | https://diataxis.fr/ | `doc_type` from directory | HIGH |
| DITA 2.0 technical content | https://dita-lang.org/2.0/dita-techcomm/toc.html | Self-contained topic discipline | HIGH |
| AWS, Writing best practices for RAG | https://docs.aws.amazon.com/pdfs/prescriptive-guidance/latest/writing-best-practices-rag/writing-best-practices-rag.pdf | Page-one summaries | MEDIUM |
| kapa.ai writing best practices | https://docs.kapa.ai/improving/writing-best-practices | Page-one summaries | MEDIUM |
| Late chunking (Jina) | https://arxiv.org/abs/2409.04701 | Skip verdict | HIGH |
| GraphRAG-Bench, "When to use Graphs in RAG" | https://arxiv.org/abs/2506.05690 | Skip GraphRAG | HIGH |
| HippoRAG 2 | https://arxiv.org/abs/2502.14802 | Deferred PPR | HIGH |
| ALCE citation metrics | https://arxiv.org/abs/2305.14627 | Citation eval | HIGH |
| Liu et al., Lost in the Middle | https://arxiv.org/abs/2307.03172 | Dilution, U-curve | MEDIUM |
| Chroma, Context Rot | https://research.trychroma.com/context-rot | Dilution | MEDIUM |
| Cuconasu et al., The Power of Noise | https://arxiv.org/pdf/2401.14887 | Distractors | MEDIUM |
| Sufficient Context (Google) | https://arxiv.org/abs/2411.06037 | Abstention | MEDIUM |
| ClashEval | https://arxiv.org/abs/2404.10198 | Stale context overrides priors | MEDIUM |
| HoH, outdated information harms RAG | https://arxiv.org/html/2503.04800v1 | Staleness | MEDIUM |
| Dated Data, effective knowledge cutoffs | https://arxiv.org/abs/2403.12958 | Cutoff margin | MEDIUM |
| Evaluating Verifiability in Generative Search Engines | https://arxiv.org/abs/2304.09848 | Citation support rates | MEDIUM |
| Greshake et al., indirect prompt injection | https://arxiv.org/abs/2302.12173 | Stored injection | MEDIUM |
| Spotlighting (Microsoft) | https://arxiv.org/html/2403.14720v1 | Delimiting non-curated text | MEDIUM |
| Design Patterns for Securing LLM Agents | https://arxiv.org/pdf/2506.08837 | Tool gating, quarantined summarization | MEDIUM |
| AgentDojo | https://arxiv.org/pdf/2406.13352v2 | Red-team fixture design | MEDIUM |
| Willison, The lethal trifecta | https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/ | Threat model | MEDIUM |
| PoisonedRAG | https://arxiv.org/abs/2402.07867 | Promotion poisoning | MEDIUM |
| OWASP Top 10 for LLM Applications 2025 | https://genai.owasp.org/llmrisk/llm082025-vector-and-embedding-weaknesses/ | Vector weaknesses, egress, unbounded spend | MEDIUM |
| Unit 42, memory poisoning via indirect injection | https://unit42.paloaltonetworks.com/indirect-prompt-injection-poisons-ai-longterm-memory/ | Persistent poisoning | MEDIUM |
| Shumailov et al., model collapse (Nature 2024) | https://oatml.cs.ox.ac.uk/publications/202407_Shumailov_ModelCollapse.html | Self-consuming loop | MEDIUM |
| Gerstgrasser et al., Is Model Collapse Inevitable? | https://arxiv.org/abs/2404.01413v1 | Accumulate, don't replace | MEDIUM |
| Alemohammad et al., Self-Consuming Generative Models Go MAD | https://arxiv.org/abs/2307.01850v1 | Self-consuming loop | MEDIUM |
| Dai et al., retrievers biased toward LLM text | https://arxiv.org/abs/2310.20501v3 | Source bias | MEDIUM |
| FABLES | https://arxiv.org/abs/2404.01261v2 | Summary omissions | MEDIUM |
| StorySumm | https://arxiv.org/html/2407.06501v3 | Faithfulness metric limits | MEDIUM |
| Zheng et al., Judging LLM-as-a-Judge (MT-Bench) | https://arxiv.org/pdf/2306.05685 | Judge bias | MEDIUM |
| Panickssery et al., LLM evaluators favor own generations | https://arxiv.org/abs/2404.13076v1 | Cross-family judges | MEDIUM |
| Deng et al., data contamination | https://arxiv.org/html/2311.09783v2 | Golden-set leakage | MEDIUM |
| Barnett et al., Seven Failure Points of RAG | https://arxiv.org/abs/2401.05856v1 | Validate in operation | MEDIUM |
| Bevendorff et al., Is Google Getting Worse? | https://gwern.net/doc/technology/google/2024-bevendorff.pdf | SEO spam in web sources | MEDIUM |
| Continue indexing | https://github.com/continuedev/continue/blob/main/core/indexing/README.md | Content-hash cache keys | LOW |
| Cursor, secure codebase indexing | https://cursor.com/blog/secure-codebase-indexing | Chunk-content embedding cache | LOW |
| LlamaIndex IngestionPipeline | https://developers.llamaindex.ai/python/framework/module_guides/loading/ingestion_pipeline/ | Hash-keyed upserts | LOW |
| Sourcegraph, How Cody understands your codebase | https://sourcegraph.com/blog/how-cody-understands-your-codebase | Lexical-first lesson | LOW |
| Cursor rules | https://cursor.com/docs/context/rules | Activation tiers | MEDIUM |
| Claude Code memory | https://code.claude.com/docs/en/memory | Always-on vs on-demand context | MEDIUM |
| GitHub Copilot Spaces | https://docs.github.com/copilot/using-github-copilot/copilot-spaces/about-organizing-and-sharing-context-with-copilot-spaces | Per-task scope | MEDIUM |
| Guru verification | https://www.getguru.com/features/verification | Badge + owner + interval | MEDIUM |
| Notion verified pages | https://notion.com/help/wikis-and-verified-pages | Verified-first ranking | MEDIUM |
| NotebookLM sources | https://support.google.com/notebooklm/answer/14276569?hl=en | Per-source scope, suggest-then-import | MEDIUM |
| ChatGPT deep research | https://help.openai.com/en/articles/10500283-deep-research | Restrict vs prioritise sites | MEDIUM |
| KCS v6 article states | https://library.serviceinnovation.org/KCS/KCS_v6/KCS_v6_Practices_Guide/030/040/010/030 | Logical archive, reuse is review | MEDIUM |
| Wikipedia Perennial Sources | https://en.wikipedia.org/wiki/Wikipedia:Reliable_sources/Perennial_sources | Categorical source reliability | MEDIUM |
| Azam, Evaluate Your Own RAG | https://huggingface.co/blog/charles-azam/rag | Benchmark on your own corpus | LOW |
| Ropes & Gray, Web Scraping in the Age of AI | https://www.ropesgray.com/en/insights/alerts/2026/05/web-scraping-in-the-age-of-ai-guidance-for-data-owners-and-scrapers | Copyright posture (needs legal review) | LOW |

## Phase 1 spike outcomes (2026-10)

Two spikes were run in Phase 1 so that the Phase 2 lexical-leg decision (IDX-07) and the Phase 5 embedder plan rest on executed facts. Both are measured, not recalled.

### S1: Turso FTS runtime probe (feeds IDX-07)

Scratch crate with `turso = { version = "=0.6.1", default-features = false, features = ["sync"] }`, the same spec as the workspace manifest, using `turso::Builder::new_local(":memory:")` (the constructor `VoxDb` uses). Observed output:

```text
compile_options query: ERR Parse error: Not a valid pragma name
CREATE VIRTUAL TABLE fts5 -> Err(Error("Parse error: no such module: fts5"))
CREATE INDEX USING fts -> Err(Error("Parse error: index method is an experimental feature. Enable with --experimental-index-method flag"))
LIKE whole-query natural-language match count = 0
experimental_index_method(true) + CREATE INDEX USING fts (fts feature OFF) -> Err(Error("Parse error: Error: unknown module name 'fts'"))
```

Consequences:

- The `vox-db` capability probes wrap `PRAGMA compile_options` in `if let Ok(...)`, so the error above is swallowed and `fts5_reported` is always false. The FTS5 virtual tables and triggers are never created, and all lexical chunk search runs as the whole-query `LIKE`, which matches nothing for a natural-language question.
- Turso native FTS needs both the turso `fts` cargo feature and `Builder::experimental_index_method(true)`. The `fts` feature is in turso's default feature set, which the workspace disables. Enabling it pulls tantivy 0.26 next to the workspace's tantivy 0.22.1, so it is a dependency-duplication decision, not a free switch.
- The executable half of S1 is `crates/vox-db/tests/turso_fts_probe.rs`. It asserts each of the behaviours above and that `sqlite_master` has no `search_document_chunks_fts`. If a turso bump changes any of them the test goes red and its message points at IDX-07.

### S2: candle ModernBERT with tokenizers 0.21.4 (feeds Phase 5)

Scratch crate with `candle-core`, `candle-nn` and `candle-transformers` at `=0.10.2` (default features off, so CPU only with no `accelerate` or `mkl`) and `tokenizers =0.21.4` with `fancy-regex`, built `--offline`. Model pins, from the Hugging Face API:

| Model | Revision SHA | License | Weights |
|---|---|---|---|
| `Alibaba-NLP/gte-modernbert-base` (embedder) | `e7f32e3c00f91d699e8c43b53106206bcc72bb22` | apache-2.0 | `model.safetensors`, 298,041,568 B, fp16 |
| `Alibaba-NLP/gte-reranker-modernbert-base` (reranker) | `f7481e6055501a30fb19d090657df9ec1f79ab2c` | apache-2.0 | `model.safetensors`, 598,436,708 B, fp32 |

Observed output (release build):

```text
config parsed: hidden=768 layers=22 classifier_config=None
stock ModernBert::load: ERR cannot find tensor model.embeddings.tok_embeddings.weight
renamed ModernBert::load: OK
dim=768 cos(q,pos)=0.7248 cos(q,neg)=0.4293 (3 short embeds in 182.226083ms)
one 603-token embed on CPU: 1.418045542s
one 603-token embed on CPU: 1.46952675s
one 253-token embed on CPU: 604.218125ms
```

Findings:

1. **tokenizers 0.21.4 loads `tokenizer.json`** (BPE, ByteLevel, NFC, `ignore_merges: false`), vocab 50368, with `[CLS] ... [SEP]` ids for a sample query. No `onig` is needed.
2. **Weight prefix mismatch.** candle's `ModernBert::load` asks for `model.embeddings.tok_embeddings`, `model.layers.N` and `model.final_norm`, but the embedder checkpoint stores keys without the `model.` prefix. Fix: `VarBuilder::rename_f(|n| n.strip_prefix("model.").unwrap_or(n).to_string())`. The reranker checkpoint does use the `model.` prefix, plus `head.*` and `classifier.{weight,bias}`.
3. **Pooling is CLS.** The embedder `config.json` says `"classifier_pooling": "mean"`, but `1_Pooling/config.json` sets `"pooling_mode_cls_token": true`. Use CLS for the embedder.
4. **The stock reranker path is broken in two ways.**
   - The reranker `config.json` has `"label2id": {"LABEL_0": 0}` (an integer value) while candle declares `label2id: HashMap<String, String>` inside a `#[serde(flatten)] Option<ClassifierConfig>`. The flattened `Option` swallows the type error into `None`, so `classifier_config` is `None`, the head becomes `linear(768, 0)`, and load fails on `classifier.weight` of shape [1, 768]. The swallow is inferred from the observed `classifier_config=None`, not separately proven.
   - Even with a fixed config, `ModernBertClassifier::forward` applies softmax over the last dimension. Over a single logit that is always 1.0, so nothing would be ranked.
   - Phase 5 must hand-assemble `ModernBert`, `ModernBertHead` and `Linear(768, 1)`, and use the raw logit with mean pooling.
5. **CPU latency** with no BLAS acceleration feature: about 1.42 s for one 603-token sequence and 0.60 s for one 253-token sequence (measured). Extrapolations, estimated (A4), not measured:
   - Embedding the full corpus (about 2.4M tokens) would take on the order of an hour or more on CPU. Estimated (A4).
   - Reranking 20 pairs at about 512 tokens would take roughly 20 s or more, so it is not viable on the per-turn auto-preamble path without `accelerate` or Metal. Estimated (A4).

Reproduction recipe: create a scratch crate under `~/dev/_scratch` (never a git repo, never referenced by a project), pin the crate versions listed above, download the two model revisions at the SHAs in the table, and build with `--offline` once dependencies are vendored. Scratch code is deliberately not committed. candle is not added to any workspace crate in Phase 1, because it must stay out of layers L0 to L3.
