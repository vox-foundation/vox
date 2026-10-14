# Phase 2: Library Index and KB Tool Surface - Research

**Researched:** 2026-10-03
**Domain:** Incremental markdown indexing (content-hash chunk identity, heading-path chunking), lexical retrieval backend selection (Turso vs Tantivy), MCP tool + resource surface, CLI wiring, docs taxonomy
**Confidence:** HIGH for the current-state map and the lexical-leg probe (read from source and executed this session); MEDIUM for the target design (it is a recommendation checked against layer/edge contracts)

<user_constraints>
## User Constraints (from CONTEXT.md)

No CONTEXT.md exists for this phase. The user chose to plan Phases 1–3 first and run one discussion afterwards. The following are treated as **locked** because they are written into REQUIREMENTS.md / ROADMAP.md / STATE.md (user-approved milestone artifacts):

### Locked Decisions
- Git markdown is the only curated SSOT: `docs/src/**` (excluding `archive/`) plus `docs/src/library/`. The DB is a derived index. No committed catalog/index file.
- Reindex is incremental: file-hash skip, chunk-hash diff, prune deleted/renamed docs, embeddings survive reindex (IDX-01).
- Heading chunking with recorded `heading_path`; `#` in code fences and YAML frontmatter never create/pollute chunks; oversize sections split by **`text-splitter`** sized to the embedder tokenizer (IDX-02).
- One summary-card row per doc; one build manifest per index build (embed model id + revision, dim, chunker version, corpus commit) (IDX-03).
- `docs/src/archive/` is never indexed (IDX-04).
- `vox kb reindex` and `vox kb status` (IDX-05).
- `docs/src/library/` + a `Library` docs category and sidebar section (IDX-06).
- The lexical leg is chosen by a runtime probe; the writer-less `TantivyDocsIndex` mirror is removed either way (IDX-07).
- MCP tools `vox_kb_search`, `vox_kb_get`, `vox_kb_cite`, `vox_kb_status`, `vox_kb_reindex` and resources `kb://doc/{path}`, `kb://chunk/{hash}` (RET-01).
- No new crate edges. `vox-search → vox-crypto` is a user-authorization proposal only. Hashing must route through `vox-crypto` via the caller (`vox-orchestrator-mcp`).
- Chunk-level qrels are re-keyed exactly once in this phase, with the delta recorded; doc-level eval must not regress against the Phase 1 baseline.
- Local embeddings are Phase 5. Trust tiers, `kb_findings`, legacy `KbStore`/12 legacy tools retirement are Phase 3. Facet schema is Phase 4. Pinning tools past the 40-tool cap is Phase 7.

### Claude's Discretion (recommendations below; confirm at discussion)
- Lexical backend given the probe result (see "Lexical-leg decision tree").
- Schema shape, chunk-hash input, summary-card representation, sizer, CLI host, resource listing strategy.

### Deferred Ideas (OUT OF SCOPE)
- Trust tier columns/enforcement, `kb_findings`, legacy VoxKB retirement (Phase 3). Facet schema/taxonomy (Phase 4). Local embeddings, contextual prefix, rerank (Phase 5). Injection, pinning, `kb_context` (Phase 7). GUI (Phase 6+).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| IDX-01 | Incremental reindex; file-hash skip; chunk-hash diff; prune; no orphaned `embeddings.source_id` | §Schema design, Pattern 1 (diff-by-hash upsert), Pitfalls 3–5, Validation rows IDX-01-* |
| IDX-02 | Heading chunks with `heading_path`; fences/frontmatter safe; oversize split by text-splitter sized to embedder tokenizer | Pattern 2 (pulldown-cmark offset walk + MarkdownSplitter with custom `ChunkSizer`), Pitfall 6–7, Code Examples |
| IDX-03 | Summary-card row per doc; build manifest | §Schema design (`chunk_kind='card'`, `kb_index_state`) |
| IDX-04 | Archive never indexed | Walker rule "any path component == `archive`" (matches sidebar.mjs), fixture test |
| IDX-05 | `vox kb reindex` / `vox kb status` | CLI via daemon `orch.tool_call` (dei.rs precedent), catalog dual projection rows |
| IDX-06 | `docs/src/library/` + `Library` category + sidebar | `VALID_CATEGORIES`, `docs-sidebar-section-order.v1.json`, governance table (test-enforced) |
| IDX-07 | Lexical leg chosen by probe; `TantivyDocsIndex` removed | Probe executed this session; decision tree; removal map |
| RET-01 | Five `vox_kb_*` tools + `kb://` resources from any MCP host | Catalog → operations-sync → registry → dispatch/input_schemas chain; rmcp 1.7 `list_resource_templates`; daemon routing |
</phase_requirements>

## Project Constraints (from CLAUDE.md / AGENTS.md)

- **Test-first:** every new `pub fn` in `crates/*/src/**` needs a test in the same file before commit (`tdd-guard`, `skeleton/untested-pub-api`). Write failing tests first.
- **No new crate edges** without user authorization (`vox ci crate-edges`; `exceptions` are user-only). Dev-dependencies are **not** counted (see Standard Stack).
- **Cryptography:** application hashing must go through `vox-crypto`. Do not copy the existing direct `blake3` use in `vox-search/src/ingest.rs`.
- **VoxScript-first:** no new `.sh/.ps1/.py` glue. Any automation (qrels re-key) is Rust or a `.vox` script.
- **SSOT generators:** edit `contracts/operations/catalog.v1.yaml`, never hand-edit `tool-registry.canonical.yaml` / `command-registry.yaml` / `*.generated.md`. Do not open "regenerate after merge" commits; CI `ssot-autoregen` handles drift.
- **Docs:** new `docs/src/**` pages need frontmatter at creation (`title`, `description`, `category`); verify with `cargo run -p vox-doc-pipeline -- --lint-only --paths <file>`. No edits under `docs/src/archive/`.
- **where-things-live:** consult before adding code; add a row in the same PR if the concept is missing.
- **Formatting:** never `cargo fmt --all`; use `vox run scripts/fmt.vox`.
- **Local verification:** `cargo check -p <crate>` + `cargo nextest run -p <crate>` only; no workspace-wide clippy/tests locally. Route cargo through the broker (plain `cargo` on PATH).
- **No hardcoded CPU/job counts** (derive from machine). Applies to the Tantivy writer thread count.
- **Retired surfaces:** do not use `recall()`, `vox-dei`, etc. Model-agnostic LLM facade rule (no LLM calls are needed in this phase).
- **Security-relevant guards** (archive exclusion, path traversal on `kb://doc`) must be mutation-verified: break the guard, confirm the test fails, restore.
- **Frozen core:** `vox-db` is in `contracts/db/data-storage-policy.v1.yaml#frozen_core_crates`; "Tier A DDL changes in frozen crates need governance sign-off" (`data-storage-ssot-2026.md` §3 rule 5). The automated guard is not implemented (no `FROZEN-CORE-TOKEN` matches in crates/.github), but the policy stands → plan a human sign-off checkpoint for the schema change.

## Summary

Three findings reshape the plan. **First, the FTS5 path is dead on this engine, verified at runtime.** `turso` 0.6.1 does not implement `PRAGMA compile_options` and has no `fts5` virtual-table module, so every FTS5 table in `vox-db/src/schema_extensions.rs` is silently skipped and chunk search falls back to `body_text LIKE '%<whole query>%' ORDER BY created_at DESC`. That matches natural-language queries almost never. Turso's own FTS (`CREATE INDEX … USING fts`, Tantivy 0.26 inside) works, but only behind the `turso` `fts` feature (off in this workspace) **and** the `experimental_index_method(true)` builder flag on every open site. **Recommendation: a Tantivy 0.22 Library index owned by the indexer in `vox-search`** (already locked, pure Rust, no new deps, no frozen-core change, no experimental flag), rebuildable from DB chunk rows, with the dead `TantivyDocsIndex` mirror deleted.

**Second, chunk identity is the core of the phase.** Today `replace_search_document_chunks_with_refs` DELETEs and re-INSERTs AUTOINCREMENT rows (non-transactionally). The line chunker splits on `# ` inside code fences, puts frontmatter in chunk 1, ignores H3+, and the walker indexes `docs/src/archive/`. Replace it with a pulldown-cmark heading walker, then `text-splitter` `MarkdownSplitter` for oversize sections. Key chunks by a `vox-crypto` BLAKE3 hash injected by the caller. Diff by hash inside one transaction, and key `embeddings` by `chunk_hash` (`source_type='kb_chunk'`) so they survive reindex.

**Third, the tool surface has no name collision.** The 12 legacy tools are `vox_kb_{add_entry,add_rule,clip,create,delete,delete_entry,get_feed,list,list_entries,list_rules,query,review_entry}`. None equals `search/get/cite/status/reindex`, so Phase 2 can ship the five new tools beside them and Phase 3 deletes the legacy set. Tools must be `tier: core` to appear in external hosts (`VOX_MCP_TIERS` defaults to `core`). Tool calls from the stdio server are proxied to the daemon (`call_tool_via_daemon`). Resources are served in-process, so `kb://` reads should call the same `vox_kb_get` handler through the daemon.

**Primary recommendation:** build `vox_search::library::{chunk, index, lexical, search}` with an injected hasher. Add a baseline-95 schema (columns on `search_documents`/`search_document_chunks` + `kb_index_state`) and a Tantivy index under the VoxDb data dir. Expose five catalog-driven MCP tools (two with CLI projections) plus `kb://` resource templates. Delete `TantivyDocsIndex`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Corpus walk, frontmatter split, heading chunking, oversize split | API/Backend: `vox-search` (L3) | — | Search SSOT: retrieval/ingest logic must not fork into callers |
| Content hashing (BLAKE3) | `vox-crypto` (L1) via caller `vox-orchestrator-mcp` / `vox-cli` | `vox-search` receives `&dyn Fn(&[u8]) -> String` | No `vox-search → vox-crypto` edge allowed |
| Chunk/doc rows, diff upsert, prune, manifest | Database: `vox-db` (L3) | — | Only `vox-db` owns DDL and SQL |
| Lexical index (Tantivy, owning writer) | `vox-search` (L3), files under VoxDb data dir | — | Derived from DB rows; single writer = daemon |
| MCP tools + `kb://` resources | `vox-orchestrator-mcp` (L3) running inside `vox-orchestrator-d` | stdio `vox mcp` proxy | Daemon is the single writer; stdio routes tools via `orch.tool_call` |
| `vox kb reindex|status` | CLI `vox-cli` (L5) | daemon via `orch.tool_call` | Mirrors `commands/dei.rs` pattern (no private in-process orchestrator) |
| `Library` category/sidebar | Docs (`vox-doc-pipeline` lint + Astro sidebar contract) | — | Category vocab is test-enforced in lint |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tantivy` | 0.22.1 (locked; workspace pin `version = "0.22", default-features = false`; vox-search adds `features = ["mmap"]`) | Library lexical leg (BM25, field boosts, `en_stem`) | Already in lockfile, pure Rust, behind `vox-search/tantivy-lexical` [VERIFIED: Cargo.lock:14134-14135 `name = "tantivy"` / `version = "0.22.1"`; Cargo.toml:319] |
| `pulldown-cmark` | 0.13.4 (locked, workspace `pulldown-cmark = "0.13"`) | Stage-1 heading walk with byte offsets (`into_offset_iter`), YAML metadata block detection (`Options::ENABLE_YAML_STYLE_METADATA_BLOCKS`) | Already used by vox-publisher/vox-scientia; CommonMark-correct fence handling [VERIFIED: Cargo.lock:10866-10867; registry src lib.rs:691, parse.rs:1434] |
| `text-splitter` | 0.33.0 (`default-features = false, features = ["markdown"]`) | Stage-2 split of oversize heading sections (`MarkdownSplitter::chunk_indices`, custom `ChunkSizer`) | Locked by IDX-02; MIT; its `markdown` feature reuses pulldown-cmark ^0.13 [VERIFIED: crates.io API deps for 0.33.0; CITED: Context7 /benbrandt/text-splitter] |
| `serde_yaml` (= `serde_yaml_ng` 0.10) | existing | Parse frontmatter YAML into a lenient struct + raw JSON | Already a vox-search dep [VERIFIED: Cargo.toml:210; vox-search/Cargo.toml:30] |
| `vox-crypto` `secure_hash` + `hex_encode` | workspace | BLAKE3 content hash for docs/chunks | Crypto SSOT [VERIFIED: crates/vox-crypto/src/facades.rs:11 `pub fn secure_hash(data: &[u8]) -> [u8; 32]`, :46 `pub fn hex_encode(bytes: &[u8]) -> String`] |
| `vox-bounded-fs` `read_utf8_path_capped` + `normalize_text` | workspace | Capped read; BOM strip + CRLF→LF before hashing | Cross-platform hash stability [VERIFIED: crates/vox-bounded-fs/src/lib.rs:21, :42] |
| `rmcp` | 1.7.0 | `list_resource_templates`, `RawResourceTemplate`, `ResourceContents::TextResourceContents` | Already the server framework [VERIFIED: Cargo.lock:11843-11844; registry src handler/server.rs:227, model/resource.rs:44] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `walkdir` | existing | Corpus walk (does not follow symlinks by default) | Indexer walker |
| `turso` | 0.6.1 (`default-features = false, features = ["sync"]`) | Existing DB engine | No feature change recommended [VERIFIED: Cargo.toml:247] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Tantivy 0.22 Library index in vox-search | Turso native FTS (`USING fts`, `fts_match`/`fts_score`) | Works (probe below) but needs `turso` feature `fts` (pulls tantivy **0.26** + default zstd/lz4/stemmer → second tantivy in graph when `heavy-retrieval` is on), `experimental_index_method(true)` on all 7 `turso::Builder` call sites incl. `sync::Builder::new_remote`, a frozen-core (`vox-db`) DDL change, and an experimental on-disk index format. Benefit: lexical rows in the same transaction as chunk rows |
| Tantivy | SQLite FTS5 | **Not available**: the engine rejects it (probe) |
| `text-splitter` | Hand-rolled pulldown-cmark block packer (~60 lines) | Avoids new transitive deps (`icu_segmenter` 2.3, `icu_provider` 2.2→2.3 bump, `itertools` 0.15 (5th copy), `auto_enums`, `strum` 0.28 already present). IDX-02 names text-splitter, so keep it unless the user drops that wording (Open Question 3) |

**Installation (root `Cargo.toml` `[workspace.dependencies]`):**
```toml
text-splitter = { version = "0.33", default-features = false, features = ["markdown"] }
```
vox-search `Cargo.toml`: add `pulldown-cmark = { workspace = true }`, `text-splitter = { workspace = true }`; dev-dep `vox-crypto = { workspace = true }` (dev-deps are not crate-edge-gated, see below). Then run `cargo hakari generate` and check that `cargo hakari generate --diff` is clean. The diff check is enforced nightly (`.github/workflows/nightly.yml:551`) and listed in `docs/src/contributors/dependency-policy.md`.

**Crate-edge fact used above:** `vox ci crate-edges` counts only Normal/Build deps — [VERIFIED: crates/vox-cli-ci/src/crate_edges.rs:201 `if !matches!(dep.kind, DependencyKind::Normal | DependencyKind::Build) {` / `continue;`]. Allowed `vox-search` edges today: `vox-actor-runtime, vox-bounded-fs, vox-config, vox-db, vox-http-client, vox-secrets` (no `vox-crypto`) [VERIFIED: contracts/ci/crate-edges.allow.v1.json, extracted]. `vox-orchestrator-mcp → vox-crypto` and `vox-cli → vox-crypto` exist.

**Version verification:** text-splitter 0.33.0 is the max/newest stable (updated 2026-09-24, 2.30M downloads, repo github.com/benbrandt/text-splitter) [VERIFIED: crates.io API]. Not yet in Cargo.lock (no `text-splitter` entry).

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| text-splitter | crates.io | since 2023-05-05 (~3.4 yrs, 65 versions) | ~52k/wk (seam), 2.3M total | github.com/benbrandt/text-splitter | OK | Approved (only new crate) |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
New transitive crates text-splitter brings in (from crates.io 0.33.0 deps; not individually seam-checked): `icu_segmenter ^2.3`, `auto_enums ^0.8`, `itertools ^0.15`. `icu_provider` 2.2.0 is already locked and bumps to ^2.3. `ahash`/`strum 0.28` are already locked. Their licences are Unicode-3.0/MIT/Apache, which `deny.toml` allows (`"Unicode-3.0"` is in `allow`); `multiple-versions = "warn"`. The planner should run `cargo deny check licenses bans` after adding the crate.

## Lexical-Leg Decision Tree (IDX-07) — probe executed this session

Probe A (workspace engine, `turso =0.6.1`, `default-features = false`; scratch crate, `cargo run --offline`):
```
compile_options: ERR Parse error: Not a valid pragma name
fts5 vtab: ERR Parse error: no such module: fts5
USING fts (no experimental flag): ERR Parse error: index method is an experimental feature. Enable with --experimental-index-method flag
```
Probe B (`turso =0.6.1` with `features = ["fts"]`, `Builder::new_local(path).experimental_index_method(true)`):
```
USING fts (fts feature + experimental flag): OK
hit id=2 score=1.9616583585739136
after delete matches=0
reopen without flag: OK
```
Source corroboration: `turso_parser` 0.6.1 `PragmaName` has no `CompileOptions`; `translate/pragma.rs:228` `bail_parse_error!("Not a valid pragma name")`. `translate/schema.rs:1675` `bail_parse_error!("no such module: {}", …)`. `turso_core` `fts = ["dep:tantivy"]` with tantivy `0.26.0`. `turso` default features `["mimalloc", "fts"]`, but the workspace sets `default-features = false`.

**Consequences for today's code (HIGH):** `has_fts5_support()` swallows the error (`if let Ok(..)`) → `false` → `apply_*_fts_cutover` returns early → `query_search_document_chunks` always takes `query_search_document_chunks_like` (`WHERE c.body_text LIKE ?1 … ORDER BY c.created_at DESC`, with `?1 = %<whole query>%`) [VERIFIED: crates/vox-db/src/store/ops_memory/search.rs:247-265; schema_extensions.rs:28-38]. `knowledge_nodes_fts` and `scientia_research_fts` are equally dead (outside the scope of this phase; report to maintainers).

**Executable decision tree:**
1. If Phase 1 recorded the probe → use its recorded outcome (it must match Probe A; if Phase 1 reports FTS5 available, stop and re-probe — the engine changed).
2. FTS5 available? **No** (Probe A) → FTS5 is excluded.
3. Turso native FTS viable? **Yes, technically** (Probe B). Adopt it only if the user accepts all of: (a) `turso` feature `fts` workspace-wide (tantivy 0.26 compile in every vox-db consumer), (b) `experimental_index_method(true)` on every `turso::Builder` site (`vox-db/src/{pool.rs:40,48,55,69; store/open.rs:81,257; migration.rs:328}` + tests), (c) a frozen-core DDL change, (d) an experimental on-disk format. Otherwise:
4. **Default (recommended): Tantivy 0.22 Library index with an owning writer** in `vox-search`, behind `tantivy-lexical`. Delete the `TantivyDocsIndex` mirror in the same plan.
5. Either way: the indexer owns the writer (single-flight in the daemon); the index is derivable from DB chunk rows; `kb_index_state.lexical_backend` records the choice (`"tantivy-0.22"` / `"turso-fts"`), and `vox kb status` prints it.

**Feature wiring for option 4 (needs a decision, Open Question 2):** `tantivy-lexical` is off by default and only `vox-cli/heavy-retrieval` turns it on (release builds) [VERIFIED: crates/vox-search/Cargo.toml:10-11 `default = ["qdrant-vector", "tavily"]` / `tantivy-lexical = ["dep:tantivy"]`; crates/vox-cli/Cargo.toml:63]. PR CI runs `cargo nextest run $SHARD_P_ARGS --profile ci-gate --locked` with no `--features`, so tests gated on `tantivy-lexical` would never run in PR CI. The daemon binary `vox-orchestrator-d` does not enable it either. Recommended: add `tantivy-lexical` to vox-search `default` features. This is a deliberate exception to "heavy stacks behind features" because KB retrieval is now core; lean consumers already use `default-features = false`. The alternative is a `kb-library = ["vox-search/tantivy-lexical"]` default feature on `vox-orchestrator-mcp`, with KB integration tests placed in that crate.

## Architecture Patterns

### System Architecture Diagram

```
 git markdown (docs/src/**, skip any `archive` component; docs/src/library/**)
        │  trigger: vox kb reindex (CLI ─orch.tool_call─► daemon) │ MCP vox_kb_reindex │ (later: daemon start)
        ▼
 vox_kb_reindex handler (vox-orchestrator-mcp, in vox-orchestrator-d; single-flight lock)
   │  injects: hasher = |b| hex(vox_crypto::secure_hash(b)), corpus_commit = git rev-parse HEAD
   ▼
 vox_search::library::index_library(db, roots, deps)
   ├─ walk → read_capped → normalize_text → file_hash
   │     file_hash == stored? ──yes──► unchanged (no writes)
   │                         └─no──► split frontmatter (pulldown-cmark metadata block → serde_yaml)
   │                                  → heading walk (heading_path, byte ranges) → oversize? MarkdownSplitter
   │                                  → card chunk + section chunks, each chunk_hash = H(rel_path, heading_path, body)
   ├─ vox-db: upsert_library_document(...) + diff chunks by hash (txn): delete gone / insert new / renumber kept
   ├─ lexical: tantivy delete_term(gone hashes) + add_document(new) → one commit
   ├─ prune docs not seen (DB + tantivy), prune embeddings(source_type='kb_chunk') whose hash vanished
   └─ write kb_index_state (manifest, counts, errors_json) ──► IndexReport
        ▼
 Query path: vox_kb_search ─► library::search (tantivy BM25, lenient parse, field boosts)
             ─► hydrate rows from vox-db by chunk_hash ─► hits {chunk_hash, path, heading_path, anchor, snippet, score, uri}
 vox_kb_get / vox_kb_cite ─► vox-db lookups (+ file read for full doc)
 External host: stdio `vox mcp` ─tools─► daemon ; ─resources kb://…─► read_resource ─► call_tool_via_daemon("vox_kb_get")
```

### Recommended Project Structure
```
crates/vox-search/src/library/
├── mod.rs        # index_library(), IndexDeps{hasher, corpus_commit}, IndexReport, roots/exclusion
├── chunk.rs      # frontmatter split, heading walk, MarkdownSplitter oversize split, EstimatedTokens sizer, anchor slug
├── lexical.rs    # LibraryLexicalIndex (tantivy schema, writer ownership, rebuild_from_rows, search)  [cfg(feature="tantivy-lexical")]
└── search.rs     # library::search(): lexical → hydrate; deterministic tie-break
crates/vox-search/src/ingest.rs        # ingest_markdown_tree → thin wrapper over library diff path (removes the orphaning bug for mirror-search-corpus)
crates/vox-search/src/lexical_tantivy.rs  # DELETED
crates/vox-db/src/schema/domains/knowledge.rs  # new columns in CREATE TABLE + kb_index_state
crates/vox-db/src/store/open.rs        # ensure_kb_library_columns() (table-driven, like ensure_latency_columns)
crates/vox-db/src/store/ops_memory/library.rs  # upsert_library_document, diff_library_chunks, prune_library_documents_not_in, kb_index_state get/put, prune_orphan_kb_embeddings
crates/vox-orchestrator-mcp/src/kb_tools/library.rs  # 5 handlers (+ params in kb_tools/params.rs)
crates/vox-orchestrator-mcp/src/kb_resources.rs      # kb:// list/templates/read (mirrors skills_resources.rs)
crates/vox-cli/src/commands/kb.rs      # reindex|status subcommands (Phase 1 creates `vox kb eval` — co-locate)
docs/src/library/index.md              # seed page, category: "Library"
```

### Pattern 1: Diff-by-hash chunk upsert (stable identity)
**What:** A doc is identified by `source_uri = "library:" + repo-relative path`. Doc version = `file_hash` (BLAKE3 of normalized text, stored in the existing `content_hash` column). Chunk identity = `chunk_hash`, never the rowid. On change, inside one `VoxDb::transaction`:
1. `SELECT id, chunk_hash FROM search_document_chunks WHERE document_id=?`
2. DELETE rows whose hash is not in the new set (and collect those hashes for tantivy/embedding prune)
3. Kept rows are not rewritten. Renumber `chunk_index` in two passes (temporary negative indices) to avoid tripping the existing `UNIQUE(document_id, chunk_index)`
4. INSERT new hashes
**Embeddings:** library vectors use `source_type = 'kb_chunk'`, `source_id = chunk_hash`, so they survive reindex. After the diff, delete `embeddings` rows with `source_type='kb_chunk'` whose `source_id` no longer appears in `search_document_chunks.chunk_hash`.
**Why:** the current path DELETEs all rows and re-INSERTs them, outside any transaction [VERIFIED: crates/vox-db/src/store/ops_memory/search.rs:99-136]. Embeddings key on rowid strings (`chunk_id.to_string()`, `vox-db/src/research.rs:153-154`).

### Pattern 2: Two-stage chunker
1. Normalize text (`normalize_text`), then parse with `Options::ENABLE_YAML_STYLE_METADATA_BLOCKS` (plus tables/footnotes as needed) via `into_offset_iter()`. The metadata block range gives the frontmatter. Body = bytes after it.
2. Maintain a heading stack (H1..H6). A section = bytes from one heading start to the next heading at any level. `heading_path` = `title › H2 › H3`, where title = frontmatter `title`, falling back to the first H1, then the file stem. Headings inside fenced code are not `Tag::Heading` events, so they never split.
3. Section estimated tokens > budget → `MarkdownSplitter::new(ChunkConfig::new(min..max).with_sizer(EstimatedTokens)).chunk_indices(section)`, with overlap 0 (Pitfall 8).
4. Prepend nothing to the stored body. Store `heading_path` separately (Phase 5 builds the contextual prefix from it).
5. Card chunk (`chunk_kind='card'`, `chunk_index=0`, `heading_path=''`): title + `description` + first body paragraph + `category`/`status` facets. Phase 4's page-one block replaces "first paragraph".

### Pattern 3: Tool contract via the operations catalog
Chain [VERIFIED: docs/src/reference/mcp-tool-registry-contract.md:19; crates/vox-mcp-registry/build.rs header "Emit `TOOL_REGISTRY` from `contracts/mcp/tool-registry.canonical.yaml`"]:
`contracts/operations/catalog.v1.yaml` (edit) → `vox ci operations-sync --target all --write` → `tool-registry.canonical.yaml` + `command-registry.yaml` → `vox-mcp-registry` build.rs → `TOOL_REGISTRY` → `every_registry_tool_has_static_dispatch` (dispatch.rs:2164) + `input_schemas.rs` arm per tool + `contracts/mcp/http-read-role-governance.yaml` (read-eligible list) → `vox ci operations-verify`.
Rows with both projections exist (e.g. `id: check` has `mcp: name: vox_check` and `cli: path: [check]`) [VERIFIED: contracts/operations/catalog.v1.yaml:4838-4866]. Recommended rows:

| id | mcp.name | lane | read-eligible | side_effect_class | cli.path |
|----|----------|------|---------------|-------------------|----------|
| `kb.search` | `vox_kb_search` | ai | true | null | — |
| `kb.get` | `vox_kb_get` | ai | true | null | — |
| `kb.cite` | `vox_kb_cite` | ai | true | null | — |
| `kb.status` | `vox_kb_status` | ai | true | null | `[kb, status]` |
| `kb.reindex` | `vox_kb_reindex` | ai | false | `writes_files` | `[kb, reindex]` |

All rows use `tier: core`. External hosts filter `tools/list` by `VOX_MCP_TIERS`, which defaults to `"core"` [VERIFIED: crates/vox-orchestrator-mcp/src/server.rs:212-213 `std::env::var("VOX_MCP_TIERS").unwrap_or_else(|_| "core".to_string());`]. Allowed lanes are `"app", "workflow", "ai", "interop", "data", "platform"` [VERIFIED: contracts/operations/catalog.v1.schema.json:107-110].

### Pattern 4: `kb://` resources
`list_resources` today returns static resources + `skill://` + workspace entries, and `read_resource` dispatches on prefix [VERIFIED: crates/vox-orchestrator-mcp/src/server.rs:259-347]. Add:
- `list_resource_templates` (rmcp default method, server.rs does not implement it today) → `kb://doc/{path}`, `kb://chunk/{hash}`.
- `list_resources` → one entry per indexed doc (~671). Use concrete URIs because template support in Claude Code is undocumented (Assumption A4).
- `read_resource` for `kb://` → `call_tool_via_daemon(&self.daemon, "vox_kb_get", {path|chunk_hash})` → return markdown text. This keeps one code path, and the stdio process needs no DB.

### Anti-Patterns to Avoid
- **`QueryParser::parse_query` on user text.** The existing mirror does this (`lexical_tantivy.rs:47`), and any `:`/quote/`+` makes the whole query fail → empty. Use `parse_query_lenient` (tantivy 0.22 `query_parser.rs:344`).
- **Tantivy index under the worktree** (`.vox/search/tantivy`, `VOX_SEARCH_TANTIVY_ROOT`). The DB is user-global (`<data_dir>/vox.db`, `vox-config/src/paths.rs:109`), so a per-worktree index silently diverges from DB rows. Put the index next to the DB (`<data_dir>/kb/library-lexical/`), or derive its path from the DB path.
- **Indexing roots from tool args.** `vox_kb_reindex` must not accept arbitrary paths (that is arbitrary-file ingestion). Roots are fixed: `<repo_root>/docs/src` + library subdir.
- **Building on `execute_search_plan`'s chunk leg** for KB search. It is the LIKE path. `vox_kb_search` calls `library::search` directly; wiring a `SearchCorpus::Library` planner leg can wait for Phase 3/7.

## Schema Design (vox-db, baseline bump)

Current DDL [VERIFIED: crates/vox-db/src/schema/domains/knowledge.rs:36-66], verbatim:
```sql
CREATE TABLE IF NOT EXISTS embeddings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_type TEXT,
    source_id TEXT NOT NULL,
    dim INTEGER NOT NULL,
    vector BLOB NOT NULL,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS search_documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_uri TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    mime_type TEXT NOT NULL DEFAULT '',
    content_hash TEXT NOT NULL DEFAULT '',
    ingested_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS search_document_chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL REFERENCES search_documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    body_text TEXT NOT NULL,
    embedding_ref TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(document_id, chunk_index)
);
```
`store_embedding` ignores its `_model` parameter (`ops_memory/embedding.rs:10`). No model column exists; adding one is Phase 5's job.

Version: `pub const BASELINE_VERSION: i64 = 94;` [VERIFIED: crates/vox-db/src/schema/manifest.rs:35]; `repository_baseline_integer: 94` [VERIFIED: contracts/db/baseline-version-policy.yaml]. Phase 2 bumps to **the next integer at plan time** (95 unless Phase 1 bumps first), updates `repository_baseline_digest_hex` to `vox_db::schema::schema_baseline_digest_hex()`, and appends a `// 95: …` comment line (`vox ci check-codex-ssot`).

Proposed additions (all in the CREATE TABLE text for fresh DBs, plus a table-driven `ensure_kb_library_columns()` run on every open, like `ensure_latency_columns` (`store/open.rs:226-252`, which exists because columns added under an already-stamped version never reach existing DBs)):

| Table | Column / object | Purpose | Later phase extends |
|-------|-----------------|---------|---------------------|
| search_documents | `corpus TEXT NOT NULL DEFAULT ''` (+ index) | `'library'` for KB rows; separates web/research rows | Phase 3 adds `trust_tier` (required) |
| search_documents | `rel_path TEXT` | repo-relative path for `kb://doc/{path}` | — |
| search_documents | `frontmatter_json TEXT` | lossless frontmatter | Phase 4 facet schema reads it |
| search_documents | `indexed_commit TEXT`, `index_error TEXT` | per-doc provenance + parse error | Phase 11 staleness |
| search_document_chunks | `chunk_hash TEXT` + `UNIQUE INDEX (document_id, chunk_hash)` + `INDEX (chunk_hash)` | stable identity | Phase 5 vectors keyed by hash |
| search_document_chunks | `chunk_kind TEXT NOT NULL DEFAULT 'section'` | `'card'` / `'section'` | Phase 7 "cards before chunks" |
| search_document_chunks | `heading_path TEXT NOT NULL DEFAULT ''`, `anchor TEXT`, `token_estimate INTEGER` | citations, budgeting | Phase 5 prefix, Phase 7 budget |
| new `kb_index_state` | `corpus TEXT PRIMARY KEY, corpus_root, corpus_commit, indexed_at, doc_count, chunk_count, card_count, chunker_version TEXT NOT NULL, lexical_backend TEXT NOT NULL, embed_model_id, embed_revision, embed_dim, errors_json TEXT NOT NULL DEFAULT '[]', duration_ms` | IDX-03 manifest + IDX-05 status | Phase 5 fills embed_*; Phase 12 health view |

Precedent for `ADD COLUMN … NOT NULL DEFAULT …` on this engine: `ALTER TABLE research_misguidance_events ADD COLUMN status TEXT NOT NULL DEFAULT 'open'` (store/open.rs:181). The new unique index is safe on existing rows because NULL `chunk_hash` values are distinct.

**Chunk hash input (recommended):** `H(rel_path ‖ 0x00 ‖ heading_path ‖ 0x00 ‖ normalized_body)`. Including `rel_path` makes `kb://chunk/{hash}` and citations globally unique. The cost is that a rename re-embeds (rare, cheap at about 6k chunks). Identical sections within one doc get a deterministic `#n` ordinal suffix appended to the hash input.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Markdown heading/fence/frontmatter detection | line scanning (`starts_with("# ")`, the current bug) | pulldown-cmark offset events + metadata block option | Fences, setext headings, indented code, `---` inside fences |
| Oversize section splitting | char-count slicer | `text_splitter::MarkdownSplitter` + custom `ChunkSizer` | Locked; never splits inside a fence; byte offsets |
| BM25 / stemming / field boosts | in-memory inverted index | tantivy 0.22 (`en_stem`, `set_field_boost`) | Ranking quality, determinism, lenient parsing |
| Hashing | direct `blake3` | `vox_crypto::secure_hash` via injected fn | Crypto policy |
| Path containment for `kb://doc` | string prefix checks | DB membership lookup by `rel_path` (+ `vox_config::paths::path_is_under` if reading disk) | Traversal, symlinks, archive exclusion |
| Single-writer enforcement | ad-hoc flags | tantivy's own writer lockfile + a `tokio::sync::Mutex` single-flight in `ServerState` | Cross-process + in-process |
| CLI → daemon calls | private in-process orchestrator | `OrchestratorDaemonEnsure` + `orch.tool_call` (copy `dei.rs::tool_call`) | Split-brain avoidance (T2.1) |

## Runtime State Inventory (removal of `TantivyDocsIndex` + schema migration)

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | (1) Empty Tantivy dirs `<VOX_SEARCH_TANTIVY_ROOT>/docs`. `TantivyDocsIndex::open` creates them via `create_dir_all` + `open_or_create` whenever the search ran with the feature and the root set (`lexical_tantivy.rs:26-31`). (2) `search_documents` rows with prefix `vox-docs:` from `vox db mirror-search-corpus` (duplicates of library docs under another URI). (3) `embeddings` rows `source_type='search_document_chunk'` keyed by rowid (research ingest) | (1) None (harmless orphan dirs; mention in changelog). (2) Decide: leave them (corpus `''`, not searched by `vox_kb_search`) or have `ingest_markdown_tree` reuse the library diff path. Recommended: leave, since Phase 3 owns corpus separation. (3) Untouched |
| Live service config | None. No external service holds KB names (verified: KB is local DB + files) | None |
| OS-registered state | A long-running `vox-orchestrator-d` from an older binary keeps serving the old tool table. The stdio `vox mcp` forwards `vox_kb_search` to it and gets "unknown tool" | Operator must restart the daemon after upgrade. Add this to the UAT step. Check whether the daemon-ensure handshake compares versions (not verified) |
| Secrets/env vars | `SecretId::VoxSearchTantivyRoot` (`canonical_env: "VOX_SEARCH_TANTIVY_ROOT"`, registry/missing.rs:1813-1814) feeds `SearchPolicy.tantivy_index_root` (policy.rs:127-128, 293-298) | Remove the policy field with the mirror, and either retire the SecretId (`vox ci secrets-parity`, `secret-env-guard`; deprecation per policy) or repurpose it as an override for the library index dir. Recommended: retire, deriving the dir from the DB location |
| Build artifacts | `vox-cli` feature `heavy-retrieval = ["vox-search/tantivy-lexical", "vox-search/web-scrape"]` and release builds enabling it (`release_build.rs:165-170`) | Keep the feature name; its meaning changes from "dead docs mirror" to "Library lexical index". Update comments in `vox-cli/Cargo.toml:58-63,194-196`, `vox-orchestrator/Cargo.toml:95-97` |

Removal map for `tantivy_doc_lines` (pub field `SearchExecution::tantivy_doc_lines`, `execution.rs:43`): `execution.rs:257-300, 542, 740, 764, 774, 814`; `a2a_contract.rs:61,108`; `bundle.rs:97,147`; `rrf.rs:23-25` + `semcov_wave21_tests.rs:69-72`; `vox-research-shim/.../web_gather.rs:59-61`; `vox-orchestrator/.../submit/goal.rs:48`; `vox-orchestrator-mcp/src/memory_tools/retrieval.rs:337`; `vox-search/src/lib.rs:7,56-57`; `policy.rs:127-128,293-298`. Docs mention: `docs/src/architecture/search-retrieval-ssot-2026.md:69` ("Docs mirror (optional) | Tantivy").

## Common Pitfalls

### Pitfall 1: Shipping on the FTS5 assumption
**What goes wrong:** the plan builds on `search_document_chunks_fts`; the lexical leg is LIKE-on-whole-query and eval numbers are near zero.
**How to avoid:** use the decision tree; `vox kb status` prints `lexical_backend`; add a test asserting `lexical_backend != "like"` for the library corpus.

### Pitfall 2: Feature-gated tests that never run
**What goes wrong:** library tests behind `tantivy-lexical` are skipped by PR CI (no `--features`). **Warning sign:** `cargo nextest run -p vox-search` passes with 0 library tests. **Avoid:** Open Question 2's resolution, plus a non-gated test that fails if `cfg!(feature = "tantivy-lexical")` is false in the crate that serves KB.

### Pitfall 3: `UNIQUE(document_id, chunk_index)` collisions during diff
Inserting a new section in the middle shifts indices. Renumber kept rows in two passes (negative temp) inside the transaction, or the insert fails half-way.

### Pitfall 4: Shared-connection transactions
`VoxDb::transaction` runs BEGIN/COMMIT on the single shared connection; "avoid spawning work that uses the same `VoxDb` concurrently inside `f`" [VERIFIED: crates/vox-db/src/facade/migrations.rs:94-113]. Keep one transaction per document, short and sequential. Never hold it across tantivy commits.

### Pitfall 5: DB ↔ Tantivy divergence
A crash between the DB commit and the tantivy commit leaves them out of sync. Mitigation: on every reindex/status, compare tantivy `num_docs` with the DB library chunk count (plus chunker_version). On mismatch, rebuild the tantivy index from DB rows (seconds at about 6–7k chunks). Tantivy BM25 doc-freq includes deleted docs until merge, so incremental and fresh scores differ slightly. Eval must rebuild fresh and tie-break by `chunk_hash`.

### Pitfall 6: Non-deterministic ranking ties
Multithreaded tantivy indexing assigns segments nondeterministically, so equal scores order differently across runs and break Phase 1's byte-identical eval. Sort by `(score desc, chunk_hash asc)` after rounding scores to ~1e-6. For eval builds, a single-thread writer is acceptable. Note the AGENTS "no hardcoded CPU counts" rule: use `index.writer(budget)`, which derives threads from `num_cpus`, in production [VERIFIED: tantivy 0.22.1 index.rs:611-620].

### Pitfall 7: Sizer coupling and the "re-key exactly once" promise
If Phase 2 sizes with an estimator and Phase 5 swaps in the real tokenizer, chunk boundaries (and hashes) change again, breaking "chunk qrels re-keyed exactly once". Fix the sizer permanently as a deterministic, model-independent estimator (e.g. `ceil(bytes/4)` with a 512 target) recorded in `chunker_version`. Phase 5 then only *asserts* that real-token length stays ≤ the embedder max (8192 for gte-modernbert), and never re-chunks. Open Question 4.

### Pitfall 8: Overlap makes edits cascade
`with_overlap(n>0)` copies neighbour text into sub-chunks, so one edit changes several hashes. Use overlap 0; the Phase 5 contextual prefix supplies context.

### Pitfall 9: Global DB, many worktrees
One user DB (`<data_dir>/vox.db`) serves every clone/worktree. Reindexing from worktree B prunes docs only in B's tree. Accept "library = last indexed tree" and record `corpus_root` + `corpus_commit`; status shows the root and staleness vs current HEAD. Open Question 6.

### Pitfall 10: Stale daemon after upgrade
See Runtime State Inventory. UAT must restart `vox-orchestrator-d`.

### Pitfall 11: LoC budget warnings
`vox-orchestrator-mcp` (`max_loc = 40_000`) and `vox-db` (`max_loc = 40_000`) already exceed their budgets by raw `wc -l` (69,658 and 46,893). Arch-check LoC is warn-level, with a delta-regression warn. Put the bulk of the logic in `vox-search` (no budget) and keep the DB/MCP additions thin.

### Pitfall 12: Category drift
`VALID_CATEGORIES` must match the sidebar JSON exactly, and nothing tests that pair. Only governance-doc ⊆ VALID is tested (`lint.rs:1020-1077`). Add a test that `VALID_CATEGORIES` minus `"archive"` equals `sections`.

### Pitfall 13: Anchor slugs that don't match the site
Astro builds heading IDs with github-slugger [CITED: docs.astro.build/en/guides/markdown-content]; an `experimental.headingIdCompat` flag exists. Compute `anchor` with github-slugger semantics (lowercase, strip punctuation, spaces→`-`, `-1` suffix for duplicates). Verify against a few real pages; this is load-bearing for Phase 8 citations.

## Code Examples

### Custom sizer + markdown splitter (text-splitter 0.33)
```rust
// Source: Context7 /benbrandt/text-splitter (ChunkSizer, ChunkConfig::with_sizer, MarkdownSplitter::chunk_indices)
use text_splitter::{ChunkConfig, ChunkSizer, MarkdownSplitter};

/// Deterministic, model-independent token estimate (never swapped later; see Pitfall 7).
pub struct EstimatedTokens;
impl ChunkSizer for EstimatedTokens {
    fn size(&self, chunk: &str) -> usize { chunk.len().div_ceil(4) }
}

fn split_oversize(section: &str) -> Vec<(usize, &str)> {
    let cfg = ChunkConfig::new(256..512).with_sizer(EstimatedTokens); // overlap 0 (default)
    MarkdownSplitter::new(cfg).chunk_indices(section).collect()
}
```

### Heading walk skeleton (pulldown-cmark 0.13)
```rust
// Source: pulldown-cmark 0.13.4 (Options::ENABLE_YAML_STYLE_METADATA_BLOCKS, Parser::into_offset_iter)
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
let opts = Options::ENABLE_YAML_STYLE_METADATA_BLOCKS | Options::ENABLE_TABLES;
for (ev, range) in Parser::new_ext(&text, opts).into_offset_iter() {
    match ev {
        Event::Start(Tag::MetadataBlock(_)) => { /* frontmatter = range; excluded from body */ }
        Event::Start(Tag::Heading { level, .. }) => { /* close previous section at range.start; push level */ }
        Event::End(TagEnd::Heading(_)) => { /* heading text collected → heading_path */ }
        _ => {}
    }
}
```

### Injected hasher (no new edge)
```rust
// vox-search (no vox-crypto dep):
pub struct IndexDeps<'a> { pub hasher: &'a (dyn Fn(&[u8]) -> String + Sync), pub corpus_commit: Option<String> }
// vox-orchestrator-mcp handler (edge exists):
let hasher = |b: &[u8]| vox_crypto::hex_encode(&vox_crypto::secure_hash(b));
```

### Tantivy owning writer (0.22)
```rust
// Source: tantivy 0.22.1 (Index::writer derives threads from num_cpus; IndexWriter::delete_term; QueryParser::parse_query_lenient/set_field_boost)
let mut w: IndexWriter = index.writer(50_000_000)?;
for h in gone { w.delete_term(Term::from_field_text(f_hash, h)); }
for c in new { w.add_document(doc!(f_hash => c.hash, f_path => c.path, f_title => c.title, f_heading => c.heading_path, f_body => c.body))?; }
w.commit()?;
let mut qp = QueryParser::for_index(&index, vec![f_title, f_heading, f_body]);
qp.set_field_boost(f_title, 2.0); qp.set_field_boost(f_heading, 1.5);
let (q, _errs) = qp.parse_query_lenient(user_query);
```
Schema: `chunk_hash`/`doc_path`/`kind` as `STRING | STORED`; `title`/`heading`/`body` as `TEXT` with the `en_stem` tokenizer (registered by default in 0.22, `tokenizer_manager.rs:70`).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| SQLite FTS5 shadow tables | Turso engine: no FTS5; native `USING fts` (Tantivy) behind feature + experimental flag | turso 0.6.x | All vox-db FTS5 code is dead on this engine |
| Rowid-keyed chunks, delete-all reindex | Content-hash chunk identity (Continue/Cursor/LlamaIndex) | — | Embeddings/citations survive reindex |
| Writer-less `TantivyDocsIndex` mirror | Indexer-owned Library index | this phase | Lexical results actually exist |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | A conservative `bytes/4` estimator at a 512 target never exceeds gte-modernbert's 8192-token max in real tokens | Pitfall 7 | Low: margin is 16×; Phase 5 asserts it |
| A2 | Phase 1 defines doc IDs as repo-relative paths and exposes a swappable retrieval seam | Validation / eval | Medium: the re-key plan changes shape |
| A3 | Rebuilding the tantivy index for about 6–7k chunks takes seconds | Pitfall 5 | Low |
| A4 | Claude Code reads arbitrary `kb://` URIs and may not use resource templates; concrete `resources/list` entries are needed for @-mention | Pattern 4 | Medium: affects listing strategy only |
| A5 | The daemon-ensure handshake does not detect a stale daemon binary | Runtime State | Low: UAT restart covers it |
| A6 | github-slugger semantics match Starlight heading IDs on Astro 6 (headingIdCompat state not checked) | Pitfall 13 | Medium for Phase 8 links |
| A7 | text-splitter's new transitive crates pass `cargo deny` | Package audit | Low |

## Open Questions (for the Phase 1–3 discussion)

1. **Lexical backend.** Recommended: Tantivy 0.22 owned by the indexer. Turso native FTS works (probe B) but needs `turso/fts` (tantivy 0.26) + an experimental flag on all builders + a frozen-core change.
2. **Where tantivy is turned on.** Recommended: add `tantivy-lexical` to vox-search `default`, so tests run in PR CI and the daemon has it, with lean consumers already on `default-features = false`. Alternative: a `vox-orchestrator-mcp` default feature, with tests in that crate.
3. **text-splitter weight.** It adds icu_segmenter/itertools 0.15/auto_enums. IDX-02 names it; keep it unless the user prefers a ~60-line pulldown-cmark block packer.
4. **Sizer.** Recommended: a permanent deterministic estimator, so the "re-key exactly once" promise holds. Requiring the real tokenizer in Phase 2 couples Phase 2 to Phase 5's model download.
5. **Chunk hash includes `rel_path`?** Recommended: yes (unique citations); re-embed on rename.
6. **Multi-worktree semantics** with one global DB: "library = last indexed tree" (recommended) vs per-repo corpus scoping.
7. **Embeddings in Phase 2:** none. The manifest records `embed_model_id = null` / "lexical-only". IDX-01's "embeddings survive" is satisfied structurally (`kb_chunk` keyed by hash) and tested with seeded vectors. Remote `EmbeddingService` must not be wired here (it would mix vector spaces before Phase 5).
8. **Legacy tools for one phase:** the new names don't collide. Should legacy `vox_kb_query`/`vox_kb_clip` descriptions be marked deprecated in Phase 2, or left for Phase 3?
9. **Frozen-core sign-off** for the vox-db DDL (policy, unenforced): who signs, and what goes in the PR body?
10. **`where-things-live.md` row:** AGENTS requires it in the same PR, but DOC-02 is Phase 11. Recommended: add the indexer row now; Phase 11 completes the set.
11. **Phase 1 qrels keying:** if Phase 1 keys chunk qrels as `doc_path#heading_path`, Phase 2's single re-key is a mechanical mapping to `chunk_hash`. Coordinate with the Phase 1 planner.
12. **Scope of indexed files:** include `*.generated.md` (4 files, e.g. CLI surface) and skip `index.mdx` (only `.md` is indexed) — confirm.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | all | ✓ | 1.98.1 (pinned) | — |
| cargo-nextest | tests | ✓ | 0.9.143 | `cargo test` |
| cargo-hakari | workspace-hack after new dep | ✓ | — | — |
| `vox` CLI | operations-sync, ci gates, fmt | ✓ | 0.6.0+build.6474 | `scripts/vox-dev.sh` |
| Node / pnpm | docs-astro sidebar build check | ✓ | v26.10.0 / 12.6.0 | lint-only check |
| docs-astro node_modules | `pnpm --dir docs-astro build` | ✗ (not installed in worktree) | — | `pnpm --dir docs-astro install` first, or rely on CI docs job |
| Claude Code CLI | RET-01 external-host UAT | ✓ (`/opt/homebrew/bin/claude`) | — | Cursor |
| crates.io offline cache | text-splitter | ✗ (not cached) | — | network fetch at `cargo update -p text-splitter` |

**Missing dependencies with no fallback:** none.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `#[test]`/`#[tokio::test]` under cargo-nextest 0.9.143 (`.config/nextest.toml`, profiles `default`/`ci`/`ci-gate`) |
| Config file | `.config/nextest.toml` |
| Quick run command | `cargo nextest run -p vox-search -E 'test(/library|kb_/)'` (per crate touched) |
| Full suite command | `cargo nextest run -p vox-db -p vox-search -p vox-orchestrator-mcp -p vox-doc-pipeline -p vox-cli` + `vox ci operations-verify` + `vox ci crate-edges` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| IDX-01a | Unchanged tree → 0 chunk inserts/deletes; rowids + hashes identical | integration | `cargo nextest run -p vox-search -E 'test(kb_reindex_unchanged)'` | ❌ W0 `crates/vox-search/tests/kb_library_index.rs` |
| IDX-01b | Edit one section → exactly 1 delete + 1 insert; other rows' id/hash unchanged | integration | same file, `test(kb_reindex_one_section)` | ❌ W0 |
| IDX-01c | Delete and rename doc → rows pruned from DB and tantivy | integration | `test(kb_reindex_prune)` | ❌ W0 |
| IDX-01d | No orphan `embeddings.source_id` (`kb_chunk`) after any reindex; seeded vector for a kept hash survives | integration | `test(kb_no_orphan_embeddings)` | ❌ W0 |
| IDX-01e | Existing v94 DB migrates (columns + unique index added, old rows intact) | integration | `cargo nextest run -p vox-db -E 'test(kb_library_migration)'` | ❌ W0 `crates/vox-db/tests/kb_library_store.rs` |
| IDX-02a | `#` inside a fence creates no chunk; frontmatter never in a body | unit + fixtures | `cargo nextest run -p vox-search -E 'test(chunk::)'` | ❌ W0 `crates/vox-search/tests/fixtures/kb/*.md` |
| IDX-02b | Every chunk has `heading_path`; H3+ respected; CRLF/BOM yields the same hashes | unit | same | ❌ W0 |
| IDX-02c | Oversize section splits; every sub-chunk ≤ budget by the sizer | unit | same | ❌ W0 |
| IDX-03 | One card per doc; `kb_index_state` manifest fields populated | integration | `test(kb_manifest_and_cards)` | ❌ W0 |
| IDX-04 | `archive/` (any depth) never indexed; mutation-verified | integration | `test(kb_archive_excluded)` | ❌ W0 |
| IDX-05 | `vox kb status` / `reindex` print report fields | integration (CLI → daemon) | `cargo nextest run -p vox-cli -E 'test(kb_cli)'` + manual | ❌ W0 |
| IDX-06 | `Library` accepted by lint; sections JSON == VALID_CATEGORIES−archive; library page lints | unit | `cargo nextest run -p vox-doc-pipeline` + `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/library/index.md` | partial (governance test exists) |
| IDX-07a | `lexical_backend` recorded and ≠ like; BM25 finds a known heading | integration | `test(kb_lexical_backend)` | ❌ W0 |
| IDX-07b | `TantivyDocsIndex`, `tantivy_doc_lines`, `lexical_tantivy` absent from source | static guard (`include_str!`, like `tests/vox_search_dispatch.rs`) | `cargo nextest run -p vox-search -E 'test(tantivy_mirror_removed)'` | ❌ W0 |
| RET-01a | 5 names in dispatch + input_schemas; catalog/registry parity | static + CI | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_tool_surface|every_registry_tool_has_static_dispatch/)'` + `vox ci operations-verify` | ❌ W0 `crates/vox-orchestrator-mcp/tests/kb_tool_surface.rs` |
| RET-01b | Handlers: search/get/cite/status/reindex on a temp corpus (`ServerState::new_test`) | integration | same file | ❌ W0 |
| RET-01c | Via real daemon (`call_tool_via_daemon`, existing harness in `stdio_daemon_routing_tests.rs`) | integration | `test(kb_via_daemon)` | ❌ W0 |
| RET-01d | `kb://` templates listed; `read_resource(kb://chunk/<hash>)` returns markdown; `kb://doc/../../etc/passwd` and archive paths rejected (mutation-verified) | unit | `test(kb_resources)` | ❌ W0 |
| RET-01e | From Claude Code: tools callable, resources readable | manual UAT (checkpoint:human-verify) | `claude mcp add vox -- vox mcp` → call each tool, read one doc + chunk | n/a |
| SC-4b | New page under `docs/src/library/` appears in the `Library` sidebar section and in `vox_kb_search` | integration + docs build | indexer test with library fixture; `pnpm --dir docs-astro build` (CI) | ❌ W0 |
| SC-5 | Doc-level `vox kb eval` ≥ Phase 1 baseline; chunk qrels re-keyed once, delta recorded | eval gate (Phase 1 harness) | `vox kb eval` (Phase 1 command) vs `contracts/kb/retrieval-baseline.v1.json` | depends on Phase 1 |

### Sampling Rate
- **Per task commit:** the quick command for the crate touched (`-p vox-db`, `-p vox-search`, `-p vox-orchestrator-mcp`)
- **Per wave merge:** full suite command above + `vox ci operations-verify` + `vox ci crate-edges` + `cargo hakari generate --diff`
- **Phase gate:** full suite green + `vox kb eval` doc-level non-regression + Claude Code UAT before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/vox-search/tests/fixtures/kb/` (fence-hash.md, frontmatter.md, oversize.md, crlf.md, archive/x.md, library/page.md)
- [ ] `crates/vox-search/tests/kb_library_index.rs` (needs a test hasher via dev-dep `vox-crypto`)
- [ ] `crates/vox-db/tests/kb_library_store.rs` (incl. seeded-v94 migration)
- [ ] `crates/vox-orchestrator-mcp/tests/kb_tool_surface.rs`
- [ ] sidebar/VALID_CATEGORIES parity test in `crates/vox-doc-pipeline/src/pipeline/lint.rs`
- [ ] Resolve Open Question 2 first so tantivy-backed tests actually run in CI

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | daemon token auth already exists (`OrchDaemonClient::with_token`) |
| V3 Session Management | no | — |
| V4 Access Control | yes | `vox_kb_reindex` `http_read_role_eligible: false`; roots fixed server-side; single-flight |
| V5 Input Validation | yes | serde-typed params with schemars schemas (`derived_tool_schema!`); clamp `limit` 1..=50; cap query length (~1 KiB); `parse_query_lenient`; hash param must be `[0-9a-f]{64}` |
| V6 Cryptography | yes | BLAKE3 via `vox-crypto` only (integrity/identity, not secrecy) |
| V12 Files | yes | `kb://doc` resolves by DB membership on `rel_path` (no FS path join from input); walker skips symlinks (walkdir default) and any `archive` component; capped reads |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via `kb://doc/{path}` / `vox_kb_get{path}` | Information disclosure | Lookup only indexed `rel_path`s; reject `..`, absolute, archive. Mutation-test the guard |
| Arbitrary-file ingestion via reindex args | Tampering/Disclosure | No root parameter; roots derived from `state.repository.root` |
| Reindex DoS (repeated calls) | DoS | Single-flight mutex; second call returns "already running" |
| Query-parser DoS / errors | DoS | Lenient parse, length cap, `limit` clamp |
| Curated docs carrying prompt-injection text | Tampering (stored injection) | Out of scope here (Phase 3 sanitizer/tiers). Note: Phase 2 returns curated text only (corpus = `library`) |
| Archive leak into the index | Policy breach | Component-level exclusion + fixture + mutation test |

## Recommended Plan / Wave Split

| Plan | Content | Wave | Depends on |
|------|---------|------|------------|
| 02-01 | vox-db schema (baseline bump, columns, `kb_index_state`, `ensure_kb_library_columns`), ops (`upsert_library_document`, `diff_library_chunks`, prune, orphan-embedding prune, state get/put), migration test. **checkpoint: frozen-core DDL sign-off** | 1 | — |
| 02-02 | `vox_search::library::chunk` (frontmatter, heading walk, text-splitter + `EstimatedTokens`, anchors, card builder) + fixtures; add deps + hakari | 1 | — |
| 02-03 | `docs/src/library/` seed page + `Library` in `VALID_CATEGORIES`, sidebar JSON, governance table + parity test | 1 | — |
| 02-04 | `library::index_library` + `library::lexical` (tantivy owning writer, rebuild-from-rows, consistency check) + `library::search`; feature wiring (OQ2); delete `TantivyDocsIndex`/`tantivy_doc_lines`/`VoxSearchTantivyRoot` plumbing; `ingest_markdown_tree` decision | 2 | 02-01, 02-02 |
| 02-05 | Catalog rows → `operations-sync --target all --write`; handlers in `kb_tools/library.rs`; `input_schemas.rs`; read-role governance; `kb_resources.rs` + `list_resource_templates`; daemon routing tests | 3 | 02-04 |
| 02-06 | `vox kb reindex|status` CLI (daemon `orch.tool_call`), co-located with Phase 1's `vox kb eval`; `search-retrieval-ssot-2026.md` + `where-things-live.md` updates | 3 | 02-05 (status/reindex tools) |
| 02-07 | Eval: point the Phase 1 harness at `library::search`; re-key chunk qrels once (Rust, in the eval crate); record the delta in `retrieval-baseline.v1.json`; doc-level non-regression. **checkpoint:human-verify** Claude Code UAT (restart daemon first) | 4 | 02-05, Phase 1 |

## Sources

### Primary (HIGH confidence)
- Executed runtime probes (scratch crates, turso 0.6.1 with and without `fts`): outputs quoted above
- Registry sources: `turso_core-0.6.1` (Cargo.toml features, `index_method/fts.rs`, `ext/mod.rs:186-200`, `translate/pragma.rs:226-228`, `translate/schema.rs:1674-1675`), `turso-0.6.1/src/lib.rs:145-212`, `turso_parser-0.6.1/src/ast.rs:1744+`, `tantivy-0.22.1` (features, `index.rs:611-620`, `query_parser.rs:288-344`, `index_writer.rs:664`), `pulldown-cmark-0.13.4`, `rmcp-1.7.0`
- Repo files read this session: `vox-db/src/{schema_extensions.rs, capabilities.rs, store/ops_memory/{search.rs,embedding.rs}, schema/domains/knowledge.rs, schema/manifest.rs, store/open.rs, facade/migrations.rs, research.rs}`; `vox-search/{Cargo.toml, src/ingest.rs, src/lexical_tantivy.rs, src/execution.rs, src/lib.rs, src/policy.rs}`; `vox-orchestrator-mcp/src/{server.rs, scope_guard.rs, llm_bridge/tool_selection.rs, graph_tools.rs, kb_tools/handlers.rs, daemon_route.rs}`; `vox-cli/src/commands/dei.rs`; `vox-cli-ci/src/crate_edges.rs`; `vox-crypto/src/facades.rs`; `vox-bounded-fs/src/lib.rs`; `vox-config/src/paths.rs`; `vox-doc-pipeline/src/pipeline/lint.rs`; `docs-astro/src/utils/sidebar.mjs`; `contracts/{operations/catalog.v1.yaml, operations/catalog.v1.schema.json, mcp/tool-registry.canonical.yaml, mcp/http-read-role-governance.yaml, documentation/docs-sidebar-section-order.v1.json, db/baseline-version-policy.yaml, db/data-storage-policy.v1.yaml, ci/crate-edges.allow.v1.json}`; `docs/src/contributors/{documentation-governance.md, dependency-policy.md}`; `docs/src/architecture/{search-retrieval-ssot-2026.md, data-storage-ssot-2026.md}`
- Context7 `/benbrandt/text-splitter`: ChunkSizer, ChunkConfig::with_sizer/with_overlap, MarkdownSplitter::chunk_indices
- crates.io API: text-splitter 0.33.0 metadata and dependency list; gsd package-legitimacy seam: OK

### Secondary (MEDIUM confidence)
- [Astro markdown content guide](https://docs.astro.build/en/guides/markdown-content/): heading IDs via github-slugger
- [Astro headingIdCompat flag](https://docs.astro.build/en/reference/experimental-flags/heading-id-compat/)
- [Claude Code MCP docs](https://code.claude.com/docs/en/mcp): `resources/list` discovery, `claude mcp add … -- <cmd>`, `.mcp.json`; templates/@-mention not documented

### Tertiary (LOW confidence)
- None relied upon

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH. Versions come from Cargo.lock/crates.io; text-splitter API from Context7.
- Lexical decision: HIGH. Runtime probe plus engine source.
- Architecture/schema: MEDIUM. Recommendation checked against edges/layers; needs discussion of OQ1–7.
- Pitfalls: HIGH for the code-derived ones (1–5, 9–12); MEDIUM for 6, 7 and 13.

**Research date:** 2026-10-03
**Valid until:** 2026-11-02 (turso 0.x and text-splitter move fast; re-probe if `turso` is bumped)
