# Pitfalls Research

**Domain:** Unified Light ↔ Deep Research Continuum, Extreme-Data Streaming, Socrates/Novelty Verification & Continuous Direct Publication Flywheel (`Milestone v7.6`)
**Researched:** 2026-11-14
**Confidence:** HIGH (grounded in live Vox Rust/TS seams + August/October 2026 divergence audits + 2025–2026 external API/benchmark literature)

---

## Critical Pitfalls

### Pitfall 1: Breaking the Day-Stable Chat KV-Cache Prefix (`v2.9–v3.2` Invariant)

**What goes wrong:**
Unifying Light (`<=1` hop) and Deep (`>1` wave) research across Chat and all channels tempts implementers to inject dynamic `[RESEARCH MODE]`, per-turn Socrates confidence bands, live wave budgets, or intent-filtered MCP tool subsets directly into the system prompt or tool declaration list. Prompt-cache hit rates on Anthropic, OpenAI, and DeepSeek collapse from `>85%` to `<10%`, replacing `0.10×` cached-prefix reads with `1.25×` cache-creation writes on every turn (~`8–12×` effective input-token cost surge and `2–4s` TTFT regression on long sessions).

**Why it happens:**
LLM KV-caches match strictly byte-for-byte from the start of `[tools, system, ...prior_messages]`. Any mutation in tool schemas or early/mid system prompt bytes invalidates every downstream token. In Vox today, [`build_system_prompt_with_skill`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/mod.rs#L193-L379) explicitly warns at line `344` to keep the prompt day-stable (`ts_to_date_str`), **yet two latent seams right next to it already threaten prefix stability**:
1. [`attention_budget`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/mod.rs#L341-L356) embeds `Attention Budget Signal: {:?}` directly into the system prompt — when Socrates clarification or multi-wave research charges `CLARIFICATION_INTERRUPT_COST_MS` (`23,250 ms`), the signal flips (`Normal` → `Elevated` → `HighLoad`) and busts the system prefix mid-session.
2. [`env.operating_mode`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/mod.rs#L361-L375) and [`interpolate_system_prompt_variables`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs#L126-L143) (`$active_file`, `$active_spoke`, `$cognitive_profile`) mutate the system prompt whenever the operator switches files or toggles Light/Deep research posture.
3. On the legacy single-string fallback in [`chat_message`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs#L212-L221), `turn_user_prompt` (containing `[WEB RESEARCH — N SOURCES]`) is concatenated *after* `[CONVERSATION HISTORY]`, which is safe, whereas putting dynamic retrieval blocks *before* `prior_conversation` busts multi-turn prefix caching.

**How to avoid:**
- Keep `build_system_prompt_with_skill` and the ordered MCP tool catalog **100% invariant across `ResearchMode::{None, Quick, Deep}`** within a session/day (`system_prompt_kv_prefix_stable == true` in [`crates/vox-orchestrator/src/models/registry.rs`](../../../../crates/vox-orchestrator/src/models/registry.rs#L125)).
- Never add/remove MCP tools per turn based on `classify_research_intent`; expose one stable continuum research tool schema (or keep `ENABLE_TOOL_SEARCH` lazy group mode invariant).
- Inject all turn-volatile research artifacts (`[WEB RESEARCH — N SOURCES]`, Deep wave summaries, Socrates turn-level epistemic state, active file skeletons, and volatile `AttentionBudget` warnings) **exclusively into `turn_user_prompt`** (the final user message block in [`preamble_and_retrieval.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/message/preamble_and_retrieval.rs#L13-L250)), never into `system_prompt`.
- Assert byte-equality of `build_system_prompt_with_skill` across consecutive `None` → `Quick` → `Deep` turns in unit tests, and gate `CachePredictor` (`overlap_ratio >= 0.70` in [`cache_predictor.rs`](../../../../crates/vox-orchestrator/src/cache_predictor.rs#L65-L81)) + `LlmSurfaceTelemetry.cache_read_tokens` in integration tests.

**Warning signs:**
- `LlmSurfaceTelemetry.cache_read_tokens` (`crates/vox-orchestrator-mcp/src/chat_socrates_meta.rs:35`) drops to `0` on turn 2+ after a `/research` or heuristic Light/Deep trigger.
- `orch.cache.hit_prediction` (`cache_predictor.rs:101`) predicts `Hit` while provider metadata (`providers/metadata.rs:5-9`) reports `cache_creation_input_tokens > 0` on consecutive turns.

**Phase to address:**
`v7.6` Phase 1 — Unified Light ↔ Deep Continuum Router & KV-Cache-Safe Chat Harness (`Phases 451`).

---

### Pitfall 2: Extreme-Data OOM, Silent `BroadcastStream` Lag & the 180s Chat Dispatch Wall

**What goes wrong:**
Scaling to multi-wave (`>1` wave) Deep Research and extreme-data corpora causes four compounding runtime failures:
1. **Chat Timeout Wall:** [`research_turn::run_deep`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs#L596-L650) currently hardcodes `waves: 1` (line `628`), buffers all progress callbacks inside `Arc<Mutex<Vec<...>>>` (lines `605–616`), and awaits the entire pipeline inline. As documented in [`verifier.rs:155-160`](../../../../crates/vox-research-shim/src/research/verifier.rs#L155-L160), a single 50-claim run took `322s` and blew the `180s` chat dispatch timeout; enabling `waves: 2..=5` inline guarantees timeout failures in Chat, MCP, and CLI channels.
2. **Silent Event Loss on `RecvError::Lagged`:** `ServerState::research_events` uses a fixed `tokio::sync::broadcast::channel(256)` ([`server_state.rs:432`](../../../../crates/vox-orchestrator-mcp/src/server_state.rs#L432)), and the HTTP gateway uses `TOPIC_CHANNEL_CAPACITY = 64` ([`scientia_feed.rs:33`](../../../../crates/vox-orchestrator-mcp/src/http_gateway/scientia_feed.rs#L33)). Under extreme-data bursts (50+ hits × 24 claims × multi-wave events), slow SSE/WS clients receive `RecvError::Lagged(skipped)` and silently miss stage transitions (`verifying_claims` → `completed`), leaving GUI timelines frozen at stage 0.
3. **Backend Heap Clones & SQLite Write-Lock Storms:** [`emit_research_event`](../../../../crates/vox-research-shim/src/research/orchestrator/pipeline.rs#L1297-L1306) clones every `ResearchEvent` and fires an unbounded `spawn_persist_research_event_for_metrics` task per event against SQLite, while [`scientia_feed.rs:59, 122`](../../../../crates/vox-orchestrator-mcp/src/http_gateway/scientia_feed.rs#L59-L128) calls `VoxDb::connect_default().await` every 5s and serializes the entire `QueueSnapshot` to a `String` for diffing. Simultaneously, storing full scraped page text in `ResearchHit.raw_content` across `all_hits` and cloning `result.clone()` into `ResearchRunArtifact` ([`pipeline.rs:1143-1154`](../../../../crates/vox-research-shim/src/research/orchestrator/pipeline.rs#L1143-L1154)) bloats RSS by tens of megabytes per concurrent session.
4. **React V8 Heap OOM:** Appending raw SSE chunks (`setEvents(prev => [...prev, ev])`) or repeatedly parsing multi-megabyte `artifact_json` blobs on the browser main thread freezes React reconciliation and crashes WebView/Chromium tabs.

**Why it happens:**
The Phase 0a/Stage 1 pipeline was built for small synchronous 1-wave runs (`max_sources: 10`), using in-memory `Vec` accumulation and fire-and-forget `tokio::spawn` tasks instead of bounded MPSC batching, monotonic `seq` watermarks (`after_seq`), and externalized content-addressed blob references.

**How to avoid:**
- **Decouple execution from streaming across all channels:** Promote `>1`-wave Deep runs in Chat/MCP/GUI to durable session handles that stream monotonic `seq`-numbered delta events (`after_seq` watermark cursor, matching `v7.5` `TERM-PTY-04` pattern) with explicit `lag` resync frames (`{"event":"lag","after_seq":N}`) when `BroadcastStreamRecvError::Lagged` fires.
- **Bound and externalize payloads:** Strip/spill `ResearchHit.raw_content` to bounded chunk storage (via `vox-crypto` BLAKE3 content hashes) immediately after snippet/span extraction so `ResearchHit` in RAM carries only `<= 2 KiB` snippets and `Arc<str>` metadata, never multi-MB HTML/PDF text.
- **Batch DB telemetry writes:** Replace per-event `spawn_persist_research_event_for_metrics` and per-tick `VoxDb::connect_default()` in `scientia_feed.rs` with a single bounded `tokio::sync::mpsc::channel(512)` batch writer flushing to `VoxDb` in one transaction every `250 ms` (or `64` events), reusing `ServerState.db`.
- **Cap frontend state:** Use a fixed ring buffer / windowed virtualizer in React for live event feeds and hydrate DAG/claim drawers via paginated/summary DTOs rather than re-parsing raw `artifact_json` on every poll tick.

**Warning signs:**
- `SQLITE_BUSY` / breaker trips in `vox-db` during concurrent multi-wave runs.
- `RecvError::Lagged` warnings in `mesh_subscriber.rs:168` or frozen `PipelineTimeline` states in `ResearchView.tsx`.
- Chat turns with `ResearchMode::Deep` failing around `180_000 ms` or daemon RSS exceeding `512 MiB` during 5-wave runs.

**Phase to address:**
`v7.6` Phase 2 — Extreme-Data Bounded Streaming, `after_seq` SSE Backpressure & Durable Multi-Wave Execution (`Phase 452`).

---

### Pitfall 3: Socrates Clarification Fatigue, Verifier Cost Blowups & Split-Brain Call-Site Divergence

**What goes wrong:**
1. **Socratic Interrogation Loops on Research Queries:** When a user asks an open-ended or cutting-edge research question, pre-retrieval confidence is naturally low (`< ask_for_help_threshold`). If Socrates gates *before* Light retrieval runs — or conflates *epistemic/world uncertainty* (missing external facts) with *specification uncertainty* (ambiguous user goal) — Chat pauses to ask up to `max_clarification_turns: 3` (`policy_types.rs:95`) micro-questions (*"Which sub-aspect of X?"*, *"Should I check web or local?"*) instead of running a `<2s` Light search pass first. Operators suffer severe clarification fatigue and abandon autonomous research.
2. **3× LLM Verifier Cost & Latency Explosion:** In [`verifier.rs:153-165`](../../../../crates/vox-research-shim/src/research/verifier.rs#L153-L165), `RESAMPLE_COUNT = 3` and `MAX_CLAIMS_VERIFIED_PER_RUN = 24` fire up to **`72` LLM calls per wave** (`216–360` LLM calls across 3–5 waves!), while `VerifierConfig.abstain_threshold` (`verifier.rs:17`) is still an ignored Phase-0a placeholder.
3. **Split-Brain Call-Site Divergence (The #1 Historical Vox Failure Mode):** As proven in [`deep-research-implementation-divergence-audit-2026-08-01.md` §5](../../../../docs/src/architecture/deep-research-implementation-divergence-audit-2026-08-01.md#L42-L55), Vox already has **two competing verifiers and two split call sites**:
   - [`crates/vox-scientia/src/claim_extractor/pipeline.rs:96`](../../../../crates/vox-scientia/src/claim_extractor/pipeline.rs#L96) calls `MiniCheckVerifier::from_env()`, which **still falls back to `MiniCheckBackend::Mock` (naive word overlap!)** ([`minicheck.rs:84-97`](../../../../crates/vox-scientia/src/claim_extractor/minicheck.rs#L84-L97)), while line `110` calls `from_env_or_calibrated()`!
   - `minicheck.rs:76, 87` reads `std::env::var("VOX_MINICHECK_ENDPOINT")` directly — **violating Vox's Clavis `vox_secrets::resolve_secret` / `vox-config` SSOT policy**!
   - `Verdict` in `verifier.rs:40-45` (`Supported`, `Contradicted`, `Contested`, `Unverified`) is still unreconciled with SciFact (`Support`, `Contradict`, `NotEnoughInfo`, `Abstain`), and `resample_stability` (`verifier.rs:99`) sits alongside an unrelated `self_consistency` field in `vox-orchestrator::orchestrator_policy` (`weight 0.20`).

**Why it happens:**
Developers wire new verification logic into one constructor (`from_env_or_calibrated`) while leaving legacy constructors (`from_env`) alive for backward compatibility, and apply Socratic `RiskDecision::Ask` uniformly without separating *user-intent ambiguity* from *retrievable world knowledge*.

**How to avoid:**
- **Epistemic vs. Specification Uncertainty Gate:** Enforce in code (matching the prose rule in [`chat_socrates_meta.rs:98`](../../../../crates/vox-orchestrator-mcp/src/chat_socrates_meta.rs#L98)) that Socrates **never interrupts the user before executing at least one Light (`<=1` hop) probe** when uncertainty is epistemic/factual. Reserve `RiskDecision::Ask` strictly for high-entropy *specification* ambiguity (`expected_information_gain_bits >= 0.25`, `max_clarification_turns <= 1` for autonomous research lanes), and auto-escalate Light → Deep when Wave 1 exhibits high contradiction ratio or low subquery coverage (`Coverage Paradox`).
- **3-Tier Verification Cascade (Cost Cap):**
  1. *Tier 0 (Deterministic Pre-Filter, `0` LLM calls):* Exact quote / normalized n-gram + numeric/unit + negation-symmetry check confirms verbatim grounded citations and rejects obvious numeric mismatches in `<1 ms`.
  2. *Tier 1 (Single Calibrated Entailment Call via `vox_actor_runtime::llm`):* Structured SciFact entailment (`Support | Contradict | NotEnoughInfo | Abstain`) enforcing `abstain_threshold`.
  3. *Tier 2 (3× Temperature>0 Resampling):* Triggered **only** on high-salience claims whose Tier 1 confidence lands inside the uncertain margin band (`[abstain_threshold, 0.80]`), cutting verifier LLM calls by `~65–80%`.
- **Delete Split-Brain Constructors & Enforce Clavis:** Remove `MiniCheckVerifier::from_env()` (restrict `MiniCheckVerifier::mock()` to `#[cfg(test)]`), route any external endpoint/credentials through Clavis / `vox-config`, and grep-verify **every** `ClaimVerdict`, `MiniCheckVerifier`, and `self_consistency` / `resample_stability` consumer in a dedicated wiring audit task.

**Warning signs:**
- Chat asks a clarifying question on factual prompts like *"Compare vLLM vs SGLang speculative decoding throughput in 2026"* instead of searching.
- `scientia_cost_telemetry` shows `>30` LLM calls on a single 10-claim session, OR `0` LLM calls because `ExtractionPipeline` hit `MiniCheckVerifier::from_env()` (`Mock`).
- `vox ci secret-env-guard` flags `std::env::var("VOX_MINICHECK_ENDPOINT")`.

**Phase to address:**
`v7.6` Phase 3 — Socrates Epistemic Gating & Calibrated 3-Tier Claim/Citation Verifier (`Phase 453`).

---

### Pitfall 4: Retired OpenAlex `mailto=` Pool, Scholarly Rate-Limit Cliffs & Offline Promotion Deadlocks

**What goes wrong:**
Hydrating `NoveltyEvidenceBundle` (two-stage lexical/MinHash → semantic/embedding prior-art scoring across OpenAlex, Crossref, and Semantic Scholar) hits immediate HTTP `429`/`403`/`503` walls in live runs and **deadlocks candidate promotion when offline**:
1. **OpenAlex Retired `mailto=` Polite Pool (2025–2026 Upstream Breaking Change):** [`crates/vox-search/src/openalex.rs:125`](../../../../crates/vox-search/src/openalex.rs#L125) relies on `&mailto=research@vox.computer`, [`gui_ops/research.rs:867, 901`](../../../../crates/vox-orchestrator-mcp/src/gui_ops/research.rs#L867-L905) resolves `SecretId::VoxOpenAlexEmail`, and [`web_dispatcher.rs:562-566`](../../../../crates/vox-search/src/web_dispatcher.rs#L562-L566) passes `api_key: None`. OpenAlex **retired the `mailto` polite pool** — `mailto` is now ignored, unauthenticated traffic is throttled to an anonymous ~`$0.10/day` budget (`10 RPS`), and authenticated access requires an OpenAlex API key (`api_key=` query param or `Authorization: Bearer`, yielding `30 RPS` and `$1.00/day` free credit). Worse, [`openalex.rs:137`](../../../../crates/vox-search/src/openalex.rs#L137) sends a non-standard `api-key` header, and [`trust.rs:130-135`](../../../../crates/vox-search/src/trust.rs#L130-L135) sends neither!
2. **Crossref Concurrency=1 Cliff:** [`TrustScorer::check_retraction`](../../../../crates/vox-search/src/trust.rs#L67-L72) queries `https://api.crossref.org/works/{doi}` **without `?mailto=`**, dropping Vox into Crossref's **Anonymous Public Pool (strict concurrency limit = 1, list query limit = 1 RPS)** instead of the Polite Pool (`10 RPS`, concurrency `3–5`). Firing parallel `research_hit_from_hybrid` calls across 20 hits immediately trips Crossref `429`/`503` errors, and each stalled request waits up to `D_30S` (`30s` timeout at `trust.rs:24`).
3. **Semantic Scholar 1 RPS Wall:** Even with `SecretId::VoxSemanticScholarApiKey`, Semantic Scholar's Academic Graph API enforces **1 request per second (1 RPS)**. Bursting per-claim novelty queries across 15 claims without a token-bucket limiter fails 90% of requests.
4. **Offline Fail-Closed Deadlock Violates Standalone Fallback:** [`TrustScorer::lookup_for_promotion`](../../../../crates/vox-search/src/trust.rs#L90-L101) treats any network failure as `TrustLookup::Unavailable`, which marks the candidate **ineligible for promotion**. In air-gapped/offline mode (a non-negotiable Vox invariant: *100% standalone/offline graceful fallback*), operators cannot promote or draft local repository/benchmark findings that cite any DOI!

**Why it happens:**
Upstream scholarly API contracts shifted from email-based polite pools to metered API keys between 2024 and 2026, while `trust.rs` and `openalex.rs` were authored separately without shared token-bucket rate limiters, SQLite response caching, or an explicit `OfflineDegraded` attestation state.

**How to avoid:**
- **Fix Scholarly Auth & Clavis Specs:** Add/migrate `SecretId` in `crates/vox-secrets/src/spec/` for OpenAlex API Key (`Authorization: Bearer` / `api_key=`), wire it through `web_dispatcher.rs`, `openalex.rs`, `trust.rs`, and `scientia_novelty_assess`, and append `?mailto=` + `User-Agent` to Crossref calls so Crossref enters the Polite Pool.
- **Per-Provider Token-Bucket Governor + VoxDb Prior-Art Cache:** Enforce process-wide async rate limiters (`Semantic Scholar: 1 RPS / concurrency 1`, `Crossref Polite: 5 RPS / concurrency 3`, `OpenAlex Keyed: 20 RPS / anonymous: 3 RPS`), batch DOI/work lookups where APIs support filter syntax (`doi:A|B|C` in OpenAlex), lower per-hit trust lookup timeouts from `30s` to `3.5s`, and cache DOI retraction + novelty query responses in `VoxDb` (`TTL = 14 days`, keyed by `vox-crypto` BLAKE3 query digest).
- **Two-Stage Offline-Capable Novelty Scoring:** Run Stage 1 (MinHash-LSH + shingle Jaccard + local `VoxDb` / repo `vox-search` tantivy+RRF prior-art scan) 100% locally first; run Stage 2 (external OpenAlex/Crossref/Semantic Scholar + `vox_actor_runtime::llm::llm_embed` cosine similarity) when online. When offline, populate `NoveltyEvidenceBundle` with `query_traces` recording `offline_local_only = true` and transition promotion to `OfflineAttested` (requiring explicit human sign-off badge rather than hard `Unavailable` crash).

**Warning signs:**
- `QueryTrace` rows in `NoveltyEvidenceBundle` showing `http_status: 429` or `503` (`novelty.rs:247-263`).
- Offline `vox scientia candidate-promote` failing with `"Crossref lookup failed for DOI ..."`.
- Multi-hit web gathering stalling for `30s` inside `research_hit_from_hybrid` (`web_gather.rs:31-35`).

**Phase to address:**
`v7.6` Phase 4 — Rate-Governed Two-Stage Novelty Engine & Offline-Resilient Scholarly Adapters (`Phase 454`).

---

### Pitfall 5: Direct Publication Data-Loss Bug, `vox-doc-pipeline` Gate Rejections, LaTeX Breakouts & Epistemic Ouroboros

**What goes wrong:**
Automating the Continuous Discovery → DB-Hydrated Direct Publication (`> nanopublication`) flywheel triggers **five severe codebase-specific traps** already verified in tree:
1. **Production Data-Loss Bug (`write_research_doc` Wipes `artifact_json`):** In [`crates/vox-orchestrator-mcp/src/gui_ops/research.rs:714-717`](../../../../crates/vox-orchestrator-mcp/src/gui_ops/research.rs#L714-L717), publishing an Architecture SSOT calls `db.store_research_artifact(session_id, "{}", content)`. Because [`store_research_artifact`](../../../../crates/vox-db/src/research_pipeline.rs#L468-L475) executes `ON CONFLICT(session_id) DO UPDATE SET artifact_json = excluded.artifact_json`, **publishing a doc overwrites the session's entire `artifact_json` (`claim_verdicts`, `citations`, `subqueries`, `wave_stability`) with `"{}"`**! Subsequent attempts to inspect the DAG or promote the session to a Scientia Publication find zero claims and zero citations.
2. **`vox-doc-pipeline` Hard-Blocks Synthesized SSOTs on Four Rules:**
   - **Prohibited `vox:skip` & Uncompilable ````vox` Fences:** Despite older prose in `AGENTS.md`, [`crates/vox-doc-pipeline/src/pipeline/doctest.rs:71-88`](../../../../crates/vox-doc-pipeline/src/pipeline/doctest.rs#L71-L88) **hard-rejects `vox:skip` (`DocTestFailed: vox:skip is prohibited`)** and compiles every ````vox` fence via `vox_compiler::pipeline::check_file`. Any LLM-synthesized pseudo-Vox snippet fails CI pre-push!
   - **Shrink-Only Mermaid Ratchet:** [`crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs:21-38`](../../../../crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs#L21-L38) enforces `legacy-mermaid-baseline.v1.yaml` (`0` ````mermaid` fences allowed in new `docs/src/` files; diagrams **must** use ````vox-diagram`).
   - **Missing `authorship_model` Facet:** [`architecture_ssot.rs:176-187`](../../../../crates/vox-scientia/src/manuscript/scaffold/architecture_ssot.rs#L176-L187) emits `authorship: "llm-assisted"` *without* `authorship_model` whenever `synthesis_model` is `None` (or when called via `render_doc_draft(session_id)` with `db = None`), failing `facet-missing-authorship-model`.
   - **Canonical-Map & Status Violation:** `gui_ops/research.rs:607` stamps `status: "current"` on synthesized `*-2026.md` files without registering a `B-canon` entry in `contracts/documentation/canonical-map.v1.yaml`, violating [`documentation-governance.md` §Authority tiers & §Claim policy](../../../../docs/src/contributors/documentation-governance.md#L205-L249) (synthesized research docs must use `status: "research"` unless human-promoted to canon, and never set `human_attested_by` or `training_eligible: true` automatically).
3. **LaTeX Compilation Crashes & TeX Injection in `vox-scientia`:**
   - **Unicode `inputenc` Crash:** [`escape.rs:60-63`](../../../../crates/vox-scientia/src/manuscript/latex/escape.rs#L60-L63) passes all non-ASCII Unicode through unchanged, while [`render.rs:38-39`](../../../../crates/vox-scientia/src/manuscript/latex/render.rs#L38-L39) uses `\usepackage[utf8]{inputenc}` + `\usepackage[T1]{fontenc}`. Any scraped title or claim containing `≤`, `≥`, `≈`, `→`, `Δ`, `α`, or CJK characters crashes `pdflatex` with `Package inputenc Error: Unicode character ... not set up for use with LaTeX`.
   - **TeX Comment & Verbatim Breakouts:** [`render.rs:107`](../../../../crates/vox-scientia/src/manuscript/latex/render.rs#L107) (`f.doi_or_url`), [`render.rs:186`](../../../../crates/vox-scientia/src/manuscript/latex/render.rs#L186) (`caption_hint`), and [`render.rs:195`](../../../../crates/vox-scientia/src/manuscript/latex/render.rs#L195) (`source_script`) interpolate strings into `%` comments without stripping `\n`/`\r`; a multiline hint breaks out into raw TeX! Similarly, [`markdown_to_latex`](../../../../crates/vox-scientia/src/manuscript/latex/render.rs#L288-L311) emits raw text inside `\begin{verbatim}...\end{verbatim}` without escaping `\end{verbatim}`, and [`render.rs:66, 157, 234`](../../../../crates/vox-scientia/src/manuscript/latex/render.rs#L157) places URLs containing `%`, `#`, `&`, or `{` inside `\href{}` / `longtable` `\url{}` cells, causing fatal unbalanced-brace or alignment errors.
   - **Venue Template Mismatch:** `render.rs:37` hardcodes `\documentclass[11pt,a4paper]{article}` regardless of requested venue (`neurips_2025`, `icml2025`, `acmart`, `jmlr2e`), and `write_abstract_block` / `write_section_todo` (`render.rs:75-112`) deliberately forbid AI narrative prose per the worthiness rubric — meaning "Direct Publication" must either populate structured empirical sections honestly + flag human narrative slots, or explicitly version the rubric contract rather than silently violating `forbidden_sections_appear_as_section_with_todo_comments_only` (`render.rs:374`)!
4. **Epistemic Ouroboros (Self-Poisoning Flywheel / D-06 Taint Bypass):**
   - [`pipeline.rs:1163-1173`](../../../../crates/vox-research-shim/src/research/orchestrator/pipeline.rs#L1163-L1173) writes every research run's `report_markdown` directly into `VoxDb::save_memory` (`memory_type: "research_result"`), and [`web_gather.rs:48-85`](../../../../crates/vox-research-shim/src/research/orchestrator/web_gather.rs#L48-L85) loads `execution.memory_lines` / `chunk_lines` in subsequent `ResearchScope::Both` runs with **`trust_score: 1.0`**! Autonomous continuous discovery will retrieve its own prior unattested syntheses as `1.0`-trusted primary evidence, amplifying hallucinations across waves and bypassing the `D-06` `FindingOrigin` taint rule ([`research_turn.rs:562-565`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs#L562-L565)).

**Why it happens:**
Doc synthesis, DB persistence, LaTeX scaffolding, and local memory retrieval evolved in separate milestones without an end-to-end pre-write lint gate (`vox-doc-pipeline` in-process check) or provenance-taint filtering on `save_memory`.

**How to avoid:**
- **Fix `write_research_doc` immediately:** Preserve existing `artifact_json` (update only `published_doc_path` / FTS index, never pass `"{}"` to `store_research_artifact`).
- **Pre-Flight `vox-doc-pipeline` Gate in `render_doc_draft` & `write_research_doc`:** Run `vox-doc-pipeline` lint + doctest + `vox-diagram` checks *in memory before writing to disk*:
  - Default synthesized docs to `status: "research"`, `training_eligible: false`, `authorship: "llm-assisted"`, and guaranteed non-empty `authorship_model` (fallback `"vox-research-cascade"` when `synthesis_model` is empty).
  - Convert any LLM-emitted ````mermaid` fences to ````vox-diagram` (or `text` if invalid), validate any ````vox` block via `vox_compiler::pipeline::check_file` (downgrading non-compiling pseudo-excerpts to ````text` rather than emitting banned `// vox:skip`), and strip leading single-backtick prose lines.
- **Harden LaTeX Escaping & Unicode Mapping:** Add a Unicode-to-LaTeX math/Greek/symbol transliteration table (`≤` → `$\leq$`, `Δ` → `$\Delta$`, `→` → `$\rightarrow$`, `±` → `$\pm$`, etc.) in `escape_latex`, strip `\r`/`\n` in all `%` comment interpolations, sanitize `\end{verbatim}` inside code blocks, percent-encode `%`/`#`/`{`/`}`/`\` in `\url{}`/`\href{}` arguments, validate figure paths against `..` traversal in `.tar.gz` bundles, and hash all RO-Crate / `sources[].hash` digests exclusively via **`vox-crypto` (`blake3` / `sha3`)**.
- **Quarantine Unattested Synthesis from Retrieval (`D-06` Enforcement):** Tag `save_memory` entries from autonomous research with `origin = "untrusted_synthesis"` and exclude unattested `research_result` memories + `status: research` synthesized drafts from receiving `trust_score: 1.0` in `web_gather.rs` (`append_local_lines`), preventing circular self-citation.

**Warning signs:**
- Clicking `"Publish Architecture SSOT"` causes `get_session` to return `claims: []` and `source_count: 0`.
- `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/<slug>-2026.md` fails on `DocTestFailed`, `MermaidIssue`, or `facet-missing-authorship-model`.
- `pdflatex -interaction=nonstopmode` fails on scraped papers with Greek letters or `%` in archive URLs.

**Phase to address:**
`v7.6` Phase 5 — DB-Hydrated Direct Publication (`> Nanopub`), Lint-Clean SSOT Synthesizer, Hardened Venue LaTeX & Provenance Quarantine (`Phase 455`).

---

### Pitfall 6: Parallel Workstream Collisions (`v7.4` GUI Primitives & `v7.5` Terminal) & Workspace Guard Violations

**What goes wrong:**
`v7.6` executes alongside two active, overlapping workstreams in `/Users/brbrainerd/dev/vox/.planning/workstreams/`:
1. **`axis-live-functional-v74` (`Phases 431–436`, currently in Phase 434–436):** Actively refactoring `ResearchView.tsx`, `ScientiaSurface.tsx`, `PublicationsView.tsx`, `Discovery`, `Chat`, and `transport.ts` to enforce:
   - Unified `radix-ui` `data-slot` primitives (`Select`, `Tabs`, `Dialog`, `Sheet`, `Resizable`, `DataTable`, `RomanRotaryKnob`) — **0 raw `<select>` or `<input type="range">` allowed across `crates/vox-gui/ui/src`** (`Phase 434` Criterion 4).
   - **Zero raw emojis or BMP dingbats** (`🔒`, `⚡`, `⚠️`, `✓`, `✕`, `▶`, `▼`, `↗`, `★`, `⚙`) enforced by `zeroEmojiAndDingbat.test.ts` and Playwright `audits.ts` (`Phase 432` Criterion 4).
   - **Single `voxTransport` funnel** (`crates/vox-gui/ui/src/transport.ts`) with `0` direct `@tauri-apps/api` `invoke`/`listen` calls (`ipcBoundaries.test.ts`), `0` `_demo` paper fallbacks (`noPaperFallbackGuard.test.ts`, `sentryHonesty.ts`), and `0` persistent `VoxDb` handles in `crates/vox-gui` (`Phase 433`).
   - `@container/surface` `<BentoGrid>` and `<BentoScrollRegion>` (`tabIndex={0}`, `role="region"`, `aria-label`) across `research`, `scientia`, and `publications` (`Phase 435`).
2. **`axis-warp-terminal-v75` (`Phases 441–446`):** Actively editing `ChatSurface.tsx`, `ChatTranscript.tsx`, `DockWorkspaceShell.tsx`, `vox-orchestrator-d`, `audits.ts`, `cv-flywheel.spec.ts`, and `contracts/operations/catalog.v1.yaml`.
3. **God-Object & Layer-Boundary Traps:**
   - `crates/vox-research-shim/src/research/orchestrator/pipeline.rs` (`1,989` lines), `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs` (`1,902` lines), `crates/vox-orchestrator-mcp/src/gui_ops/research.rs` (`1,690` lines), `crates/vox-scientia/src/review_flow.rs` (`79 KB`), and `ResearchView.tsx` (`1,199` lines) already exceed the `500` non-blank-line `arch/god_object` threshold (`layers.toml:60`). Stuffing new v7.6 continuum, novelty, or Direct Publication code inline into those files triggers `vox audit code` / `vox-arch-check` failures and massive merge conflicts.
   - Re-serializing `contracts/operations/catalog.v1.yaml` (1k+ lines) instead of inserting sorted rows + running `operations-sync --write` corrupts `x-vox-version` and collides with `v7.5` `TERM-HARNESS-01`.

**Why it happens:**
Subagents designing `v7.6` UI or MCP tools from older patterns introduce raw `<select>`, Unicode `✓`/`⚠` badges in DAG nodes, direct `invoke()` calls, or inline 400-line additions to `ResearchView.tsx` and `ChatTranscript.tsx` without checking `v7.4`/`v7.5` invariants.

**How to avoid:**
- **Decompose into new focused modules (`< 400` LoC each):** Put new Rust logic in dedicated submodules (`continuum_router.rs`, `novelty_hydrator.rs`, `scaffold_hydrator.rs`, `socrates_research_gate.rs`) and new React views in standalone components (`ContinuumTraceCard.tsx`, `WorthinessRadarPanel.tsx`, `DirectPublicationDrawer.tsx`) mounted via thin 5–15 line seams in `ResearchView.tsx` / `ChatTranscript.tsx` / `ScientiaSurface.tsx`.
- **Honor `v7.4` UI Gates from Day 1:** Use only `<VoxIcon>` / `lucide-react` / `Icons.tsx` SVGs (zero emojis/dingbats), Radix `data-slot` `<Select>`/`<Tabs>`/`<Sheet>`/`<Resizable>`/`<BentoScrollRegion>` (zero native `<select>`/`<input type="range">`, zero banned `transition-all`), and route 100% of frontend RPC/SSE calls through `crates/vox-gui/ui/src/transport.ts` with honest disabled states when `!backendAvailable()`.
- **Sequence Shared Hot-Files:** Coordinate edits to `contracts/operations/catalog.v1.yaml`, `audits.ts`, `cv-flywheel.spec.ts`, `ChatTranscript.tsx`, and `DockWorkspaceShell.tsx` in isolated `[SEQUENTIAL]` tasks using targeted single-row insertions (`git diff --numstat` sanity check by orchestrator).
- **Enforce Workspace Build & Crypto/LLM Policies:** Zero vendor SDKs/hostnames (`vox_actor_runtime::llm` only), zero `blake3`/`sha2`/`sha3` direct imports (`vox-crypto` only), zero `cmake`/`nasm`/`Go`/`perl`/`libclang` native C++ ML/ONNX crates in `Cargo.lock`, and `.vox` scripts only (`0` new `.py`/`.sh`/`.ps1` glue).

**Warning signs:**
- `zeroEmojiAndDingbat.test.ts`, `ipcBoundaries.test.ts`, `noPaperFallbackGuard.test.ts`, or `themeContract.test.ts` failing in `crates/vox-gui/ui`.
- `vox audit arch` (`vox-arch-check`) or `vox audit code` (`arch/god_object`, `llm_provider_call`, `vox/crypto/banned-crate-import`) emitting errors.

**Phase to address:**
All phases, with explicit frontend/contract convergence in `v7.6` Phase 6 — Parallel-Safe Scientia/Research Studio & Inline Before/After Audit Loop (`Phase 456`).

---

### Pitfall 7: Goodharting, Same-Family LLM-Judge Bias & Non-Deterministic Before/After Benchmarks

**What goes wrong:**
An inline Before/After audit + next-lesson loop meant to prove `v7.6` gains instead reports illusory `95%+` scores while real factual accuracy regresses — or swings `±25%` across runs so regressions cannot be distinguished from live-web noise:
1. **Current `vox-cli-research/src/eval.rs` Metrics Are Directly Goodhartable:**
   - [`multi_hop_pipeline_score`](../../../../crates/vox-cli-research/src/eval.rs#L364-L376) returns **`1.0` unconditionally** for every query lacking the literal substrings `"compare"`, `"trace"`, `"then"`, `"FRAMES-style"`, or `"BrowseComp-style"`, and on matching queries awards `1.0` merely for `subquery_count >= 2` and `sources_count >= 3` — even if Wave 2 completely ignored Wave 1's findings!
   - [`groundedness`](../../../../crates/vox-cli-research/src/eval.rs#L63-L64) computes bag-of-words `answer_keyword_hit_rate`, rewarding verbatim keyword stuffing over logical entailment.
   - [`citation_precision_from_answer`](../../../../crates/vox-cli-research/src/eval.rs#L325-L347) awards `1.0` whenever `[n]` satisfies `1 <= n <= evidence_count`, even when source `[n]` contradicts the sentence citing it! Meanwhile [`audit_citations`](../../../../crates/vox-research-shim/src/research/orchestrator/pipeline.rs#L1335-L1360) checks snippet overlap against cached spans rather than independent evidence verification.
2. **Same-Family LLM-Judge Self-Preference & Verbosity Bias:** In [`pipeline.rs:937-951`](../../../../crates/vox-research-shim/src/research/orchestrator/pipeline.rs#L937-L951), `judge_quality` often resolves `judge_model` from the same provider/family as `synthesis_model`. Per 2025–2026 *DeepResearch Bench* / NeurIPS perplexity-bias findings, same-family judges systematically inflate verbose, markdown-heavy reports from their own model family by `10–18` points.
3. **Live-Web Drift Destroys Before/After Reproducibility:** [`default_golden_queries`](../../../../crates/vox-cli-research/src/eval.rs#L475-L500) includes time-varying live queries (*"What is the current price of Ethereum in USD?"* at line `479`), SearXNG upstream ranking churn, and unseeded LLM temperatures. Comparing `Before` and `After` on live web hits measures SearXNG/network weather rather than algorithmic improvement, and feeding noisy "next-lesson" rules back into prompts causes prompt bloat and rubric overfitting.

**Why it happens:**
Heuristic proxy metrics written for fast smoke tests get reused as optimization targets for automated self-improvement ("next-lesson") loops — the textbook trigger for Goodhart's Law.

**How to avoid:**
- **Two-Track Before/After Harness (Deterministic Replay Cassettes + Stratified Live Canary):**
  1. *Deterministic Replay Track (CI & Before/After Gate):* Record frozen HTTP/search/scholarly cassette fixtures (including adversarial retracted-DOI, conflicting-source, and unanswerable/abstention cases) so `Before` and `After` run against **identical retrieved bytes** at `temperature = 0.0`.
  2. *Live Canary Track (Diagnostic Only):* Separate time-sensitive web queries from timeless repository/scientific multi-hop ground-truth queries.
- **Replace Toy Heuristics with Decomposed Binary Assertions:** Score (a) true cross-wave bridge entity recall (Wave 2 subquery contains entity discovered only in Wave 1), (b) claim-level entailment F1 against gold claim labels (including false-support penalty on contradicted sources), (c) calibration ECE / Brier score on abstentions, and (d) cost/latency efficiency (`verified_claims_per_dollar`, `TTFT_ms`).
- **Cross-Family / Decoupled Judge + Blinded Pairwise Order Shuffling:** Force `judge_model` in Before/After audits to select a distinct model family from `synthesis_model` via `vox-orchestrator::models`, randomize `(Before, After)` vs `(After, Before)` presentation order to cancel position bias, and gate "next-lesson" proposals through human-attested or held-out validation splits so lessons never train on the eval set.

**Warning signs:**
- `multi_hop_completion` sits at `1.00` while 1-wave and 3-wave answers are identical.
- Answer length doubles across "next-lesson" iterations while gold-fact recall stays flat.
- Re-running the exact same commit twice on `vox research eval` shifts `avg_quality` by `> 5%`.

**Phase to address:**
`v7.6` Phase 6 — Deterministic Cassette Benchmark, Cross-Family Before/After Audit & Guarded Next-Lesson Loop (`Phase 456`).

---

## Technical Debt Patterns

Shortcuts that seem reasonable during implementation but create severe downstream failures in Vox.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Appending Light/Deep/Socrates state into `build_system_prompt_with_skill` | Avoids threading turn-local metadata into `turn_user_prompt` | Busts Anthropic/OpenAI/DeepSeek KV-cache prefix on every turn (`8–12×` prompt token cost jump) | **Never** |
| Keeping legacy constructors (`MiniCheckVerifier::from_env()`) alongside calibrated ones | Zero-diff compatibility with older tests | Production call sites silently stay on `Mock` word-overlap (`pipeline.rs:96` divergence pattern) | **Never** (gate `mock()` behind `#[cfg(test)]`) |
| Calling `store_research_artifact(id, "{}", md)` during doc publish | Reuses existing FTS-indexing DB method in 1 line | Destroys persisted `artifact_json` (`claims`, `citations`, `DAG`) via `ON CONFLICT DO UPDATE` | **Never** |
| Emitting `// vox:skip` or ````mermaid` in synthesized `docs/src/architecture/*.md` | Lets LLMs dump unchecked pseudo-code and Mermaid charts | Hard-blocked by `vox-doc-pipeline` (`doctest.rs:77` & `mermaid_gate.rs:30`), breaking pre-push CI | **Never** |
| Storing full scraped HTML/PDF text in `ResearchHit.raw_content` inside `ResearchRunArtifact` | Simplifies post-hoc re-reading without a blob store | Multi-MB JSON clones in `broadcast::channel`, SQLite row bloat, and React OOM | Only for `<= 2 KiB` trimmed spans; spill raw text to content-addressed disk/DB chunks |
| Persisting unattested research `report_markdown` directly into `VoxDb::save_memory` | Instant recall in future chat sessions | Epistemic Ouroboros: future `ResearchScope::Both` runs ingest unattested LLM prose at `trust_score: 1.0` | **Never** without `untrusted_synthesis` taint quarantine |

---

## Integration Gotchas

Common mistakes when connecting `v7.6` research & publication stages to external and internal services.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| **OpenAlex API** (`vox-search/src/openalex.rs`, `trust.rs`) | Relying on retired `&mailto=` polite pool (`SecretId::VoxOpenAlexEmail`) or custom `api-key` header with `None` passed from `web_dispatcher.rs` | Resolve OpenAlex API key via Clavis (`api_key=` param or `Authorization: Bearer`), enforce `20 RPS` keyed / `3 RPS` anon limiter, and cache responses in `VoxDb` |
| **Crossref REST API** (`vox-search/src/trust.rs`) | Hitting `/works/{doi}` with no `?mailto=` and `30s` timeout across parallel hits (lands in Public Pool with **concurrency limit = 1**) | Include `?mailto=` + `User-Agent` (Polite Pool: `10 RPS`, concurrency `3`), bound timeout to `3.5s`, and serialize via a `Semaphore(3)` + `VoxDb` DOI cache |
| **Semantic Scholar API** (`vox-publisher` / novelty) | Firing parallel prior-art searches per extracted claim (`10–24` concurrent requests) | Enforce strict `1 RPS` (`Semaphore(1)` + `1050 ms` pacing) even when `SecretId::VoxSemanticScholarApiKey` is set; batch or pre-cluster claims into `<= 3` topic queries |
| **LLM Providers** (`vox_actor_runtime::llm`) | Instantiating vendor HTTP URLs (`api.openai.com`, `openrouter.ai`, `VOX_MINICHECK_ENDPOINT` via `std::env::var`) | Route 100% of chat, planner, entailment, embedding, and critic calls through `vox_actor_runtime::llm` + `vox-orchestrator::models` + Clavis `resolve_secret` |
| **Zenodo / OpenReview / Crossref Deposit** (`vox-publisher`) | Triggering external archival directly from autonomous background discovery without human gate or sandbox dry-run | Default autonomous Direct Publication to local DB + lint-verified repo `status: research` draft + staged RO-Crate bundle; require explicit operator promotion + Zenodo Sandbox preflight before external minting |
| **Axis GUI IPC** (`crates/vox-gui/ui`) | Calling Tauri `invoke()`/`listen()` directly or shelling out to `vox scientia ...` subprocesses (`PublicationsView.tsx` Gap 10) | Route exclusively through typed `crates/vox-gui/ui/src/transport.ts` → `/api/v2/{research,scientia}/*` daemon endpoints with `0` persistent `VoxDb` handles in `vox-gui` |

---

## Performance Traps

Patterns that pass unit tests on 3-hit toy queries but collapse under extreme-data multi-wave research.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| **3× LLM resampling on every claim without pre-filtering** (`verifier.rs:153`) | `180s+` run duration; `$0.50–$2.00` burned per Deep query | Tier 0 lexical/numeric containment pre-filter + Tier 1 single call + Tier 2 resampling *only* inside margin band `[abstain, 0.80]` | `> 12` claims or `>= 2` waves |
| **Inline synchronous `run_deep` inside `vox_chat_message`** (`research_turn.rs:596`) | MCP/HTTP tool call times out at `180s`; zero tokens streamed to user until completion | Async session handle + live `after_seq` SSE stage/wave/partial-synthesis streaming | `>= 2` waves or slow search provider |
| **Per-event `tokio::spawn` DB inserts + 5s `VoxDb::connect_default()` polling** (`pipeline.rs:1304`, `scientia_feed.rs:59`) | `SQLITE_BUSY` lock contention, connection churn, dropped events | Bounded MPSC batch writer (`250 ms` / `64` events per tx) + push-driven watermark notification over shared `ServerState.db` | `>= 3` concurrent research sessions or `> 100` events/run |
| **Cloning `ResearchResult` with full `raw_content` into `ResearchRunArtifact` & `broadcast::channel`** | Daemon RSS spikes `> 1 GiB`; `BroadcastStreamRecvError::Lagged` drops UI frames | Spill raw scraped text to BLAKE3-keyed chunk store; cap in-memory `ResearchHit.snippet` to `<= 2 KiB` and wrap shared events in `Arc` | `> 40` scraped pages or PDF/long-HTML sources |
| **Sequential `30s`-timeout `TrustScorer` calls per hit** (`web_gather.rs:34`, `trust.rs:24`) | Retrieval stage hangs for minutes when Crossref/OpenAlex throttles | Batch/dedupe by domain+DOI, cap per-lookup timeout at `3.5s`, memoize in `VoxDb` | `> 15` academic hits on throttled network |

---

## Security Mistakes

Domain-specific security and epistemic-integrity hazards in autonomous web research and publication.

| Mistake | Risk | Prevention |
|---------|------|------------|
| **Indirect Prompt Injection via Scraped Web Snippets / Titles** | Hostile web page injects fake `[1]` citations, system instructions, or tool-call directives into Chat or Direct Publication synthesis | Enforce `SanitizedText` + newline flattening (`research_turn.rs:195-212`) across *every* Deep wave, verifier prompt, novelty title, and manuscript scaffolder input — not only `run_quick` |
| **LaTeX `\write18` / `\input` / `\end{verbatim}` Breakout** (`render.rs:107, 186, 305`) | Attacker-controlled paper title, URL, or code snippet breaks out of `%` comment or `verbatim` block and reads local files during `pdflatex` compilation | Strip `\r`/`\n` in all `%` comments, sanitize `\end{verbatim}`, escape URLs, and run any local `pdflatex`/`tectonic` check with `-no-shell-escape` and `openin_any=p` (paranoid path confinement) |
| **Path Traversal in Slug & RO-Crate / arXiv `.tar.gz` Staging** (`render.rs:177`) | Crafted `FigureEntry.path` (`../../.env`) or `slug` bundles secrets or overwrites workspace files | Enforce `research_doc_target` traversal rejection (`gui_ops/research.rs:680-683`) AND canonicalize/confine every `FigureEntry.path` and RO-Crate artifact inside the staging tempdir |
| **Terminal / Workspace Secret Leakage in Autonomous Discovery** | `vox scientia scout` or `@term` / repo local search embeds API keys or local absolute user paths into published manuscripts or Zenodo bundles | Pass all candidate evidence, `ScaffoldInput`, and `ArchitectureSsotInput` through `vox-redact` + Clavis exact-secret scrubbing before DB persistence and export |
| **Bypassing `vox-crypto` for Provenance Hashes** | Ad-hoc `sha2`/`blake3` crate imports violate `vox/crypto/banned-crate-import` and drift hash formats (`blake3:<64hex>` vs `sha3_256`) | Compute all KB source hashes, Trusty URIs, and RO-Crate digests exclusively through `crates/vox-crypto` |

---

## UX Pitfalls

Common operator-experience failures in unified research and scholarly publication interfaces.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| **Pre-search Socratic interrogation** | User asks a research question in Chat and gets quizzed instead of answered | Run `<2s` Light search silently first; ask a clarifying question *only* when user intent (not world knowledge) has multiple incompatible interpretations |
| **Opaque binary jump between 2s Chat reply and 120s Deep Research** | User cannot tell whether Vox is frozen, searching 1 hop, or running 3 waves | Show a compact inline Continuum Pill in Chat (`Light · 1 hop · 6 sources` with 1-click *"Escalate to Deep (3 waves)"*, or live `Wave 2/3 · Verifying 14 claims · $0.04` progress stream) |
| **Dead-end Discovery Inbox items (`finding-<uuid>` `"not_found"`)** | Clicking `"Open review"` in `DiscoveryInbox.tsx` fails because no `publication_manifests` row exists (Gap 2) | Provide atomic 1-click `"Promote to Publication Manifest"` from both `DiscoveryInbox` and `ResearchView`, auto-hydrating claims and novelty bundle |
| **Presenting AI-drafted prose as peer-review-ready IMRaD narrative** | Violates SCIENTIA's own worthiness rubric (`render.rs:78-96`) and erodes scientific trust | Clearly visually distinguish *Machine-Verified Empirical Sections* (Claims Ledger, Methods Provenance, Replay Digest, Novelty Overlap) from *Author Narrative Slots / Assisted Drafts* requiring human attestation |
| **Emoji/dingbat clutter or raw `<select>` controls in new UI cards** | Breaks Romanesque/Basalt visual cohesion and fails `v7.4` CV/Axe gates | Use stroke-normalized `<VoxIcon>` / Romanesque SVGs, Radix `data-slot` controls, and `<BentoScrollRegion>` containers across all 4 themes |

---

## "Looks Done But Isn't" Checklist

Verification gates for executors and reviewers before marking any `v7.6` phase complete:

- [ ] **KV-Cache Prefix Stability:** `build_system_prompt_with_skill` output and serialized MCP tool schemas are byte-identical across `ResearchMode::None`, `Quick`, and `Deep` turns in the same session — verify via unit test + non-zero `cache_read_tokens` on turn 2.
- [ ] **All Call Sites Wired (Zero Divergence):** Every constructor/caller of `MiniCheckVerifier`, `TrustScorer`, `NoveltyEvidenceBundle`, and `ClaimVerdict.resample_stability` is grep-audited — verify `MiniCheckVerifier::from_env()` is deleted and `0` production paths hit `MiniCheckBackend::Mock`.
- [ ] **Doc Publish Preserves `artifact_json`:** Calling `publish_doc` / `write_research_doc` on a completed session leaves `get_research_artifact(session_id).artifact_json` intact (`claims.len() > 0`) AND the written `docs/src/architecture/<slug>-2026.md` passes `cargo run -p vox-doc-pipeline -- --lint-only --paths <file>` with `0` errors (`0` `vox:skip`, `0` ````mermaid`, valid `authorship_model`, 40–120 word `kb:summary`).
- [ ] **LaTeX Compiles on Adversarial Inputs:** `render_latex` output hydrated purely from `VoxDb` (`--publication-id`, zero manual JSON files) compiles cleanly with zero errors when claims/titles/URLs contain `≤`, `Δ`, `%`, `#`, `_`, `\n`, and `\end{verbatim}`.
- [ ] **Offline Graceful Degradation:** With network disabled (`127.0.0.1:9` blackhole), Light/Deep local research, two-stage novelty (Stage 1 local + `OfflineAttested` trace), candidate promotion, and DB-hydrated Direct Publication all succeed without panics or hard `TrustLookup::Unavailable` deadlocks.
- [ ] **Epistemic Ouroboros Blocked:** Autonomous research outputs saved to `VoxDb` carry untrusted/unattested provenance and never re-enter `web_gather.rs` with `trust_score: 1.0` unless human-attested.
- [ ] **`v7.4` / `v7.5` Non-Regression:** `zeroEmojiAndDingbat.test.ts`, `ipcBoundaries.test.ts`, `noPaperFallbackGuard.test.ts`, `themeContract.test.ts`, `vox ci secret-env-guard`, and `vox audit code` + `vox audit arch` all pass with `0` new warnings/errors.

---

## Recovery Strategies

When a pitfall slips past initial implementation, how to recover cleanly without destructive rollbacks.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| **KV-cache prefix regression in Chat** | LOW | Diff rendered `system_prompt` + `tools` JSON across turns 1 and 2; relocate any dynamic research/Socrates/budget string from `chat_tools/mod.rs` into `preamble_and_retrieval.rs` (`turn_user_prompt`). |
| **`write_research_doc` wiped `artifact_json` rows in local DB** | MEDIUM | Fix `write_research_doc` to update only `report_markdown` / doc index; reconstruct missing `artifact_json` for affected sessions from `scientia_claims` + `scientia_research_sources` tables in `VoxDb`. |
| **Scholarly API 429 ban / daily credit exhaustion (OpenAlex / Semantic Scholar)** | LOW | Trip provider circuit breaker to `Open`, serve cached `VoxDb` novelty/trust records, fall back to Stage 1 local MinHash/BM25 novelty + arXiv/Crossref Polite Pool until UTC midnight reset, and store free OpenAlex API key via `vox secrets`. |
| **Synthesized SSOT broke `vox-doc-pipeline` pre-push gate** | LOW | Run `render_architecture_ssot` sanitizer pass over the generated `.md` file (convert ````mermaid` → ````vox-diagram`, non-compiling ````vox` → ````text`, add `authorship_model`, set `status: "research"`) and re-verify with `--lint-only --paths`. |
| **Ouroboros memory poisoning from unattested runs** | MEDIUM | Filter/purge `memories` rows where `agent_id = 'vox-research'` and `memory_type = 'research_result'` lacking human attestation, and enforce provenance discount in `web_gather.rs`. |
| **Merge collision with `v7.4` / `v7.5` on `ResearchView.tsx` or `catalog.v1.yaml`** | MEDIUM | Extract `v7.6` UI additions into standalone child components, re-apply the 5-line mount against `v7.4`'s `<BentoGrid>`/`transport.ts` base, and regenerate catalog artifacts via `operations-sync --write` rather than hand-merging generated files. |

---

## Pitfall-to-Phase Mapping

How `Milestone v7.6` (`scientia-deep-research-v76`, sequential slot `Phases 451–456` after `v7.5` `Phases 441–446`) must sequence prevention and verification.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| **Pitfall 1:** Chat KV-cache prefix invalidation (`v2.9–v3.2` day-stable invariant) | **Phase 1 (`451`)**: Unified Light ↔ Deep Continuum Router & Chat Harness | Unit test asserting byte-identical `system_prompt` + `tools` across `None`/`Quick`/`Deep` turns; telemetry test asserting `cache_read_tokens > 0` on turn 2+ |
| **Pitfall 2:** Extreme-data OOM, `BroadcastStream` `Lagged` drops & `180s` Chat timeout | **Phase 2 (`452`)**: Extreme-Data Streaming, `after_seq` Backpressure & Batched Telemetry | Prime-lagged SSE consumer test recovering via `after_seq`; 5-wave / 50-hit stress test with bounded heap (`raw_content` externalized) and `0` `SQLITE_BUSY` errors |
| **Pitfall 3:** Socrates over-questioning fatigue, 3× verifier cost blowup & `Mock` call-site divergence | **Phase 3 (`453`)**: Socrates Epistemic Gate & Calibrated 3-Tier Claim/Citation Verifier | Epistemic-vs-specification gate test (`0` pre-search questions on factual queries); grep audit confirming `MiniCheckVerifier::from_env()` removed; `< 1.4` avg LLM calls/claim with Tier 0 pre-filter |
| **Pitfall 4:** Retired OpenAlex `mailto=`, Crossref/S2 429 cliffs & offline promotion deadlock | **Phase 4 (`454`)**: Two-Stage Novelty Engine, Rate Governors & Offline Scholarly Fallback | WireMock 429/blackhole suite proving `1 RPS` S2 / `3-concurrency` Crossref Polite / Bearer OpenAlex auth + offline `OfflineAttested` promotion succeeding |
| **Pitfall 5:** `artifact_json` wipeout, `vox-doc-pipeline` rejections, LaTeX breakouts & Ouroboros taint | **Phase 5 (`455`)**: DB-Hydrated Direct Publication (`> Nanopub`), Lint-Clean SSOT & Venue LaTeX | Round-trip test: `publish_doc` preserves `artifact_json`, passes `vox-doc-pipeline --lint-only`, passes adversarial Unicode/URL LaTeX test, and quarantines unattested `save_memory` hits |
| **Pitfall 6:** Parallel workstream collisions (`v7.4` GUI / `v7.5` Terminal) & god-object/layer guards | **Phase 6 (`456`)** (enforced across Phases 1–6): Parallel-Safe Scientia/Research Studio | `zeroEmojiAndDingbat`, `ipcBoundaries`, `noPaperFallbackGuard`, `themeContract`, `vox audit arch`, and `vox audit code` (`< 500` LoC new files) all green |
| **Pitfall 7:** Goodharted `eval.rs` heuristics, same-family judge bias & live-web drift | **Phase 6 (`456`)**: Deterministic Cassette Before/After Audit & Guarded Next-Lesson Loop | Frozen-cassette Before/After replay variance `< 1%`, cross-family blinded pairwise judge check, and adversarial false-citation / fake-multi-hop rejection tests |

---

## Sources

### Vox Codebase & Internal Audits (Verified 2026-11-14)
- [`docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`](../../../../docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md) — 10 concrete backend/GUI illusions (`Gaps 1–10`).
- [`docs/src/architecture/deep-research-implementation-divergence-audit-2026-08-01.md`](../../../../docs/src/architecture/deep-research-implementation-divergence-audit-2026-08-01.md) — Stage 1 divergence root-cause analysis (under-specified call-site wiring, NLI substitution, unpopulated `NoveltyEvidenceBundle`, `self_consistency` collision).
- [`docs/src/contributors/documentation-governance.md`](../../../../docs/src/contributors/documentation-governance.md) — KB facet contract, `status` vocabulary, `B-canon` authority rules, and doctest/Mermaid policies.
- [`crates/vox-orchestrator-mcp/src/chat_tools/mod.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/mod.rs#L193-L379) & [`crates/vox-orchestrator/src/cache_predictor.rs`](../../../../crates/vox-orchestrator/src/cache_predictor.rs) — Day-stable system prompt prefix seams and `CachePredictor`.
- [`crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs#L596-L650) & [`crates/vox-research-shim/src/research/orchestrator/pipeline.rs`](../../../../crates/vox-research-shim/src/research/orchestrator/pipeline.rs) — Synchronous `run_deep`, `save_memory` Ouroboros seam, and `emit_research_event` spawning.
- [`crates/vox-orchestrator-mcp/src/gui_ops/research.rs`](../../../../crates/vox-orchestrator-mcp/src/gui_ops/research.rs#L700-L723) & [`crates/vox-db/src/research_pipeline.rs`](../../../../crates/vox-db/src/research_pipeline.rs#L454-L478) — `write_research_doc` → `store_research_artifact(id, "{}", content)` overwrite bug.
- [`crates/vox-doc-pipeline/src/pipeline/doctest.rs`](../../../../crates/vox-doc-pipeline/src/pipeline/doctest.rs#L70-L88) & [`mermaid_gate.rs`](../../../../crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs#L21-L38) — Hard `vox:skip` ban and shrink-only legacy Mermaid baseline ratchet.
- [`crates/vox-scientia/src/manuscript/latex/render.rs`](../../../../crates/vox-scientia/src/manuscript/latex/render.rs) & [`escape.rs`](../../../../crates/vox-scientia/src/manuscript/latex/escape.rs) — T1/UTF-8 math crash, `%` newline breakout, `\end{verbatim}` injection, and `longtable` `\url{}` hazards.
- [`crates/vox-scientia/src/claim_extractor/minicheck.rs`](../../../../crates/vox-scientia/src/claim_extractor/minicheck.rs#L75-L97), [`crates/vox-search/src/openalex.rs`](../../../../crates/vox-search/src/openalex.rs#L111-L151), [`crates/vox-search/src/trust.rs`](../../../../crates/vox-search/src/trust.rs#L49-L162), and [`crates/vox-cli-research/src/eval.rs`](../../../../crates/vox-cli-research/src/eval.rs#L325-L395).
- Active parallel workstreams: [`.planning/workstreams/axis-live-functional-v74/ROADMAP.md`](../../axis-live-functional-v74/ROADMAP.md) (`Phases 431–436`) and [`.planning/workstreams/axis-warp-terminal-v75/ROADMAP.md`](../../axis-warp-terminal-v75/ROADMAP.md) (`Phases 441–446`).

### External 2025–2026 Prior Art & Upstream API Contracts
- **Prompt / KV-Cache Prefix Engineering (2025–2026):** Anthropic Prompt Caching (`cache_control` breakpoints, 1.25× write vs 0.10× read), OpenAI & DeepSeek automatic prefix caching (`>=1,024` token byte-identical prefix requirement; tool-schema & early system-prompt mutation invalidation).
- **Scholarly API Rate-Limit & Auth Shifts (2025–2026):** OpenAlex retirement of the `mailto=` polite pool in favor of API keys (`api_key=` / `Bearer`, `$0.10/day` anonymous vs `$1.00/day` + `30 RPS` free key); Crossref REST API Pool tiers (Public Pool concurrency=1 vs Polite Pool `?mailto=` `10 RPS` / concurrency `3–5`); Semantic Scholar Academic Graph API (`1 RPS` standard authenticated limit).
- **Claim Verification & Socratic Friction (2024–2026):** *MiniCheck* (Tang et al., EMNLP 2024 / 2025–2026 compact reasoning verifiers on *LLM-AggreFact*), *FActScore* atomic claim explosion, *SciFact* (`Support | Contradict | NotEnoughInfo | Abstain`), and Socratic agent clarification-fatigue studies (distinguishing specification uncertainty from retrievable epistemic uncertainty).
- **Deep Research Agent Evaluation & Goodharting (2025–2026):** *DeepResearch Bench*, *BrowseComp*, and *FRAMES* multi-hop evaluation methodology; NeurIPS/ICLR analyses of perplexity-driven LLM-as-a-judge self-preference bias, verbosity bias, and position shuffling.

---
*Pitfalls research for: Milestone v7.6 — Unified Light ↔ Deep Research Continuum, Extreme-Data Streaming, Socrates/Novelty Verification & Continuous Direct Publication Flywheel*
*Researched: 2026-11-14*
