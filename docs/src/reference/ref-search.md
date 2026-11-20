---
title: "Search & Hybrid Retrieval Reference"
description: "Search voxlang.org via Pagefind and /llms.txt, navigate Vox Axis via the Cmd/Ctrl+K Omnibar, and query multi-repo code and docs with vox-search (BM25 + HNSW + Graphify Personalized PageRank)."
category: "Language Reference"
status: "current"
topic: "search-retrieval"
domain: "knowledge"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
schema_type: "TechArticle"
---

# Search & Hybrid Retrieval Reference

<!-- kb:summary -->
Authoritative guide to all three search planes in the Vox ecosystem: client-side Pagefind full-text search and `/llms.txt` agent manifests on `voxlang.org`, the federated `Cmd/Ctrl+K` Omnibar and Memory/VoxGraph surfaces inside Vox Axis (`vox-gui`), and the multi-repo `vox-search` hybrid engine uniting Tantivy BM25 lexical search, HNSW vectors, polyglot Graphify cross-edges, and HippoRAG Personalized PageRank.
<!-- /kb:summary -->

Vox separates fast static website search from deep repository-grounded retrieval so both human readers and MCP agents get exact, citation-backed results without context overload.

```vox-diagram
graph {
    direction: right
    node Query: "User / Agent Query + KbScopeMask"
    node BM25: "Tantivy 0.22 BM25 (Heading Chunks)"
    node Vec: "HNSW Semantic Vector Leg"
    node Graph: "Polyglot Graphify Cross-Edges"
    node PPR: "HippoRAG Personalized PageRank + Leiden"
    node RRF: "Reciprocal Rank Fusion -> Grounded Hits"
    Query -> BM25
    Query -> Vec
    Query -> Graph
    Graph -> PPR
    BM25 -> RRF
    Vec -> RRF
    PPR -> RRF
}
```

## 1. Website Search on `voxlang.org` (Pagefind + `/llms.txt`)

On the public documentation site, **Pagefind** indexes all built HTML routes at static build time — zero backend round-trips required:

- **Keyboard shortcuts:** Press `/` or `Cmd/Ctrl+K` (inside the Starlight header search bar) to open full-text search across guides, references, crate APIs, and `vox` CLI pages.
- **LLM & agent manifests:** Automated coding agents can fetch compact, structured documentation feeds directly from [`/llms.txt`](https://voxlang.org/llms.txt), `/llms-small.txt`, and `/llms-full.txt` (`status: research` / `roadmap` Internals and `archive/` pages are automatically excluded).

## 2. Vox Axis Workbench Search (`Cmd/Ctrl+K`, `Memory`, `VoxGraph`)

Inside the native Tauri 2 desktop workbench (`vox-gui`), three complementary entry points share one federated capability and knowledge index ([GUI Navigation](gui-navigation.md)):

- **Federated Omnibar (`Cmd/Ctrl+K`):** Prefix with `>` for commands, `@` for agents, `/` for docs and skills, or `#` for crate architecture (`#L3` filters layer-3 crates). Selecting any hit opens the target surface or routes directly into the non-modal **Doc Dock**.
- **Memory View (`memory`):** Filter across the **3-layer Federated KB** (`Vox Baseline`, `Global Library ~/.vox/library/`, and `Repo-Local` docs root (`docs/src/` on Vox)) while tuning `Top-K Depth`, viewing provenance pills (`Memory Shard`, `Code Chunk`, `Codex Canon`), and launching `Expand`, `Research`, or `Open`.
- **Search Index / `VoxGraph` (`vox-search`):** Inspect active shard health, `blake3`-incremental graph rebuild state, `TTL` / `LOD` dials, and jump directly into Memory via `Search Shard in Memory`.

## 3. CLI & MCP Hybrid Search (`vox-search` + `vox kb` + `vox repo`)

| Plane | CLI Commands | MCP Tools | Output Guarantees |
|---|---|---|---|
| **Federated KB Library** | `vox kb status`, `vox kb reindex`, `vox kb eval` | `vox_kb_search`, `vox_kb_get`, `vox_kb_cite` | Partitioned by `(repository_id, corpus_tier)` with zero cross-repo leak when scoped to `repo_local` |
| **Deep Research Artifacts** | `vox research search <query>`, `vox research probe` | `vox_research_seed_query` | Returns verified sessions, cited domains, and penalized sources |
| **Cross-Repo Text & Symbols** | `vox repo query text`, `vox repo query file`, `vox repo query history` | `vox_repo_query_text`, `vox_repo_query_file`, `vox_repo_query_history` | Respects `.voxignore` boundaries across cataloged `.vox/repositories.yaml` entries |

## Related Documentation

- **[Tutorial: Managing Knowledgebases, Self-Healing OpenWiki & the Doc Studio](../tutorials/tut-knowledgebase-and-openwiki.mdx)** — Full hands-on setup for federated indexing, Graphify PPR, and 5-layer OpenWiki generation.
- **[Tutorial: Deep Research, Knowledge Preflight & Scientia Publication](../tutorials/tut-deep-research-and-scientia.mdx)** — Pairing Live KB Prior-Art Preflight with multi-wave research steering.
- **[Decorators Reference](ref-decorators.md)** &amp; **[Language Syntax Reference](ref-syntax.md)** — Core grammar keywords and decorator contracts.
