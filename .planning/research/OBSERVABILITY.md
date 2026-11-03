# Observability & Evidence — Research Trial Flywheel

Verified against the working tree on `feat/research-trial-flywheel` (2026-09-30). Paths are relative to the repo root.

## 1. Current telemetry surface and gaps in trial-identity propagation

**What exists**
- `crates/vox-telemetry/src/types.rs:271-334`: `TelemetryEvent` is a serde-tagged (`event_type`) `#[non_exhaustive]` enum. Nothing enforces a key allowlist. The only write-time guard is `validate_research_metric_row` (`types.rs:214-261`). It checks length caps (`session_id` ≤512, `metric_type` ≤128 with a charset rule, `metadata_json` ≤256 KiB). For `model_route_event` it also does a **substring** check for `"trace_id"`/`"route_policy_profile"` (`types.rs:251-257`). That check is not structural: an unknown key such as `"query"` passes.
- `recorder.rs:13-26`: the global recorder is a first-writer-wins `OnceLock`. `CompositeRecorder::record` (`recorder.rs:39-47`) fans out unfiltered. It is the natural single choke point for an allowlist, but it has no filtering today.
- `span.rs:12-23`: `TraceContext` carries `task_id: u64`, `trace_id: Uuid`, `span_depth` and `caller_agent_id`. There are no campaign, run, trial, stage or config-hash fields.
- `crates/vox-research-events/src/events.rs:43-161`: `ResearchEvent`. Several fields carry content or free text: `ClaimExtracted.text` (:67), `FindingCandidateProposed.finding_candidate` (full JSON, :99), `CampaignAborted.reason` (:124), `PublicationFailed.error` (:141). `CampaignStarted`/`CampaignAborted` exist but are emitted only in a unit test (`emitter.rs:45`).
- `crates/vox-research-shim/src/research/research_event_metrics_bridge.rs:53-61,101-104`: a hard-coded `matches!` metric-name allowlist. This is useful precedent. However, `FindingCandidateProposed` copies the whole candidate JSON into `research_metrics` (:140-142). The contract `contracts/telemetry/research-event-bridge.v1.schema.json` declares `"additionalProperties": true`.
- `ModelCallEvent` (`types.rs:428-450`) already has tokens, latency, cost and trace fields, plus a free-text `selection_rationale`. Only `crates/vox-orchestrator-mcp/src/llm_bridge/infer.rs:613` emits it. **The research pipeline never does.**

**Gaps (trial identity)**
- **Every LLM call in a research run gets its own `trace_id`.** The research pipeline never scopes `TRACE_CTX`, so `infer_with_retry` mints a fresh UUID per call (`crates/vox-actor-runtime/src/llm/chat.rs:407`).
- **LLM turn records can't be joined to a run.** Research never sets `telemetry_session_id`, so `record_telemetry_outcome` files every turn under `"anon-session"` (`chat.rs:287-290`).
- **Session identity collides when a query is repeated.** `session_key = fnv1a(query|scope)` (`orchestrator/pipeline.rs:106-109`). It is inserted with `INSERT OR IGNORE` and then read back with `last_insert_rowid()` (`crates/vox-db/src/research_pipeline.rs:33-42`). A second trial of the same query therefore gets a stale or foreign id. This is a race bug for campaigns.
- **The eval `run_id` never reaches the pipeline.** `vox-cli-research/src/eval.rs:132` mints a `run_id`, but `run_research_with_context` (:58-60) never receives it.
- **There is no per-stage timing.** `set_session_stage` overwrites `finished_at_ms` on every status update (`research_pipeline.rs:58-60`).
- **Stage→model attribution is partial and depends on the caller.** `record_research_model_uses` (`cascade.rs:54-64`) records `(stage, requested, resolved)` only. It carries no tokens, latency or cost, and only the MCP path wraps it (`chat_tools/chat/research_turn.rs:622`). The CLI, eval and campaign paths do not.

## 2. Proposed structural event schema and allowlist mechanism

**Identity:**
- `campaign_id` and `run_id` (ULIDs).
- `arm_id`, `stage` (closed enum mirroring `cascade::ResearchStage` plus `retrieve`, `audit` and `persist`), `seq` and `attempt`.
- `config_hash` (BLAKE3 over the canonicalized resolved `ResearchConfig` plus model pins), `query_hash`, `prereg_digest`, and `build_version` (`vox-build-meta`).

**Propagation:** add a `ResearchTrialCtx` task-local, or extend `TraceContext` with `Option<TrialIds>`. Scope it once at run entry. Have `chat_stage_with_model` stamp `LlmConfig.telemetry_session_id = run_id` and `telemetry_trace_id`. Replace the fnv `session_key` with `run_id`.

**New `TelemetryEvent::ResearchTrial(..)` variants.** All fields are IDs, hashes, enums or numbers. There are no strings outside typed newtypes.
- `Campaign { campaign_id, prereg_digest, arm_count, cost_cap_usd_micros, status: Started|Completed|Aborted, reason_code }`
- `Run { campaign_id, run_id, arm_id, config_hash, query_hash, build_version, status, duration_ms, served_from_cache, evidence_manifest_hash, totals{tokens_in,tokens_out,cost_usd_micros,llm_calls,tool_calls} }`
- `Stage { run_id, stage, seq, status, started_at_ms, duration_ms, reason_code }`
- `ModelCall { run_id, stage, attempt, requested_model_id, resolved_model_id, provider_id, prompt_tokens, completion_tokens, cache_read_tokens, latency_ms, ttft_ms, ttft_source: measured|equals_latency|none, cost_usd_micros, cost_source: provider|estimate|none, error_class, request_hash, response_hash }`
- `ToolCall { run_id, stage, tool_id (enum: provider/engine id), status, elapsed_ms, result_count, response_hash }`

**Allowlist (lands before any producer):**
- Create `contracts/telemetry/research-trial.v1.yaml`, with one row per event×key: type, bound (max length or enum set) and sensitivity (S0/S1). Register it in `contracts/telemetry/events.v1.yaml`, which already carries `tier` and `retention_days`.
- Generate Rust consts and validators from that contract.
- Enforce at two points:
  - (a) `CompositeRecorder::record`, before fan-out.
  - (b) `validate_research_metric_row`: parse `metadata_json`, reject unknown keys, and reject any string longer than the declared bound.
- Deny-list as defence in depth: `query|answer|url|title|snippet|excerpt|prompt|response|text|content|reason|error`.
- Add `#[serde(deny_unknown_fields)]` on payload structs. Type strings as newtypes (`ModelId`, `Hash`, `ReasonCode`) whose constructors validate charset and length.
- Strict mode (an error) in tests and CI. Prod behavior is a decision: drop and count (`telemetry.rejected_total{event,key}`), never panic.

## 3. Latency / TTFT / tokens / cost / tool calls: where captured today

| Signal | Captured at | Reaches research telemetry? |
|---|---|---|
| Per-call latency | `chat.rs:182` (`LlmResponse.latency_ms`) → unified LLM turn row | No. `response_to_content_and_model` keeps only `(content, model)` (`orchestrator/stages.rs:553-558`) |
| TTFT | Non-streaming sets `ttft_ms = latency_ms` (`chat.rs:217`). That is not real TTFT | No, and it would be misleading without `ttft_source` |
| Tokens (input, output, cache-read) | `LlmResponse` (`chat.rs:208-216`) | No (dropped at `stages.rs:557`) |
| Cost | Provider-reported, else `estimate_cost(cost_per_1k)` (`chat.rs:173-180`). `cost_source` is lost and `f64` is used | No |
| Model tool calls | `LlmResponse.tool_calls` | No (dropped) |
| Search tool calls | `ProviderOutcome`, aggregated per provider. `elapsed_ms` = slowest call (`research/provider.rs:27-40`); plus Tavily credits | Only inside the `ResearchResult` diagnostics. No per-call events |
| Run duration | `pipeline.rs:927`, measured **before** self-verification (:931-948), the citation audit and persistence | Undercounts |
| Eval latency | `eval.rs:62` | Includes cache hits (`served_from_cache`, `pipeline.rs:65-73`) |
| Stage outcome | `PromptDispatch` (`cascade.rs:117-124`): stage and outcome, but `error` is free text and there are no tokens | Partial |

Content-leak finding: `record_telemetry_outcome` persists the **full prompt and response text** through `record_unified_llm_turn` (`chat.rs:304-340`). That breaks the "structural-only" ruling unless the table is reclassified as governed evidence.

## 4. Evidence capture and Tier C options (decisions required)

**What the repo actually has:**
- **`vox-spool` does not exist.** It is scheduled for M-03 (`docs/src/architecture/data-storage-ssot-2026.md:114-117`). The Tier B writer today is `crates/vox-cli/src/telemetry_spool.rs` (`enqueue`/`prune`/`ack`, one JSON per file).
- **`vox-checksum-manifest` is not in `crates/` at all.** The SSOT says it was a release-asset verifier, "NOT a blob store" (`data-storage-ssot-2026.md:81`, F62 :246). The milestone's crate names are therefore aspirational.
- **The existing CAS is `vox-db`'s `objects`/`names` tables** (`schema/domains/cas_codex.rs:3-8`, `VoxDb::store`/`get` in `store/ops_cas.rs:16-51`). It uses SHA3-512 Base32Hex with blobs stored inline in the DB. That contradicts Tier C ("rows store only `sha256` + `size_bytes`", `data-storage-ssot-2026.md:80`). It also imports `sha3` directly (`vox-db/src/lib.rs:110-121`), which the crypto policy forbids outside `vox-crypto`.
- **Current evidence storage is lossy and unhashed.** `web_gather.rs:350-389` writes the URL and title into research sources and ingests the raw body into the knowledge base with `content_hash: String::new()` (:374). `scientia_research_artifacts` stores `artifact_json` and `report_markdown` inline (`scientia.rs:190-196`). LLM requests and responses go only to the unified turn table.

**Options:**
- **(A)** A `vox-db::cas` submodule: filesystem `$VOX_DATA_DIR/artifacts/<algo>/<2>/<rest>` plus a `cas_blobs` index row. This is the SSOT's default assumption (§5.4, :130).
- **(B)** A new `vox-cas` crate. It needs a layer row in `docs/src/architecture/layers.toml`, and any new crate edge needs a **user-authorized** exception.
- **(C)** Reuse `objects` as-is. Fastest, but violates the 4 KiB indirection rule and bloats `store.db`.

**Decision questions for the user:**
1. **Ownership.** A, B or C? Who owns garbage collection, and is it `vox db doctor --gc` or a new command?
2. **Hash.** BLAKE3 (crypto policy) or SHA-256 (the SSOT path layout)? Migrate or dual-index the existing SHA3-512 `objects` table? Should the manifest hash cover canonical JSON (and which canonicalization)?
3. **Retention, per class.** Raw page bodies, LLM requests/responses, manifests and telemetry each need a kind from `contracts/db/retention-policy.yaml` (`keep_forever|manual|days|…`). Do evidence classes need new kinds?
4. **Deletion and tombstones.** On delete, keep `(hash, size, class, deleted_at, reason_code)` so replay reports "evidence redacted" instead of failing silently. Who may delete, and do campaign results get invalidated?
5. **Encryption and access.** Seal at rest with `vox-crypto` ChaCha20-Poly1305 and a key via `vox-secrets`? Is evidence MCP-readable? Is bundle export allowed? Third-party page bodies raise licensing questions for redistribution.
6. **Tier A references.** Does the `runs` row store `evidence_manifest_hash` + `size_bytes` with a foreign key to the index? What happens to the reference count when a run row is purged?
7. **Interim Tier B.** Build trial events on `research_metrics` now, or wait for M-03 `vox-spool`? Is evidence capture independent of the telemetry consent switch (`vox-telemetry/src/config.rs:126-166`)?

## 5. Evidence bundle contents and replay command shape

**Manifest** (canonical JSON, Tier C, hashed). Every leaf is `{hash, size, class}`:
- Ids: campaign, run, arm, prereg digest, `build_version`.
- Config snapshot: resolved `ResearchConfig`, per-stage model pins, temperature and `max_tokens`, search policy, lane, waves, and whether the cache was bypassed.
- Plan: subqueries, `planner_degraded`.
- Retrieval: per call, a request descriptor (provider id, query hash) and the response blob (hits with `raw_content`, `http_status`, `trust_score`).
- LLM: per call, request messages and params plus the response (content, `resolved_model`, usage, cost source).
- Derived: claim verdicts, judge output, citation audit, corroboration, competence, the final answer and report.
- The run's structural telemetry events, and a hash index over everything above.

**Commands:**
- `vox research replay <manifest-hash|path> [--verify-only] [--from-stage <stage>] [--json]`
  - `verify-only` rehashes every blob and checks tombstones.
  - Default mode re-runs the deterministic stages (citation audit, corroboration, competence, gates, scoring) **offline**. It injects a `CapturedRetrieval` provider (a seam into `ProviderRegistry`) and a `CapturedLlm` (a seam behind `chat_stage_with_model`). Both serve responses by `request_hash`. A missing key is a hard error, and network egress is disabled.
- `vox research campaign replay <campaign_id>` recomputes the arm comparison from the stored manifests.

## 6. Discriminating red tests (each must fail on today's tree)

1. `telemetry_metadata_rejects_unknown_key`: `metadata_json` containing `{"query":"x"}` passes `validate_research_metric_row` today, but must be rejected. Mutation check: delete the key check and the test must fail.
2. `research_run_emits_no_content_canary`: run the pipeline with fake providers whose query, URL, title, snippet and LLM text contain `CANARY_7f3e`. Collect every recorded `TelemetryEvent` and `research_metrics` row and assert the canary never appears. This fails today via `FindingCandidateProposed`.
3. `concurrent_trials_same_query_get_distinct_run_ids`: two concurrent runs of one query must get distinct ids and disjoint events. This fails today because of `INSERT OR IGNORE` + `last_insert_rowid`.
4. `all_llm_calls_in_run_share_run_id`: the number of `ModelCall` events with `run_id` = R must equal the number of stage calls. Today each call gets a fresh trace UUID.
5. `stage_model_attribution_records_resolved_and_usage`: when the cascade falls through, the event must carry the resolved model plus tokens and cost for **every** stage, including on the CLI/eval path.
6. `non_streaming_ttft_is_labeled`: `ttft_source == equals_latency` whenever streaming is off.
7. `cache_key_includes_config_hash`: the same query under two configs must not short-circuit (`pipeline_cache.rs:61`), and cache hits must be marked on `Run`.
8. `run_duration_covers_all_stages`: the sum of `Stage.duration_ms` must be ≤ `Run.duration_ms`, and the run duration must include the audit and self-verification.
9. `replay_offline_is_byte_deterministic_and_tamper_evident`: capture, then replay twice with no network; the derived metrics must be identical. Flip one blob byte and replay must fail with a hash mismatch.
10. `tombstoned_blob_replay_reports_redaction`.
11. `every_trial_event_key_has_allowlist_row`: a contract-parity test over the variants and serialized keys.

## 7. Pitfalls

- **Fake structure.** Substring checks (`types.rs:251`) and `additionalProperties: true` look like validation but are not. Validate parsed keys.
- **Id collisions.** FNV ids collide across trials: the session key (`pipeline.rs:108`), and `scientia_claims.claim_id` UNIQUE FNV-1a of the claim text (`scientia.rs:201`), so the same claim in two arms merges.
- **Test isolation.** `OnceLock` first-writer-wins makes capture-recorder tests order-dependent. Inject a recorder per run instead.
- **Lost events.** `tokio::spawn` fire-and-forget sinks (`vox-db/src/telemetry_sink.rs:38`) lose events at process exit and make tests flaky. Add a flush/await path for trials.
- **Float cost.** `f64` cost sums are order-dependent. Store `cost_usd_micros: u64`.
- **Replay inputs.** Replay must never call a model, because temperature 0 is not determinism. Replay must also use captured timestamps, never `now_ms`.
- **Cross-config cache hits.** The research cache short-circuits across configs and contaminates arms. Bypass it or key it by `config_hash` during campaigns.
- **Free-text fields.** `reason`, `error`, `selection_rationale` and `summary` creep in. Map them to `reason_code` enums at the producer.
- **Content in turn rows.** The unified turn table stores full prompts and responses (`chat.rs:304-340`). Either reclassify it as governed evidence (with retention and access rules) or stop writing text there for research runs.
- **Crypto policy.** Do not add new direct hash imports; hash through `vox-crypto` (`vox-db` already violates this with `sha3`).

**Summary**
1. Telemetry has no key allowlist. Only length caps and a substring check exist (`types.rs:214-261`), and the bridge contract allows extra properties.
2. Trial identity breaks today: a fresh `trace_id` per LLM call, an `anon-session` id, and colliding FNV session keys with `INSERT OR IGNORE` (a race bug).
3. Latency, tokens, cost and tool calls exist on `LlmResponse` but research drops them. TTFT is fake for non-streaming calls, and run duration undercounts.
4. Neither `vox-spool` nor a Tier C blob store exists. The only CAS is inline SHA3-512 in `vox-db`, so decide ownership, hash, retention, tombstones and encryption before building replay.
5. Plan: land the allowlist and canary/race red tests first, then `ResearchTrial` events, then content-addressed bundles plus `vox research replay` offline.
