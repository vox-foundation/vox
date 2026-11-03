# Architecture Research

**Domain:** Curated-knowledge retrieval and context injection (git markdown → index → model context → web fallback → inbox → promotion PR → reindex), integrated into the existing Vox workspace
**Milestone:** v1.1 Vox Knowledge Base (workstream `kb`)
**Researched:** 2026-10-03
**Confidence:** HIGH for the current-state map (read directly from source in this worktree). MEDIUM for the target design (it is a recommendation, checked against the layer and edge contracts). External prior-art citations are LOW per the classify-confidence seam. They are primary-source pages, but they are supporting context, not authority.

---

## 0. TL;DR for the roadmapper

1. **Most of the plumbing already exists. What's missing is a curated corpus and a single path through it.** `vox-db` already has `search_documents` + `search_document_chunks` + an FTS5 shadow + `embeddings` + `fuse_hybrid_results`. `vox-search` already has `ingest_markdown_tree`, hybrid execution, RRF, and the web tier. `vox-orchestrator-mcp` already has a chat preamble, a tool-calling agent loop with tool pinning, MCP resources (`skill://`), and a per-turn trace event the GUI renders. Build on these. Don't add a parallel stack.
2. **The 2026-06-17 VoxKB foundation plan was implemented, but not as a knowledge base.** It shipped a topic-tagged *notes table* (`knowledge_bases`/`kb_entries`/`kb_routing_rules`, `KbStore`, `KbRouter`, 12 `vox_kb_*` MCP tools, a MENS `KbSignals` stage). Three things make it a liability:
   - It auto-ingests **assistant output** as `accepted=1` (`signal_chat.rs`).
   - It searches with `LIKE '%<entire prompt>%'`, which bypasses `vox-search` and breaks the search SSOT's "do not fork retrieval" rule.
   - Two of its three signal adapters have zero callers.
   **Retire it and replace it**, don't extend it (Section 2, Section 9).
3. **Findings currently land in six different sinks.** None of them is an inbox with provenance. Collapse them to one new `kb_findings` table.
4. **The Tantivy "docs mirror" is dead.** `TantivyDocsIndex::rebuild` has no caller, so the read path queries an always-empty index. Qdrant is an optional sidecar. Neither belongs in the KB path. Use SQLite FTS5 + `embeddings` (Section 2).
5. **The reindex today is not incremental and corrupts the vector join.** `replace_search_document_chunks_with_refs` DELETEs and re-INSERTs AUTOINCREMENT chunk rows on every ingest, which orphans `embeddings.source_id`. Key everything by **chunk content hash** (the Cursor/Continue pattern). This is the first phase.
6. **OpenRouter already publishes `knowledge_cutoff`.** Vox's catalog parser (`crates/vox-orchestrator/src/catalog.rs`) just doesn't read it. The model cutoff and the context budget belong in `ModelCapabilities` beside `max_context`/`released_at`.
7. **Ownership by layer:**
   - L3: indexer in `vox-search`.
   - L3: schema in `vox-db`.
   - L3: MCP tools, injection, and inbox in `vox-orchestrator-mcp`.
   - L3: model metadata in `vox-orchestrator`.
   - L5: promotion-PR side effects in the CLI/GUI tier.
   This plan needs **zero new crate edges**, with one optional, user-authorized edge (`vox-search → vox-crypto`, Section 7).
8. **Suggested build order:** index foundation → retrieval/MCP surface (+ retire legacy) → chat injection + model metadata → web fallback + inbox → promotion PR + reindex loop. The GUI Library work runs in parallel once the MCP surface exists (Section 10).

---

## 1. Standard Architecture (prior art, distilled)

Comparable systems all share five stages: **ingest → index → retrieve → inject → feedback**. Where they differ is in what they key their cache on and who controls injection.

| System | Ingestion / change detection | Index | Retrieval / injection | Feedback loop |
|---|---|---|---|---|
| Continue.dev | `cacheKey` = **file content hash**; tags `(dir, branch, artifactId)`; refresh emits `compute / delete / addTag / removeTag` lists; SQLite catalog of last-indexed state | FTS5, chunk index, tree-sitter snippets, LanceDB vectors (four `CodebaseIndex` impls) | `nRetrieve` → rerank → `nFinal` | None (read-only index) |
| Cursor | Merkle tree of file hashes, sync only differing subtrees | Turbopuffer vectors + FTS | Agent-driven semantic search | Embeddings cached **by chunk content**; unchanged chunks hit the cache |
| LlamaIndex IngestionPipeline | docstore map `doc_id → document_hash`; same hash ⇒ skip, changed hash ⇒ re-process + upsert | Any vector store | Retriever + node postprocessors | Transformation results cached per `(node, transformation)` hash |
| Haystack | `DocumentWriter` with `DuplicatePolicy {NONE, SKIP, OVERWRITE, FAIL}` | Document store | Pipeline components | — |
| Sourcegraph Cody | Search index (no embeddings for enterprise) | Adapted BM25 + learned ranking signals | Retrieval stage then ranking stage; "first N snippets (N a function of snippet length)" packed into prompt | Dropped embeddings: egress, admin complexity, scale |
| Anthropic Contextual Retrieval | Prepend 50–100-token chunk context before embedding **and** BM25 | Contextual embeddings + contextual BM25, rank-fused | Top-20 chunks; reranking | −35% (ctx embeddings), −49% (+ctx BM25), −67% (+rerank) retrieval failures. Corpora under ~200K tokens can skip RAG entirely |
| MCP spec (2025-06-18) | — | — | **Resources = application-driven** (host decides inclusion; URI + templates + `lastModified`/`priority` annotations). **Tools = model-controlled**, human-in-the-loop SHOULD | Tools may return `resource_link`s and `structuredContent` |

**Implications for Vox (opinionated):**

- **Key on content hashes, not row IDs or mtimes.** That means file hash → skip unchanged docs, and chunk hash → skip re-embedding (Continue + Cursor + LlamaIndex). Vox's ~670 live docs (~9.8 MB, ≈2.4M tokens; measured with `find docs/src -name '*.md' -not -path '*/archive/*'`) are too large for whole-corpus stuffing (Anthropic's ~200K threshold) and far too small for a Merkle tree. A flat hash walk takes milliseconds. **Skip the Merkle tree.**
- **Lexical-first, vectors optional** (the Cody lesson). FTS5 BM25 over heading-bounded chunks is the baseline. Vectors and contextual prefixes are an evaluated upgrade, not a prerequisite.
- **Hybrid injection = resource-style auto preamble + model-controlled tools.** This mirrors the MCP resource/tool split, and Vox already ships the exact pattern for skills: a tier-1 catalog in the system prompt + the tier-2 `vox_skill_use` tool (`chat_tools/skill_catalog.rs`).

---

## 2. Current-State Map (what exists, half-built, dead, duplicated)

### 2.1 Storage surfaces touching "knowledge" (VoxDb, `crates/vox-db/src/schema/domains/knowledge.rs`)

| Table / store | Written by | Read by | Verdict |
|---|---|---|---|
| `search_documents` (+`content_hash`, `source_uri` UNIQUE) / `search_document_chunks` / `search_document_chunks_fts` (FTS5 shadow, `schema_extensions.rs`) | `vox_search::ingest_markdown_tree` (CLI `vox db mirror-search-corpus`, `vox scientia mirror-search-corpus`); `persist_text_document_chunk` (web hits mirrored as `web-ingest:*`, default ON, `execution.rs:618`); `VoxDb::ingest_research_document_async` (`research.rs`) | `VoxDb::query_search_document_chunks[_hybrid]` via `vox-search` DocumentChunks corpus | **KEEP: this is the KB index substrate.** Fix it: (a) not incremental (always DELETE+INSERT chunks), (b) AUTOINCREMENT chunk ids churn, which orphans embeddings, (c) curated docs, unvetted web hits, and research packets share one corpus with no class column, (d) vanished files are never pruned |
| `embeddings` (`source_type`, `source_id`, BLOB vector) | `EmbeddingService::embed_and_store`, research ingest (`model` recorded as `"research_ingest"`, not the real model) | `search_similar_embeddings` (brute force) | **KEEP, re-key** by chunk hash + model. `where-things-live.md` guardrail: move out of inline BLOB past ~10k rows |
| `knowledge_nodes` / `knowledge_edges` | GUI `ingest_text`/`ingest_url` (`vox-gui/src/commands/knowledge.rs`); `persist_research_findings_to_knowledgebase` (`research_turn.rs:507`); research ingest; `MemoryManager::sync_verified_research_findings` | `vox_knowledge_query`, GUI Memory surface, retrieval bundle `knowledge_lines` | **KEEP as the graph**, but **stop using it as a findings sink.** Findings move to `kb_findings` |
| `knowledge_bases` / `kb_entries` / `kb_routing_rules` (VoxKB) | `KbStore` via 12 `vox_kb_*` MCP tools; `signal_chat::ingest_chat_turn` (spawned after **every** chat turn, `message.rs:1274`) | `run_retrieval_bundle` `kb_lines` (LIKE on whole prompt, `ops_memory/kb.rs:350`); `@kb-name` mentions (`message.rs:869`); MENS `KbSignals` stage (`vox-ml-cli/.../mens/pipeline.rs:375`) | **RETIRE/REPLACE.** Auto-accepts model output (poisoning loop). Bypasses `vox-search`. `signal_research.rs` / `signal_scientia.rs` have **zero callers**. No GUI uses `vox_kb_*` |
| `scientia_research_fts` / research sessions | Deep research pipeline | `vox_research_search`, `/research search` intercept (`message.rs` ~line 680) | **KEEP separate.** This is the episodic research log, not curated knowledge. The inbox can cite it as provenance |
| `web_cache` | scraper | scraper | Keep (fetch cache only) |
| Tantivy docs mirror (`vox-search/src/lexical_tantivy.rs`, `tantivy-lexical` feature, `.vox/search/tantivy/docs`) | **Nobody.** `TantivyDocsIndex::rebuild` has zero callers | `tantivy_supplemental_lines` (`execution.rs:258`) opens an empty index | **DEAD.** Don't build on it. Delete in the consolidation phase |
| Qdrant sidecar (`vector_qdrant.rs`, `qdrant-vector` feature) | External | `qdrant_lines` when `VoxSearchQdrantUrl` is set | Optional, off by default. **Out of KB scope** (local-first, no sidecar) |
| `MEMORY.md` + `memories` | `MemoryManager::persist_fact` / research sync | Memory corpus | Keep. This is agent *memory*, distinct from the curated *library* (the SOTA research §1 split) |

### 2.2 Retrieval/injection paths in one chat turn (`vox-orchestrator-mcp/src/chat_tools/chat/message.rs`, 2533 lines)

1. `/research search` → `research_search` (scientia FTS) intercept.
2. `run_retrieval_bundle(AutoChatPreamble, limit=3)` → `vox_search::run_search_with_verification`. It emits memory / KG / chunks (+ tantivy + qdrant) / repo / RRF lines, and appends **`kb_lines` from `KbStore::search_entries` outside vox-search**.
3. Research intent → `research_turn::run_quick` / `run_deep` (web tier: SearXNG → DDG → Tavily), then `persist_research_findings_to_knowledgebase` → `knowledge_nodes`.
4. `@kb-name` mentions → `KbStore::list_entries` (the first 10 entries, unranked).
5. Agent loop (`agent_loop.rs`): tools come from `TOOL_REGISTRY` via `select_tools_for_turn` (cap 40, **alphabetical truncation, `pin_names` hoist**).

**Budgeting:** none for injected context. Each corpus is capped at `limit=3` lines, but KB/mention lines carry full content. Conversation history gets a fixed `DEFAULT_CONVERSATION_TOKEN_BUDGET = 4_000` with chars/4 estimation (`conversation.rs`), and it is **not model-aware**. An unused, ready-made budget helper exists: `vox_actor_runtime::retrieval::{ContextBudget, apply_context_budget, ProvenanceRecord}`.

**Observability:** `ResearchTrace::to_event()` is inserted as `events[0]` of the reply. Its `stages` include `"preamble"`, and the GUI renders it in `ResearchTracePanel.tsx`. Event kinds are contract-checked in `contracts/gui/turn-event-kinds.v1.json` from both sides. **This is the hook for the injected-context inspector.**

### 2.3 Findings sinks today (duplicated; none has review status + provenance)

`knowledge_nodes` (research_turn, GUI ingest) · `kb_entries` (chat auto-ingest) · `search_documents` `web-ingest:*` (every web hit) · `scientia_research_fts` · `MEMORY.md` research sync · `docs/src/architecture/*-2026.md` written **directly into the working tree** by GUI `publish_research_doc` (`vox-gui/src/commands/research.rs`; no lint, no PR).

### 2.4 Model metadata (`crates/vox-orchestrator/src/models/spec.rs`, `catalog.rs`)

`ModelCapabilities` has `max_context`, `released_at` (OpenRouter `created`), `intelligence_index`, `is_alias_target`. **No knowledge cutoff.** `OpenRouterModelData` parses `context_length`, `created`, `expiration_date`, `benchmarks`, `alias_target`, but not `knowledge_cutoff`. OpenRouter's `/api/v1/models` returns `knowledge_cutoff`: ISO-8601 `YYYY-MM-DD` or null.

### 2.5 GUI ↔ backend IPC patterns (`crates/vox-gui`)

- **Pattern A (preferred):** `voxTransport.callTool(name, args)` → Tauri `invoke_mcp_tool` → persistent orchestrator daemon `orch.tool_call` (`commands/mcp.rs`). The same code path agents use, the same HITL gates, and a single writer.
- **Pattern B (legacy/direct):** dedicated Tauri commands holding `GuiDbPool`. Examples: `list_knowledge_nodes`, `ingest_url`, `ingest_text` (`commands/knowledge.rs`); `vox_search_query`, which runs `vox-search` **in-process inside the GUI** (`commands/search.rs:205`); `publish_research_doc`. This forks state from the daemon.
- `readDocMarkdown(path)` (Tauri, `docs_index.rs`) reads docs for `DocReader`. `docs_index.rs` keeps its own frontmatter parser, a third copy alongside `vox-doc-pipeline` lint and the new indexer.

### 2.6 Constraints verified in-tree

- `vox-search` edges: actor-runtime, bounded-fs, config, db, http-client, secrets. **No `vox-crypto`.** It hashes with `blake3` directly (`ingest.rs`, `vox-db/research.rs`), which violates AGENTS §Cryptography. The `crypto_ban` detector doesn't flag blake3 (a detector gap).
- `vox-orchestrator-mcp` already depends on `vox-crypto`, `vox-search`, `vox-db`, `vox-orchestrator`, `vox-publisher`, `vox-scientia`, and `vox-research-shim`.
- `vox-forge` (`create_change_request { draft }`) is reachable **only** via `vox-cli-review` (L5) and `vox-publisher`, and the publisher only does releases/issues. `vox-cli` → `vox-cli-review`, `vox-git`, and `vox-doc-pipeline` all exist. `vox-gui` → `vox-cli` exists.
- New docs `category` values must be added to `VALID_CATEGORIES` (`crates/vox-doc-pipeline/src/pipeline/lint.rs`) **and** `contracts/documentation/docs-sidebar-section-order.v1.json`.
- No committed catalog/index file is allowed (`2026-09-06-research-index-obsolete.md`). The index must be DB-derived; frontmatter is the metadata SSOT.
- New MCP tools: edit `contracts/operations/catalog.v1.yaml`, then run `vox ci operations-sync --target mcp --write`. Never hand-edit `tool-registry.canonical.yaml`.

---

## 3. Target Architecture

### 3.1 System overview

```
 L5 SURFACES ─────────────────────────────────────────────────────────────────────────
 ┌──────────────────────── vox-gui (Tauri + React) ───────────────────────────┐ ┌─ vox-cli ──────────────┐
 │ Library surface: Browse · Search · Doc · Inbox · Controls                  │ │ vox kb reindex|status  │
 │ Chat: KbContextPanel (per-turn injected-context inspector)                 │ │ vox kb promote <id>    │
 │   └─ voxTransport.callTool('vox_kb_*') ─► invoke_mcp_tool ─► daemon        │ │   (worktree+lint+PR)   │
 │   └─ invoke('kb_open_promotion_pr') ─► vox-cli lib ─► vox-cli-review::kb_promote ─► vox-forge (draft PR) │
 └────────────────────────────────────────────────────────────────────────────┘ └────────────────────────┘
 L3 RUNTIME ──────────────────────────────────────────────────────────────────────────
 ┌──── vox-orchestrator-mcp ────────────────────────────────────────────────────────────┐
 │ kb_tools/ (rewritten): vox_kb_search · vox_kb_get · vox_kb_cite · vox_kb_propose     │
 │                        vox_kb_inbox_list · vox_kb_inbox_review · vox_kb_reindex      │
 │ kb:// resources (list/read/templates) beside skill://                                │
 │ chat/kb_context.rs: preamble (budgeted) + low-confidence → research_turn::run_quick  │
 │   → kb_findings (provenance) ; emits `kb_context` turn event                         │
 └──────────┬───────────────────────────────┬───────────────────────────────────────────┘
            │                               │
 ┌──── vox-search ──────────────┐   ┌──── vox-orchestrator ─────────────────────────────┐
 │ library/ indexer: walk →     │   │ models: ModelCapabilities.knowledge_cutoff        │
 │ frontmatter → heading chunks │   │ catalog.rs parses OpenRouter knowledge_cutoff     │
 │ → chunk hash → upsert/prune  │   │ (budget = f(max_context))                         │
 │ SearchCorpus::Library leg    │   └───────────────────────────────────────────────────┘
 │ (FTS5 BM25 [+vec] → RRF)     │
 └──────────┬───────────────────┘
 ┌──── vox-db ───────────────────────────────────────────────────────────────────────────┐
 │ search_documents(+corpus, +frontmatter_json, +git_sha) · search_document_chunks       │
 │ (+chunk_hash, +heading_path, +ctx_prefix) · _fts · embeddings(key=chunk_hash+model)   │
 │ kb_findings (inbox) · kb_index_state                                                  │
 └───────────────────────────────────────────────────────────────────────────────────────┘
 SOURCE OF TRUTH: git markdown — docs/src/** (excl. archive/) + docs/src/library/**
```

### 3.2 Component responsibilities (NEW vs MODIFIED vs RETIRED, explicit)

| Component | Crate (layer) | Status | Responsibility |
|---|---|---|---|
| Library indexer `vox_search::library::{index_library, IndexReport}` | `vox-search` (L3) | **NEW** (grows out of `ingest.rs`) | Walk the corpus roots and skip `archive/`. Parse frontmatter. Heading-bounded chunking with `heading_path`. File hash → skip unchanged. Chunk hash → keep/insert/delete chunk rows. Prune vanished docs. Embed only new `(chunk_hash, model)` pairs when vectors are enabled |
| `ingest_markdown_tree` | `vox-search` | **MODIFIED** → thin wrapper over the indexer (keeps the `vox db mirror-search-corpus` CLI working) | — |
| `SearchCorpus::Library` + planner/execution leg | `vox-db-types` (L0) enum, `vox-search` execution | **NEW variant**, **MODIFIED** planner | Query chunks with `corpus='library'` only. Returns `UnifiedHit`s carrying `chunk_hash`, `path`, `heading_path`, `score`, `git_sha` |
| Schema: corpus/frontmatter/git_sha on `search_documents`; `chunk_hash`/`heading_path`/`ctx_prefix`/`token_estimate` on chunks; `kb_findings`; `kb_index_state` | `vox-db` (L3) | **MODIFIED** + **NEW** tables (bump `BASELINE_VERSION` 94→95 + `contracts/db/baseline-version-policy.yaml` + explicit ALTERs in `VoxDb::migrate`) | Index substrate + inbox |
| Chunk upsert ops (`upsert_library_chunks(doc_id, &[ChunkRow])` diff by hash; `prune_documents_not_in(corpus, &[uri])`) | `vox-db` | **NEW** (replaces DELETE-all for library) | Stable chunk identity |
| `knowledge_cutoff: Option<String>` on `ModelCapabilities`; parse in `catalog.rs` | `vox-orchestrator` (L3) | **MODIFIED** | Model metadata SSOT |
| `kb_tools/` (search/get/cite/propose/inbox_list/inbox_review/reindex) | `vox-orchestrator-mcp` (L3) | **REWRITTEN** (old 12 CRUD handlers deleted) | Single KB surface for agents, external MCP clients, and GUI |
| `kb://` resources (`kb://doc/{path}`, `kb://chunk/{hash}`) | `vox-orchestrator-mcp/src/server.rs` + new `kb_resources.rs` | **MODIFIED** + **NEW** | Application-driven attachment for external MCP hosts |
| `chat/kb_context.rs` | `vox-orchestrator-mcp` | **NEW** module (do not grow `message.rs`, already 2533 lines) | Budgeted preamble, low-confidence decision, findings capture, `kb_context` event |
| `run_retrieval_bundle` | `vox-orchestrator-mcp/memory_tools/retrieval.rs` | **MODIFIED**: delete `kb_lines` + `parse_kb_mentions` | Back to pure vox-search |
| `persist_research_findings_to_knowledgebase` | `research_turn.rs` | **MODIFIED** → writes `kb_findings` (pending) instead of `knowledge_nodes` | One findings sink |
| `kb_promote` (worktree → write → doc-lint → commit → push → draft PR) | `vox-cli-review` (L5 lib) via `vox-cli` `vox kb promote` | **NEW** | All git/forge side effects at L5, behind an explicit human gesture |
| `kb_open_promotion_pr` Tauri command | `vox-gui` (L5) | **NEW** | GUI trigger for the above |
| Library surface + `KbContextPanel` | `vox-gui/ui` | **NEW** surfaces; **MODIFIED** `ChatTurnEventRow` | Browse/search/inbox/inspector/controls |
| `publish_research_doc` (writes into live tree) | `vox-gui/src/commands/research.rs` | **MODIFIED** → files a `kb_findings` draft, then routes through promotion PR | Stop unreviewed writes to `docs/src` |
| GUI `ingest_url` / `ingest_text` | `vox-gui/src/commands/knowledge.rs` | **MODIFIED** → `vox_kb_propose` (inbox) | Same sink as agents |
| `knowledge_bases` / `kb_entries` / `kb_routing_rules`, `KbStore`, `KbRouter`, `signal_*`, 12 `kb.*` catalog ops, `@kb-name` mentions | `vox-db`, `vox-orchestrator`, `vox-orchestrator-mcp`, catalog | **RETIRED** (tables quarantined via the existing `domains/quarantine.rs` mechanism after a row-count check) | — |
| MENS `KbSignals` stage | `vox-ml-cli` | **MODIFIED**: reads `kb_findings` (approved → SFT, rejected → DPO) | Keeps the training flywheel |
| `TantivyDocsIndex` + `tantivy_supplemental_lines` | `vox-search` | **RETIRED** (zero writers) | — |
| `docs/src/library/` + `category: "Library"` | docs + `vox-doc-pipeline` lint + sidebar contract | **NEW** | Curated-knowledge home |

---

## 4. Recommended Project Structure

```
crates/vox-search/src/
├── library/                  # NEW — curated corpus indexer (moved out of ingest.rs)
│   ├── mod.rs                # index_library(), IndexReport { added, changed, unchanged, pruned }
│   ├── frontmatter.rs        # minimal YAML frontmatter split + serde_yaml (dep already present)
│   └── chunk.rs              # heading-bounded chunker, heading_path, token_estimate
├── ingest.rs                 # MODIFIED — ingest_markdown_tree delegates to library::
└── execution.rs              # MODIFIED — SearchCorpus::Library leg; tantivy docs leg removed

crates/vox-db/src/
├── schema/domains/knowledge.rs   # MODIFIED — columns + kb_findings + kb_index_state
└── store/ops_memory/library.rs   # NEW — upsert_library_chunks, prune, findings CRUD

crates/vox-orchestrator/src/models/spec.rs   # MODIFIED — knowledge_cutoff
crates/vox-orchestrator/src/catalog.rs       # MODIFIED — parse knowledge_cutoff

crates/vox-orchestrator-mcp/src/
├── kb_tools/                 # REWRITTEN — search.rs, get.rs, cite.rs, propose.rs, inbox.rs, params.rs
├── kb_resources.rs           # NEW — kb:// list/read/templates (mirrors skills_resources.rs)
└── chat_tools/chat/kb_context.rs   # NEW — preamble + fallback + kb_context event

crates/vox-cli-review/src/kb_promote.rs     # NEW — worktree/lint/PR (forge+git edges already exist)
crates/vox-cli/src/commands/kb.rs           # NEW — `vox kb reindex|status|promote`

crates/vox-gui/src/commands/kb.rs           # NEW — kb_open_promotion_pr only (everything else = callTool)
crates/vox-gui/ui/src/components/surfaces/Library/   # NEW — LibraryView, InboxView, KbControls
crates/vox-gui/ui/src/components/surfaces/Chat/KbContextPanel.tsx   # NEW
crates/vox-gui/ui/e2e/library-*.spec.ts     # NEW — Playwright + review-bundle screenshots

docs/src/library/                           # NEW — curated knowledge (frontmatter-required)
contracts/operations/catalog.v1.yaml        # MODIFIED — kb.* ops replaced
contracts/gui/turn-event-kinds.v1.json      # MODIFIED — + kb_context
```

### Structure rationale

- **Indexer lives in `vox-search`.** It already owns `ingest.rs`, chunking, `EmbeddingService`, and the `vox-db` + `vox-actor-runtime` edges. The search SSOT §2 forbids forking retrieval into callers.
- **Tools and injection live in `vox-orchestrator-mcp`.** That's where the chat preamble, agent loop, tool registry, resources, and turn events already live.
- **Git/PR side effects live at L5.** L3 must never push branches: the HITL principle, plus `vox-forge` isn't an L3 edge from MCP.

---

## 5. Architectural Patterns

### Pattern 1: Content-hash-keyed incremental index (Continue/Cursor/LlamaIndex)

**What:** Doc identity = `source_uri` (`library:docs/src/<path>`). Doc version = file content hash. Chunk identity = **chunk content hash**, never the AUTOINCREMENT rowid. Embeddings and contextual prefixes are keyed `(chunk_hash, model_id)`, so moving a paragraph or re-chunking costs nothing.
**When to use:** Always, for every reindex. Full and incremental reindex are the same code path; "full" just ignores the file-hash shortcut.
**Trade-offs:** Needs a `chunk_hash` column + unique index on `(document_id, chunk_hash)`. Chunk *order* is a separate `chunk_index` column that may change freely.

```rust
// vox_search::library — sketch, not final API
for file in walk(roots).filter(not_archive) {
    let bytes = read_capped(&file)?;
    let h = content_hash(&bytes);                       // vox-crypto (see §7 edge note)
    if db.library_doc_hash(&uri(&file)).await? == Some(h.clone()) { report.unchanged += 1; continue; }
    let fm = frontmatter::parse(&bytes)?;               // title, description, category, status, tags
    let chunks = chunk::by_heading(&body, max_chars);   // Vec<ChunkRow{hash, heading_path, text}>
    db.upsert_library_chunks(doc_id, &chunks).await?;   // diff by hash: keep / insert / delete
}
db.prune_documents_not_in("library", &seen_uris).await?;
```

### Pattern 2: Two-tier injection (auto "resource-like" preamble + model-controlled tools)

**What:**
- **Tier 1** is injected on every turn under a hard budget. It contains (a) up to ~3 top Library chunks, with citations, that clear a score floor, and (b) a one-line note that the `vox_kb_search`/`vox_kb_get` tools exist.
- **Tier 2** is the model pulling more through tools.

This is the same shape as `skill_catalog.rs` (tier-1 catalog) + `vox_skill_use` (tier-2).
**When to use:** Every chat turn where the KB is enabled. External MCP clients get Tier 2 + `kb://` resources only. Their host decides auto-inclusion (MCP: resources are application-driven).
**Trade-offs:**
- The tools **must be pinned** in `TurnContext.pin_names`. Otherwise `select_tools_for_turn`'s alphabetical 40-cap can drop them.
- Budget derives from `ModelSpec.capabilities.max_context`. Suggested: `min(1_500 tokens, 2% of max_context)`, fed through the existing `vox_actor_runtime::retrieval::apply_context_budget`, which already returns `ProvenanceRecord`s for the inspector.

### Pattern 3: Single inbox with provenance, human promotion via draft PR

**What:** Every non-curated finding goes into `kb_findings` with `status ∈ {pending, approved, rejected, drafted, promoted, stale}`. Sources include web fallback, deep research, GUI ingest, and explicit `vox_kb_propose`. Provenance (`query`, `source_urls`, `retrieved_at`, `model_id`, `turn_id/session_id`, `engine`, `kb_confidence_at_time`) is stored as JSON.
- `vox_kb_propose` (L3) may LLM-draft the markdown through `vox_actor_runtime::llm`, with frontmatter + a Sources section, into `draft_markdown` + `target_path`.
- **Only a human gesture** (GUI button or `vox kb promote <id>`) runs the L5 side effects:
  1. `git worktree add` from `origin/main` into `.vox/cache/kb-promote/<id>`
  2. Write the file.
  3. `vox-doc-pipeline` lint.
  4. Commit and push `kb/promote-<slug>`.
  5. `vox_forge` `create_change_request { draft: true }` with the token from `vox-secrets`.
  6. Record `promoted_ref`; remove the worktree.
**When to use:** All promotion. Never write into the user's working tree. Both `publish_research_doc` and `vox-cli-review::submit_branch` do `git checkout -b` / atomic writes in the main checkout, which is unsafe in this multi-agent tree.
**Trade-offs:** Promotion is asynchronous. Knowledge reaches the Library only after merge + reindex. That's by design: merged git is the only curated SSOT. Pending findings stay retrievable as **low-trust, clearly labeled** inbox hits (`vox_kb_search include_pending=true`), but they are never auto-injected.

### Pattern 4: Model-aware freshness gate

**What:** `knowledge_cutoff` (OpenRouter) falls back to `released_at` minus a margin, flagged "estimated" in the inspector. The web-fallback decision combines:
- (a) Library confidence: top Library score below τ, or zero hits.
- (b) Recency: the query references things newer than the cutoff (explicit dates or versions), or the top hit's git `last_updated` is newer than the cutoff *and* the turn is intent-classified as factual.
- (c) User/session control toggle.

Fallback reuses `research_turn::run_quick`. **No new web path.**
**Trade-offs:** The heuristics are crude at first. Log the decision inputs into the `kb_context` event so the inspector and later eval can tune τ.

---

## 6. Data Flow

### 6.1 Index lifecycle

```
git change (merge / pull / checkout / manual edit)
   │  triggers: daemon start · `vox kb reindex` · GUI "Reindex" (vox_kb_reindex)
   │            · optional lefthook post-merge → installed `vox kb reindex --quiet`
   │              (hooks must not build; this is a runtime call, advisory, time-boxed)
   ▼
vox_search::library::index_library(roots=[docs/src, docs/src/library], skip=archive/)
   ├─ file hash == stored? → skip
   ├─ parse frontmatter (title, description, category, status, tags, sources)
   ├─ chunk by heading (heading_path, ~1–2k chars, no cross-heading merge)
   ├─ diff chunk hashes → INSERT new / DELETE gone / keep same (FTS5 triggers follow)
   ├─ [optional, evaluated phase] embed new (chunk_hash, model) only; ctx_prefix cache
   └─ prune docs whose files vanished
   ▼
kb_index_state { corpus, head_sha, indexed_at, files, chunks, embed_model, duration_ms }
```

- **Single writer:** the daemon (or the CLI when no daemon is running) holds a DB-level advisory lock row in `kb_index_state`. The GUI's in-process `vox_search_query` must stay read-only.
- **Summary cards:** frontmatter `description` *is* the card. It's required by docs governance, so it's free. LLM contextual prefixes (Anthropic, 50–100 tokens/chunk) are a later, evaluated phase. They run as a background job through `vox_actor_runtime::llm`, are cached by `(chunk_hash, model)`, and never run on the chat hot path.

### 6.2 Chat turn (query → injection → feedback)

```
user prompt ─► message.rs ─► chat/kb_context.rs
                  │  1. spec = resolved ModelSpec (max_context, knowledge_cutoff)
                  │  2. hits = vox_search Library leg (FTS5 BM25 [+vec] → RRF)
                  │  3. (sel, prov) = apply_context_budget(hits, budget(spec))
                  │  4. if low_confidence(hits) && fallback_allowed(spec, prompt):
                  │        block = research_turn::run_quick(...)   // existing web tier
                  │        kb_findings.insert(pending, provenance)  // replaces knowledge_nodes sink
                  │  5. context_parts.push("[LIBRARY]…" with [kb:path#heading] cites)
                  │  6. TurnContext.pin_names += vox_kb_search, vox_kb_get, vox_kb_cite
                  │  7. events.push(kb_context { selected: prov, budget, cutoff,
                  │                              confidence, fallback: {ran, reason} })
                  ▼
           agent_loop ─► model may call vox_kb_* (dispatch.rs) ─► more Library hits
                  ▼
           reply + events ─► GUI ChatTurnEventRow → KbContextPanel (inspector)
```

### 6.3 Promotion loop

```
kb_findings(pending) ─► GUI Inbox: approve/reject (vox_kb_inbox_review)
   approve ─► vox_kb_propose(draft) ─► draft_markdown + target_path (status=drafted)
   human "Open draft PR" ─► invoke('kb_open_promotion_pr') ─► vox-cli lib
        ─► vox_cli_review::kb_promote: worktree → write → doc-lint → commit → push → draft PR
        ─► kb_findings.promoted_ref = PR URL (status=promoted)
   human merges PR on GitHub ─► next reindex trigger ─► Library contains it
   rejected/approved rows ─► MENS KbSignals (DPO/SFT) — retargeted from kb_entries
```

### 6.4 GUI ↔ Tauri IPC contracts

| UI action | Transport | Backend | Contract owner |
|---|---|---|---|
| Browse library tree | `callTool('vox_kb_search', {query:'', facets:true, group_by:'category'})`, or a dedicated `vox_kb_list` if facets bloat search | MCP | `catalog.v1.yaml` + `input_schemas.rs` |
| Search | `callTool('vox_kb_search', {query, limit, include_pending})` → `{hits:[{chunk_hash, path, heading_path, title, snippet, score, git_sha, trust:'curated'\|'pending'}], confidence}` | MCP | same |
| Read doc | existing `readDocMarkdown(path)` Tauri | `docs_index.rs` | existing |
| Cite | `callTool('vox_kb_cite', {chunk_hash})` → `{path, anchor, git_sha, quote}` | MCP | same |
| Inbox list/review | `callTool('vox_kb_inbox_list', {status})` / `('vox_kb_inbox_review', {id, decision, note})` | MCP | same |
| Draft promotion | `callTool('vox_kb_propose', {finding_id \| text+sources})` | MCP (LLM via facade) | same |
| Open draft PR | `invoke('kb_open_promotion_pr', {finding_id})` → `{pr_url, branch}` | Tauri → vox-cli lib → vox-cli-review | `crates/vox-gui/src/commands/kb.rs` |
| Reindex / status | `callTool('vox_kb_reindex')`, `('vox_kb_status')` | MCP | same |
| Inspector | read `events[]` kind `kb_context` from `vox_chat_message` reply | existing chat path | `contracts/gui/turn-event-kinds.v1.json` |
| Controls (KB on/off, budget %, web fallback) | chat params on `vox_chat_message` (per-turn) + persisted pref via existing preferences commands | MCP / Tauri prefs | `ChatMessageParams` |

Playwright: `installTauriMock` stubs `invoke_mcp_tool` for each `vox_kb_*` tool. The `kb_context` fixture event comes from the contract example. Screenshots go to `crates/vox-gui/ui/review-bundle/latest/` (AGENTS §GUI Visual Verification).

### 6.5 MCP tool surface (replaces the 12 `kb.*` ops)

| Tool | Side effect | Pinned in chat? | `http_read_role_eligible` | Notes |
|---|---|---|---|---|
| `vox_kb_search` | none | yes | yes | Library corpus only by default; `include_pending` opt-in returns inbox hits labeled `trust:"pending"`. Returns `structuredContent` + `resource_link`s (`kb://chunk/<hash>`) |
| `vox_kb_get` | none | yes | yes | Full doc or chunk by path/hash, with an optional heading slice |
| `vox_kb_cite` | none (logs to turn trace) | yes | yes | Stable citation `{path, anchor, git_sha, quote}`. The inspector shows which cites the model used |
| `vox_kb_propose` | writes `kb_findings` | yes (writes DB only) | no | Never touches git |
| `vox_kb_inbox_list` / `vox_kb_inbox_review` | read / write DB | no (GUI/humans) | list yes | Excluded from model tool selection (`exclude_name_prefixes` or lane) |
| `vox_kb_reindex` / `vox_kb_status` | index write / none | no | status yes | — |
| Resources `kb://doc/{path}`, `kb://chunk/{hash}` | none | n/a | — | `resources/templates/list`. `lastModified` = git date |

**Naming collision to resolve in the roadmap:** `vox_kb_query` and `vox_kb_clip` exist today. Replace, don't alias. No GUI caller exists; external callers are unknown, so note it in the changelog.

---

## 7. Crate ownership vs `layers.toml` / crate-edges

| Need | Where | Edge status |
|---|---|---|
| Index + chunk + Library corpus | `vox-search` (L3) → `vox-db` (L3) | exists |
| Hashing via `vox-crypto` in the indexer | `vox-search` → `vox-crypto` (L1) | **NEW edge (downward) = USER-AUTHORIZED-ONLY.** Recommend proposing it: it also fixes the pre-existing direct-`blake3` policy violation in `ingest.rs`. **Fallback if declined:** keep the existing `blake3_hex` helper (status quo, flagged) or compute hashes in `vox-orchestrator-mcp`, which already has `vox-crypto`, and pass them in. Do NOT add a `crate-edges.allow` entry yourself |
| Model cutoff | `vox-orchestrator` | exists |
| Tools/injection/inbox/resources | `vox-orchestrator-mcp` → search/db/orchestrator/crypto | exists |
| LLM drafting / contextual prefixes | `vox_actor_runtime::llm` from mcp/search | exists |
| PR drafting | `vox-cli-review` (L5) → `vox-forge`, `vox-git`, `vox-db`, `vox-secrets` | exists. **Caveat:** the crate is CodeRabbit-flavored (CodeRabbit is retired). If maintainers plan to delete it, propose `vox-cli → vox-forge` (downward, user-authorized) and host `kb_promote` in `vox-cli` |
| Doc lint before PR | `vox-cli` → `vox-doc-pipeline` | exists |
| GUI trigger | `vox-gui` → `vox-cli` | exists |
| `SearchCorpus::Library` | `vox-db-types` (L0) | exists (enum edit only) |

---

## 8. Scaling Considerations

| Scale | Adjustments |
|---|---|
| Today: ~670 docs, ~2.4M tokens → ~5–7k chunks at ~1.5k chars | SQLite FTS5 + brute-force vectors are fine. A full hash walk takes milliseconds; a full re-chunk takes seconds. No sidecar |
| Library grows past ~10k chunks with vectors on | `embeddings` crosses the `where-things-live.md` 10k-row BLOB guardrail. Move vectors to file storage (path+hash pointer), or enable the existing optional Qdrant feature for the Library corpus only |
| Many docs change per merge (bulk refactors) | Reindex is bounded by the changed-chunk count, not the corpus size. Embedding/prefix generation runs as a throttled background job with resume via the `(chunk_hash, model)` cache |

**First bottleneck:** injected-context quality, not speed. Mitigate with heading-bounded chunks, a score floor, and a budget. Add an eval fixture (recall@k on ~30 Vox Q/A pairs) **before** tuning or adding vectors.

**Second bottleneck:** the embedding cost/egress (the Cody lesson). Keep vectors opt-in, through the model-agnostic facade.

---

## 9. Anti-Patterns (observed in-tree; do not repeat)

### AP1: Auto-ingesting model output as accepted knowledge
**What people do:** `signal_chat::ingest_chat_turn` routes assistant paragraphs into `kb_entries` with `accepted=1`, which then flow back into future preambles and MENS SFT.
**Why it's wrong:** A self-reinforcing hallucination loop (context poisoning, SOTA research §6c).
**Do this instead:** Model/web output goes only to `kb_findings(pending)`. Only merged git is curated.

### AP2: Retrieval forked outside `vox-search`
**What people do:** `KbStore::search_entries` (`LIKE '%prompt%'`) in `run_retrieval_bundle`; `@kb` mention dumps; GUI in-process `vox_search_query`.
**Why it's wrong:** It violates the search SSOT §2 "must not fork retrieval logic". It is unranked and unbudgeted, and it has a second writer.
**Do this instead:** One `SearchCorpus::Library` leg; every surface calls it through MCP.

### AP3: Row-id-keyed chunks with DELETE-all reindex
**What people do:** `replace_search_document_chunks_with_refs` deletes and re-inserts AUTOINCREMENT rows.
**Why it's wrong:** It orphans `embeddings.source_id` and breaks the SSOT's "stable `chunk_id` alignment is mandatory". It re-does unchanged work.
**Do this instead:** Pattern 1 (chunk content hash).

### AP4: Mixing curated and unvetted text in one corpus
**What people do:** Every web hit is mirrored into `search_documents` as `web-ingest:*` by default, sharing DocumentChunks with docs.
**Why it's wrong:** "Similarity ≠ truth" (SOTA §6d). Curated answers get diluted by scraped snippets.
**Do this instead:** Add a `corpus` column; the Library leg filters `corpus='library'`. Consider defaulting `VOX_SEARCH_PERSIST_WEB_HITS_DISABLED` on, or tagging those rows `corpus='web'`.

### AP5: Writing generated docs straight into the working tree
**What people do:** `publish_research_doc` `atomic_write`s into `docs/src/architecture/`; `submit_branch` runs `git checkout -b` in the main checkout.
**Why it's wrong:** It skips review, lint, and frontmatter governance. It clobbers other agents' trees.
**Do this instead:** Pattern 3 (isolated worktree + draft PR).

### AP6: Building on dead indexes
**What people do:** Point new features at the Tantivy docs mirror because the read path exists.
**Why it's wrong:** It has no writer; it returns nothing forever.
**Do this instead:** Use FTS5. Delete the Tantivy docs leg.

### AP7: Committing a generated catalog of the library
**Why it's wrong:** It is explicitly forbidden by the research-index retirement plan (it drifts, and it becomes a completeness gate).
**Do this instead:** Frontmatter + DB-derived index + sidebar.

---

## 10. Suggested Build Order (dependency-driven)

| # | Phase | Delivers | Depends on | Notes / research flags |
|---|---|---|---|---|
| 1 | **Index foundation** | Schema (corpus, frontmatter_json, git_sha, chunk_hash, heading_path, `kb_index_state`); `vox_search::library` incremental indexer; `vox kb reindex/status`; `docs/src/library/` + `Library` category (lint + sidebar contract) | — | Decide the `vox-search → vox-crypto` edge up front. Test-first: hash-skip, chunk-diff, prune, archive-exclusion |
| 2 | **Retrieval + MCP surface + consolidation** | `SearchCorpus::Library` leg; `vox_kb_search/get/cite/status/reindex`; `kb://` resources; catalog ops replaced + `operations-sync`; delete `kb_lines`, `@kb` mentions, `KbStore`/`KbRouter`/signal adapters, Tantivy docs leg; recall@k eval fixture | 1 | Row-count check before quarantining the `kb_*` tables. Retarget MENS `KbSignals` in phase 4 (temporarily point it at empty/disabled) |
| 3 | **Chat injection + model metadata** | `ModelCapabilities.knowledge_cutoff` (+ catalog parse); `chat/kb_context.rs` budgeted preamble via `apply_context_budget`; tool pinning; `kb_context` turn event + contract | 2 | Needs a test that the pinned tools survive the 40-cap; mutation-test the budget guard |
| 4 | **Low-confidence web fallback + inbox** | `kb_findings` table; fallback gate → `run_quick`; `persist_research_findings_to_knowledgebase` → `kb_findings`; GUI ingest_url/text → inbox; `vox_kb_propose`/`inbox_list`/`inbox_review`; MENS `KbSignals` retargeted | 3 (confidence signal), 1 (schema) | **Needs deeper research:** confidence threshold τ calibration; recency detection heuristics |
| 5 | **Promotion PR + reindex loop** | `vox_cli_review::kb_promote` (worktree/lint/draft PR), `vox kb promote`, `kb_open_promotion_pr`; `publish_research_doc` rerouted; optional post-merge hook | 4 | **Needs deeper research:** crate home (vox-cli-review retirement status); GitHub token SecretId reuse (`forge_token`) |
| 6 | **GUI Library surface** (parallel track) | 6a browse/search/doc (after 2) · 6b inspector `KbContextPanel` (after 3) · 6c inbox + controls + Open-PR (after 4/5); Playwright specs + review-bundle screenshots | 2 / 3 / 4–5 | Use `callTool` only, plus one Tauri command |
| 7 | *(Optional, eval-gated)* Contextual retrieval upgrade | Chunk `ctx_prefix` + embeddings keyed `(chunk_hash, model)`; RRF with vector leg; optional rerank | 2 + eval fixture | Ship only if recall@k improves. The cost is computable up front (≈$1 per M doc tokens with caching, per Anthropic) |

**Ordering rationale:** Every downstream feature reads chunk identity, so stable chunk identity (1) must precede vectors, citations, and the inspector. The MCP surface (2) is the single contract shared by chat, external MCP clients, and the GUI, so it gates them all. Consolidation goes in phase 2 so phases 3–6 never build on `KbStore`. The inbox (4) needs the confidence signal from injection (3). PRs (5) need the inbox. GUI subphases trail their backend contracts.

---

## 11. Integration Points

### External services

| Service | Integration | Notes |
|---|---|---|
| OpenRouter `/api/v1/models` | `vox-orchestrator/src/catalog.rs` add `knowledge_cutoff: Option<String>` | null for many models → fall back to `released_at`, labeled "estimated" |
| Web search (SearXNG/DDG/Tavily) | Reuse `research_turn::run_quick` → `vox-search` `WebSearchDispatcher` | Budget/circuit-breaker already exist; no new web client |
| Embeddings (optional) | `vox_search::EmbeddingService` → `vox_actor_runtime::llm::llm_embed` | Opt-in. Record the real model id (today research ingest writes `"research_ingest"`) |
| GitHub | `vox_forge::GitHubProvider::create_change_request{draft:true}`, token via `vox-secrets` | Human-gesture only; isolated worktree |

### Internal boundaries

| Boundary | Communication | Notes |
|---|---|---|
| GUI ↔ daemon | `invoke_mcp_tool` → `orch.tool_call` | One writer; HITL gates apply |
| chat ↔ KB | direct fn calls in `vox-orchestrator-mcp` (`kb_context.rs` → `vox_search`) | Must not re-enter via MCP dispatch for the preamble |
| model ↔ KB | agent loop tool calls (`dispatch.rs`) | Pinned tools |
| external MCP hosts ↔ KB | tools + `kb://` resources | Resources for host-driven inclusion |
| indexer ↔ docs pipeline | none at L3; promotion lint at L5 (`vox-cli` → `vox-doc-pipeline`) | Indexer tolerates bad frontmatter (indexes, flags `lint_ok=false`) |

---

## 12. Open Questions / Gaps

1. **`vox-search → vox-crypto` edge.** User decision. The fallback is in §7.
2. **Promotion-PR crate home.** `vox-cli-review` (zero new edges, CodeRabbit legacy) vs a proposed `vox-cli → vox-forge`.
3. **Row counts in the legacy `kb_entries`/`knowledge_bases` on real user DBs.** Check before quarantine. If non-zero, migrate rows into `kb_findings(pending)`.
4. **The `@name` mention syntax.** Drop it (recommended) or remap it to `@library:<path>`. Chat file `@path` mentions already exist in `chat/mentions.rs`.
5. **Should architecture/ADR docs be in the Library corpus, or only `docs/src/library/` + help categories?** Recommended: index all non-archive docs with `corpus='library'` and a `category` facet. Down-weight `status: research|roadmap` so aspirational pages don't read as shipped.
6. **Low-confidence threshold τ and recency heuristics.** Needs phase-4 research and the eval fixture.
7. **`docs_index.rs`** (GUI omnibar) keeps a separate frontmatter walk. Out of scope; later it could read from the Library index.

---

## Sources

**Codebase (HIGH, read directly in this worktree):**
- `crates/vox-db/src/schema/domains/knowledge.rs`
- `crates/vox-db/src/store/ops_memory/{search,kb}.rs`
- `crates/vox-db/src/research.rs`
- `crates/vox-search/src/{ingest,lexical_tantivy,execution,embeddings,policy}.rs`
- `crates/vox-orchestrator/src/knowledge_base/*`
- `crates/vox-orchestrator/src/models/spec.rs`
- `crates/vox-orchestrator/src/catalog.rs`
- `crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs`
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/{message,research_turn,conversation,agent_loop,turn_events}.rs`
- `crates/vox-orchestrator-mcp/src/kb_tools/*`
- `crates/vox-orchestrator-mcp/src/llm_bridge/tool_selection.rs`
- `crates/vox-orchestrator-mcp/src/server.rs`
- `crates/vox-actor-runtime/src/retrieval.rs`
- `crates/vox-gui/src/commands/{knowledge,search,mcp,research,docs_index}.rs`
- `crates/vox-gui/ui/src/{transport.ts, components/surfaces/Chat/ResearchTracePanel.tsx, components/surfaces/Memory/MemoryView.tsx}`
- `crates/vox-forge/src/{provider,github}.rs`
- `crates/vox-cli-review/src/coderabbit/github/reviews/submit_branch.rs`
- `contracts/ci/crate-edges.allow.v1.json`
- `docs/src/architecture/{layers.toml, where-things-live.md, search-retrieval-ssot-2026.md, kb-systems-sota-research-2026.md, deep-research-self-correction-and-knowledgebase-ssot-2026.md}`
- `docs/superpowers/plans/{2026-06-17-vox-kb-foundation, 2026-09-14-self-correcting-code-research-and-knowledgebase, 2026-09-06-research-index-obsolete}.md`
- `docs/src/contributors/documentation-governance.md`

**External (LOW per the classify-confidence seam; primary-source pages):**
- [Continue `core/indexing/README.md`](https://github.com/continuedev/continue/blob/main/core/indexing/README.md): cacheKey = content hash, compute/delete/addTag/removeTag, four index types
- [Continue codebase docs](https://docs.continue.dev/reference/deprecated-codebase): local SQLite index, nRetrieve/nFinal/rerank
- [Cursor: Secure codebase indexing](https://cursor.com/blog/secure-codebase-indexing): Merkle tree, embeddings cached by chunk content, simhash reuse
- [LlamaIndex IngestionPipeline](https://developers.llamaindex.ai/python/framework/module_guides/loading/ingestion_pipeline/): doc_id → hash docstore upserts, transformation cache
- [Haystack DocumentWriter](https://docs.haystack.deepset.ai/docs/documentwriter): DuplicatePolicy NONE/SKIP/OVERWRITE/FAIL
- [Sourcegraph: How Cody understands your codebase](https://sourcegraph.com/blog/how-cody-understands-your-codebase): retrieval+ranking, BM25-first, why embeddings were dropped
- [Anthropic: Contextual Retrieval](https://www.anthropic.com/news/contextual-retrieval): −35/−49/−67% failure reductions, top-20, ~$1.02/M tokens, <200K-token note
- [MCP spec — Resources (2025-06-18)](https://modelcontextprotocol.io/specification/2025-06-18/server/resources): application-driven, templates, annotations
- [MCP spec — Tools (2025-06-18)](https://modelcontextprotocol.io/specification/2025-06-18/server/tools): model-controlled, human-in-the-loop, resource_link, structuredContent
- [OpenRouter API — List models](https://openrouter.ai/docs/api/api-reference/models/get-models): `knowledge_cutoff` (ISO date or null)
- [Sourcegraph Cody keyword search docs](https://5.5.sourcegraph.com/cody/core-concepts/keyword-search) (corroborating)

---
*Architecture research for: Vox Knowledge Base (corpus → context) milestone v1.1*
*Researched: 2026-10-03*
