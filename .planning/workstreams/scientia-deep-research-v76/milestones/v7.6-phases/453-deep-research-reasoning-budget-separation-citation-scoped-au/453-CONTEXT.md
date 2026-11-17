# Phase 453 Context — Deep Research Reasoning-Budget, Citation Audit & Daemon DB Proxy

**Status:** Locked from November 2026 Live Parallel Deep-Research & Scientia Funnel Audit (`LIVE-09..12`)
**Workstream:** `.planning/workstreams/scientia-deep-research-v76/`

## Locked Decisions

1. **`LIVE-09` — Reasoning-Token vs. Visible-Output Budget Separation:**
   - Frontier thinking models (`gemini-4-carbon`, DeepSeek R1/Pro, OpenRouter reasoning models) currently share `synthesis_max_tokens = 4000`, causing mid-sentence Markdown truncation or `0`-char visible output (`finish_reason = "length"`, `reasoning_tokens > 0`, `content.is_empty()`).
   - Raise `ResearchConfig::default().synthesis_max_tokens` to `12_000` (`judge_max_tokens` to `6_000`).
   - Add optional `reasoning_max_tokens: Option<u64>` on `vox_llm_egress::ChatParams` and `vox_actor_runtime::llm::LlmConfig` (serialized as `reasoning: { "max_tokens": N }` on OpenRouter wire payloads when set so internal thinking never starves visible completion).
   - In `vox_actor_runtime::llm::cascade::chat_with_cascade` and `vox-research-shim::stages`, treat a response with `content.trim().is_empty()` and (`finish_reason == Some("length")` or `reasoning_tokens > 0`) as a reasoning-budget exhaustion failure and retry/fall through cleanly to the next cascade candidate instead of returning an empty report.

2. **`LIVE-10` — Citation-Scoped Precision, Contrastive `"not only"` Guard & Domain-Mode Cache Hygiene:**
   - Parse 1-based `[N]` inline citation markers from the synthesized `answer` in `pipeline.rs`. Extract/verify claims across referenced cited hits (not solely `all_hits[..8]`), and evaluate `audit_citations` precision over citations actually referenced `[N]` in `answer` (falling back to `citations` only when zero `[N]` markers appear).
   - Scrub contrastive affirmative idioms (`"not only"`, `"not just"`, `"not merely"`) before `has_negation_token` in `crates/vox-research-shim/src/research/verifier.rs` so `"not only X, but also Y"` never triggers false negation conflict.
   - Include `query.domain_mode`, `query.waves`, and `query.site_scope` in `research_cache_key` (`pipeline_cache.rs`) and refuse to store or serve empty answers or `[INSUFFICIENT_EVIDENCE]` abstentions.

3. **`LIVE-11` — Markdown-Aware `AtomicDecomposer`, Novelty Overlap Projection, Empirical Science Class & Research Cost Rollup:**
   - Strip/mask YAML frontmatter (`--- ... ---`), HTML comments (`<!-- ... -->`), and Markdown table separator/pipe chrome in `vox_scientia::claim_extractor` while preserving source-accurate byte offsets for `SpanChecker`.
   - Project `prior_art_max_lexical_overlap` (`max_lexical_score`) and `prior_art_query_succeeded` into `metadata_json.scientia_evidence` inside `merge_novelty_bundle_into_metadata_json_str` (`crates/vox-cli-research/src/db/publication/discovery.rs`) so `publication-discovery-scan` drops `novelty_overlap_assumed_default`.
   - Add `EmpiricalScience` classification support to `FindingCandidateClass` (`vox-research-events`), `FindingClass` (`vox-scientia::class_routing`), and `infer_candidate_class` (`discovery_bridge.rs`) for materials/chemistry/physics/biology queries.
   - Include `research` / `general` / `codegen` / `shopping` / `scholarly` runs in `vox_scientia::dashboard::cost::CostInputs` (`#[serde(default)] pub research_usd: f64`) and `QuarterlyCostSummary.total_usd` (`review_flow.rs`).

4. **`LIVE-12` — Transparent Daemon IPC Fallback When Canonical `vox.db` Is `flock`-Locked:**
   - When `VOX_DB_PATH` is not explicitly overridden and `VoxDb::connect` fails with a Turso/POSIX file lock error (`"Failed locking file"` / `"Resource temporarily unavailable"` / `"locked"`), `vox research` and `vox scientia` commands automatically proxy reads/writes over the running `vox-orchestrator-d` daemon (`daemon_tool::daemon_client()` via `API_V2_GET`, `TOOL_CALL`, and `RESEARCH_RUN`).
