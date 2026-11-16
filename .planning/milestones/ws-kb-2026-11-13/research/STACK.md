# Stack Research

**Domain:** Curated, agent-facing knowledge base (docs-as-code corpus → hybrid retrieval → context injection for any OpenRouter model), inside an existing Rust workspace
**Researched:** 2026-10-03
**Confidence:** MEDIUM-HIGH overall (crate/model facts verified against crates.io, Hugging Face model cards, official docs; model-quality claims are vendor self-reported and must be re-checked by this milestone's own retrieval eval)

> Method note: the GSD `research-plan` / `research-store` cache seam could not be used (the Bash guard in this worktree refuses the shim's `eval`), so nothing was cached. Confidence tiers below were assigned by hand using the source hierarchy: crates.io / npm / PyPI registry APIs, Hugging Face model cards, official vendor docs, Context7 and arXiv = HIGH; a single secondary web source = MEDIUM; unverified or contradictory = LOW.

---

## 0. Bottom line (read this first)

1. **Don't add a new retrieval engine.** The existing pieces already cover most of the stack: Tantivy 0.22 (lexical), VoxDb `search_document_chunks` + `embeddings` (storage), `rrf.rs` (fusion), `crag.rs` (web fallback), and `pulldown-cmark` 0.13, `serde_yaml_ng` 0.10, `jsonschema` 0.46, `schemars` 1, `tokenizers` 0.21, `hf-hub` 1.0, `candle-*` 0.10.2. The KB milestone needs only **three new dependencies**: `text-splitter` (markdown feature only), plus `react-markdown` and `remark-gfm` in the GUI. Add `dom_smoothie` only if web capture quality proves poor.
2. **Run local embeddings and reranking on Candle, which the workspace already ships. Don't use fastembed/ONNX Runtime.** Default embedder: **`Alibaba-NLP/gte-modernbert-base`** (149M params, Apache-2.0, 8k context). Default reranker: **`Alibaba-NLP/gte-reranker-modernbert-base`** (149M, Apache-2.0). Both use candle-transformers' existing `modernbert` module, so one code path loads both models. Optional quality tier: **`Qwen/Qwen3-Embedding-0.6B`** (Apache-2.0), but only on Metal/CUDA and only if the eval shows a gain.
3. **Use contextual retrieval in its deterministic form** (doc title + page-one summary + heading path prepended to each chunk before embedding and BM25). Gate the LLM-generated per-chunk context (Anthropic style) behind an eval. **Skip** late chunking, RAPTOR beyond the per-doc summary card, GraphRAG/LightRAG, ColBERT and SPLADE at this corpus size.
4. **Build evals in Rust, with no Python.** IR metrics (Recall@k, MRR, nDCG@10) are about 60 lines. RAGAS/TruLens-style faithfulness and context-relevance judges run as prompts through the `vox_actor_runtime::llm` facade. Store the golden set in **BEIR file format** so it stays portable.
5. **Use OpenRouter's `/api/v1/models` for model-cutoff-aware ranking and per-model context budgets.** It now exposes `knowledge_cutoff` and `context_length` per model.

Corpus size measured: **669 non-archive `.md` files, about 9.8 MB, roughly 2.4M tokens.** That is about 12x over Anthropic's "under 200k tokens, just stuff the prompt" threshold, so retrieval is required. It is also far below the point where ANN indexes, vector databases or graph RAG pay off (HIGH; measured locally, threshold from [Anthropic][anthropic-cr]).

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended | Conf. |
|---|---|---|---|---|
| **Candle** (`candle-core`/`candle-nn`/`candle-transformers`) | **0.10.2** (already pinned; do NOT bump to 0.11 inside this milestone; ADR-034 owns the upgrade train) | Local embedding + cross-encoder inference | Already in the lockfile, pure Rust, so it satisfies the build-toolchain invariant (no cmake/nasm/libclang). It ships `modernbert` (`ModernBert::forward(xs, mask)` + `ModernBertForSequenceClassification` with CLS/MEAN pooling), plus `qwen3`, `bert`, `gemma3`, `nomic_bert`. `vox-populi` already has `inference`-feature CPU/CUDA/Metal device selection (`inference/backends/candle_device.rs`) to reuse. | HIGH ([candle models/mod.rs][candle-mod], [modernbert.rs][candle-modernbert]) |
| **gte-modernbert-base** (`Alibaba-NLP/gte-modernbert-base`) | pin HF revision SHA | Default local dense embedder (768-d, CLS pooling, no query prefix, 8192 ctx) | Apache-2.0, ungated (zero-config download via existing `hf-hub` 1.0). Self-reported BEIR 55.33, CoIR 79.31: strongest *code* retrieval in the 150M class, which matters because Vox docs are code-dense. Mask-aware ModernBERT means batched ingest works. | HIGH license/specs, MEDIUM quality ([HF card][gte-embed]) |
| **gte-reranker-modernbert-base** | pin HF revision SHA | Default local cross-encoder reranker (single logit, 8192 ctx) | Apache-2.0, same architecture and tokenizer as the embedder, so only one candle loader is needed. Self-reported BEIR 56.73, CoIR 79.99. IBM's comparison puts it at 56.1 BEIR@top-20 vs granite-reranker 55.8 and bge-reranker-large 54.3. | HIGH license, MEDIUM quality ([HF card][gte-rerank], [granite reranker card][granite-rerank]) |
| **Tantivy** | **0.22.1** (already locked; 0.26.2 exists but no KB feature needs it) | KB lexical leg: multi-field BM25 with boosts on `title`, `heading_path`, `summary`, `aliases` (from taxonomy altLabels), `body` | Already a feature-gated dep (`vox-search/tantivy-lexical`). It is deterministic and independent of the DB build. See the "FTS5 probe" finding under What NOT to Use: the VoxDb lexical leg likely degrades to `LIKE` on Turso. | HIGH ([crates.io][tantivy-crate]) |
| **VoxDb (`turso` 0.6.1)** existing `search_document_chunks` + `embeddings` tables | existing | Derived index storage + inbox tables | Locked decision: git markdown is canonical and the DB is a derived index plus the inbox. Linear-scan cosine over about 20-30k chunks x 768 f32 (about 60-90 MB) takes milliseconds, so no ANN index is needed. Turso exposes `vector32`/`vector_distance_cos` but documents linear scan by default. | HIGH ([Turso vector docs][turso-vec]) |
| **text-splitter** | **0.33.0** (MIT, 2026-09-24, MSRV 1.90 < pinned 1.98.1) | Split oversize heading sections into token-bounded chunks with byte offsets (`MarkdownSplitter::chunk_indices`) | Respects markdown semantic levels (headings > blocks > sentences) and never splits inside a code fence. Its `markdown` feature uses `pulldown-cmark` 0.13, the version already in the workspace, so there's no duplication. **Enable only `markdown`**. Implement `ChunkSizer` over the existing `tokenizers` 0.21 `Tokenizer`. Don't enable the crate's `tokenizers` feature: it requires tokenizers 0.23 **and** forces `tokenizers/onig` (C Oniguruma). | HIGH ([crates.io][ts-crate], [Cargo.toml][ts-cargo], Context7 `/benbrandt/text-splitter`) |
| **OpenRouter models API** (`GET /api/v1/models`) | live | `knowledge_cutoff` (ISO date or null), `context_length`, `top_provider.max_completion_tokens` per model | Source for model-cutoff-aware ranking (boost KB facts dated after the model's cutoff) and for the per-model context-budget fraction in the model registry. Matches the "no pinned models, live latest-per-family" preference. | HIGH ([OpenRouter API ref][or-models]) |

### Supporting Libraries

| Library | Version | Purpose | When to Use | Conf. |
|---|---|---|---|---|
| `pulldown-cmark` | 0.13.4 (existing) | Heading-path extraction, frontmatter-stripped AST walk, link graph (`related:` + inline links) for lint and "see also" | Always; this is the first-stage chunker (heading sections). text-splitter only handles sections over budget. | HIGH |
| `serde_yaml_ng` (aliased `serde_yaml`) | 0.10 (existing) | Parse frontmatter + taxonomy contract | Always. Don't add a frontmatter crate; splitting on the `---` fences takes a few lines. | HIGH |
| `jsonschema` + `schemars` | 0.46 / 1.x (existing) | Validate the taxonomy contract and per-doc facet frontmatter (`topic`, `domain`, `kb_tier`, `review_by`, `sources`, `trust_tier`) in `vox-doc-pipeline` lint | Always. Derive the schema from Rust types with schemars so the lint and the indexer share one definition. | HIGH |
| `tokenizers` | 0.21.4 (existing, `fancy-regex`, no onig) | Load each model's `tokenizer.json` for embedding and for chunk sizing | Always. Chunk budgets are measured in the *embedding model's* tokens. | HIGH |
| `hf-hub` | 1.0.0 (existing, rustls) | Download + cache pinned model revisions | Always. Pin `revision` to a commit SHA, never `main` (vectors are model-specific). | HIGH |
| `blake3` via `vox-crypto` | existing | Doc/chunk content hashes for incremental re-embed, provenance `hash` field for captured web snapshots | Always. AGENTS.md routes hashing through `vox-crypto`. `vox-search` currently imports `blake3` directly; don't copy that pattern into new code. | HIGH |
| `html2text` / `scraper` | 0.12 / 0.20 (existing, `web-scrape` feature) | Turn fetched web pages into text for inbox capture | Default capture path | HIGH |
| `dom_smoothie` | 0.18.2 (MIT, Readability.js port) | Main-content extraction (strip nav/boilerplate) before summarizing a captured page | **Only if** inbox summaries are polluted by boilerplate and Tavily Extract (already `tavily_extract.rs`) is outside the free budget. Check for an html5ever version duplicate against `scraper` 0.20 before adding. | MEDIUM ([crates.io][domsmoothie]) |
| `react-markdown` + `remark-gfm` | 10.1.0 / 4.0.1 (MIT) | GUI Library reader: GFM tables, fenced code, task lists | Add to `crates/vox-gui/ui`. It renders to React elements, not `innerHTML`, so it's safe by default for untrusted inbox content. The existing hand-rolled `ResearchReportMarkdown.tsx` lacks tables and code blocks. **Don't enable `rehype-raw`.** | HIGH ([npm][react-md], [npm][remark-gfm]) |
| `starlight-llms-txt` | **stay on 0.10.0** | Generates `llms.txt` / `llms-full.txt` / `llms-small.txt` for the docs site | Already installed. 0.12.0 needs `astro ^7` and `@astrojs/starlight >=0.41`, and the site is on astro ^6 / starlight 0.38.3. Bump only together with an Astro upgrade, not in this milestone. | HIGH ([npm][sllms]) |

### Optional / gated (adopt only if the retrieval eval shows a win)

| Item | Version | Purpose | Gate | Conf. |
|---|---|---|---|---|
| **Qwen3-Embedding-0.6B** | pin SHA | Quality-tier embedder (1024-d, MRL 32-1024, 32k ctx, last-token pooling, `Instruct: …\nQuery:` prefix on queries) | MTEB-eng-v2 retrieval 61.83 is self-reported. candle `qwen3::Model::forward(input, offset)` takes **no padding mask** and holds KV-cache state, so embed one sequence at a time and call `clear_kv_cache`. That's fine for query-time use but slow for bulk ingest on CPU (about 2.4M tokens). Offer it only when Metal/CUDA is detected **and** nDCG@10 beats gte by ≥ 2 points on the Vox golden set. | HIGH specs ([HF card][qwen3-embed], [candle qwen3.rs][candle-qwen3]), MEDIUM quality |
| **granite-embedding-english-r2** / **-small-r2** | pin SHA | Alternative ModernBERT embedder (149M / 47M; Apache-2.0; trained only on "enterprise-appropriate", governed data) | Same candle loader. Pick it over gte if the Foundation wants clean training-data provenance, or use small-r2 (384-d) for low-end machines. Self-reported numbers conflict across IBM's own pages (BEIR 53.1 vs 56.4), so let the Vox eval decide. | HIGH license ([HF card][granite-embed], [arXiv 2508.21085][granite-paper]), LOW numbers |
| LLM-generated chunk context (Anthropic contextual retrieval) | n/a | 50-100-token situating context per chunk, generated via `vox_actor_runtime::llm` | Only if the deterministic prefix leaves a measurable gap. Anthropic's cost estimate assumes prompt caching (US$1.02 per M doc tokens, about US$2.5 for this corpus), and prompt caching isn't uniform across OpenRouter providers. Re-run only for changed docs (hash-keyed). | HIGH ([Anthropic][anthropic-cr]) |
| `Ollama` provider in `llm_embed` | n/a | Lets users who already run Ollama use it for embeddings (OpenAI-compatible `/v1/embeddings`) | Just a `base_url` arm in the facade. Not the default, because it isn't zero-config. | MEDIUM |

### Development / Eval Tools

| Tool | Purpose | Notes |
|---|---|---|
| Rust eval harness in `vox-search` (`evaluation.rs` already exists, with intent-only checks) | Recall@5/10, MRR@10, nDCG@10, "answer-bearing chunk in injected preamble" rate | Write it by hand, about 60 lines. No crate needed. Run as `vox ci kb-eval` (or a `.vox` script). Fixtures go in BEIR layout: `corpus.jsonl`, `queries.jsonl`, `qrels/test.tsv`. |
| LLM-judge metrics via facade | RAGAS-equivalent **faithfulness** (claim decomposition → per-claim support check), **context precision/recall**, TruLens **RAG-triad** (context relevance, groundedness, answer relevance), ALCE-style **citation recall/precision** | Port the *definitions*, not the Python packages. The judge model comes from the registry and must be from a **different family** than the generator. Runs nightly/scheduled (it costs money and is non-deterministic). Gate it on deltas against a stored baseline, not absolute thresholds. |
| Synthetic query generation | Bootstrap the golden set: LLM writes 2-3 questions per doc section, followed by a round-trip filter (keep a question only if the source chunk retrieves in the top-k) and human review of a sample | Promptagator/InPars-style. Store `generated_by` and model id in qrels metadata so a judge-model change triggers re-labeling. |
| Playwright (`@playwright/test` ^1.62, `@axe-core/playwright`) | GUI Library visual stepper + a11y | Already installed; AGENTS.md makes it mandatory for new GUI surfaces. |
| `vox-doc-pipeline` lint | Frontmatter facet schema, taxonomy membership, page-one summary length bounds, `review_by` in the past → warn, dead `related:` links | Extend the existing lint; don't add a new tool. |

---

## 1. Knowledge-base management practices to adopt

| Practice | Adopt as | Rationale | Conf. |
|---|---|---|---|
| **Docs-as-code** | Already the locked decision (git markdown canonical, PR review, CI lint) | Review, provenance and rollback come free from git. The DB is only a derived index plus the inbox. | HIGH |
| **Diátaxis** (tutorial / how-to / reference / explanation) | **Derive `doc_type` from the existing directory** (`docs/src/{tutorials,how-to,reference,explanation}`); add only `research`, `adr`, `decision` for the other trees. Don't add a frontmatter field authors must keep in sync with the path. | Retrieval can filter or boost by type: "how do I" queries favor how-to, "what is" queries favor explanation/reference. ([diataxis.fr][diataxis]) | HIGH |
| **DITA 2.0 topic typing** | Borrow two ideas only. (a) **One topic = one question answered**: a section should be self-contained. (b) Add `troubleshooting` and `glossary` as `doc_type` values for `docs/src/library/`. **Don't adopt DITA XML/maps.** | DITA 2.0 technical content defines concept / task / reference / troubleshooting / glossentry. Its value for RAG is the self-contained-topic discipline, not the XML. ([dita-lang.org][dita]) | HIGH |
| **Controlled vocabulary (SKOS-shaped)** | `contracts/documentation/kb-taxonomy.v1.yaml` with SKOS-lite fields per concept: `id`, `prefLabel`, `altLabel[]`, `definition`, `broader`, `related[]`, `scopeNote`. Validate with jsonschema. **No RDF/Turtle tooling.** | SKOS is the W3C standard shape for thesauri, so reusing its field names costs nothing and keeps a later RDF export trivial. The retrieval payoff: `altLabel`s become BM25 query-expansion synonyms (e.g. "Clavis" ↔ "vox-secrets"), and `broader` drives facet roll-up in the GUI. ([W3C SKOS][skos]) | HIGH |
| **Metadata schema** | Locked facets (`topic`, `domain`, `kb_tier`, `review_by`, `sources`) plus existing `title`/`description`/`category`/`status`. Add `trust_tier` on library pages and `sources[]` entries `{url, fetched, hash, quote?}` per the locked provenance rule. Keep `schema_type: TechArticle` (schema.org) where it already exists. | One schemars type serves the lint, the indexer and the GUI facet panel. | HIGH |
| **"Page one" / BLUF summary block** | Required first block after frontmatter: 40-120 words that answer "what is this and when do I need it", with no forward references. Lint the length. The frontmatter `description` stays one sentence. | It's the per-doc summary card and the contextual prefix for every chunk. AWS and kapa.ai guidance converge on front-loaded, self-contained, acronym-expanded sections. ([AWS RAG writing guide][aws-rag], [kapa.ai][kapa]) | HIGH (practice), MEDIUM (magnitude) |
| **llms.txt** | Already generated (`docs/src/.well-known/llms.txt`, `llms-full.txt`) via `starlight-llms-txt` 0.10. Add an `## Optional` H2 for low-priority sets, plus `.md` companion URLs per page. | The spec (Jeremy Howard, 2024-09-03, revised 2026-08-10) requires only an H1. Blockquote, H2 file lists and the `Optional` section are conventions. **`llms-full.txt` is not part of the spec**; it's a vendor convention (Mintlify/Anthropic docs), so don't treat it as normative. llms.txt targets external agents; internal agents should use `vox_kb_*` tools. ([llmstxt.org][llmstxt]) | HIGH |
| **AGENTS.md** | Already in use. Keep it a short policy file that **points at** KB entry points (`vox_kb_search`). Don't inline KB content. | Donated to the Linux Foundation Agentic AI Foundation on 2025-12-09 and adopted by 60k+ projects. Its value is a stable always-loaded pointer, not storage. ([OpenAI AAIF][aaif]) | HIGH |
| **Staleness** | `review_by` date facet plus the scheduled re-verification agent (locked). Expired pages get a ranking penalty and a GUI badge, never silent deletion. | Matches the locked decision. | HIGH |

---

## 2. Local models: embedding, reranking, chunking

### Embedding model landscape (open-weight, 2025-2026)

| Model | Params | License | Ctx | Notes for Vox | Verdict |
|---|---|---|---|---|---|
| **gte-modernbert-base** | 149M | Apache-2.0 | 8192 | CLS pooling, no prefix; BEIR 55.3, CoIR 79.3 (self-reported) | **Default** |
| granite-embedding-english-r2 / small-r2 | 149M / 47M | Apache-2.0 | 8192 | Clean-data provenance; same loader | A/B candidate, low-end fallback |
| Qwen3-Embedding-0.6B | 0.6B | Apache-2.0 | 32k | Best small-model quality; no padding mask in candle `qwen3` | Gated quality tier |
| EmbeddingGemma-300M | 308M | **Gemma Terms of Use, gated download** | 2048 | MTEB-eng-v2 69.67; no fp16 | **Reject as default**: not OSI-licensed, needs an HF token (breaks zero-config), and its 2k context is shorter than many Vox sections |
| nomic-embed-text-v2-moe | 475M MoE | Apache-2.0 | 512 (v2) | MoE, multilingual focus | No; corpus is English, and the MoE path needs a custom candle impl |
| bge-m3 | 568M | MIT | 8192 | Dense+sparse+multi-vector | No; larger and older, quality lead gone |
| DenseOn / LateOn (LightOn, arXiv 2607.27178, 2026-07) | 149M | open (details TBD) | — | BEIR 56.2 / 57.2, "SOTA in size class" | Watch-list; re-check license and candle compatibility next milestone |
| Qwen3-Embedding-4B/8B, NV-Embed-v2 | 4-8B | Apache / CC-BY-NC | — | Top of MTEB | No; too heavy for the local default, and NV-Embed is non-commercial |

Sources: [Qwen3 card][qwen3-embed], [EmbeddingGemma card][gemma-embed], [granite card][granite-embed], [gte card][gte-embed], [DenseOn arXiv][denseon]. Leaderboard aggregators (premai, presenc) disagree on licenses and dates. For example, one lists Qwen3-Embedding as a custom license and "early 2026", while the HF card says Apache-2.0 and the model shipped June 2025. Treat aggregator numbers as LOW.

**Why ModernBERT-class over a 0.6B decoder as default:** about 2.4M corpus tokens, embedded on every clean clone and re-embedded incrementally. A mask-aware 149M encoder batches on CPU, while a 0.6B decoder with no padding mask runs at batch size 1. In the quality numbers we found, the gap at this size on code-heavy English is a couple of nDCG points, and the reranker plus contextual prefix recover more than that (Anthropic: reranking took the failure-rate reduction from 49% to 67%). **MEDIUM**, so verify it with the eval harness in the first retrieval phase.

### Rerankers

| Model | Params | License | Verdict |
|---|---|---|---|
| **gte-reranker-modernbert-base** | 149M | Apache-2.0 | **Default**; shares the loader with the embedder |
| granite-embedding-reranker-english-r2 | 149M | Apache-2.0 | Equivalent alternative (BEIR 55.8, MLDR 45.8) |
| bge-reranker-v2-m3 | 568M | Apache-2.0 | Multilingual strength isn't needed; 4x the cost |
| Qwen3-Reranker-0.6B | 0.6B | Apache-2.0 | Highest quality, but decoder-style yes/no logit scoring; candidate for a later quality tier |
| jina-reranker-v2 | — | **CC-BY-NC** | **Reject**: non-commercial license is incompatible with FOSS redistribution |

**Rerank only the fused top-20** (Anthropic's configuration), with passages truncated to about 512 tokens. CPU latency for 20 pairs is unmeasured, so the first retrieval phase must benchmark it. If the auto-preamble path blows its latency budget, rerank only in `vox_kb_*` tool calls and keep the preamble on fused scores.

### Chunking

- Stage 1: split on headings with `pulldown-cmark`, recording `heading_path` ("Doc title › H2 › H3"). Strip frontmatter first. **The current `ingest.rs::chunk_markdown_sections_with_size` is line-based. It splits on any `# ` / `## ` line, so a `#` comment inside a code fence becomes a boundary. It also embeds the raw YAML frontmatter into chunk 1 and ignores H3+.** Replace it rather than extend it. (HIGH; read locally.)
- Stage 2: sections over budget go through `text_splitter::MarkdownSplitter` with `ChunkConfig::new(256..512)` sized by the embedder's tokenizer (custom `ChunkSizer`), with small overlap (about 10%).
- Stage 3 (indexing text, not display text): `"{title} › {heading_path}\n{page_one_summary}\n\n{chunk}"`. This is the deterministic contextual prefix, applied to **both** the embedding input and the Tantivy `body`, mirroring Anthropic's "contextual embeddings + contextual BM25". Store the display text separately so injection doesn't repeat the prefix for every chunk of the same doc.
- Plus one **summary-card** row per doc (title + description + page-one + facets). That's the single-level RAPTOR equivalent, and it's what the small auto-preamble should prefer.

---

## 3. Advanced retrieval techniques: which are worth it at about 1k docs / 2.4M tokens

| Technique | Evidence | Verdict for this milestone | Conf. |
|---|---|---|---|
| **Contextual retrieval** (Anthropic, 2024-09-19) | Top-20 failure rate 5.7% → 3.7% (contextual embeddings), → 2.9% (+ contextual BM25), → 1.9% (+ rerank) | **Adopt.** Deterministic prefix by default; LLM-generated context gated on the eval. This is the highest-ROI item. | HIGH ([Anthropic][anthropic-cr]) |
| **Hybrid BM25 + dense + RRF** | Already implemented (`rrf.rs`, `VOX_SEARCH_RRF_K`) | **Reuse.** Add field boosts and taxonomy-synonym expansion on the BM25 side. | HIGH |
| **Cross-encoder rerank** | Biggest single step in Anthropic's ablation | **Adopt** (local gte-reranker) | HIGH |
| **Late chunking** (Jina, arXiv 2409.04701) | +2.7-3.6% nDCG@10 on BEIR; largest gains for very small chunks; designed for **mean-pooled** long-context models | **Skip.** The default models are CLS-pooled; the gain overlaps with the deterministic prefix; it complicates batching. | HIGH ([arXiv][latechunk]) |
| **RAPTOR** (recursive cluster-summaries) | Higher hallucination than HippoRAG 2 in that paper's eval; heavy offline LLM summarization | **Skip** multi-level trees. The locked per-doc summary card is level 1, and authored `description` + page-one summaries beat LLM cluster summaries for a curated corpus. | MEDIUM ([HippoRAG 2][hipporag2]) |
| **GraphRAG / LazyGraphRAG / LightRAG** | GraphRAG-Bench (ICLR 2026): vanilla RAG matches or beats GraphRAG on simple fact retrieval; graphs help multi-hop/summarization at 10-100x token cost | **Skip.** The corpus already has an explicit, human-authored graph (frontmatter `related:`, inline links, taxonomy `broader/related`, `vox graph`). Use it for one-hop "see also" expansion, not LLM-extracted entity graphs. | HIGH ([arXiv 2506.05690][graphbench]) |
| **HippoRAG 2** (PPR over a KG) | Improves multi-hop without hurting simple QA; cheaper offline than GraphRAG/RAPTOR/LightRAG | **Defer.** If multi-hop failures show up in the eval, run Personalized PageRank over the *existing* link and taxonomy graph (no LLM triple extraction). That captures most of the idea for about 100 lines. | MEDIUM ([arXiv 2502.14802][hipporag2]) |
| **ColBERT / late interaction** | The superseded `kb-systems-sota-research-2026.md` calls it "the third pillar" | **Skip.** Multi-vector storage is about 100x dense; no candle ColBERT loader in use; a reranker gives comparable precision at this scale. | MEDIUM |
| **SPLADE / learned sparse** | — | **Skip.** BM25 + synonyms from the taxonomy covers vocabulary mismatch in a controlled corpus. | MEDIUM |
| **CRAG web fallback** | Already in `crag.rs` | **Reuse** (locked) with the free-first budget (`tavily_budget.rs`). | HIGH |

---

## 4. Evaluation: what runs in Rust vs as jobs

| Need | Tool decision | Where it runs |
|---|---|---|
| Retrieval quality (Recall@k, MRR, nDCG@10, MAP) | Hand-rolled Rust over BEIR-format fixtures | `cargo test -p vox-search` (deterministic, local embedder, small golden set, about 50-100 queries) **and** a full scheduled run |
| Injection quality ("is the answer-bearing chunk inside the token budget for model X?") | Rust, using registry `context_length` × budget fraction | CI-deterministic |
| Faithfulness / groundedness, context precision/recall, answer relevance | RAGAS/TruLens **metric definitions** re-implemented as judge prompts via `vox_actor_runtime::llm` | Scheduled job (nightly or weekly), costs tracked through the facade; compare against a stored baseline |
| Citation faithfulness (each claimed `[n]` supports the sentence) | ALCE-style citation recall/precision as judge prompts | Scheduled job; also usable as a gate on promotion PR drafts |
| Inbox → promotion draft quality | Same judge + human merge (locked) | Per draft |

**Don't adopt the Python frameworks** (RAGAS 0.4.3, DeepEval 4.2.8, TruLens, ARES; all Apache-2.0 and current). They're good, but AGENTS.md forbids new Python glue, and they'd bypass the LLM facade (secrets, cost telemetry, model registry). ARES specifically requires fine-tuning judge classifiers plus prediction-powered inference, which is overkill for a curated corpus with a human merge gate. Their published metric definitions are what we take. (HIGH for versions: [PyPI ragas][ragas], [PyPI deepeval][deepeval]; MEDIUM for "definitions port cleanly", since RAGAS faithfulness is claim extraction followed by a per-claim NLI-style verdict, which maps directly onto two facade calls.)

---

## Installation

```toml
# Cargo: workspace root [workspace.dependencies] (one new entry)
text-splitter = { version = "0.33", default-features = false, features = ["markdown"] }

# Reused, no change: candle-core/-nn/-transformers 0.10, tokenizers 0.21, hf-hub 1,
# tantivy 0.22, pulldown-cmark 0.13, serde_yaml(_ng) 0.10, jsonschema 0.46, schemars 1,
# html2text 0.12, scraper 0.20, rayon 1.10, memmap2 0.9, safetensors 0.7
```

```bash
# GUI (crates/vox-gui/ui)
pnpm add react-markdown@^10.1.0 remark-gfm@^4.0.1
```

Model weights are downloaded at runtime via `hf-hub` to the HF cache, pinned by revision SHA. They're never vendored into git and never fetched at build time.

**Placement constraint (from `layers.toml`):** "unconditional candle-core/candle-nn deps must stay out of CORE L0-L3." So the local embedder and reranker must live either behind a cargo feature in an L3 crate (`vox-populi`'s existing `inference` feature already has the candle device backends) or in an L4 plugin. They must be reachable through the `llm_embed` facade as a `provider: "local"` arm, so `vox-search` keeps calling `llm_embed` and the model-agnostic-LLM rule holds. Architecture research should pick between these two options. Neither needs a new crate edge if it goes through the facade's provider dispatch, but check `vox ci crate-edges`.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|---|---|---|
| Candle + ModernBERT loaders | **fastembed-rs 7.1.0** (Apache-2.0, 3.8M downloads; bundles bge/gte/EmbeddingGemma/Qwen3/rerankers) | Only in a crate outside the workspace (or a prototype) that isn't bound by the toolchain/ledger rules. In-tree it drags: `ort =2.0.0-rc.13` (pre-release, **non-optional dependency**, downloads prebuilt ONNX Runtime at build time with `tls-native`/`tls-rustls`, which needs a transport-crypto ledger entry that only the user can authorize); candle **0.11** (conflicts with the 0.10.2 pin + `patches/candle-metal-kernels-0.10.2`); tokenizers **0.23 with onig**; hf-hub **0.5** (workspace is on 1.0). That's four duplicate or conflicting stacks for convenience. |
| gte-modernbert-base | Qwen3-Embedding-0.6B as default | GPU/Metal-only installs, or if the eval shows ≥ 2 nDCG points on Vox queries |
| Linear-scan cosine in VoxDb/in-process | Qdrant sidecar (existing optional `qdrant-vector`) | Above about 200k chunks, or multi-tenant mesh search. Not at about 25k. |
| Tantivy for KB lexical | Turso native FTS (`CREATE INDEX … USING fts`, Tantivy-backed, per-column weights) | Once vox-db migrates its FTS5 probe to Turso's FTS syntax and verifies the `turso` crate feature that enables it. That would put lexical search in the same transaction as the inbox tables. Revisit after this milestone. |
| Deterministic contextual prefix | LLM-generated per-chunk context | Eval shows residual misses on chunks whose heading path is uninformative (e.g. "Notes", "Details") |
| react-markdown | Render HTML server-side with pulldown-cmark → `dangerouslySetInnerHTML` | Never for inbox content (untrusted web text). Would need `ammonia` (not in lockfile). |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|---|---|---|
| `fastembed` / `ort` in-tree | See above: pre-release ORT, build-time binary download, TLS-provider ledger impact, candle/tokenizers/hf-hub version conflicts | Candle 0.10.2 `modernbert` / `qwen3` |
| `text-splitter` `tokenizers` feature | Pulls tokenizers 0.23 **+ `onig` C library**; the workspace deliberately uses `fancy-regex` | `markdown` feature + custom `ChunkSizer` over tokenizers 0.21 |
| EmbeddingGemma as default | Gemma Terms of Use (not OSI) + gated HF download (needs a token) + 2k context + no fp16 | gte-modernbert-base |
| jina-reranker-v2, NV-Embed-v2, any CC-BY-NC weights | Non-commercial license incompatible with a FOSS Foundation's redistribution | Apache/MIT models above |
| Python RAG eval stacks (RAGAS/DeepEval/TruLens/ARES) as runtime or CI glue | VoxScript-first rule; bypasses LLM facade/secrets/cost telemetry | Rust metrics + facade judge prompts |
| GraphRAG / LightRAG / RAPTOR trees | 10-100x indexing tokens; no gain on fact lookup (GraphRAG-Bench); corpus already has authored links | Existing link/taxonomy graph, optional PPR later |
| Relying on VoxDb "FTS5" for the KB lexical leg | `vox-db/src/schema_extensions.rs` creates FTS5 tables only if `pragma compile_options` reports FTS5. The Turso Rust engine (turso 0.6.1) implements FTS via Tantivy (`USING fts`), not FTS5, so the probe most likely fails and `knowledge_nodes`/chunk lexical search silently falls back to `LIKE` (`capabilities.rs` even boosts vector weight to compensate). **MEDIUM**: verify with a runtime probe in phase 1. | Tantivy index (`tantivy-lexical` feature) rebuilt from git markdown |
| Embedding with remote providers by default (current `embedding_env.rs` order: HF router `all-MiniLM-L6-v2` → OpenAI → OpenRouter `text-embedding-3-small`) | Contradicts the locked local-default decision; mixed providers produce incompatible vector spaces in one `embeddings` table | Local default; record `(model_id, revision, dim)` with every vector and refuse cross-model similarity |
| Bumping `starlight-llms-txt` to 0.12 | Requires Astro 7 / Starlight ≥ 0.41 | Stay on 0.10.0 |
| ColBERT / SPLADE / AMP "agent memory protocol" verbs | Unjustified at this scale; AMP was described as "emerging" with no stable spec in the superseded doc | Dense + BM25 + rerank; MCP `vox_kb_*` tools |

---

## Stack Patterns by Variant

**If the machine has Metal or CUDA (detected via `vox-populi` `candle_device`):**
- Use the same default models on GPU. Offer the Qwen3-Embedding-0.6B tier only if the eval justifies it.
- Changing the embedder invalidates the index, so trigger a full re-embed keyed on `(model_id, revision)`.

**If the machine is CPU-only / low-end:**
- Use `granite-embedding-small-english-r2` (47M, 384-d) and rerank only in explicit tool calls.

**If the user has no network at first run:**
- Lexical-only retrieval (Tantivy) still works, so degrade gracefully and surface a `vox doctor` hint to fetch model weights.

---

## Version Compatibility

| Package | Compatible With | Notes |
|---|---|---|
| `text-splitter 0.33` (`markdown`) | `pulldown-cmark 0.13.x` (workspace 0.13.4), Rust ≥ 1.90 | No duplicate pulldown-cmark |
| `candle-transformers 0.10.2` `modernbert` | `tokenizers 0.21` | ModernBERT `tokenizer.json` is a standard BPE; verify load in a phase-1 spike |
| `candle 0.11` | — | Do not adopt here; ADR-034 upgrade train + metal-kernels patch |
| `starlight-llms-txt 0.10.0` | astro ^6, starlight 0.38 | 0.12 needs astro 7 |
| `react-markdown 10.1` | React ≥ 18 (GUI on 19.2) | OK |
| `ort 2.0.0-rc.13` | ONNX Runtime 1.28 | Pre-release; listed only as the reason fastembed is rejected |

---

## What the superseded `kb-systems-sota-research-2026.md` (2026-06-17) got right vs stale

- **Still valid:** RAG vs memory split; hybrid BM25+dense+RRF; cross-encoder rerank on top-k; provenance (source/timestamp/confidence); "similarity ≠ truth"; staleness/TTL; context-poisoning risk for web-captured content; MCP as the exposure layer.
- **Stale or wrong for this milestone:** (1) "ColBERT is the third pillar". Unjustified at 1k docs. (2) "Chunking matters more than embedding model" was stated without evidence; Anthropic's ablation shows contextualization + rerank dominate. (3) It is memory-centric (episodic/TTL classes) and says nothing about **curated docs** (taxonomy, page-one summaries, Diátaxis). (4) No concrete models, crates, versions or licenses. (5) No contextual retrieval, late chunking or GraphRAG-Bench evidence. (6) Recommends "LoCoMo / LongMemEval", which are conversational-memory benchmarks, not doc-retrieval benchmarks. Use a Vox-specific BEIR-format golden set instead. (7) AMP verbs: drop.

---

## Sources

- [anthropic-cr]: https://www.anthropic.com/engineering/contextual-retrieval (contextual retrieval results, prompt, cost, 200k-token note; 2024-09-19), HIGH
- [candle-mod]: https://raw.githubusercontent.com/huggingface/candle/main/candle-transformers/src/models/mod.rs (modernbert, qwen3, bert, gemma3, nomic_bert present), HIGH
- [candle-modernbert]: https://raw.githubusercontent.com/huggingface/candle/main/candle-transformers/src/models/modernbert.rs (mask-aware forward, SequenceClassification), HIGH
- [candle-qwen3]: https://raw.githubusercontent.com/huggingface/candle/main/candle-transformers/src/models/qwen3.rs (forward(input, offset), no padding mask), HIGH
- https://crates.io/api/v1/crates/candle-transformers (0.11.0 latest, 2026-06-26), HIGH
- [gte-embed]: https://huggingface.co/Alibaba-NLP/gte-modernbert-base, HIGH
- [gte-rerank]: https://huggingface.co/Alibaba-NLP/gte-reranker-modernbert-base, HIGH
- [granite-embed]: https://huggingface.co/ibm-granite/granite-embedding-english-r2, HIGH
- [granite-rerank]: https://huggingface.co/ibm-granite/granite-embedding-reranker-english-r2, HIGH
- [granite-paper]: https://arxiv.org/abs/2508.21085, HIGH
- [qwen3-embed]: https://huggingface.co/Qwen/Qwen3-Embedding-0.6B; https://arxiv.org/abs/2506.05176, HIGH
- [gemma-embed]: https://huggingface.co/google/embeddinggemma-300m; https://arxiv.org/abs/2509.20354, HIGH
- [denseon]: https://arxiv.org/abs/2607.27178 (DenseOn/LateOn, 2026-07-29), MEDIUM
- https://www.premai.io/blog/best-embedding-models-for-rag-2026-ranked-by-mteb-score-cost-and-self-hosting/ ; https://presenc.ai/research/best-open-weight-embedding-models-2026 (aggregators; contradict primary sources on licenses), LOW
- https://futureagi.com/blog/best-rerankers-for-rag-2026/ ; https://presenc.ai/research/best-open-weight-reranker-models-2026 (reranker landscape), LOW-MEDIUM
- fastembed: https://crates.io/api/v1/crates/fastembed (7.1.0, 2026-09-22); https://raw.githubusercontent.com/Anush008/fastembed-rs/main/Cargo.toml (ort =2.0.0-rc.13 non-optional, candle 0.11, tokenizers 0.23 onig, hf-hub 0.5); https://github.com/Anush008/fastembed-rs, HIGH
- ort: https://crates.io/api/v1/crates/ort (2.0.0-rc.13, no stable); https://ort.pyke.io/setup/linking, HIGH / MEDIUM (download mechanics)
- [ts-crate]: https://crates.io/api/v1/crates/text-splitter; [ts-cargo]: https://raw.githubusercontent.com/benbrandt/text-splitter/main/Cargo.toml; Context7 `/benbrandt/text-splitter` (ChunkSizer, MarkdownSplitter::chunk_indices), HIGH
- [tantivy-crate]: https://crates.io/api/v1/crates/tantivy (0.26.2), HIGH
- [turso-vec]: https://docs.turso.tech/sql-reference/functions/vector ; https://docs.turso.tech/sql-reference/functions/fts (Tantivy-backed FTS, per-column weights), HIGH (docs) / MEDIUM (crate feature gating)
- [or-models]: https://openrouter.ai/docs/api/api-reference/models/get-models (`knowledge_cutoff`, `context_length`), HIGH
- [llmstxt]: https://llmstxt.org/ (spec; no llms-full.txt), HIGH
- [sllms]: https://registry.npmjs.org/starlight-llms-txt/latest (0.12.0 peers astro ^7), HIGH
- [aaif]: https://openai.com/index/agentic-ai-foundation/ (AGENTS.md donated 2025-12-09), HIGH
- [diataxis]: https://diataxis.fr/, HIGH
- [dita]: https://dita-lang.org/2.0/dita-techcomm/toc.html (DITA 2.0 topic types), HIGH
- [skos]: https://www.w3.org/TR/skos-reference/ (W3C Recommendation), HIGH
- [aws-rag]: https://docs.aws.amazon.com/pdfs/prescriptive-guidance/latest/writing-best-practices-rag/writing-best-practices-rag.pdf ; [kapa]: https://docs.kapa.ai/improving/writing-best-practices, MEDIUM
- [latechunk]: https://arxiv.org/abs/2409.04701, HIGH
- [graphbench]: https://arxiv.org/abs/2506.05690 (When to use Graphs in RAG; ICLR 2026), HIGH
- [hipporag2]: https://arxiv.org/abs/2502.14802, HIGH
- https://arxiv.org/abs/2502.11371 (RAG vs GraphRAG systematic evaluation), MEDIUM
- [ragas]: https://pypi.org/pypi/ragas/json (0.4.3); [deepeval]: https://pypi.org/pypi/deepeval/json (4.2.8), HIGH
- ALCE citation metrics: https://arxiv.org/abs/2305.14627, HIGH (well-known; not re-fetched this session)
- [domsmoothie]: https://crates.io/api/v1/crates/dom_smoothie (0.18.2); https://crates.io/api/v1/crates/htmd (0.5.5), HIGH
- [react-md]: https://registry.npmjs.org/react-markdown/latest (10.1.0); [remark-gfm]: https://registry.npmjs.org/remark-gfm/latest (4.0.1), HIGH
- Local repo (HIGH): `Cargo.toml`/`Cargo.lock` versions; `crates/vox-search/{Cargo.toml,src/embeddings.rs,src/embedding_env.rs,src/ingest.rs,src/evaluation.rs}`; `crates/vox-db/src/{schema_extensions.rs,capabilities.rs}`; `crates/vox-populi/Cargo.toml` (`inference` feature, candle backends); `docs/src/architecture/layers.toml` (candle out of L0-L3); `docs-astro/package.json`; `crates/vox-gui/ui/package.json`; `docs/src/.well-known/llms.txt`

---
*Stack research for: Vox Knowledge Base (milestone v1.1, workstream `kb`)*
*Researched: 2026-10-03*
