---
title: "Environment variables (SSOT)"
description: "Official documentation for Environment variables (SSOT) for the Vox language. Detailed technical reference, architecture guides, and impl"
category: "reference"
last_updated: 2026-03-27
training_eligible: true
---

# Environment variables (SSOT)

Canonical names and precedence for tooling that spans CLI, MCP, orchestrator, and Codex. **Implementations** live in the crates cited below; update this page when adding or renaming variables.

## Codex / Turso (`vox-db`, `vox-pm`)

| Variable | Role |
|----------|------|
| `VOX_DB_URL` | Remote libSQL / Turso URL (with `VOX_DB_TOKEN`). |
| `VOX_DB_TOKEN` | Auth token for `VOX_DB_URL`. |
| `VOX_DB_PATH` | Local database file path (`local` / replication features). |
| `VOX_TURSO_URL` / `VOX_TURSO_TOKEN` | **Compatibility** aliases read after canonical `VOX_DB_*` fails in [`DbConfig::resolve_standalone`](../../../crates/vox-db/src/config.rs). |
| `TURSO_URL` / `TURSO_AUTH_TOKEN` | **Legacy** Turso env names; same compatibility tier as `VOX_TURSO_*`. |
| `VOX_EMBEDDING_SEARCH_CANDIDATE_MULT` | Integer ≥ 1: multiplier for brute-force embedding search window (`limit * mult`, capped). See [`capabilities`](../../../crates/vox-db/src/capabilities.rs). |
| `vox-db` / **`replication`** feature | Cargo feature enabling Turso embedded-replica connect paths (`vox-pm` exposes `replication = ["vox-db/replication"]`). Pair with [`VoxDb::sync`](../../../crates/vox-db/src/store/open.rs) / [`ReadConsistency::ReplicaLatest`](../../../crates/vox-db/src/lib.rs) before reads that need fresher remote state. |

**Precedence (remote):** `VOX_DB_URL`+`VOX_DB_TOKEN` → `VOX_TURSO_*` → `TURSO_*`. **Project VoxDb** (operational store + snippets/share) uses [`DbConfig::resolve_project_code_store_config`](../../../crates/vox-db/src/config.rs): empty env maps to the project-relative default store path, not the user-data default.

See [ADR 004: Codex / Arca / Turso](../adr/004-codex-arca-turso-ssot.md).

## Ludus (`vox-ludus`, `vox ludus`)

| Variable | Role |
|----------|------|
| `VOX_LUDUS_EMERGENCY_OFF` | When `1`/`true`/`yes`, hard-disables all Ludus side effects (rewards, teaching DB writes, overlays). See [`config_gate`](../../../crates/vox-ludus/src/config_gate.rs). |
| `VOX_LUDUS_SESSION_ENABLED` | Session-only override: `true` / `false` toggles `gamify_enabled` without touching on-disk config. |
| `VOX_LUDUS_SESSION_MODE` | `balanced` \| `serious` \| `learning` \| `off` (`off` disables for the session). |
| `VOX_LUDUS_VERBOSITY` | `quiet` \| `normal` \| `rich` — CLI celebration / overlay verbosity. See [`output_policy`](../../../crates/vox-ludus/src/output_policy.rs). |
| `VOX_LUDUS_MAX_MESSAGES_PER_HOUR` | Cap on bursty Ludus CLI messages per rolling hour (default `12`). |
| `VOX_LUDUS_CHANNEL` | UX channel override: `off` \| `serious` \| `balanced` \| `digest-priority` (also `digest` / `digest_priority`). When unset, derived from [`GamifyMode`](../../../crates/vox-config/). `digest-priority` suppresses inline CLI celebrations; use `vox ludus digest-weekly` for summaries. |
| `VOX_LUDUS_EXPERIMENT` | When non-empty: appended to `gamify_policy_snapshots.mode_label`, and scales teaching hint frequency (deterministic A/B multiplier from the string). |
| `VOX_LUDUS_MCP_TOOL_ARGS` | How MCP tool call `args` are stored in routed Ludus events: `full` (default) \| `hash` \| `omit` (see [`mcp_privacy`](../../../crates/vox-ludus/src/mcp_privacy.rs), [`config_gate`](../../../crates/vox-ludus/src/config_gate.rs)). |
| `VOX_LUDUS_EXPERIMENT_REWARD_MULT` | When set to a finite positive number (e.g. `1.1`), multiplies policy XP/crystal rewards in addition to mode + streak (Ludus experiment branch); unset keeps prior behavior. |
| `VOX_LSP_LUDUS_EVENTS` | When `0`/`false`/`off`, disables Ludus `diagnostics_clean` emission from `vox-lsp` (project Codex must still open successfully). |
| `VOX_LUDUS_ROUTE_LOG_SAMPLE` | Optional integer **N** ≥ 1: log roughly **1/N** `route_event` calls at `INFO` (`target = vox_ludus::route_event`) using a deterministic hash (user id + event type). |

## Repository root (`vox-repository`, `vox ci`)

| Variable | Role |
|----------|------|
| `VOX_REPO_ROOT` | Absolute or normalized path to the logical repo root for **`vox ci`**, doc-inventory, **`vox upgrade --source repo`** (when **`--repo-root`** is omitted), and other tools that must not depend on cwd alone. |
| `VOX_REPOSITORY_ROOT` | Compatibility alias read **before** `VOX_REPO_ROOT` in some tools ([`lineage`](../../../crates/vox-orchestrator/src/lineage.rs), TOESTUB/MCP/repo-id probes). Prefer `VOX_REPO_ROOT`; set both only if tooling disagrees. |

## User data directory (`vox-config`)

| Variable | Role |
|----------|------|
| `VOX_DATA_DIR` | Absolute path overriding the platform default Vox **data directory** (configs, canonical local store parent, etc.). See [`resolve_vox_data_dir`](../../../crates/vox-config/src/paths.rs). |

## Toolchain self-update (`vox upgrade`)

| Variable | Role |
|----------|------|
| `VOX_UPGRADE_PROVIDER` | `github` (default), `gitlab`, or `http` — override release backend when not passing **`--provider`**. |
| `VOX_UPGRADE_REPO` | `owner/repo` (GitHub) or `namespace/project` (GitLab). Default upstream: **`vox-foundation/vox`**. |
| `VOX_UPGRADE_BASE_URL` | For **`http`**: base URL such as `https://github.com/org/repo/releases` (requires **`--version`** or **`VOX_UPGRADE_VERSION`**). |
| `VOX_UPGRADE_VERSION` | Pinned tag for **`http`** mirror when omitted on the CLI. |
| `VOX_UPGRADE_GITLAB_HOST` | GitLab API root (default `https://gitlab.com`). |
| `VOX_UPGRADE_GITHUB_API_URL` | GitHub API base (Enterprise), e.g. `https://github.example.com/api/v3`. |
| `GITHUB_TOKEN` / `GH_TOKEN` / `VOX_GITHUB_TOKEN` | Optional; raises GitHub API rate limits and enables **private** release assets. |
| `GITLAB_TOKEN` / `VOX_GITLAB_TOKEN` | Optional GitLab **private-token** style access for private releases / asset URLs. |
| `CARGO` | Optional: path to the **`cargo`** executable for **`vox upgrade --source repo --apply`** (defaults to **`cargo`** on `PATH`). |

## Orchestrator (`vox-orchestrator`)

| Variable | Role |
|----------|------|
| `VOX_ORCHESTRATOR_ENABLED` | Enable/disable orchestrator. |
| `VOX_ORCHESTRATOR_MAX_AGENTS` | Cap on concurrent agents. |
| `VOX_ORCHESTRATOR_LOCK_TIMEOUT_MS` | File lock TTL. |
| `VOX_ORCHESTRATOR_TOESTUB_GATE` | TOESTUB post-task gate. |
| `VOX_ORCHESTRATOR_MAX_DEBUG_ITERATIONS` | Re-route cap on validation failures. |
| `VOX_ORCHESTRATOR_SOCRATES_GATE_SHADOW` | Log Socrates decisions without blocking. |
| `VOX_ORCHESTRATOR_SOCRATES_GATE_ENFORCE` | Requeue on risky Socrates outcome. |
| `VOX_ORCHESTRATOR_SOCRATES_REPUTATION_ROUTING` | Blend Arca `agent_reliability` into routing. |
| `VOX_ORCHESTRATOR_SOCRATES_REPUTATION_WEIGHT` | Weight for reliability blend (default in config: `1.0`). |
| `VOX_ORCHESTRATOR_LOG_LEVEL` | Tracing/log level string. |
| `VOX_ORCHESTRATOR_FALLBACK_SINGLE` | Ambiguous routing → single agent. |
| `VOX_ORCHESTRATOR_MESH_CONTROL_URL` | Base URL of the mens HTTP control plane for **read-only** node snapshots in MCP/orchestrator (e.g. `http://mens-ctrl:9847`). See [mens SSOT](populi.md), [deployment compose SSOT](deployment-compose.md). |
| `VOX_ORCHESTRATOR_MESH_POLL_INTERVAL_SECS` | Poll interval for mens HTTP client (see [`OrchestratorConfig::merge_env_overrides`](../../../crates/vox-orchestrator/src/config.rs)). |
| `VOX_A2A_CONSUMER_ID` | Override the **claim owner** string for [`VoxDb::poll_a2a_inbox`](../../../crates/vox-db/src/store/ops_ludus/gamify_extended.rs) (default `pid:<process_id>`). |
| `VOX_ORCH_LINEAGE_OFF` | When `1` / `true` / `yes`, skips append-only `orchestration_lineage_events` writes from the orchestrator (rollback toggle). |
| `VOX_WORKFLOW_JOURNAL_CODEX_OFF` | When `1` / `true` / `yes`, skips Codex persistence for interpreted workflow journals after `vox mens workflow run` (see [`workflow_journal_codex`](../../../crates/vox-cli/src/workflow_journal_codex.rs)). |
| `VOX_DB_CIRCUIT_BREAKER` | When enabled in [`DbCircuitBreaker::from_env`](../../../crates/vox-db/src/circuit_breaker.rs), gates selected Turso writes (locks, heartbeats, lineage, CAS, sessions, LLM logs, `agent_events`, Codex skills + **`chat_*`** user chat / usage / topics, generic `actor_state`, registry preference wipe, research ingest + capability map, `populi_training_run`, legacy JSONL data rows + `legacy_import_extras`, TOESTUB persistence, schemaless `Collection` document writes, agent memory/knowledge/search/embeddings, publication + scholarly/external jobs + planning + news + mens cloud + questioning, Ludus `gamify_*` / A2A / oplog / Ludus `actor_state`, learning + workflow journal + retention deletes + MCP chat transcripts, build observability + `components` — see `circuit_breaker.rs`). |
| `VOX_DB_SYNC_INTEGRATION` | Set to `1` with remote URL+token to enable the opt-in [`sync_for(ReplicaLatest)`](../../../crates/vox-db/src/store/open.rs) integration test (`vox-db` `sync_remote_integration.rs`). |
| `VOX_DB_EMBEDDED_REPLICA_INTEGRATION` | Set to `1` with URL+token to run the opt-in embedded-replica test (`cargo test -p vox-db --features replication sync_embedded_replica_smoke`). |
| `VOX_ORCHESTRATOR_MESH_HTTP_TIMEOUT_MS` | HTTP timeout for mens control-plane requests. |
| `VOX_ORCHESTRATOR_MESH_ROUTING_EXPERIMENTAL` | Experimental routing hooks (see [mens SSOT](populi.md)). |
| `VOX_ORCHESTRATOR_MESH_TRAINING_ROUTING_EXPERIMENTAL` | Enables training-task-specific scoring boosts/penalties in local routing. |
| `VOX_ORCHESTRATOR_MESH_TRAINING_BUDGET_PRESSURE` | Soft scalar (`0.0-1.0`) to reduce expensive training placements under budget pressure. |
| `VOX_ORCHESTRATOR_MESH_REMOTE_EXECUTE_EXPERIMENTAL` | When `1`/`true`, best-effort fan-out of [`RemoteTaskEnvelope`](../../../crates/vox-orchestrator/src/a2a/envelope.rs) over populi A2A **after** local enqueue (local execution still owns the task). |
| `VOX_ORCHESTRATOR_MESH_REMOTE_EXECUTE_RECEIVER_AGENT` | Destination **numeric** A2A agent id (string form) for experimental remote relay. |
| `VOX_ORCHESTRATOR_MESH_REMOTE_EXECUTE_SENDER_AGENT` | Originator agent id for relay (defaults to `1` when unset/invalid). |
| `VOX_ORCHESTRATOR_MESH_REMOTE_RESULT_POLL_INTERVAL_SECS` | When experimental remote execute is on, MCP polls populi A2A inbox for **`remote_task_result`** on this interval (default **5**). **`0`** disables the dedicated poller. Independent of **`VOX_ORCHESTRATOR_MESH_POLL_INTERVAL_SECS`**. |
| `VOX_ORCHESTRATOR_MIN_AGENTS` / `SCALING_*` / `COST_PREFERENCE` / `RESOURCE_*` | Scaling and economy knobs — see [`OrchestratorConfig::merge_env_overrides`](../../../crates/vox-orchestrator/src/config.rs). |
| `POPULI_MODEL` | Default **Ollama** model id when routing uses local inference ([`usage`](../../../crates/vox-orchestrator/src/usage.rs), [`spec`](../../../crates/vox-orchestrator/src/models/spec.rs)). |
| `GROQ_API_KEY` / `CEREBRAS_API_KEY` / `MISTRAL_API_KEY` / `DEEPSEEK_API_KEY` / `SAMBANOVA_API_KEY` / `CUSTOM_OPENAI_API_KEY` | Bare provider keys read for optional **key presence** checks in [`usage`](../../../crates/vox-orchestrator/src/usage.rs). Prefer **Clavis** / `VOX_*` secret resolution for real credential storage (see [`AGENTS.md`](../../../AGENTS.md)). |
| `VOX_NEWS_PUBLISH_ARMED` | When `1`/`true`, satisfies the **armed** gate for live news/scientia syndication (in addition to two DB approvers). See [news syndication security](../architecture/news_syndication_security.md). |
| `VOX_SCHOLARLY_ADAPTER` | Scholarly submit adapter: `local_ledger` (default), `echo_ledger`, `zenodo`, `openreview`, etc. Unknown values error. See [`scholarly::flags`](../../../crates/vox-publisher/src/scholarly/flags.rs). |
| `VOX_SCHOLARLY_DISABLE` | When truthy (`1`, `true`, `yes`, `y`, `on`), blocks all scholarly submit/status paths. |
| `VOX_SCHOLARLY_DISABLE_LIVE` | When truthy, blocks **live** adapters (Zenodo/OpenReview); local/echo ledgers still allowed. |
| `VOX_SCHOLARLY_DISABLE_ZENODO` | Per-adapter kill-switch for Zenodo when truthy. |
| `VOX_SCHOLARLY_DISABLE_OPENREVIEW` | Per-adapter kill-switch for OpenReview when truthy. |
| `VOX_OPENREVIEW_API_BASE` / `OPENREVIEW_API_BASE` | Optional override for the OpenReview API v2 base URL (default `https://api2.openreview.net`). Used for mocks and self-hosted stacks; see [`api_base`](../../../crates/vox-publisher/src/scholarly/openreview.rs). |
| `VOX_ZENODO_SANDBOX` | When truthy, Zenodo REST uses sandbox API host instead of production. |
| `VOX_ZENODO_API_BASE` | Optional override for the Zenodo REST API root (e.g. `https://zenodo.org/api` or `https://sandbox.zenodo.org/api`). Used for mocks and non-standard endpoints; when unset, production vs sandbox follows `VOX_ZENODO_SANDBOX`. See [`ZenodoHttpClient::new`](../../../crates/vox-publisher/src/scholarly/zenodo.rs). |
| `VOX_ZENODO_HTTP_MAX_ATTEMPTS` | Max attempts per Zenodo HTTP call (deposit create, get, bucket `PUT`, `publish`) for retryable errors (5xx, 429, timeouts). Integer **1–10**, default **3**. |
| `VOX_ZENODO_ATTACH_MANIFEST_BODY` | When truthy, after creating a draft deposition, uploads `manifest.body_markdown` as `body.md` to `links.bucket` (Zenodo files API). |
| `VOX_ZENODO_PUBLISH_DEPOSITION` | When truthy, calls deposit `publish` after file attach. Requires **`VOX_ZENODO_ATTACH_MANIFEST_BODY`** or files from **`VOX_ZENODO_STAGING_DIR`** (Zenodo rejects publish with zero files). |
| `VOX_ZENODO_DRAFT_ONLY` | When truthy, never calls `publish` (overrides **`VOX_ZENODO_PUBLISH_DEPOSITION`** and **`VOX_ZENODO_PUBLISH_NOW`**). |
| `VOX_ZENODO_PUBLISH_NOW` | Convenience profile: attach `body.md` and publish when the deposition is otherwise valid (still respects **`VOX_ZENODO_DRAFT_ONLY`**). |
| `VOX_ZENODO_STAGING_DIR` | Directory produced by `publication-scholarly-staging-export` (Zenodo layout). When set, Zenodo submit uploads files from this tree (plan + optional **`VOX_ZENODO_UPLOAD_ALLOWLIST`**) instead of or in addition to manifest-only attach; see [`zenodo_relpaths_to_upload`](../../../crates/vox-publisher/src/scholarly/zenodo.rs). |
| `VOX_ZENODO_UPLOAD_ALLOWLIST` | Comma-separated relative paths under **`VOX_ZENODO_STAGING_DIR`** to upload; when empty, uploads all Zenodo plan files present (excluding arXiv-only artifacts). |
| `VOX_ZENODO_VERIFY_STAGING_CHECKSUMS` | When truthy, requires `staging_checksums.json` and verifies SHA3-256 per file before bucket `PUT`. |
| `VOX_ZENODO_REQUIRE_METADATA_PARITY` | When truthy, requires `zenodo.json` metadata title to match manifest title (trim / ASCII space normalization). |
| `VOX_OPENREVIEW_HTTP_MAX_ATTEMPTS` | Max attempts per OpenReview HTTP call (`notes`, `notes/edits`) for retryable errors. Integer **1–10**, default **3**. |
| `VOX_SCHOLARLY_JOB_LOCK_OWNER` | Optional lock-owner string for `external_submission_jobs` lease ticks (default `vox:<pid>`). |
| `VOX_NEWS_SITE_BASE_URL` | Public site base URL for RSS links (overrides `[orchestrator.news].site_base_url`). |
| `VOX_NEWS_RSS_FEED_PATH` | Repo-relative path to `feed.xml` (overrides `[orchestrator.news].rss_feed_path`). |
| `VOX_NEWS_SCAN_RECURSIVE` | `0`/`1`: whether `NewsService` walks `news_dir` recursively (default `1`). |
| `VOX_NEWS_TWITTER_TEXT_CHUNK_MAX` | Optional integer override for tweet chunk length (defaults to publisher contract value). |
| `VOX_NEWS_TWITTER_TRUNCATION_SUFFIX` | Optional suffix used when shortening non-thread tweets (default `...`). |
| `VOX_SOCIAL_REDDIT_CLIENT_ID` | Reddit OAuth client id for scientia/news syndication submission paths. |
| `VOX_SOCIAL_REDDIT_CLIENT_SECRET` | Reddit OAuth client secret for token refresh on publish. |
| `VOX_SOCIAL_REDDIT_REFRESH_TOKEN` | Reddit refresh token used to mint short-lived access tokens for `/api/submit`. |
| `VOX_SOCIAL_REDDIT_USER_AGENT` | Required descriptive Reddit User-Agent (`platform:app:version (by /u/name)`). |
| `VOX_SOCIAL_YOUTUBE_CLIENT_ID` | YouTube OAuth client id for channel upload automation. |
| `VOX_SOCIAL_YOUTUBE_CLIENT_SECRET` | YouTube OAuth client secret for channel upload automation. |
| `VOX_SOCIAL_YOUTUBE_REFRESH_TOKEN` | YouTube refresh token for user-channel upload scopes. |
| `VOX_SOCIAL_YOUTUBE_DEFAULT_CATEGORY_ID` | Optional default YouTube `categoryId` used when a manifest omits `youtube.category_id` (publisher fallback defaults to `28`). |
| `VOX_SOCIAL_TWITTER_SUMMARY_MARGIN_CHARS` | Optional integer reserve applied when deriving `twitter.short_text` from markdown (`twitter_text_chunk_max - margin`). |
| `VOX_SYNDICATION_TEMPLATE_PROFILE` | When `1`/`true`, applies `distribution_policy.channel_policy.<channel>.template_profile` to derived social copy caps (Twitter margin, Reddit self-post summary, YouTube description). When unset/false, profiles are ignored and `SyndicationResult.decision_reasons` may record `template_profile_inert` if a profile key is set. |
| `VOX_SOCIAL_REDDIT_SELFPOST_SUMMARY_MAX` | Optional integer cap for derived Reddit self-post body text when `text_override` is empty. |
| `VOX_SOCIAL_HN_MODE` | Hacker News publish mode (`manual_assist` only; official HN API is read-only). |
| `VOX_SOCIAL_WORTHINESS_ENFORCE` | `0`/`1`: enforce aggregate worthiness floor before **live** fan-out (orchestrator news tick, `vox db publication-publish`, MCP `vox_scientia_publication_publish` when not dry-run). On MCP, `[orchestrator.news].worthiness_enforce` also applies. |
| `VOX_SOCIAL_WORTHINESS_SCORE_MIN` | Minimum worthiness score when enforcement is on (default **0.85** if unset). MCP may set `[news].worthiness_score_min` instead. |
| `VOX_SOCIAL_CHANNEL_WORTHINESS_FLOORS` | Optional CSV `channel=floor` map (e.g., `reddit=0.82,hacker_news=0.86`) merged into runtime channel policy. |

Socrates numeric thresholds default from [`vox-socrates-policy`](../../../crates/vox-socrates-policy/src/lib.rs); optional TOML overrides live under `[orchestrator]` as `socrates_policy` (see `OrchestratorConfig`).

## MCP / Socrates questioning (vox-mcp) {#mcp-socrates-questioning}

Wall-time and attention telemetry for information-theoretic clarification (chat, plan, inline, ghost). Policy defaults (including default max attention when env is unset) also come from [`QuestioningPolicy`](../../../crates/vox-socrates-policy/src/lib.rs).

| Variable | Role |
|----------|------|
| `VOX_QUESTIONING_MIRROR_GLOBAL_ATTENTION` | When **`0`** or **`false`**, questioning debits apply only to the **per-`session_id`** tally. When **unset** or any other value, the same milliseconds also increment the orchestrator [`BudgetManager`](../../../crates/vox-orchestrator/src/budget.rs) global **`AttentionBudget::spent_ms`** (see [`add_questioning_attention_debit_ms`](../../../crates/vox-orchestrator/src/budget.rs)); this does **not** emit an interrupt EWMA event. Implemented in [`ServerState::record_questioning_attention_spend`](../../../crates/vox-mcp/src/server/lifecycle.rs). |
| `VOX_QUESTIONING_MAX_ATTENTION_MS` | Optional **unsigned** cap (milliseconds) for the per-session clarification attention analogue. **Unset** or invalid → `QuestioningPolicy::default().max_clarification_attention_ms`. Used by [`questioning_attention_bounds`](../../../crates/vox-mcp/src/server/lifecycle.rs). |
| `VOX_SUBMIT_TASK_BYPASS_QUESTIONING_GATE` | When truthy, allows orchestrator **task submit** via MCP to skip the “pending Socrates clarification” gate (operator / CI escape hatch). See [`task_tools`](../../../crates/vox-mcp/src/tools/task_tools.rs). |
| `VOX_MCP_AGENT_FLEET` | When **unset** or truthy, **vox-mcp** spawns the embedded `AgentFleet` loop (`sync_fleet` + periodic `tick`) so vox-runtime worker handles are registered and queued tasks receive `ProcessQueue` wakes (**default on**). Set **`0`**, **`false`**, **`no`**, or **`off`** to disable. See [`spawn_embedded_agent_fleet_if_enabled`](../../../crates/vox-mcp/src/server/lifecycle.rs). |
| `VOX_EMBEDDING_MODEL` | Optional embedding model id override for MCP memory retrieval (`vox-mcp` [`retrieval`](../../../crates/vox-mcp/src/memory/retrieval.rs)). |
| `VOX_OPENROUTER_HTTP_REFERER` | Optional `HTTP-Referer` header for OpenRouter-compatible calls ([`provider_auth`](../../../crates/vox-mcp/src/llm_bridge/provider_auth.rs)). |
| `VOX_OPENROUTER_APP_TITLE` | Optional `X-Title` header for OpenRouter-compatible calls ([`provider_auth`](../../../crates/vox-mcp/src/llm_bridge/provider_auth.rs)). |
| `VOX_MCP_GRAMMAR_MASK` | Grammar-mask knob for speech constraints ([`speech_constraints`](../../../crates/vox-mcp/src/speech_constraints.rs)). |
| `VOX_MCP_LLM_COST_EVENTS` | When truthy, enables LLM cost telemetry emission ([`infer`](../../../crates/vox-mcp/src/llm_bridge/infer.rs)). |
| `OLLAMA_HOST` | Upstream Ollama base URL override read by MCP provider metadata ([`metadata`](../../../crates/vox-mcp/src/llm_bridge/providers/metadata.rs)). |
| `VOX_ORCHESTRATOR_EVENT_LOG` | Path to a **JSONL** file: **`vox-mcp`** appends one JSON object per orchestrator [`AgentEvent`](../../../crates/vox-orchestrator/src/events.rs) when set ([`spawn_orchestrator_event_log_sink`](../../../crates/vox-mcp/src/server/lifecycle.rs)). **`vox live`** can tail the same file when built with the `live` feature. |

**MCP tools (VoxDb required for persistence):** `vox_questioning_pending` (unanswered assistant questions + structured `question_options` and session `belief_state_json`), `vox_questioning_submit_answer`, `vox_questioning_sync_ssot`. Canonical names: [`contracts/mcp/tool-registry.canonical.yaml`](../../../contracts/mcp/tool-registry.canonical.yaml). Protocol SSOT: [Information-theoretic questioning](information-theoretic-questioning.md).

## Mens / Candle

| Variable | Role |
|----------|------|
| `VOX_CANDLE_DEVICE` | Forces Candle device (e.g. `cpu`); see Mens training SSOT. |
| `VOX_VRAM_OVERRIDE_GB` | Overrides VRAM autodetect for preset hints in `vram_autodetect` (useful in CI/headless hosts). |
| `VOX_MENS_EXPERIMENTAL_OPTIMIZER` | Guard flag required when `optimizer_experiment_mode` is set to a non-`off` value. |
| `VOX_INFERENCE_PROFILE` | `desktop_ollama` (default), `cloud_openai_compatible`, `mobile_litert`, `mobile_coreml`, `lan_gateway`; gates **vox-mcp** local Ollama + Ollama fallback to `desktop_ollama` / `lan_gateway` only; see [`vox_config::inference`](../../../crates/vox-config/src/inference.rs) and [mobile-edge-ai.md](mobile-edge-ai.md). |
| `VOX_AUTO_MODEL_STRATEGY` | OpenRouter strategy for auto model ids: `provider_auto` or `preferred_model`; see [`vox_config::routing_policy`](../../../crates/vox-config/src/routing_policy.rs). |
| `VOX_AUTO_ROUTING_PRIORITY` | Weighted MCP auto-routing priorities (`efficiency,precision,latency,availability,balance,mobile`) as `k=v` CSV. |
| `VOX_GEMINI_ROUTE_POLICY` | Gemini routing policy: `openrouter_first` (default), `google_direct_only`, or `registry_default`. |
| `OPENROUTER_GEMINI_MODEL` / `GEMINI_DIRECT_MODEL` | Explicit OpenRouter/GoogleDirect Gemini model pair for policy routing/fallback. |
| `VOX_PROVIDER_DAILY_LIMIT_DEFAULT` / `VOX_PROVIDER_LIMIT_PROVIDERS` | Dynamic provider quota defaults before JSON/file overrides in [`usage_policy`](../../../crates/vox-orchestrator/src/usage_policy.rs). |
| `VOX_PROVIDER_DAILY_LIMITS_FILE` | Optional JSON file of per-provider daily limits (merged after defaults in [`usage_policy`](../../../crates/vox-orchestrator/src/usage_policy.rs)). |
| `VOX_PROVIDER_DAILY_LIMITS_JSON` | Inline JSON for the same structure as the file variant. |

## Mens (`vox-populi`, orchestrator probe)

Full table: [mens SSOT](populi.md). Common entries:

| Variable | Role |
|----------|------|
| `VOX_MESH_ENABLED` | Enables mens registry publish and related hooks. |
| `VOX_MESH_CONTROL_ADDR` | This process’s control plane URL (publish/join target). |
| `VOX_MESH_TOKEN` / `VOX_MESH_WORKER_TOKEN` / `VOX_MESH_SUBMITTER_TOKEN` / `VOX_MESH_ADMIN_TOKEN` | Populi control-plane bearer roles (Clavis SSOT); legacy single-token mode uses `VOX_MESH_TOKEN` only. See [mens SSOT](populi.md). |
| `VOX_MESH_JWT_HMAC_SECRET` | Optional HS256 secret so clients can use `Authorization: Bearer <jwt>` with claims `role`, `jti`, `exp` (Clavis SSOT). |
| `VOX_MESH_WORKER_RESULT_VERIFY_KEY` | Optional Ed25519 public key (hex or Standard base64) to verify signed `job_result` / `job_fail` deliveries (worker signs raw BLAKE3 digest). |
| `VOX_MESH_SCOPE_ID` | Tenancy for join/heartbeat when enforced server-side. |
| `VOX_MESH_A2A_LEASE_MS` | Inbox claim lease duration (default 120s, clamped). |
| `VOX_MESH_MAX_STALE_MS` | Client-side staleness filter for mens snapshots (MCP). |
| `VOX_MESH_CODEX_TELEMETRY` | Emit Codex `populi_control_event` rows when set. |
| `VOX_MESH_HTTP_JOIN` | `0`/`false` disables MCP HTTP join to the control plane; see [mens SSOT](populi.md). |
| `VOX_MESH_HTTP_HEARTBEAT_SECS` | MCP heartbeat interval after join (`0` = no background heartbeat). |
| `VOX_MESH_HTTP_RATE_LIMIT` | When `1`/`true`/`on`/`yes`, enables per–client-IP HTTP rate limiting on **`vox populi serve`** (see `tower_governor` in `vox-populi` transport). |
| `VOX_MESH_HTTP_RATE_LIMIT_PER_SEC` | Steady-state requests per second per key when rate limiting is on (default **50**). |
| `VOX_MESH_HTTP_RATE_LIMIT_BURST` | Burst capacity (default scales with per-sec). |
| `VOX_MESH_ADVERTISE_GPU` | Legacy: sets `gpu_cuda` on the host capability snapshot. |
| `VOX_MESH_ADVERTISE_VULKAN` | Sets `gpu_vulkan`. |
| `VOX_MESH_ADVERTISE_WEBGPU` | Sets `gpu_webgpu`. |
| `VOX_MESH_ADVERTISE_NPU` | Sets `npu`. |
| `VOX_MESH_DEVICE_CLASS` | Optional `TaskCapabilityHints.device_class` string. |

## GPU probe overrides (Mens training)

| Variable | Role |
|----------|------|
| `VOX_GPU_MODEL` | With `VOX_GPU_VRAM_MB`, overrides [`probe_gpu`](../../../crates/vox-populi/src/mens/tensor/device.rs) (CI / headless / Android host injection). |
| `VOX_GPU_VRAM_MB` | Paired with `VOX_GPU_MODEL` for VRAM heuristics. |

## CI / diagnostics

| Variable | Role |
|----------|------|
| `VOX_SECRET_GUARD_GIT_REF` | Git revision range for **`vox ci secret-env-guard`** on clean checkouts (e.g. `origin/main...HEAD` on PRs, `${{ github.event.before }}...${{ github.sha }}` on push). Avoids an empty diff scope when `git diff` would otherwise scan nothing. See [`guards.rs`](../../../crates/vox-cli/src/commands/ci/run_body_helpers/guards.rs). |
| `VOX_BUILD_TIMINGS_BUDGET_WARN` | Soft budget warnings for **`vox ci build-timings`**. |
| `SKIP_CUDA_FEATURE_CHECK` | Skip optional `nvcc` gates (documented escape hatch in [runner contract](../ci/runner-contract.md)). |

## TOESTUB / scaling-audit (`vox-toestub`, `emit-reports`)

| Variable | Role |
|----------|------|
| `VOX_TOESTUB_MAX_RUST_PARSE_FAILURES` | Maximum allowed `rust_parse_failures` in the **`toestub --format json`** v1 envelope before **`vox ci scaling-audit emit-reports`** fails (and before PR CI’s full-`crates/` audit step fails). Non-negative integer. **Unset or invalid** ⇒ no limit (historical `emit-reports` behavior). **PR CI** sets this to **`3`** while the repo baseline is low (recent full `crates/` runs reported **`1`**); tighten to **`0`** once every Rust file parses under `syn::parse_file`, or raise the cap when adding deliberate snapshot exclusions. |

**CLI feature flag (not an env var):** `toestub --feature-flags unresolved-regex-fallback` (comma-separated with other flags) relaxes unresolved-ref’s AST `call_sites` gate so regex-only matches can surface again (e.g. macro-expanded calls). Default remains AST-gated for fewer false positives. See [scaling TOESTUB rules](../architecture/scaling-toestub-rules.md).

## Web / Vite / TanStack codegen

| Variable | Role |
|----------|------|
| `VOX_WEB_TANSTACK_START` | When `1` / `true`, enables TanStack **Start** scaffold + TS codegen path (`VoxTanStackRouter` / `voxRouteTree` when `routes:` is present). Must stay aligned with **`Vox.toml`** `[web] tanstack_start` for **`vox build`**. See [`VoxConfig::merge_env_overrides`](../../../crates/vox-config/src/config.rs), [TanStack how-to](../how-to/tanstack-ssr-with-axum.md). |
| `VOX_EMIT_EXPRESS_SERVER` | Opt-in: emit legacy **`server.ts`** (Express-style) from `vox-codegen-ts`; default product is **Axum** + **`api.ts`**. See [vox-fullstack-artifacts.md](vox-fullstack-artifacts.md). |
| `VOX_ORCHESTRATE_VITE` | If `1`, **`vox run`** spawns **`pnpm run dev:ssr-upstream`** in `dist/.../app` (Vite on **3001**). See [`OrchestratedViteGuard`](../../../crates/vox-cli/src/frontend.rs). |
| `VOX_SSR_DEV_URL` | Origin (e.g. `http://127.0.0.1:3001`) for generated Axum to proxy non-`/api` **GET** document requests before `rust_embed`. Often injected when **`VOX_ORCHESTRATE_VITE=1`**. |
| `VOX_WEB_VITE_SMOKE` | Opt-in: set to **`1`** when running **`cargo test -p vox-integration-tests --test web_vite_smoke -- --ignored`** (full **`pnpm install`** + **`vite build`** on a golden `.vox` fixture). |
| `VOX_WEB_TS_OUT` | Optional: absolute or relative directory where **`vox build`** writes generated **`*.tsx`** (same path as the build output). When set, **`vox doctor`** scans **`*.vox`** under the current tree for **`@v0`** declarations and verifies each **`{Name}.tsx`** in this directory uses a **named** export suitable for TanStack **`routes:`** (`export function Name`, etc.). See [`v0_tsx_normalize.rs`](../../../crates/vox-cli/src/v0_tsx_normalize.rs). |
| `VOX_EXAMPLES_STRICT_PARSE` | When **`1`**, **`cargo test -p vox-parser --test parity_test`** fails if any `examples/**/*.vox` fails to parse (default CI only requires the **`MUST_PARSE`** golden set). See [`examples/PARSE_STATUS.md`](../../../examples/PARSE_STATUS.md). |
| `VOX_SUPPRESS_LEGACY_HOOK_LINTS` | When **`1`** / **`true`**, suppresses compiler **warnings** for direct Vox `use_*` hook calls inside classic **`@component fn …`** bodies (Path C reactive syntax is still preferred). Implemented in [`react_bridge::legacy_hook_lint_suppressed`](../../../crates/vox-compiler/src/react_bridge.rs) + [`lint_ast_declarations`](../../../crates/vox-compiler/src/typeck/ast_decl_lints.rs). |
| `VOX_WEBIR_VALIDATE` | When **`1`** / **`true`**, **`vox_compiler::codegen_ts::generate`** runs Web IR lower + [`validate_web_ir`](../../../crates/vox-compiler/src/web_ir/validate.rs) after HIR and **fails codegen** if validation returns diagnostics (opt-in hard gate). See [`maybe_web_ir_validate`](../../../crates/vox-compiler/src/codegen_ts/emitter.rs). |
| `VOX_WEBIR_EMIT_REACTIVE_VIEWS` | When **`1`** / **`true`**, Path C reactive **`view:`** may use Web IR preview TSX **only when** validation is clean **and** whitespace-normalized TSX matches legacy `emit_hir_expr` (parity guard). See [`codegen_ts::reactive`](../../../crates/vox-compiler/src/codegen_ts/reactive.rs). |
| `VOX_WEBIR_REACTIVE_TRACE` | When **`1`** / **`true`**, logs one **`eprintln!`** line per reactive view decision (`component=…` + `pathway=…`). Pairs with aggregate counters via [`reactive_view_bridge_stats`](../../../crates/vox-compiler/src/codegen_ts/reactive.rs). |
| `VOX_RUNTIME_PROJECTION_INCLUDE_HOST_PROBE` | When **`1`** / **`true`**, [`project_runtime_from_hir`](../../../crates/vox-compiler/src/runtime_projection.rs) includes [`probe_host_capabilities`](../../../crates/vox-repository/src/capabilities.rs) in the serialized runtime projection (telemetry / envelope alignment). Default **off** so JSON stays machine-independent in tests. |
| `VOX_ISLAND_MOUNT_V2` | Reserved: when **`1`** / **`true`**, **`vox-cli`** logs once that **V2** `index.html` injection is not implemented and continues with the **V1** `/islands/island-mount.js` snippet ([`apply_island_mount_script_to_index_html`](../../../crates/vox-cli/src/frontend.rs)). |

## Related

- [Deployment compose SSOT](deployment-compose.md) — Compose profiles and Coolify/GitLab notes.
- [CI runner contract](../ci/runner-contract.md) — self-hosted labels and CUDA workflow notes.
- [ADR 005 / Socrates](../adr/) — policy and orchestration gates (index in repo).
- [Clavis SSOT](clavis-ssot.md) — canonical managed secret env names and secret-resolution precedence.

## Social credentials precedence

For scientia/news social distribution credentials, resolve in this order:

1. `VOX_SOCIAL_*` environment variables (preferred for CI/production injection),
2. OS keyring (`vox_db::secrets`) when explicitly configured by operator tooling,
3. local `~/.vox/auth.json` fallback for developer-only sessions.

Do not persist raw social API credentials in publication metadata or VoxDb domain tables.
