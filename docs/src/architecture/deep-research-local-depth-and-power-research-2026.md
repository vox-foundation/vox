---
title: "Deep Research Local Depth, Structure, and Power Expansion (2026)"
description: "Comprehensive architectural research blueprint and actionable catalogue of dozens of high-value improvements to dramatically scale Vox Deep Research locally and overcome online web reconnaissance weaknesses."
category: "Architecture SSOTs"
status: "current"
training_eligible: true
training_rationale: "Defines the concrete architectural roadmap for scaling deep research depth, local compute, autonomous crawling, multi-language sandboxing, and epistemic graph reasoning."
---

# Deep Research Local Depth, Structure, and Power Expansion (2026)

This document establishes the architectural roadmap and comprehensive engineering blueprint for scaling **Vox Deep Research** locally, systematically eliminating current online web retrieval weaknesses, and establishing decisive superiority over cloud-bound proprietary systems (Google Gemini Deep Research and Anthropic Claude Deep Research).

---

## 1. Executive Summary & Problem Space

While Vox Deep Research has successfully established **empirical compiler verification, multi-wave contradiction resolution, and SQLite FTS5 persistence**, its current implementation faces two distinct challenges:

1. **Online Retrieval Weaknesses**:
   - **Shallow Snippet Trap**: Standard search APIs (Tavily, SerpApi, Brave) return 150-word excerpts. They truncate technical specs, missing crucial implementation edge cases, compiler constraints, and complete API examples.
   - **JavaScript / Single-Page Application Blindness**: Modern technical documentation (React, Docusaurus, VitePress, Next.js, GitBook) requires client-side execution. Simple HTTP GET requests return empty `<div id="root"></div>` shells or CSR bootstrap scripts.
   - **Bot-Mitigation & Rate Limits**: Cloudflare Turnstile, anti-bot WAFs, and strict API rate limits (HTTP 429/403) frequently starve web gathering waves during deep sweeps.
   - **Amnesic Network I/O**: Duplicate fetches of identical URLs occur across separate research sessions due to the lack of content-addressed caching.

2. **Local Compute & Architectural Underutilization**:
   - **Cloud Model Reliance**: Subquery generation, intermediate summarization, and preliminary claim extraction are frequently offloaded to external LLMs rather than utilizing zero-cost local silicon (Apple Silicon Metal / CUDA via MENS).
   - **Language Isolation**: Sandboxed code verification is currently limited to Rust (`rustc` / `cargo check`), leaving TypeScript, Python, SQL, and Vox source unverified.
   - **Flat Text Knowledgebase**: Research claims are indexed as flat FTS5 text rows rather than an interconnected epistemic knowledge graph with entity resolution and relational traversing.
   - **Passive / On-Demand Execution**: Research only triggers when explicitly prompted, rather than operating autonomically to monitor repositories and pre-validate breaking changes.

---

## 2. Track 1: Overcoming Online Retrieval Weaknesses Locally

### 1.1. Autonomous Depth-2/3 Recursive Link Traversal
- **Problem**: Search engines index landing pages, but the real technical truth lives 2 to 3 clicks deeper in sub-pages (e.g. `docs.rs/crate/latest/.../struct.Config.html#method.builder`).
- **Solution**: Implement an autonomous link-expansion engine within `vox-search::web_gather`. When a high-relevance domain (e.g. `docs.rs`, `github.com`, `kernel.org`, `python.org`) is identified:
  1. Extract candidate internal hyperlinks matching the path prefix.
  2. Prioritize links containing semantic keywords (`architecture`, `api`, `spec`, `internals`, `guide`, `changelog`).
  3. Recursively fetch child pages up to configurable depth (default depth = 2), applying readability extraction to discard redundant site headers, footers, and sidebars.

### 1.2. Headless Browser Extraction for SPA Documentation
- **Problem**: Raw HTTP GET returns un-rendered JS skeletons on modern documentation portals.
- **Solution**: Wire `vox-plugin-browser` as a dynamic fallback scraper. When `reqwest` fetches a page with low text density ($<200$ text characters or containing script loader tags):
  1. Hand off the URL to `vox-plugin-browser` via CDP.
  2. Wait for `networkidle2` or DOM ready.
  3. Extract both distilled markdown (via Readability.js) and the semantic Accessibility Tree (`ax_snapshot`), eliminating hydration issues completely.

### 1.3. Multi-Provider Search Pool with Automatic Failover
- **Problem**: Relying solely on Tavily or DuckDuckGo creates hard single-point-of-failure risks.
- **Solution**: Implement a tiered multi-search provider pool in `ProviderRegistry`:
  - **Tier 1 (Zero-Cost Local)**: Local SearXNG instance / local SearX gateway if configured.
  - **Tier 2 (High-Accuracy Direct APIs)**: Tavily, Brave Search API, Kagi Search API, or SerpApi based on available Clavis credentials.
  - **Tier 3 (Headless Web Scraper)**: Direct headless search scraper using Chromium CDP, extracting search result links directly without third-party API keys.
  - **Failover Logic**: Exponential backoff on HTTP 429/403 with automatic downstream provider escalation.

### 1.4. Anti-Scraping Bypass via Chromium CDP Accessibility Tree
- **Problem**: Bot detection heuristics (Cloudflare Turnstile, DataDome) flag automated HTTP headers and headless user-agents.
- **Solution**: When headless navigation encounters bot protection:
  1. Utilize the embedded Chrome DevTools Protocol (CDP) session with persistent profile cookies (`vox-gui-browser-support`).
  2. Extract the Accessibility Tree (`ax_snapshot`) directly from the browser process rather than querying the DOM via `document.body.innerHTML`.
  3. Because the Accessibility Tree reflects the native OS accessibility bridge, it bypasses DOM anti-scraping traps and honeypots while reducing token payload size by 90–99%.

### 1.5. Content-Addressed Web Cache (CAS with BLAKE3)
- **Problem**: Repeated deep research waves on related subjects fetch the same online URLs dozens of times, wasting network bandwidth and triggering rate limits.
- **Solution**: Implement `WebArtifactStore` in `vox-db`:
  1. Key all web pages by the `BLAKE3(normalized_url)`.
  2. Store raw HTML, extracted text, and headers with HTTP `ETag` and `Last-Modified`.
  3. Support conditional HTTP requests (`If-None-Match`, `If-Modified-Since`), returning cached content instantly on HTTP 304.

### 1.6. Local Ecosystem Offline Indexing (Offline Mirrors)
- **Problem**: Developers frequently research standard library APIs or well-known crates without requiring the public internet.
- **Solution**: Pre-index local documentation:
  - Standard library docs (`rustup doc`, MDN offline, Python stdlib).
  - Local workspace dependencies (`cargo doc --no-deps`).
  - Index directly into Tantivy with lexical symbol lookups, satisfying queries in $<10$ms completely offline.

---

## 3. Track 2: Local Compute, MENS Acceleration & Zero-Token Subagents

### 2.1. Asymmetric Division of Labor: Local MENS Subagents
- **Problem**: Calling external frontier models for mundane research tasks (tokenization, subquery expansion, claim extraction, syntax triage) is slow, expensive, and token-constrained.
- **Solution**: Route sub-tasks to local MENS (Qwen 2.5 Coder 3B/7B running on Apple Silicon Metal or CUDA):
  - **Subquery Generation**: Local MENS expands user queries into 4–6 lexical variations in $<40$ms.
  - **Claim-Evidence Triplet Extraction**: Local MENS extracts `(Subject, Predicate, Object, Confidence)` tuples in parallel chunks.
  - **Syntactic Code Triage**: Validates whether extracted snippets look like valid Rust/Python before launching the compiler sandbox.
  - **Zero Cost**: Thousands of queries and evaluations run locally with zero API tokens consumed.

### 2.2. Local Vector & Cross-Encoder Re-Ranking Pipeline
- **Problem**: BM25 lexical ranking often misses semantic synonyms, while cloud embeddings add network latency.
- **Solution**: Integrate native local embedding and cross-encoder re-ranking inside `vox-search`:
  1. **Dense Retrieval**: Local Candle-based embedding model (e.g. BGE-small or MiniLM-L6) running on Metal/CUDA.
  2. **RRF Fusion**: Reciprocal Rank Fusion ($k=60$) combining Tantivy lexical BM25 and dense vector hits.
  3. **Local Cross-Encoder**: Rerank the top 50 fused candidates with a local cross-encoder (e.g. FlashRank / BGE-Reranker-v2) down to the top 10 most informative evidence passages.

### 2.3. Parallel Worktree Subagent Dispatch
- **Problem**: Multi-wave exploration can take minutes if executed sequentially.
- **Solution**: Dispatch parallel subagents across independent query branches using ephemeral git worktrees (`using-git-worktrees`) or isolated Tokio task pools. Each subagent explores one facet of the research DAG and reports back verified findings to the orchestrator.

### 2.4. Autonomic Stability Budgeting
- **Problem**: Static wave counts either quit too early on complex topics or waste compute on simple factual questions.
- **Solution**: Dynamic compute scaling based on query entropy and stability convergence:
  - Simple factual questions converge in Wave 1 ($S \ge 0.90$) and terminate in $<2$ seconds.
  - Complex architectural questions with conflicting claims scale dynamically to Wave 4, spawning specialized adversarial subagents to break contradictions.

---

## 4. Track 3: Polyglot Empirical Sandboxing & Dynamic Verification

### 3.1. Polyglot Language Sandboxes
- **Problem**: Code verification is currently limited to Rust.
- **Solution**: Expand `vox-research-shim::research::domain::codegen` into a general polyglot sandbox:
  - **TypeScript / JavaScript**: Verify via `bun run --check` or `tsc --noEmit` with type-aware harnesses.
  - **Python**: Verify via `python3 -m py_compile` and sandboxed `mypy --ignore-missing-imports`.
  - **SQL**: Verify query syntax and schema validity using an ephemeral in-memory SQLite/VoxDB connection.
  - **Vox Source**: Verify via `vox check` compiler pipeline.

### 3.2. Micro-Benchmarking & Performance Claim Validation
- **Problem**: Claims regarding performance (e.g. "Library A is 3x faster than Library B") cannot be verified by static compilation alone.
- **Solution**: Integrate an empirical micro-benchmark runner. The engine scaffolds a lightweight test harness, executes $N=100$ iterations under strict timeout limits (e.g. 500ms max) in an isolated process, and measures CPU time and memory allocation to verify empirical speed claims.

### 3.3. Upstream Git Repository Cloning & Live Probe
- **Problem**: Documentation often lags behind the latest repository commits, or claims features that only exist in unreleased pull requests.
- **Solution**: When researching a specific GitHub project:
  1. Perform an ephemeral shallow clone (`git clone --depth 1 <url>`) into a sandboxed temp directory.
  2. Inspect the latest `Cargo.toml` / `package.json`, commit logs, and run existing test suites (`cargo test --test <probe>`).
  3. Ground research in the literal source code of the upstream repository rather than secondary web blogs.

### 3.4. Property-Based Fuzz Probing
- **Problem**: Code may compile but panic on edge cases (e.g. integer overflow, empty strings, null pointers).
- **Solution**: Automatically wrap candidate functions in property-based test harnesses (`proptest` in Rust or `hypothesis` in Python) to subject research code snippets to automated fuzzing before declaring them verified.

---

## 5. Track 4: Epistemic Knowledge Graph & Temporal Truth

### 4.1. Relational Epistemic Knowledge Graph
- **Problem**: SQLite FTS5 searches text tokens, but cannot trace multi-hop relationships (e.g. "What libraries depend on Hyper 1.0 and also support AWS-LC-RS?").
- **Solution**: Build an epistemic graph layer in `vox-db`:
  - **Nodes**: `Entity` (Crate, API, Author, Framework), `Claim`, `Source`, `Probe`.
  - **Edges**: `Supports`, `Contradicts`, `DependsOn`, `Deprecates`, `Extends`.
  - **Query Engine**: Recursive SQL Common Table Expressions (CTEs) or graph traversal queries enabling multi-hop associative recall.

### 4.2. Temporal Validity & Semantic Version Tagging
- **Problem**: Technical claims rot as software versions evolve (e.g. Tokio 0.2 syntax vs. Tokio 1.0).
- **Solution**: Tag every claim with explicit version constraints:
  ```json
  {
    "subject": "tokio::time::sleep",
    "predicate": "replaces",
    "object": "tokio::time::delay_for",
    "valid_since": "1.0.0",
    "verified_at": "2026-09-14T19:00:00Z"
  }
  ```
  When deep research encounters claims discussing older versions, it flags them as temporally obsolete.

### 4.3. Graph Spreading Activation
- **Problem**: Retrieval is bounded by the exact query keywords.
- **Solution**: When a query hits a graph node (e.g. "Tauri v2 IPC"), perform spreading activation to boost neighboring related nodes ("Wry", "Tao", "Custom Protocol", "Security Scopes"), pulling relevant contextual constraints into the research synthesis.

### 4.4. Federated Cross-Repository Knowledge Sharing
- **Problem**: Research conducted in one repository is unavailable when working in a different workspace.
- **Solution**: Federated memory architecture:
  - Local repository storage: `<repo>/.vox/db/` (project-specific findings).
  - Global user storage: `~/.vox/knowledge/` (ecosystem-wide libraries, SOTA papers, language patterns).
  - Queries automatically search local repo first, then fall back to the global federated knowledgebase.

---

## 6. Track 5: Multimodal Visual Evidence Ingestion

### 5.1. Architecture Diagram Extraction & AST Verification
- **Problem**: System architecture documentation relies heavily on Mermaid diagrams, SVG flows, and PNG schematics that text scrapers ignore.
- **Solution**:
  1. Extract Mermaid and PlantUML fenced blocks from markdown and HTML pages.
  2. Parse the diagram syntax into structured dependency graphs.
  3. Validate whether the documented component flow matches actual codebase module boundaries.

### 5.2. Visual Chart & Benchmark Digitization
- **Problem**: Performance benchmarks are often published as PNG line charts or bar graphs without raw CSV data.
- **Solution**: Route chart images through multimodal vision models (Gemini Flash or local vision encoders) using a specialized prompt to extract the visual data points into structured tabular tables, incorporating verified benchmark numbers into the evidence ledger.

---

## 7. Track 6: Continuous Autonomic Research & Background Mining

### 7.1. Daemonized Topic Watcher & Release Feeds
- **Problem**: Research is purely reactive; developers only find out about breaking changes or security vulnerabilities when building or querying.
- **Solution**: An autonomous background research worker inside `vox-orchestrator`:
  - Monitors configured RSS feeds, GitHub release webhooks, and security advisory databases (e.g. RustSec, CVE).
  - Autonomously conducts research on relevant updates, runs compiler probes against project dependencies, and alerts developers to breaking changes before they pull updates.

### 7.2. Continuous Regression Parity Auditing (`vox audit research-parity`)
- **Problem**: Over time, codebase refactoring can silently break architectural invariants established in past research SSOTs.
- **Solution**: A new audit subcommand: `vox audit research-parity`. It parses code probes from all published SSOTs in `docs/src/architecture/` and compiles/runs them against current HEAD, ensuring that documented architectural guarantees remain strictly green.

### 7.3. Cryptographic Nanopublication Publishing
- **Problem**: Verified research findings remain isolated to the developer's machine.
- **Solution**: Wire `vox-scientia`'s Nanopublication engine to sign verified research findings with the developer's Ed25519 key (`vox-crypto`), exporting machine-verifiable `.trig` linked-data assertions that can be shared across teams or published to decentralized scientific registries.

---

## 8. Track 7: GUI Vox Axis & Human-in-the-Loop Power Features

### 8.1. Interactive Research DAG Canvas Visualizer
- **Problem**: The current `ResearchView` lists sessions as static cards.
- **Solution**: Implement an interactive graph canvas (using Cytoscape / WebGL):
  - Visually renders the live multi-wave DAG in real time.
  - Displays research nodes, active contradiction edges (red), and empirically resolved edges (green).
  - Allows the user to click any node to view evidence spans, confidence scores, or raw compiler outputs.

### 8.2. Interactive Sandbox REPL
- **Problem**: When a compiler probe fails, the user cannot easily interact with or modify the code snippet in place.
- **Solution**: Add an "Open in Sandbox REPL" button in `ResearchView` and `DocReader`. Clicking it opens a live Monaco editor pre-filled with the probe harness and compiler diagnostics, letting the developer test modifications with live $<100$ms re-compilation feedback.

### 8.3. Direct Bridge from Research Gaps to Implementation Plans (`/writing-plans`)
- **Problem**: Moving from research findings to code changes requires manual planning.
- **Solution**: Add a "Generate Implementation Plan" button in `DocReader` and `ResearchView`. It automatically ingests the P0/P1/P2 gaps from the research SSOT and scaffolds a complete, task-decomposed implementation plan adhering to `writing-plans` guidelines into `docs/superpowers/plans/`.

---

## 9. Implementation Roadmap & Priority Phases

| Phase | Focus Areas | Key Deliverables | Expected Impact |
| :--- | :--- | :--- | :--- |
| **Phase 1: Deep Web Reconnaissance** | Links, Spas, and CAS Cache | Depth-2 link crawler, headless browser fallback, BLAKE3 CAS cache, SearXNG pool | Eliminates shallow snippet limits; unlocks JS-heavy docs; 80% reduction in external API calls |
| **Phase 2: Local Compute & MENS Cascade** | Zero-Token Research | MENS Qwen 2.5 Coder subagents, local BGE/FlashRank reranker, parallel worktree dispatch | Sub-50ms local research triage; zero API token costs for subquery & claim extraction |
| **Phase 3: Polyglot Empirical Sandboxing** | Multi-Language & Benchmarks | TypeScript/Python/SQL sandboxes, micro-benchmark runner, property-based fuzzing | Extends empirical overrule guarantees across all languages in the tech stack |
| **Phase 4: Epistemic Graph & Continuous Mining** | Graph Memory & Autonomic Daemon | Graph relations in VoxDB, temporal version tagging, `vox audit research-parity` | Transforms static document storage into a living, continuously verified knowledge engine |

---

## 10. Architectural Invariants

1. **Empirical Primacy**: No text-generated consensus may overrule a failed local compiler or test probe.
2. **Local Privacy First**: Sensitive code context must never be dispatched to external search providers; local MENS models must handle context sanitization before web dispatch.
3. **Zero-Token Retrieval**: Common or previously researched technical topics must resolve via local VoxDB FTS5 and graph memory without hitting external networks.
4. **Deterministic Reproducibility**: Every empirical claim published to documentation must be backed by a runnable, reproducible probe file.
