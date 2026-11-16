# Stack Research

**Domain:** Unified Light ↔ Deep Research Continuum, Extreme-Data Streaming Ingestion, Calibrated Socrates/NLI/Novelty Verification, Continuous Direct Publication Flywheel & Deterministic Before/After Benchmark Diff Telemetry (Milestone v7.6)
**Researched:** 2026-11-14
**Confidence:** HIGH (verified line-by-line against `/Users/brbrainerd/dev/vox/Cargo.toml`, `docs/src/architecture/layers.toml`, `crates/vox-*/`, `crates/vox-gui/ui/package.json`, and 2025–2026 upstream API/crate specifications)

## Executive Thesis: Zero-External-Crate-Addition Architecture

**Milestone v7.6 requires ZERO new third-party Rust crates in `Cargo.toml` and ZERO new npm packages in `crates/vox-gui/ui/package.json`.** Every required primitive—pure-Rust MinHash-LSH (`vox-similarity`), pure-Rust neural inference (`candle-core` / `candle-transformers` / `safetensors` / `tokenizers`), model-agnostic LLM chat/streaming/embeddings (`vox_actor_runtime::llm`), lexical/hybrid search (`tantivy` + `turso` FTS5 + RRF in `vox-search`/`vox-db`), streaming chunkers (`text-splitter`, `tiktoken-rs`, `quick-xml`, `csv`, `std.frame` lazy Polars), inline text diffing (`similar` with `inline` feature), scholarly packaging (`nanopub`, `tar`, `flate2` pure-Rust backend, `askama`), rate limiting (`governor`), and virtualized/graph/radar GUI rendering (`@tanstack/react-virtual`, `@xyflow/react`, `recharts`)—is **already pinned, audited, and building cleanly in the workspace**.

Crucially, earlier August 2026 research (`docs/src/architecture/deep-research-trust-novelty-scoring-landscape-2026-08-01.md:50,60`) floated adding `fastembed`, `ort`, and C++ `hnswlib` bindings. **Do not adopt those crates**: `ort` (`onnxruntime-sys`) and C++ `hnswlib` introduce prebuilt C++ binary fetches or `cmake` + `libclang` (`bindgen`) build requirements that violate `AGENTS.md §Build-toolchain invariant`. Instead, v7.6 achieves higher reliability and 100% offline portability by combining fast deterministic Rust pre-filters (`aho-corasick`, `strsim`, `vox-similarity`), `vox_actor_runtime::llm` (`infer_with_retry`, `llm_embed`, `local_embed_text`), optional pure-Rust `candle-transformers` Safetensors cross-encoders/embedders, and `VoxDb`'s `scientia_embedding_cache`.

---

## Recommended Stack

### Core Technologies

| Technology | Version (Workspace Pin) | Purpose | Why Recommended |
|------------|-------------------------|---------|-----------------|
| **`vox_actor_runtime::llm`** | `0.6.0` (`crates/vox-actor-runtime/src/llm/`) | Model-agnostic chat, durable SSE/event streaming, embeddings & offline local feature embeddings | Normative workspace LLM facade (`infer_with_retry`, `llm_chat`, `llm_stream_events_activity`, `llm_embed`, `local_embed_text` 256-dim L2 projection at `embed.rs:28`). Enforced by `vox-code-audit` (`llm_provider_call.rs`). |
| **`vox-search` + `tantivy` + `turso`** | `vox-search 0.6.0`, `tantivy 0.22` (`mmap`), `turso 0.6` (`sync`) | Unified retrieval substrate (BM25 + FTS5 + vector embeddings + RRF fusion + CRAG web dispatch) | Already powers KB Library (`tantivy-lexical`), `search_document_chunks_fts` (`crates/vox-db/src/schema_extensions.rs:81`), `scientia_embedding_cache` (`ops_embedding_cache.rs:19`), and `TrustScorer` (`crates/vox-search/src/trust.rs:14`). |
| **`vox-orchestrator::socrates` + `vox-research-shim`** | `0.6.0` (`socrates.rs`, `crates/vox-research-shim/`) | `<=1`-hop Light ↔ `>1`-wave Deep Research continuum router, multi-wave gather/verify, `Shopping` & `CodeGen` engines | `OrientReport` + `ConfidencePolicy` (`socrates.rs:6-8,166`) already compute `evidence_gap`, `contradiction_hints`, and `SocratesResearchDecision` to escalate `<=1`-hop retrieval into multi-wave Deep Research (`waves: 1..=5`). |
| **`vox-similarity`** | `0.6.0` (`crates/vox-similarity/`, Layer 2) | Stage-1 near-duplicate detection: k-shingling, 64-bit SimHash, `blake3`-seeded MinHash & banded `LshIndex` | Already in tree (`signature.rs:27-95`, `index.rs:25-180`), pure Rust, zero async/network/C deps, Layer 2 (`layers.toml:218`) so any Layer 3 research/scientia crate can consume it downward without layer inversion. |
| **`candle-core` / `candle-nn` / `candle-transformers` + `safetensors` + `tokenizers`** | `0.10` (patched in `patches/candle-core-0.10.2`), `0.7`, `0.21` (`fancy-regex`) | Optional in-process neural NLI cross-encoder & dense scientific embeddings (`SPECTER2` / `BGE` / `DeBERTa-v3`) | 100% pure Rust (CPU + Metal + CUDA); `tokenizers` uses `fancy-regex` (`Cargo.toml:296`) avoiding C `onig`/`cmake`, and `hf-hub = "1"` (`rustls-tls`) fetches Safetensors cleanly without `ort` / `onnxruntime-sys` / `libclang`. |
| **`vox-scientia` + `vox-publisher`** | `0.6.0` (`crates/vox-scientia/`, `crates/vox-publisher/`, Layer 3) | Claim extraction, calibrated NLI/hybrid verification, IMRaD/LaTeX/arXiv `.tar.gz`, RO-Crate, Finding Pages, Nanopubs, Zenodo/OpenReview/Crossref submission | Houses `MiniCheckVerifier` (`minicheck.rs:18`), `AtomicNoveltyScorer` (`novelty.rs:62`), `render_arxiv_bundle_with_handoff` (`bundle.rs:80`), `nanopub::spec` (`spec.rs:74`), and scholarly REST adapters (`scholarly/mod.rs:227`). |
| **`vox-crypto` + `nanopub` (scoped exception)** | `vox-crypto 0.6.0`, `nanopub 0.2` (`Cargo.toml:433`) | Workspace cryptographic hashing/signing (`BLAKE3`, `XXH3-128`, `SHA3-256`, `ed25519`) + RDF/TriG Trusty-URI RSA nanopublications | Honors `AGENTS.md §Cryptography Policy`: all Vox-owned digests use `vox-crypto` (`secure_hash`, `hash_fast_hex`), while `nanopub 0.2` is strictly isolated in `vox-scientia::nanopub` with `secrecy::SecretString` + Clavis (`vox-secrets`). |
| **`vox-gui` (React 19 + Tauri 2 + `voxTransport`)** | `react ^19.2.8`, `@tauri-apps/api ^2.11.1`, `@xyflow/react ^12.11.6`, `recharts ^3.8.1`, `@tanstack/react-virtual ^3.14.10` | Live stage/wave streaming UI, Epistemic DAG canvas, 6-axis Worthiness Radar, virtualized extreme-data & inline diff views | All required UI packages are already installed in `crates/vox-gui/ui/package.json:26-45` and bridged through `safeInvoke` / `safeListen` in `crates/vox-gui/ui/src/transport.ts:65-180`. |

### Supporting Libraries (Already in `[workspace.dependencies]`)

| Library | Version | Purpose | When to Use in Milestone v7.6 |
|---------|---------|---------|-------------------------------|
| **`tokio-stream` + `async-stream` + `futures-util` + `bytes`** | `0.1` / `0.3` / `0.3` / `1` | Backpressure-aware async byte/chunk/event streams | Streaming extreme-data HTTP/file ingestion and live `research_session_events` / SSE / MCP progress frames without buffering whole corpora in RAM. |
| **`text-splitter` + `tiktoken-rs`** | `0.33` (`markdown`) / `0.12` | Semantic structure-aware Markdown/text splitting with exact token budgets | Chunking ingested web pages, PDFs/HTML transcripts, and repo files into bounded `search_document_chunks` rows before FTS5 + embedding indexing. |
| **`quick-xml` + `csv` + `memmap2` + `zstd` + `flate2`** | `0.41` / `1.3` / `0.9` / `0.13.3` / `1` (`rust_backend`) | Zero-DOM streaming XML/RSS/arXiv/Crossref Deposit parsing, streaming CSV/TSV, mmap & pure-Rust decompression | Extreme-data ingestion of multi-hundred-MB XML/JSONL/CSV/compressed dumps; `flate2` uses `rust_backend` (`Cargo.toml:359`) for deterministic `.tar.gz` arXiv bundles without C `zlib`. |
| **`std.frame` (`vox-python-bridge` Polars) + pure-Rust fallback** | `polars==2.1.0`, `pyarrow==26.0.1` (`py_frame_report_run.rs:44`) | Lazy out-of-core tabular scan/filter/group-by (`frame.scan_csv`) | Tabular extreme-data synthesis in `.vox` research scripts when `@uses(python, fs)` is active; fall back to streaming Rust `csv 1.3` + `rust_decimal` when running standalone/offline without Python worker. |
| **`aho-corasick` + `strsim` + `unicode-segmentation`** | `1` / `0.11` / `1.12` | Multi-pattern span matching, normalized Damerau-Levenshtein / Jaro-Winkler similarity & Unicode grapheme/word bounds | Post-hoc verbatim & fuzzy quote verification against retrieved source spans (`SpanChecker` upgrade) and fast lexical containment pre-filtering before LLM entailment. |
| **`rust_decimal` + `rand_distr` + `rayon`** | `1.36` / `0.4` / `1.10` | Exact decimal arithmetic, statistical distributions & data-parallel slice scans | `statcheck` / `GRIM` numeric-claim consistency diagnostics (`WorthinessSignalsV2` diagnostic gate), parallel cosine similarity over `scientia_embedding_cache`, and bootstrap/Wilson benchmark intervals. |
| **`governor`** | `0.10` (`Cargo.toml:315`) | GCRA in-process async rate limiter | Enforcing strict per-provider API quotas (`1 req/s` Semantic Scholar key pool, `10 req/s` OpenAlex/Crossref polite pools) across concurrent research waves. |
| **`similar`** | `2` (`features = ["inline"]` in `workspace-hack/Cargo.toml:160`) | Myers/Patience line, word, and grapheme inline diffs (`TextDiff`) | Deterministic Before/After inline diff telemetry for SSOT drafts, claim revisions, `CodeGen` repair iterations, and research eval regression reports. |
| **`serde_yaml` (`serde_yaml_ng`) + `pulldown-cmark` + `askama` + `tar`** | `0.10` / `0.13` / `0.12` / `0.4` | YAML frontmatter (`CITATION.cff`, KB facets), CommonMark AST verification, typed LaTeX/HTML templates, POSIX tar builder | Direct Publication flywheel: `docs/src/architecture/*-research-2026.md` SSOTs, venue LaTeX (`neurips_2025`, `icml2025`, `acmart`, `jmlr2e`), deterministic arXiv `.tar.gz`, and HTML Finding Pages. |
| **`axum` + `axum-extra` + `rmcp`** | `0.8` / `0.12.5` / `1.7` | Typed `/api/v2/scientia/*` & `/api/v2/research/*` HTTP/SSE routes + MCP progress/tool streaming | Replacing slow CLI subprocess calls in `PublicationsView.tsx` with typed daemon endpoints and unified streaming across CLI, Chat, MCP, and GUI. |
| **`opentelemetry` + `tracing-opentelemetry`** | `0.29` / `0.30` | Span/metric correlation (`UsageSnapshot` -> `scientia_cost_telemetry`) | Attributing exact token count, USD cost, TTFT, and wall-clock latency per research wave and publication stage (`extraction`, `novelty`, `critic`, `scholarly`). |

### Development & Verification Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| **`wiremock` (`0.6`) + `#[ctor]` Egress Guard** | Hermetic HTTP mocking for Crossref, OpenAlex, Semantic Scholar, Zenodo, OpenReview & MiniCheck | Pair every external REST test with `vox-search`'s `block_non_loopback_egress` pattern (`crates/vox-search/src/lib.rs:117-120`) so unit tests never touch live scholarly hosts. |
| **`insta` (`1`, `json`) + `proptest` (`1`)** | Golden snapshot testing for LaTeX/IMRaD/SSOT/DAG outputs & property testing for MinHash/quote-span invariants | Snapshot `ScaffoldInput -> main.tex`, `DocSynthesizer -> .md`, and `similar` diff JSON payloads; fuzz UTF-8 multibyte boundaries in quote/span verifiers. |
| **`criterion` (`0.5`) + `vox-research-shim::micro_benchmark`** | Deterministic Before/After micro- and pipeline-level benchmarking | Combine `criterion` for Rust hot paths (MinHash, chunking, cosine scan) with `BenchEvidence::pass_interval` (95% Wilson CI at `bench_evidence.rs:88`) for eval gates. |
| **`vox-doc-pipeline --lint-only` + `vox-arch-check` + `vox-code-audit`** | Automated governance gates | Run `vox-doc-pipeline` in-process (or as pre-commit gate) on every synthesized `docs/src/architecture/*.md` SSOT so Gap 1 frontmatter/`kb:summary` failures become impossible. |

---

## Installation & Workspace Wiring

No external `cargo add` or `pnpm add` commands are permitted or needed. Wire existing workspace dependencies into target crates strictly along `docs/src/architecture/layers.toml` downward edges (`L4/L3 -> L2 -> L1/L0`):

```toml
# 1. Existing workspace pins to reuse in crates/vox-scientia/Cargo.toml or crates/vox-research-shim/Cargo.toml
# (NOTE: adding any new normal first-party crate edge requires user approval for `vox ci crate-edges` per AGENTS.md!)
[dependencies]
aho-corasick         = { workspace = true } # Quote & multi-pattern span pre-filter
strsim               = { workspace = true } # Fuzzy quote verification
governor             = { workspace = true } # Scholarly REST rate-limiting (S2 1 RPS, OpenAlex 10 RPS)
rust_decimal         = { workspace = true } # Numeric claim / statcheck / GRIM precision
similar              = { workspace = true } # Deterministic Before/After inline diff telemetry
rayon                = { workspace = true } # Parallel cosine similarity over scientia_embedding_cache

# Optional feature-gated local neural NLI / SPECTER2 embeddings (100% pure Rust, zero C++/cmake/libclang):
candle-core          = { workspace = true, optional = true }
candle-nn            = { workspace = true, optional = true }
candle-transformers  = { workspace = true, optional = true }
safetensors          = { workspace = true, optional = true }
tokenizers           = { workspace = true, optional = true }
hf-hub               = { workspace = true, optional = true }
```

```bash
# Scoped verification commands (per .agents/rules/00-core.md — never `cargo test --workspace`)
timeout 900s cargo test -p vox-similarity -p vox-search -p vox-scientia -p vox-research-shim -p vox-publisher
timeout 300s cargo run -p vox-arch-check
timeout 300s cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/<generated-ssot>.md
```

---

## External REST API Contracts & 100% Offline Fallback Matrix

All outbound HTTP clients MUST build from `vox_http_client::client_builder()` (`reqwest 0.12` + `rustls-tls`), resolve credentials exclusively via `vox_secrets::resolve_secret(...)` (`crates/vox-secrets/src/spec/registry/scholarly.rs`), enforce per-host quotas via `governor`, cache responses in `VoxDb`, and degrade cleanly when offline.

| Service | Exact v7.6 Endpoint(s) | Clavis `SecretId` & Quota | v7.6 Signal Extracted | Offline / Failure Fallback |
|---------|------------------------|---------------------------|-----------------------|----------------------------|
| **Crossref REST API v1** (+ Retraction Watch) | `GET https://api.crossref.org/works/{doi}`<br>`GET https://api.crossref.org/works?query={q}&rows={n}&mailto={email}` | `VoxScientiaCrossrefMailto` (polite pool ~50 RPS) / `VoxCrossrefPlusApiKey` (`Crossref-Plus-API-Token`) | Production Retraction Watch & publisher retractions via `message.updated-by[]` + `message.update-to[]` (`type == "retraction"`, `source == "retraction-watch"`), DOI metadata (`trust.rs:56-84`). | **Ranking:** fail-open neutral `trust_score` + cached `VoxDb` DOI status.<br>**Publication Promotion:** fail-closed `TrustLookup::Unavailable` (`trust.rs:97`) unless operator overrides with offline attestation. |
| **OpenAlex REST API** | `GET https://api.openalex.org/works?search={q}&per-page={n}&mailto={email}`<br>`GET https://api.openalex.org/works/https://doi.org/{doi}` | `VoxOpenAlexEmail` (polite pool `10 req/s`, `100k/day`) | Top-level `is_retracted: bool` (daily Retraction Watch + PubMed + Crossref sync), `primary_location.source.type` (`journal`, `conference`, `repository`), `cited_by_count`, `authorships[].author` h-index, `abstract_inverted_index` (`openalex.rs:36`). | Reconstruct hits from local `VoxDb` external-intelligence / prior-art cache + static domain-tier reputation table (`WorthinessProfile::Repository` / `Preprint`). |
| **Semantic Scholar Graph API v1** | `GET https://api.semanticscholar.org/graph/v1/paper/search?query={q}&fields=paperId,title,year,citationCount,publicationVenue,isOpenAccess`<br>`POST https://api.semanticscholar.org/graph/v1/paper/batch?fields=title,abstract,embedding.specter_v2` | `VoxSemanticScholarApiKey` (`x-api-key` header; strict **1 RPS** enforced via `governor`; batch up to 500 IDs/call) | Precomputed 768-d `embedding.specter_v2` vectors for prior-art papers + citation counts (`scientia_prior_art.rs:256`). | Skip remote `specter_v2` fetch; compute embeddings via `vox_actor_runtime::llm::llm_embed` or offline `local_embed_text` (`embed.rs:28`) + `vox-similarity` MinHash-LSH against local `VoxDb` corpus; emit `NoveltyVerdict::InsufficientEvidence` if zero sources succeeded (`novelty.rs:88`). |
| **Zenodo Deposit API** | `POST/PUT https://zenodo.org/api/deposit/depositions` (+ `/files`, `/actions/publish`) or `https://sandbox.zenodo.org/...` | `VoxZenodoAccessToken`, `VoxZenodoSandbox`, `VoxZenodoDraftOnly` | Pre-reserved DOI (`prereserve_doi`), staged artifact checksum verification, draft/published archival receipt (`zenodo_metadata.rs:37`). | Export deterministic `.zenodo.json` + staging directory (`publication-scholarly-staging-export`) + `local_ledger` receipt in `VoxDb` for later replay when connectivity returns. |
| **OpenReview API v2** | `POST https://api2.openreview.net/login`<br>`POST https://api2.openreview.net/notes` | `VoxOpenReviewAccessToken` (or Email/Password), `VoxOpenReviewInvitation`, `VoxOpenReviewSignature` | Venue note submission, `note_id` receipt, and active comment/rebuttal reply-window polling (`scholarly/openreview.rs:18,328`). | Stage `openreview_submit_profile.json` + manuscript bundle locally; mark `external_submission_jobs` row `staged_offline`. |
| **Crossref Direct Deposit** | `POST https://doi.crossref.org/servlet/deposit` (`crossref5.3.1.xsd` multipart XML) | `VoxCrossrefPlusApiKey` / deposit credentials | Direct DOI registration & `retraction_notice` Crossref notification (`crossref_deposit.rs:61`). | Emit validated `crossref_deposit.xml` + `crossref_work.json` into staging bundle (`submission/staging.rs:29`). |
| **arXiv Staging (`arxiv_assist`)** | Local bundle builder (`render_arxiv_bundle_with_handoff`) -> operator upload to `https://arxiv.org/submit` | None (no public unendorsed automated submit REST API) | Deterministic `.tar.gz` (`main.tex`, `.bib`, venue `.sty`, figures, `arxiv-metadata.json`, `UPLOAD-CHECKLIST.md`) (`bundle.rs:80`, `arxiv_api.rs:19`). | 100% offline native execution by design. |

---

## Architectural Invariants: Build Toolchain, Crypto & LLM Boundary

1. **Zero C/C++/CMake/Bindgen Toolchain Invariant (`AGENTS.md §Cryptography Policy §3`)**:
   - Clean clones build with **only** pinned `rustc 1.96` (`edition = "2024"`), platform C compiler, and `Node + pnpm`.
   - **Never introduce `cmake`, `nasm`, `Go`, `perl`, or `libclang`**.
   - Embedding & NLI inference therefore uses `vox_actor_runtime::llm` + pure-Rust `candle-*` (`0.10`) + `tokenizers` (`fancy-regex`), **never** `ort` / `fastembed` / `rten-onnx-c` / C++ `hnswlib`.
2. **Two-Tier Cryptography Discipline + Scoped `nanopub` Exception (`AGENTS.md §Cryptography Policy §1–2`)**:
   - **Vox Application Data**: Every content fingerprint, chunk hash, CAS digest, replay output digest, and manifest signature MUST call `vox-crypto` (`vox_crypto::secure_hash` BLAKE3, `vox_crypto::hash_fast_hex` XXH3-128, or `vox-crypto` SHA3-256 / `ed25519`). **Remediation item in v7.6**: replace legacy direct `use sha2::{Digest, Sha256}` in `crates/vox-publisher/src/scientia_prior_art.rs:11` with `vox-crypto`.
   - **`nanopub = "0.2"` Scoped Exception**: The Nanopublication standard (`http://www.nanopub.org/nschema#`) mandates RSA PKCS#8 signatures and SHA-256 Trusty URIs (`RA<base64>`). Upstream `nanopub = "0.2"` is isolated strictly inside `crates/vox-scientia/src/nanopub/{spec,network}.rs`, its private keys are wrapped in `secrecy::SecretString` (`spec.rs:40`) and persisted exclusively through Clavis (`vox-secrets`), and no other crate may import `nanopub` directly.
3. **Model-Agnostic LLM Boundary (`AGENTS.md §Model-Agnostic LLM Boundary`)**:
   - All NLI entailment escalations, query decompositions, claim extractions, critic-gate reviews, and IMRaD slot syntheses MUST route through `vox_actor_runtime::llm::{infer_with_retry, llm_chat, llm_stream_events_activity, llm_embed}` with model selection resolved by `vox-orchestrator::models::{registry, select, autonomic}`.
   - **Remediation item in v7.6**: in `crates/vox-scientia/src/claim_extractor/minicheck.rs:76,325`, replace `std::env::var("VOX_MINICHECK_ENDPOINT")` with `vox_secrets::resolve_secret` / `vox-config`, and replace `LlmConfig::openrouter(model)` with model-agnostic task routing via `infer_with_retry`.

---

## Alternatives Considered

| Recommended (Workspace-Native) | Alternative | When to Use Alternative |
|--------------------------------|-------------|-------------------------|
| **Calibrated Hybrid (`aho-corasick`/`strsim` + optional pure-Rust `candle-transformers` DeBERTa/BERT + `vox_actor_runtime::llm::infer_with_retry`)** | **`fastembed` (`4.x`) / `ort` (`2.x`)** | **Never in default Vox builds**—`ort` (`onnxruntime-sys`) drags prebuilt C++ binaries or `cmake` + `libclang` (`bindgen`). Only viable inside an external out-of-process container/HTTP sidecar (`MiniCheckBackend::Http` via `VOX_MINICHECK_ENDPOINT`). |
| **`vox-similarity` (`LshIndex`) + `VoxDb` (`scientia_embedding_cache`) + `rayon` parallel cosine scan** | **Pure-Rust `hnsw_rs` (`0.3`) or `instant-distance`** | Only if local persistent `FindingCandidateV1` + prior-art vectors in a single `VoxDb` instance exceed **~100,000 vectors** (below 50k vectors, a `rayon` SIMD dot-product scan over contiguous `f32` blobs takes `<2 ms` with zero index-rebuild overhead or `Cargo.lock` additions). |
| **Zenodo `/api/deposit/depositions` (`vox-publisher::zenodo_metadata`)** | **Zenodo InvenioRDM `/api/records/{id}/draft`** | Only after InvenioRDM resolves `PUT /draft` metadata-overwrite bugs that orphan auto-reserved `prereserve_doi` PIDs; `/api/deposit/depositions` remains Zenodo's stable automation surface in 2026. |
| **`std.frame` (`polars==2.1.0` via `vox-python-bridge`) + pure-Rust `csv` / JSONL stream fallback** | **Direct Rust `polars` crate in `Cargo.toml`** | **Do not add Rust `polars` directly to workspace**: it adds ~250 transitive crates, heavy compile times/rlib bloat, and `cmake`/`zstd-sys` feature hazards, whereas `std.frame` already isolates Polars 2.1.0 in `vox-python-bridge`. |
| **`similar` (`2`, `inline` feature) + custom React virtualized diff renderer (`@tanstack/react-virtual`)** | **`react-diff-viewer-continued` / `monaco-editor` diff** | Never needed: `similar` already computes exact inline word/char diff hunks in Rust (`vox-gui/Cargo.toml:70`), keeping the frontend bundle small and rendering 100k+ diff lines smoothly via `@tanstack/react-virtual`. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **`ort` / `onnxruntime-sys` / `fastembed` / `embed_anything` (default features)** | Pulls Microsoft ONNX Runtime C++ shared libraries (breaking offline/hermetic builds) or requires `cmake` + C++ compiler + `libclang` (`bindgen`), violating `AGENTS.md §3 Build-toolchain invariant`. | `vox_actor_runtime::llm::{llm_embed, infer_with_retry, local_embed_text}` + optional pure-Rust `candle-transformers` (`0.10`) + `safetensors` (`0.7`) + `tokenizers` (`0.21`, `fancy-regex`). |
| **C++ `hnswlib` / `faiss-sys` / `usearch` / `sqlite-vss` FFI crates** | Require C++17, `cmake`, `libclang`, or OpenMP/BLAS system libraries across Windows/macOS/Linux. | Stage 1: `vox-similarity::LshIndex` (pure Rust, already in Layer 2). Stage 2: `VoxDb` `scientia_embedding_cache` + `rayon` cosine scan (or `qdrant-vector` HTTP feature in `vox-search` when external Qdrant is configured). |
| **`reqwest 0.13` in new first-party manifests** | Root `Cargo.toml:235` pins `reqwest 0.12` (`rustls-tls` / `ring`); adding `0.13` widens the dual-TLS-provider split documented in `contracts/crypto/transport-providers.v1.json:37-59`. | `reqwest = { workspace = true }` (`0.12`) via `vox_http_client::client_builder()`. |
| **`serde_yml` or unmaintained `serde_yaml 0.9`** | `serde_yml` is flagged `RUSTSEC-2025-0068` (unsound); `serde_yaml 0.9` is deprecated (`Cargo.toml:214-217`). | `serde_yaml = { workspace = true }` (aliased to maintained `serde_yaml_ng = "0.10"`). |
| **Direct `sha2`, `sha3`, `blake3`, `md5`, `sha1`, or `ring` imports in feature crates** | Violates `AGENTS.md §Cryptography Policy §1` and trips `vox-code-audit` detector `vox/crypto/banned-crate-import` (`crypto_ban.rs:141`). | `vox-crypto` (`secure_hash`, `hash_fast_hex`, SHA3-256, `ed25519`), with `nanopub = "0.2"` confined strictly to `crates/vox-scientia/src/nanopub/`. |
| **Direct `std::env::var(...)` for API keys/endpoints or direct LLM vendor URLs** | Bypasses Clavis doctor/redaction (`vox ci secret-env-guard`) and trips `llm_provider_call` audit error. | `vox_secrets::resolve_secret(SecretId::...)` + `vox_actor_runtime::llm`. |
| **Automated headless browser / HTTP scraping of `arxiv.org/submit`** | Violates arXiv ToS, triggers CAPTCHA/anti-bot blocks, and ignores per-category endorsement gates. | `vox-scientia::manuscript::latex::bundle::render_arxiv_bundle_with_handoff` + `arxiv_assist` operator handoff (`arxiv_api.rs:19`) + Zenodo/OpenReview direct REST archival. |

---

## Stack Patterns by Variant

**If query complexity is `<=1` hop (`Socrates` `OrientReport.evidence_gap < threshold` and `waves == 1`):**
- Route through `vox-search::bundle::run_search_with_verification` (`tantivy` KB Library + `VoxDb` `search_document_chunks_fts` + optional web leg via `WebSearchDispatcher`), run fast deterministic lexical/quote containment (`aho-corasick` + `SpanChecker`), and stream citations inline to Chat/CLI/MCP/GUI in `<1.5s` without multi-wave planning overhead.
- **Because:** Unifies Light search and Deep Research on one `SessionRetrievalEnvelope` (`socrates.rs:18`) and epistemic verdict schema instead of maintaining separate Chat RAG and Deep Research codepaths.

**If query escalates to `>1` wave (explicit Deep Research or `Socrates` escalation on high `evidence_gap` / `contradiction_hints > 0`):**
- Dispatch `vox_research_shim::research::run_research_with_context` (`waves: 2..=5`, `domain_mode: General | Shopping | CodeGen`), streaming typed `research_session_events` (`wave_started`, `subquery_executed`, `claim_verified`, `contradiction_detected`, `sandbox_check_completed`, `wave_completed`) over daemon SSE / Tauri `voxTransport` (`listenEvent`) into `MultiWaveProgressTimeline.tsx` and `ResearchDagCanvas.tsx` (`@xyflow/react`).
- **Because:** Gives live wave stability, contradiction edges, and real-time `UsageSnapshot` -> `scientia_cost_telemetry` attribution (`Gap 5`, `Gap 7`, `Gap 8` closure).

**If ingesting extreme-scale files or streams (100 MB–10+ GB logs, XML dumps, JSONL, CSVs):**
- Stream bytes via `tokio::fs::File` / `reqwest::Response::bytes_stream` -> optional `zstd`/`flate2` async decoder -> `quick-xml::Reader` / `csv::Reader` / `serde_json::StreamDeserializer` / `text-splitter` (`tiktoken-rs` bounded chunks) -> batched `VoxDb` `search_document_chunks` transactions + CAS blobs; for columnar aggregations in `.vox` automation, delegate to lazy `std.frame` (`frame.scan_csv` -> Polars 2.1.0 lazy plan).
- **Because:** Keeps resident memory `O(batch_size)` (~8–32 MB) and keeps UI rendering `O(viewport)` via `@tanstack/react-virtual`.

**If running completely air-gapped / offline (or external scholarly APIs return `429`/`5xx`):**
- Embedding path uses `vox_actor_runtime::llm::local_embed_text` (or cached `candle` Safetensors in `vox-hf-layout` + `scientia_embedding_cache`); Stage-1 novelty runs `vox-similarity::LshIndex` over historical `VoxDb` findings; claim verification runs lexical/quote/numeric `CalibratedHybrid` fallback; publication pipeline writes validated `docs/src/architecture/*.md` SSOTs, IMRaD/LaTeX, `.tar.gz` arXiv bundles, HTML Finding Pages, offline-validated signed TriG nanopubs (`Nanopub::check()`), and `.zenodo.json` staging directories locally.
- **Because:** Satisfies Vox's mandatory 100% standalone/offline graceful fallback invariant without degrading epistemic honesty (`InsufficientEvidence` / `TrustLookup::Unavailable` rather than fabricated scores).

---

## Version Compatibility & Constraints Matrix

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `candle-core = "0.10"` (patched `patches/candle-core-0.10.2`) | `candle-nn = "0.10"`, `candle-transformers = "0.10"`, `safetensors = "0.7"`, `qlora-rs = "1.0.5"`, `peft-rs = "1.0"` | Workspace `[patch.crates-io]` (`Cargo.toml:445-455`) unifies all Candle crates on `0.10.2` with Metal buffer-pool fixes. Never pull `candle-* 0.8/0.9` alongside `0.10`. |
| `tokenizers = "0.21"` | `default-features = false, features = ["fancy-regex"]` | **Critical**: enabling `tokenizers` default features activates `onig` (`onig_sys`), which compiles Ciguruma via C/cmake and breaks Windows/cross builds. Keep `default-features = false`. |
| `flate2 = "1"` | `default-features = false, features = ["rust_backend"]`, `tar = "0.4"` | Uses pure-Rust `miniz_oxide` (`Cargo.toml:359`) for deterministic byte-reproducible arXiv `.tar.gz` archives without C `zlib-sys`. |
| `reqwest = "0.12"` | `reqwest-middleware = "0.4"`, `reqwest-retry = "0.7"`, `nanopub = "0.2"`, `rustls = "0.23"` (`ring`) | All 28+ first-party HTTP consumers share `reqwest 0.12` via `vox-http-client`. |
| `turso = "0.6"` (`sync`) | `rusqlite = "0.32"` (`bundled`, dev-only), `tantivy = "0.22"` (`mmap`) | `VoxDb` production paths use async `turso 0.6` (`crates/vox-db/Cargo.toml:16`); `rusqlite` is dev-dependency only. |
| `wiremock = "0.6"` | `deadpool = { version = "0.12", features = ["rt_tokio_1"] }` | Under `rustc 1.96`, any crate adding `wiremock` in `[dev-dependencies]` MUST also include `deadpool = { workspace = true }` (`Cargo.toml:378-385`) so `Pool::timeout_get` type inference succeeds. |
| `react = "^19.2.8"` | `@xyflow/react = "^12.11.6"`, `recharts = "^3.8.1"`, `@tanstack/react-virtual = "^3.14.10"`, `@tauri-apps/api = "^2.11.1"` | Verified compatible and already building under `vite ^8.2.2` + `typescript ^5.0.2` in `crates/vox-gui/ui/package.json`. |

---

## Sources

### Vox Codebase SSOTs (Verified in Tree)
- `/Users/brbrainerd/dev/vox/Cargo.toml:61-456` — Workspace crate inventory, centralized dependency versions, `rust-version = "1.96"`, `edition = "2024"`, and `[patch.crates-io]` pins.
- `/Users/brbrainerd/dev/vox/docs/src/architecture/layers.toml:150-271` — Layer assignments (`vox-similarity` L2, `vox-eval` L2, `vox-scientia` L3, `vox-publisher` L3, `vox-research-shim` L3, `vox-search` L3).
- `/Users/brbrainerd/dev/vox/docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md:18-171` — Grounded 10-gap matrix (`RES-01..04`, `PUB-01..04`, `SCIGUI-01..04`).
- `/Users/brbrainerd/dev/vox/docs/src/architecture/deep-research-trust-novelty-scoring-landscape-2026-08-01.md:17-79` — Prior trust/novelty/NLI survey (corrected above to replace `fastembed`/`ort` with pure-Rust `candle` + `vox_actor_runtime::llm` + `vox-similarity`).
- `/Users/brbrainerd/dev/vox/docs/src/architecture/cryptography-ssot-2026.md:13-63` & `crates/vox-code-audit/src/detectors/crypto_ban.rs:23-150` — Build-toolchain invariant and `vox/crypto/banned-crate-import` enforcement.
- `/Users/brbrainerd/dev/vox/crates/vox-similarity/src/{signature,index}.rs`, `crates/vox-actor-runtime/src/llm/{embed,stream,chat}.rs`, `crates/vox-search/src/{trust,openalex,memory_hybrid}.rs`, `crates/vox-scientia/src/{claim_extractor/minicheck.rs,inspect_bridge/novelty.rs,nanopub/spec.rs,manuscript/latex/bundle.rs}`, `crates/vox-publisher/src/{scientia_prior_art.rs,zenodo_metadata.rs,scholarly/openreview.rs,scholarly/crossref_deposit.rs}`, `crates/vox-db/src/store/ops_embedding_cache.rs`, `crates/vox-gui/ui/{package.json,src/transport.ts}`.

### External 2025–2026 Specifications & Prior Art
- **Crossref & Retraction Watch REST API Integration (Production Jan 2025–2026)**: Crossref REST `/works/{doi}` `updated-by` / `update-to` (`type: "retraction"`) synced daily with Retraction Watch (`https://www.crossref.org/blog/retraction-watch-retractions-now-in-the-crossref-api/`). — *HIGH confidence*
- **OpenAlex Work Schema (2025–2026)**: `is_retracted` boolean (Retraction Watch + Crossref + PubMed daily sync), `primary_location.source.type`, and `abstract_inverted_index` (`https://docs.openalex.org/api-entities/works/work-object`). — *HIGH confidence*
- **Semantic Scholar Academic Graph API v1 (`SPECTER2`)**: `/graph/v1/paper/search` and `/graph/v1/paper/batch` (`fields=embedding.specter_v2`, 500 IDs/batch, `x-api-key` 1 RPS quota) (`https://api.semanticscholar.org/api-docs/`). — *HIGH confidence*
- **Zenodo Deposit vs. InvenioRDM Records API (2025–2026)**: Stability of `/api/deposit/depositions` `prereserve_doi` workflow vs. InvenioRDM `/api/records/{id}/draft` PUT metadata-overwrite pitfalls (`https://developers.zenodo.org/`). — *HIGH confidence*
- **Nanopublication Trusty-URI & `nanopub` Rust Crate (`0.2`)**: Offline RSA signature & Trusty-URI verification against `Nanopublication/nanopub-testsuite` (`https://docs.rs/nanopub/0.2.0/nanopub/`). — *HIGH confidence*
- **Hybrid NLI + LLM Claim Entailment & Reference Hallucination Detection**: SciFact / MiniCheck calibrated abstention & reference grounding (`arXiv:2604.03173`, `arXiv:2607.06528`). — *HIGH confidence*

---
*Stack research for: Vox Milestone v7.6 (Unified Light ↔ Deep Research Continuum, Extreme-Data Streaming, Socrates/Novelty Verification & Continuous Direct Publication Flywheel)*
*Researched: 2026-11-14*
