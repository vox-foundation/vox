---
title: "Centralized Postgres Telemetry, vox-ops Private Repo & Community Intelligence (SSoT)"
description: "Canonical architecture for fenced Postgres telemetry on helios-postgres, the private vox-ops repository split, and offline-first community-collated intelligence."
category: "Architecture SSOTs"
status: "current"
topic: "telemetry"
domain: "security"
authorship: "llm-assisted"
authorship_model: "gemini-4-argon"
schema_type: "TechArticle"
---

# Centralized Postgres Telemetry, `vox-ops` Private Repo & Community Intelligence (SSoT)

<!-- kb:summary -->
Defines the canonical architecture for Vox Milestone v1.7: migrating centralized telemetry from the prototype ClickHouse server in `server/telemetry/` to a three-role fenced PostgreSQL 17 database (`vox_ops`) on `helios-postgres`, extracting live social media syndication and community automation into the private `~/dev/vox-ops` repository, completing all client telemetry sinks (`vox-cli`, `vox-gui`, `vox-orchestrator-d`), and delivering Ed25519-signed, $k$-anonymous community performance intelligence bundles that enhance local routing while preserving 100% offline-first operation and explicit opt-in/opt-out transparency.
<!-- /kb:summary -->

## 1. Non-Negotiable Principles

1. **100% Offline & Standalone Operation**: Vox (`vox-cli`, `vox-gui`, `vox-orchestrator-d`, `vox-compiler`) MUST operate with full functionality when offline, air-gapped, or when `vox-ops` is unreachable. Bundled SSOT defaults and local `VoxDb` (Turso/SQLite) are always authoritative fallbacks; centralized community intelligence is strictly an opportunistic, cached enhancement.
2. **Zero-Surprise, Transparent Telemetry**:
   - Local telemetry collection is governed by the 5-layer `TelemetryConfig` hierarchy (`/etc/vox/telemetry-policy.toml` > `~/.config/vox/config.toml` > `VOX_TELEMETRY=off|on|debug` > per-category flags > default).
   - Remote telemetry upload is **strictly opt-in** (`ConsentState::Granted` via `vox telemetry consent grant` AND `is_master_enabled()`).
   - Every outgoing field is constrained by `contracts/telemetry/collection-taxonomy.v1.json` (only `enum`, `int`, `bool`, and per-install salted `hash` types; zero free-form strings, file paths, repo names, prompts, or secrets).
   - Users can inspect exact pending payloads before any network transmission via `vox telemetry preview` and `vox telemetry export`.
3. **Public Language Repo vs. Private Operations Repo (`~/dev/vox-ops`)**:
   - Public `vox` (`vox-foundation/vox`) owns the language, compiler, runtime, local orchestrator, local GUI, scholarly/academic local packaging (`CITATION.cff`, BibTeX, Zenodo/arXiv/OpenReview bundle preparation), and the pure OTLP client projection/redaction layer (`vox-telemetry`, `vox-telemetry-otlp`).
   - Private `~/dev/vox-ops` (`brbrainerd/vox-ops`) owns operational server infrastructure, fenced PostgreSQL schemas/migrations, live social media publication & community automation adapters (X/Twitter, Reddit, YouTube, Discord, Bluesky, Mastodon, LinkedIn, Hacker News, OpenCollective), durable Postgres syndication queues, and the $k$-anonymous community intelligence aggregator.

---

## 2. Telemetry Surface Audit Findings (2026-10-19)

A full workspace audit across `crates/vox-telemetry`, `crates/vox-telemetry-otlp`, `crates/vox-db`, `crates/vox-cli`, `crates/vox-gui`, `crates/vox-orchestrator-d`, and `server/telemetry` identified five concrete gaps to close in Milestone `v1.7`:

| Surface | File / Symbol | Audit Finding | v1.7 Resolution |
|---|---|---|---|
| **Client OTLP Uploader** | `crates/vox-telemetry-otlp/src/upload.rs` (`upload_pending_otlp`) | Stubbed (`TODO(track-B5)` returning `Ok(0)`). `vox-cli` uses legacy raw-file `telemetry_spool::upload_pending` instead of batched OTLP/HTTP `ResourceLogs` with rate limiting. | Implement real batched OTLP/HTTP uploader in `vox-telemetry-otlp::upload` (behind feature `remote`) with `governor` rate limiting, Clavis secret resolution, and `vox telemetry preview`. |
| **Daemon & GUI Recorder Registration** | `crates/vox-orchestrator-d/src/main.rs`, `crates/vox-gui/src/main.rs` | `set_global_recorder` is only called in `crates/vox-cli/src/lib.rs:808`. All `record_event!` emissions inside `vox-orchestrator-d` and `vox-gui` are silent no-ops. | Wire shared `init_telemetry_sinks` in `vox-orchestrator-d` (and forward GUI telemetry via daemon RPC / composite sink) respecting `TelemetryConfig::from_env()`. |
| **Local `VoxDb` Sink Coverage** | `crates/vox-db/src/telemetry_sink.rs` (`ResearchMetricsSink`) | Handles only 6 of 21 `TelemetryEvent` variants (`ResearchMetric`, `ModelCall`, `TaskRootSummary`, `BuildSummary`, `Error`, `AiFixture`); drops `CommandUsage`, `SkillActivation`, `EditPattern`, `HarnessUsage`, `ErrorSurface`, `DefaultDecision`, `ModelPrompt`, `LintFinding`, `RepairAttempt`, `RepairOutcome`, `AuditRun`, `SelectionDecision`, `ModelDiscovery`, `ModelClassification`, `ConfidencePromotion`, and `DoctorProjectCheck`. | Expand `ResearchMetricsSink` so all 21 structured variants persist locally into `research_metrics` for local rollups and `vox doctor` diagnostics. |
| **Server Datastore & Field Coverage** | `server/telemetry/` (`src/schema.rs`, `src/ingest.rs`) | Built against ClickHouse (`clickhouse = "0.13.3"`) inside the public monorepo; `EventRow` hardcodes 19 columns and silently drops 20+ taxonomy fields (`model_id_hash`, `provider`, `prompt_tokens_bucket`, `canonical_model_id`, `profile_variant_id`, `rule_id_hash`, etc.). | Remove `server/telemetry/` from public `vox`; build Postgres-backed ingest in `~/dev/vox-ops` with 100% taxonomy column parity generated from `collection-taxonomy.v1.json`. |
| **Social & Community Automation Coupling** | `crates/vox-publisher/src/adapters/` (`twitter.rs`, `reddit.rs`, `youtube.rs`, `discord.rs`, `bluesky.rs`, `mastodon.rs`, `linkedin.rs`, `hacker_news.rs`, `opencollective.rs`) | Live social media syndication adapters, OAuth refresh flows, and retry loops live inside the public compiler workspace without durable multi-worker Postgres outbox locking. | Move social media & community automation adapters into `~/dev/vox-ops` backed by Postgres `syndication.outbox` (`FOR UPDATE SKIP LOCKED`); keep local academic/scholarly prep and an optional webhook/HTTP handoff in `vox-publisher`. |

---

## 3. Fenced PostgreSQL Architecture on `helios-postgres`

Following the proven house standard across `ramada` (ADR-0008, `ramada-facts-grants.md` D-36) and `ngi.video` (`deploy/helios/ngi-video-db-setup`), `vox-ops` shares the existing `helios-postgres` container (`q40ck888ggkkcsosgcwgc0oo`, `pgvector/pgvector:pg17`, ~7 MB marginal disk, ~0 extra RAM) with strict **three-role privilege fencing**:

```vox-diagram
graph {
    direction: right
    node Client: "Vox Client (Opt-In)\nvox telemetry upload"
    node Ingest: "vox-ops Ingest API\n(vox_ingest_app role)"
    node RawTable: "telemetry.events_raw\nOwner: vox_ops_owner (NOLOGIN)" { shape: cylinder }
    node Views: "telemetry.v_*_k20\nRead-Only Rollup Views" { shape: cylinder }
    node OpsWorker: "vox-ops Aggregator & API\n(vox_ops_app role)"
    node Bundle: "GET /v1/intelligence/bundle.json"
    node LocalVox: "Local Vox (CLI / Daemon / GUI)\n100% Functional Offline"
    Client -> Ingest: "HTTPS POST /v1/logs\nOTLP JSON"
    Ingest -> RawTable: "INSERT ONLY\n(No SELECT/UPDATE/DELETE)"
    RawTable -> Views: "SECURITY DEFINER +\nsecurity_barrier Views\nHAVING count(DISTINCT install_id) >= 20"
    Views -> OpsWorker: "SELECT ONLY"
    OpsWorker -> Bundle: "Ed25519 Signed JSON"
    Bundle -> LocalVox: "Optional Pull (Online)\nCached in ~/.vox/cache/"
}
```

### 3.1 Roles & Least-Privilege Fencing

1. **`vox_ops_owner` (`NOLOGIN`)**:
   - Owns schemas `telemetry`, `community`, and `syndication`, all base tables, and all `SECURITY DEFINER` views/functions.
   - Migrations execute via `SET ROLE vox_ops_owner` during provisioning.
2. **`vox_ingest_app` (`LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS CONNECTION LIMIT 10`)**:
   - `ALTER ROLE vox_ingest_app SET statement_timeout = '5s';`
   - Granted `CONNECT` on database `vox_ops`, `USAGE` on schema `telemetry`, and **`INSERT` only** on `telemetry.events_raw`.
   - Holds **zero** `SELECT`, `UPDATE`, `DELETE`, or `TRUNCATE` privileges on any table or view. Even under total compromise of the public `/v1/logs` worker pool, historical telemetry rows cannot be read, enumerated, or mutated.
3. **`vox_ops_app` (`LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS CONNECTION LIMIT 15`)**:
   - `ALTER ROLE vox_ops_app SET statement_timeout = '15s';`
   - **Denied** direct `SELECT`, `INSERT`, `UPDATE`, `DELETE` on `telemetry.events_raw`.
   - Granted **`SELECT` only** on owner-owned `SECURITY DEFINER` (`WITH (security_barrier = true)`) views `telemetry.v_<category>_k20`, each enforcing `GROUP BY ... HAVING count(DISTINCT install_id) >= 20`.
   - `REVOKE INSERT, UPDATE, DELETE, TRUNCATE ON ALL TABLES IN SCHEMA telemetry FROM vox_ops_app;` (and explicitly on each view so Postgres auto-updatable view rules can never be abused to mutate base rows).
   - Granted `SELECT, INSERT, UPDATE` on `community.intelligence_snapshots`, `syndication.outbox`, `syndication.attempt_ledger`, and `community.moderation_audit` (append-only audit tables revoke `DELETE` and `UPDATE` where appropriate).
4. **Database-Level Isolation & SCRAM Provisioning**:
   - `REVOKE CONNECT, TEMPORARY ON DATABASE vox_ops FROM PUBLIC;`
   - Passwords are never sent in plaintext SQL or logged: `deploy/helios/vox-ops-db-setup` verifies `log_statement = 'none'`, `log_min_duration_statement = '-1'`, and `password_encryption = 'scram-sha-256'`, generates a 32-byte random secret locally, computes the `SCRAM-SHA-256$4096:...` verifier via `scram-verifier.py`, executes `SET log_min_error_statement = panic; ALTER ROLE ... PASSWORD '<verifier>';`, and injects runtime-only `VOX_INGEST_DATABASE_URL` and `VOX_OPS_DATABASE_URL` into Coolify via `php artisan tinker`.
   - Every SQL setup run ends with a deterministic single-line privilege probe (`has_table_privilege`, `rolsuper`, `rolcanlogin`, `tableowner`, cross-database `CONNECT` check) that must match the exact expected vector before deployment proceeds.

---

## 4. Definitive Separation of Concerns: Local `VoxDb` vs. Centralized `vox-ops` Postgres

### 4.1 Architectural Decision Rule

Every persistence and automation surface in the Vox ecosystem is assigned to **Local `VoxDb` (Turso/SQLite in public `vox`)**, **Centralized `vox-ops` (PostgreSQL 17 on `helios-postgres`)**, or **Hybrid (Local Authoritative + Optional Signed Community Priors)** using four strict criteria:

| Criterion | Local `VoxDb` (`~/.vox/db/`) + Public `vox` | Centralized `vox-ops` Postgres (`helios-postgres`) |
|---|---|---|
| **Offline Sovereignty** | Required for compiling `.vox`, running `vox chat`/`vox term`/`vox-gui`, local KB search, local Mens training, or preparing reproducible scholarly manuscripts (`CITATION.cff`, arXiv/Zenodo bundles). | Never required for local operation; provides asynchronous cross-install priors or runs Foundation-operated 24/7 background workers. |
| **Cold-Start vs. Fleet Sample Size** | Single-user sample size (`n_calls = 0..5` per model/task on a new install) suffers from cold-start blindness across 30+ frontier/OpenRouter models and 6 benchmark suites. | Collates $k \ge 20$ distinct installs (`telemetry.v_*_k20`) to eliminate cold-start blindness while preserving differential privacy ($\epsilon = 1.0$). |
| **Secret & Credential Blast Radius** | Uses only the developer's own local `Clavis` keyring (`OPENAI_API_KEY`, personal `ZENODO_ACCESS_TOKEN`, local `ORCID` key). | Holds Vox Foundation operational credentials (official X/Reddit/Bluesky/Discord/YouTube/LinkedIn/OpenCollective tokens, Foundation Crossref/Zenodo prefix keys, Ed25519 bundle signing key). |
| **Concurrency & Scheduling** | Single-user daemon (`vox-orchestrator-d`) with local SQLite/Turso actor writer. | Multi-worker transactional outbox (`SELECT ... FOR UPDATE SKIP LOCKED`), 24/7 webhook receivers, rate-limit window coordination, and scheduled feed crawlers. |

---

### 4.2 Pillar A: Model Performance Stats, Benchmarks, Pricing, Hardware Throughput & Prompt Profiles

Local `VoxDb` already defines six model-intelligence tables (`crates/vox-db/src/schema/domains/scientia.rs` and `publish_cloud.rs`), but on any single workstation they are sparse or cold-start empty for unvisited `(model_id, provider, task_category)` pairs. Centralizing their anonymized rollups in `vox-ops` Postgres and redistributing them via `CommunityIntelligenceBundleV1` gives every user immediate fleet-wide empirical priors while keeping local measurements authoritative for their own machine:

| Capability / Table | Local `VoxDb` Role (Authoritative for Local Machine) | Centralized `vox-ops` Postgres Role (`telemetry.*` & `community.*`) | Client Hydration & Blending Rule (Offline-Safe) |
|---|---|---|---|
| **Empirical Routing Scoreboard** (`model_scoreboard`) | Tracks local `(model_id, task_category, strength_tag, window_days)` -> `n_calls`, `success_rate`, `p50_latency_ms`, `p99_latency_ms`, `p95_ttft_ms`, `p95_tpot_ms`, `goodput_tokens_per_sec`, `cost_per_success_usd`. | `telemetry.v_model_routing_priors_7d` ($k \ge 20$ installs, $N \ge 50$ calls) computes fleet-wide p50/p95 TTFT, TPOT, goodput, success rate, and 429 rate per `(provider, canonical_model_id, task_category)`. | **Empirical Bayes blending**: $S_{\text{eff}} = w_{\text{local}} S_{\text{local}} + (1 - w_{\text{local}}) S_{\text{comm}}$, where $w_{\text{local}} = \frac{n_{\text{local}}}{n_{\text{local}} + \tau}$ ($\tau = 10$). At $n_{\text{local}} = 0$ (cold start), community prior guides routing (bounded to $\pm 15\%$ of static registry score); as $n_{\text{local}} \gg 10$, local measurements dominate. |
| **Graded Benchmark Suites** (`model_benchmark_results` via `vox models bench`) | Stores local per-case runs across `codegen`, `reasoning`, `json`, `tool_call`, `needle`, and `vox_gen`, including `resolved_model_id` (detecting `-latest` / `openrouter/auto` alias targets). | `telemetry.v_model_benchmark_priors_14d` collates anonymized `(model_id, resolved_model_id, provider, suite, task_category)` pass rates, mean scores, and `cost_per_solve_usd` from opt-in benchmark telemetry + nightly `vox-ops` canary runs. | Hydrates baseline `quality_score` and detects silent upstream alias regressions (e.g., when `openrouter/auto` or a provider alias shifts to a weaker backend) before a user spends tokens locally. |
| **Observed Pricing & Free-Tier Liveness** (`model_pricing_catalog`) | Tracks local `observed_blended_per_1k`, `observed_input_per_1k`, `observed_output_per_1k`, `n_free`, and `confidence` from local `llm_interactions`. | `telemetry.v_model_pricing_observed_7d` aggregates provider-reported cost per 1k tokens and OpenRouter `:free` availability/throttle rates across the fleet. | Updates local `model_pricing_catalog` priors so `TurnCostReceipt` and budget gates (`uses { Spend }`) have accurate cost estimates and automatically avoid dead/throttled free-tier endpoints. |
| **Provider Atlas Behavior Profiles** (`scientia_model_profile_learning`) | Learns local per-model `refusal_rate`, `malformation_rate`, `json_schema_adherence`, and `tool_call_retry_rate`. | `telemetry.v_provider_atlas_profiles_14d` aggregates failure/malformation rates per `(provider, model_id, profile_key)` across $\ge 20$ installs. | Pre-populates `scientia_model_profile_learning` so structured-output and tool-calling tasks select appropriate grammar/retry strategies on the first call. |
| **Learned System-Prompt Profiles** (`model_prompt_profiles`) | Stores `(prompt_profile_key, variant_id, preamble_text, confidence, quality_delta, applications)` approved locally. | `telemetry.v_prompt_profile_priors_14d` measures empirical `quality_delta` across variants; Foundation council signs `Confirmed` `ModelPromptProfile` variants into `community.intelligence_bundles`. | Only `Confirmed` Ed25519-verified variants or locally human-approved variants are injected by `crates/vox-orchestrator/src/models/prompt_profile.rs`. |
| **GPU Training Throughput Profiles** (`training_throughput_profiles`, `local_train_log`, `cloud_dispatch_log`) | Records local QLoRA/GRPO `ms_per_step` and `tokens_per_dollar` per `(gpu_name, seq_len, batch_size)` on the user's GPU. | `telemetry.v_training_throughput_priors_30d` aggregates `ms_per_step` by `(gpu_name, seq_len, batch_size)` across community hardware (RTX 4090, 4080 Super, Apple M2/M3/M4 Max, RunPod/Vast A100/H100). | Allows `vox mens train --estimate` and cloud GPU offer selection (`vox mens cloud`) to predict wall-clock time and cost accurately on GPUs the user has never benchmarked locally. |

---

### 4.3 Pillar B: Scientia Epistemic Discovery, Research Trials & Publication Pipelines

The Scientia and Publication stack (`crates/vox-publisher` and `crates/vox-db/src/schema/domains/{scientia,publish_cloud,trials}.rs`) is partitioned into **three cleanly separated sub-pipelines**:

```vox-diagram
graph {
    container LocalVox: "1. Local Vox & VoxDb (100% Standalone / Public Repo)" {
        node Producers: "Self-Observation Producers\n(commit_watcher, code_uniqueness,\nharness_issues, trial_campaigns)"
        node Discovery: "Epistemic Discovery & Verification\n(scientia_discoveries, scientia_claims,\nscientia_claim_verdicts, scientia_nanopubs)"
        node Manuscript: "Local Manuscript & Bundle Prep\n(publication_manifests, CITATION.cff,\nBibTeX, arXiv tar.gz, Zenodo/OpenReview JSON)"
        Producers -> Discovery
        Discovery -> Manuscript
    }
    container DualScholarly: "2. Scholarly Venue Submission (Dual-Mode)" {
        node PersonalCLI: "Mode A: Direct Personal CLI\nvox scientia publication-submit\n(User's own Zenodo/OpenReview/ORCID token)"
        node FoundationQueue: "Mode B: Foundation scientia_ops Queue\n(vox-ops Postgres FOR UPDATE SKIP LOCKED\n24/7 DOI/peer-review polling & co-author gate)"
    }
    container CentralOps: "3. Centralized vox-ops (Private Repo + helios-postgres)" {
        node Syndication: "vox-ops-syndication\nsyndication.outbox (X, Reddit, YouTube,\nDiscord, Bluesky, Mastodon, LinkedIn, HN, OC)"
        node IntelDigest: "vox-ops-intelligence\n1. Domain Reputation (v_domain_reputation_k20)\n2. Curated Prior-Art Feed Digest\n3. Model & Benchmark Priors"
    }
    LocalVox.Manuscript -> DualScholarly.PersonalCLI: "Personal Research"
    LocalVox.Manuscript -> DualScholarly.FoundationQueue: "SyndicationHandoffPayloadV1\n(Optional Webhook / JSON Export)"
    LocalVox.Manuscript -> CentralOps.Syndication: "SyndicationHandoffPayloadV1"
    CentralOps.IntelDigest -> LocalVox.Discovery: "Ed25519 Signed Bundle\n(Optional Pull)"
}
```

#### Sub-Pipeline B1: Local Epistemic Discovery, Trials, Nanopubs & Manuscript Authoring (**100% Local `VoxDb` + Public `vox`**)
All scientific discovery, verification, reproducibility, and manuscript preparation stays in public `vox` (`crates/vox-publisher` + `crates/vox-db`) with zero server dependency:
- **Self-Observation & Finding Producers (`scientia_producers/*`):** `commit_watcher.rs`, `code_uniqueness.rs`, `scientia_producer_cursor`, `scientia_finding_candidates`, `scientia_discovery_inbox`, and `scientia_harness_issues` / `scientia_harness_decisions` / `scientia_harness_fix_proposals` (which inspect local git history and local chat sessions).
- **Epistemic Claims, Socrates Verification & Nanopublications:** `scientia_research_sessions`, `scientia_research_artifacts`, `scientia_claims`, `scientia_claim_verdicts`, `user_identities`, and `scientia_nanopubs` (offline-validated RDF/TriG nanopublications with `trusty_uri`).
- **Pre-Registered Empirical Trials (`SCHEMA_TRIALS`):** `trial_campaigns`, `trial_campaigns_log`, `trial_runs`, and `trial_samples` (cryptographically sealed pre-registrations bound to `git_commit` and `cargo_lock_blake3`).
- **Manuscript Authoring, Worthiness & Reproducible Packaging:** `publication_manifests`, `publication_approvals`, `scientia_review_decisions`, `publication_media_assets`, `scientia_evidence/*`, `publication_worthiness.rs`, `publication_preflight/*`, `peer_review_gate.rs`, `citation_cff.rs`, `scientific_metadata.rs`, `crossref_metadata.rs`, `zenodo_metadata.rs`, and `submission/arxiv.rs`.

#### Sub-Pipeline B2: Scholarly Venue Deposits (`Zenodo`, `OpenReview`, `Crossref`, `OSF`, `Software Heritage`, `ORCID`) — **Dual-Mode Architecture**
Scholarly venue adapters (`crates/vox-publisher/src/scholarly/{zenodo,openreview,crossref_deposit,osf,software_heritage,orcid_oauth,arxiv_api}.rs`) serve two distinct actors:
1. **Individual Researcher (Local Direct Mode in Public `vox`):**
   - A researcher using Vox to write and submit their own paper or dataset keeps `scholarly/*` and local `VoxDb` tables (`scholarly_submissions`, `external_submission_jobs`, `external_submission_attempts`, `external_status_snapshots`) so `vox scientia publication-submit` works directly from their laptop using their personal `ZENODO_ACCESS_TOKEN` / `OPENREVIEW_TOKEN` / `ORCID` credentials in local `Clavis`.
2. **Vox Foundation Official Preprints & Multi-Author Campaigns (Centralized `scientia_ops` Schema in `vox-ops` Postgres):**
   - Official Foundation preprints (e.g., `CR-L8` empirical LLM codegen benchmarks), Foundation DOI prefix minting (`Crossref`), multi-author sign-off ledgers (`atlas/coauthor.rs`, `atlas/governance.rs`, `atlas/retraction.rs`), and 24/7 asynchronous peer-review status polling run in `vox-ops` backed by Postgres (`scientia_ops.submission_jobs`, `scientia_ops.coauthor_approvals`, `scientia_ops.retraction_ledger`) so Foundation deposit credentials never leave `helios`.

#### Sub-Pipeline B3: Web Domain Reputation, Prior-Art Feed Crawling & Social/Community Syndication (**Centralized in `vox-ops` Postgres**)
Three Scientia/Publication surfaces gain major reliability, security, and performance benefits by moving their active execution and state coordination to `vox-ops` Postgres:
1. **Live Social Media & Community Automation (`vox-ops-syndication` + Postgres `syndication.*`):**
   - **Moved from `crates/vox-publisher/src/adapters/` to `~/dev/vox-ops/crates/vox-ops-syndication/`:** `twitter.rs`, `reddit.rs`, `youtube.rs`, `discord.rs`, `bluesky.rs`, `mastodon.rs`, `linkedin.rs`, `hacker_news.rs`, `opencollective.rs`, `social_retry.rs`, `switching.rs`, and `syndicate.rs`.
   - **Why Postgres:** Multi-channel syndication requires transactional outbox locking (`SELECT ... FOR UPDATE SKIP LOCKED` on `syndication.outbox`), cross-worker rate-limit windows (`syndication.rate_limit_windows`), OAuth refresh token rotation under advisory locks, digest-bound approval gates (`syndication.approvals`), and immutable delivery receipts (`syndication.delivery_receipts`).
   - **What stays in public `vox-publisher`:** Deterministic `SyndicationHandoffPayloadV1` generation (`distribution_compile.rs`, `templates.rs`, `contract.rs`), local RSS/Atom feed rendering (`adapters/rss.rs`), repo-native GitHub release draft generation (`adapters/github.rs`), and an optional authenticated webhook push (`VOX_OPS_SYNDICATION_WEBHOOK_URL`) to hand off approved campaigns to `vox-ops`.
2. **Community-Collated Web Research Domain Reputation (`research_misguidance_events` -> `telemetry.v_domain_reputation_priors_30d`):**
   - Local `VoxDb` records `research_misguidance_events` and `research_domain_reputation` (`culprit_domain`, `defect_class`, `penalty_score`, `is_blacklisted`) whenever web research returns hallucinated or broken API docs that fail compilation/verification.
   - By emitting a privacy-safe telemetry event containing only `(culprit_domain, defect_class)` (never the URL path, query, or code snippet) and aggregating across $k \ge 20$ distinct installs in `telemetry.v_domain_reputation_priors_30d`, `CommunityIntelligenceBundleV1` distributes a signed **Domain Reputation Prior** that local `vox-search` / Scientia RAG blends into `research_domain_reputation`—protecting every developer from known low-quality/SEO-spam documentation domains on their very first query.
3. **Centralized Prior-Art & External Intelligence Feed Crawler (`scientia_feed_sources` / `scientia_external_intelligence`):**
   - Instead of every developer workstation redundantly polling arXiv, OpenReview, and RSS feeds (`scientia_feed_sources`), `vox-ops` runs the continuous feed crawler on `helios`, scores candidate papers with `scientia_prior_art.rs` / `scientia_worthiness_enrich.rs` into Postgres `scientia_ops.external_intelligence`, and ships top high-worthiness prior-art digests in `CommunityIntelligenceBundleV1` (while keeping local `scientia_feed_sources` in `VoxDb` for custom user-configured feeds).

---

### 4.4 Master Ownership Matrix (Public `vox` + Local `VoxDb` vs. Private `vox-ops` + Postgres)

| Subsystem / Artifact | Public `vox` + Local `VoxDb` (Turso/SQLite) | Private `~/dev/vox-ops` + `helios-postgres` (`vox_ops`) | Sync / Handoff Mechanism |
|---|---|---|---|
| **Telemetry Recording & Local Sink** | `vox-telemetry`, `vox-telemetry-otlp`, `ResearchMetricsSink` (all 21 `TelemetryEvent` variants in local `research_metrics` + `~/.vox/telemetry/*.jsonl`) | `vox-ops-ingest` (`POST /v1/logs`) writing via `vox_ingest_app` (`INSERT`-only) to `telemetry.events_raw` | Opt-in `upload_pending_otlp` (`ConsentState::Granted`); offline inspectable via `vox telemetry preview`. |
| **Model Routing Scoreboard & Priors** | `model_scoreboard` (local calls), `vox-orchestrator::models::{registry,select}` | `telemetry.v_model_routing_priors_7d` ($k \ge 20$) -> `CommunityIntelligenceBundleV1` | Non-blocking pull of Ed25519-signed bundle; Empirical Bayes blend ($w_{\text{local}} = \frac{n}{n + 10}$). |
| **Model Benchmark Suites (`vox models bench`)** | `model_benchmark_results` (local runs across 6 suites) | `telemetry.v_model_benchmark_priors_14d` + scheduled `vox-ops` canary benchmark runs | Included in `CommunityIntelligenceBundleV1` (`model_benchmark_priors`). |
| **Observed Model Pricing & Free-Tier Health** | `model_pricing_catalog` (local `llm_interactions` cost tracking) | `telemetry.v_model_pricing_observed_7d` ($k \ge 20$ cost & 429 rate rollups) | Included in `CommunityIntelligenceBundleV1` (`pricing_and_liveness_priors`). |
| **Provider Atlas & Prompt Profiles** | `scientia_model_profile_learning`, `model_prompt_profiles` | `telemetry.v_provider_atlas_profiles_14d`, `telemetry.v_prompt_profile_priors_14d` | Council-signed `Confirmed` profiles in `CommunityIntelligenceBundleV1`. |
| **GPU Training Throughput Estimation** | `training_throughput_profiles`, `local_train_log`, `cloud_dispatch_log` | `telemetry.v_training_throughput_priors_30d` (per `gpu_name`, `seq_len`, `batch_size`) | Included in `CommunityIntelligenceBundleV1` (`training_throughput_priors`). |
| **Scientia Discovery, Claims, Nanopubs & Trials** | `scientia_discoveries`, `scientia_claims`, `scientia_claim_verdicts`, `scientia_nanopubs`, `trial_campaigns`, `trial_runs`, `trial_samples`, `scientia_harness_*` | *None* (100% local & private to the researcher's workspace) | Local-only by design; never uploaded to telemetry. |
| **Scholarly Manuscript Packaging & Venue Deposit** | `publication_manifests`, `CITATION.cff`, BibTeX, arXiv tar.gz, and personal direct `scholarly/{zenodo,openreview,crossref_deposit,osf,orcid_oauth}.rs` | `scientia_ops.submission_jobs`, `scientia_ops.coauthor_approvals`, `scientia_ops.retraction_ledger` (for official Vox Foundation multi-author publications) | `vox scientia publication-submit` runs locally for personal tokens or exports `ScholarlySubmissionHandoffV1` to `vox-ops` for Foundation publications. |
| **Research Domain Reputation & Prior-Art Feeds** | `research_misguidance_events`, `research_domain_reputation`, custom local `scientia_feed_sources` | `telemetry.v_domain_reputation_priors_30d` ($k \ge 20$ `culprit_domain` penalties) + `scientia_ops.external_intelligence` crawler | Included in `CommunityIntelligenceBundleV1` (`domain_reputation_priors` & `prior_art_digest`). |
| **Social Media & Community Syndication** | `SyndicationHandoffPayloadV1` compiler (`distribution_compile.rs`, `templates.rs`), local RSS (`rss.rs`), GitHub Release notes (`github.rs`) | `vox-ops-syndication` (`twitter`, `reddit`, `youtube`, `discord`, `bluesky`, `mastodon`, `linkedin`, `hacker_news`, `opencollective`) + `syndication.outbox` (`FOR UPDATE SKIP LOCKED`) | `vox scientia syndication export` (JSON file) or optional authenticated webhook (`VOX_OPS_SYNDICATION_WEBHOOK_URL`). |

---

## 5. Community-Collated Intelligence Feed (`CommunityIntelligenceBundleV1` Contract)

To let every Vox user benefit from community-wide performance, benchmark, pricing, hardware throughput, and domain-reputation observations without introducing any server dependency:

1. **Signed Intelligence Bundle (`CommunityIntelligenceBundleV1`)**:
   - Generated periodically by `vox-ops-intelligence` querying only the owner-defined `SECURITY DEFINER` $k \ge 20$ views (`telemetry.v_model_routing_priors_7d`, `telemetry.v_model_benchmark_priors_14d`, `telemetry.v_model_pricing_observed_7d`, `telemetry.v_provider_atlas_profiles_14d`, `telemetry.v_default_decision_priors_14d`, `telemetry.v_prompt_profile_priors_14d`, `telemetry.v_training_throughput_priors_30d`, `telemetry.v_domain_reputation_priors_30d`, and `telemetry.v_skill_reliability_14d`).
   - Contains seven coarse, anonymous statistical sections:
     1. `model_routing_priors`: Per-provider/model p50/p95 TTFT & TPOT, goodput tokens/sec, success rate, and 429 throttle rate;
     2. `model_benchmark_priors`: Graded pass rates, mean scores, and `cost_per_solve_usd` across the 6 `vox models bench` suites (`codegen`, `reasoning`, `json`, `tool_call`, `needle`, `vox_gen`), plus `resolved_model_id` drift alerts;
     3. `pricing_and_liveness_priors`: Observed per-1k token pricing and OpenRouter `:free` endpoint liveness;
     4. `provider_atlas_profiles`: Per-model `refusal_rate`, `malformation_rate`, and tool-schema strictness priors;
     5. `default_decision_priors` & `confirmed_prompt_profiles`: Empirical `default_decision` constants and council-signed `Confirmed` `ModelPromptProfile` preambles;
     6. `training_throughput_priors`: Empirical `ms_per_step` per `(gpu_name, seq_len, batch_size)` for `vox mens train --estimate`;
     7. `domain_reputation_priors` & `prior_art_digest`: $k \ge 20$ `culprit_domain` penalty scores for `vox-search` / Scientia RAG and curated high-worthiness scholarly feed digests.
   - Signed with the Vox Foundation Ed25519 key (`vox-crypto`) and served over HTTPS (`GET /v1/intelligence/manifest.json` and `GET /v1/intelligence/bundle.v1.json.zst`).
2. **Client Hydration & Fallback Rules**:
   - Cached locally at `~/.vox/cache/community-intelligence.v1.json`.
   - Verified against the compiled-in Vox Foundation Ed25519 public key before parsing.
   - **Never blocks startup or command execution**: refreshed only on explicit `vox telemetry intelligence sync` or a background non-blocking daemon tick when opt-in remote intelligence is enabled (`VOX_COMMUNITY_INTELLIGENCE != off`).
   - If offline, disabled, expired, or unconfigured, `vox-orchestrator`, `vox-search`, `vox-publisher`, and `vox-config` use the compiled-in SSOT defaults and local `VoxDb` tables (`model_scoreboard`, `model_benchmark_results`, `model_pricing_catalog`, `training_throughput_profiles`, `research_domain_reputation`) with zero warnings or degradation.

---

## 6. Strong Typing Contract Across Telemetry, Router & Database Boundaries

Every boundary in the telemetry, model routing, and database stack MUST enforce strong static typing in both Rust code and database schemas:

1. **Client Telemetry & OTLP Projection (`vox-telemetry` & `vox-telemetry-otlp`)**:
   - All taxonomy-governed fields (`DurationBucket`, `TokenBucket`, `CountBucket`, `SizeBucket`, `CommandExitClass`, `EditOpType`, `EditFileKind`, `HarnessToolCallKind`, `HarnessMode`, `SkillTriggerSource`, `SkillSurface`, `ErrorSubsystem`, `TelemetryErrorClass`, `GpuVendor`, `VramBucket`, `TokPerSecBucket`, `DomainCategory`, `DomainDefectClass`, `SaltedHashHex`) provide typed Rust representations and validation helpers so invalid enum values or unhashed strings are rejected at construction/serialization time rather than silently dropped downstream.
   - `TelemetryEvent` variant handling in `ResearchMetricsSink::record` (`crates/vox-db/src/telemetry_sink.rs`) and `project_event` (`crates/vox-telemetry-otlp/src/project.rs`) uses `#[deny(unreachable_patterns)]` over explicit match arms for all 26 variants.
2. **Local `VoxDb` Strong-Typed Shape Verification (`crates/vox-db`)**:
   - `VoxDb` exposes a strongly-typed row auditor (`audit_research_metric_shapes` / `TypedResearchMetricRow`) that reads `research_metrics`, `model_scoreboard`, and `scientia_model_profile_learning` rows and deserializes every `metadata_json` payload back into its concrete Rust struct (`ModelCallEvent`, `SelectionDecisionEvent`, `TaskRootSummaryEvent`, `ModelBenchCaseEvent`, `TrainingThroughputEvent`, `DomainMisguidanceEvent`, etc.), flagging any malformed JSON, unknown `metric_type`, or violated `session_id` prefix as a hard validation failure.
3. **Server Postgres & `sqlx` Domain Types (`vox-ops-db` & `vox-ops-ingest`)**:
   - PostgreSQL tables under `telemetry_ingest.*`, `syndication.*`, and `scientia_ops.*` use explicit column types (`TIMESTAMPTZ`, `BIGINT`, `DOUBLE PRECISION`, `BOOLEAN`, and `TEXT` with `CHECK` constraints matching `contracts/telemetry/collection-taxonomy.v1.json` enum sets) — never untyped `JSONB` bags for taxonomy-defined attributes.
   - `vox-ops` Rust crates bind all database reads/writes through strongly-typed `sqlx::FromRow` structs and `#[derive(Serialize, Deserialize)]` domain types shared with `CommunityIntelligenceBundleV1`.

---

## 7. Multi-Surface Model Router Verification Matrix

The model router (`crates/vox-orchestrator/src/models/{registry,select,auto_select,autonomic,scoring,ranking,pareto,prompt_profiles}.rs`) MUST be verified across all four execution surfaces with both unit and end-to-end integration tests:

| Surface | Entry Point | Router Verification Contract | Expected Telemetry & DB Shape |
|---|---|---|---|
| **1. CLI Surface** | `vox model list`, `vox check`, `vox doctor` | Loads `ModelRegistry`, applies `SelectionPolicy` and `AutoRoutingPriority`, selects candidate model deterministically, and falls back cleanly when offline. | Emits `TelemetryEvent::SelectionDecision` (`vox.model.selection_decision`) and `TelemetryEvent::CommandUsage` (`vox.command.usage`) into local `research_metrics` and OTLP spool. |
| **2. Daemon Surface (`vox-orchestrator-d`)** | `step_db` -> `step_model_registry` -> `step_routing_preferences` (`crates/vox-orchestrator-d/src/daemon_steps/run.rs`) | Attaches `VoxDb`, initializes process telemetry via `init_process_telemetry(Some(db))`, loads persisted `routing_priority` (`AutoRoutingPriority`) and `selection_policy` (`SelectionPolicy`) from `user_preferences`, and serves RPC model queries. | Persists `SelectionDecisionEvent`, `DiscoveryEvent`, `ClassificationEvent`, and `ConfidencePromotionEvent` rows into `.vox/store.db` `research_metrics`. |
| **3. MCP Surface (`vox-orchestrator-mcp`)** | `llm_bridge::infer` & `chat_tools::build_system_prompt_with_skill` | Resolves model via `select_model`, injects `Confirmed` `ModelPromptProfile`, records latency/tokens/cache hits, and updates `model_scoreboard` + `scientia_model_profile_learning`. | Emits `ModelCallEvent` (`model_call_event`), `ModelPromptEvent` (`vox.model.prompt`), `SkillActivationEvent` (`vox.skill.activation`), and `HarnessUsageEvent` (`vox.harness.usage`). |
| **4. GUI Surface (`vox-gui`)** | `vox-gui/src/main.rs` + `PersistentDaemon` IPC | Initializes non-competing GUI process telemetry (`init_process_telemetry(None)`), reads/writes `routing_priority` and `selection_policy` through the daemon, and renders live model scoreboard & autonomic status. | Zero SQLite lock contention (`CORE-04`); all model routing decisions and LLM calls persist via `vox-orchestrator-d` into `.vox/store.db`. |

---

## 8. Live Helios Deployment & End-to-End Database Shape Audit

Milestone `v1.7` requires live production verification on **Helios** (`178.156.212.19`, Coolify project **"Vox Ecosystem"** `no88080okk0c0gk4ss0cgw0o`, shared PostgreSQL 17 instance `helios-postgres` `q40ck888ggkkcsosgcwgc0oo`):

1. **Live Database Provisioning on `helios-postgres`**:
   - Provision database `vox_ops` and the three SCRAM-SHA-256 fenced roles (`vox_ops_owner`, `vox_telemetry_ingest`, `vox_intelligence_ro`) on `helios-postgres`, apply SQL migrations (`0001_roles_and_schemas.sql` through `0004_syndication_and_scientia.sql`), and verify role fencing (`SET ROLE vox_telemetry_ingest; SELECT * FROM telemetry_ingest.events` -> `ERROR: permission denied`).
2. **Live Coolify API Deployment of `vox-ops`**:
   - Drive the Coolify REST API (`COOLIFY_BASE_URL`, `COOLIFY_TOKEN` via `vox_secrets`) and Helios deployment tooling to deploy the `vox-ops` service stack (`vox-ops-ingest` + `vox-ops-intelligence`) and verify public/container health probes (`GET /healthz` -> `200 OK`).
3. **End-to-End Client -> Helios Ingest & Bundle Hydration**:
   - Run real local CLI, daemon, MCP, and model-router operations to populate local `VoxDb` (`research_metrics`, `model_scoreboard`, `scientia_model_profile_learning`) and the local OTLP spool (`.vox/telemetry-upload-queue/pending/`).
   - Inspect queued payloads with `vox telemetry preview --format json` and `vox telemetry preview --format table`.
   - Execute `vox telemetry upload` against the live Helios ingest endpoint (`POST /v1/logs`), confirming HTTP `200 OK` batch acknowledgement and local spool cleanup.
   - Query both local `VoxDb` and live `helios-postgres` (`vox_ops`) to confirm normal data is gathered in the exact strongly-typed columns and JSON schemas with zero malformed rows, and verify `GET /v1/intelligence/bundle.v1.json.zst` Ed25519 signature verification and Empirical Bayes hydration in `vox-orchestrator`.

---

## 9. Ludus & Gamification 3-Way Separation of Concerns (SoC) Architecture

An architectural audit of `crates/vox-gamify` (`Ludus`) and `crates/vox-cli/src/commands/extras/ludus/` revealed four distinct responsibilities entangled in a single crate. Milestone `v1.7` enforces a strict **3-Way Separation of Concerns (SoC)**:

| Tier | Responsibility & Modules | Target Home | Rationale |
|---|---|---|---|
| **Tier 1: Core Agent Session, Cost & Streaming Infrastructure** | `AgentEventRecord`, `CostRecord`, `AgentSessionRecord`, `get_events`, `insert_event`, `insert_cost_record`, `list_cost_records`, `get_agent_session`, `get_tenant_monthly_token_usage` (formerly `vox_gamify::db::agent_telemetry`); plus `FreeAiClient` / `StreamRoute` / `LudusStreamBackend` routing primitives | **`crates/vox-db`** (`store::ops_agent_telemetry`) & **`crates/vox-orchestrator`** / **`crates/vox-actor-runtime`** | Core orchestrator, MCP (`dei_tools`), and actor runtime must not depend on a gamification crate for LLM session tracking, tenant token budgets, or stream routing. |
| **Tier 2: Multi-User Competitive & Social Ludus** | Guilds (`collegium.rs`: `gamify_collegium`, `gamify_collegium_members`), Community Raids (`arena.rs`: `gamify_arena_events`, `gamify_arena_participants`), Global Rankings (`leaderboards.rs`: `leaderboard`, `lumens_leaderboard`), Anti-Cheat Peer Juries (`disputes.rs`: `gamify_disputes`, `gamify_dispute_votes`, `gamify_dispute_jury`), and GitHub OAuth / Event Sync (`vox ludus auth github`, `vox ludus sync`) | **Private `vox-ops`** (`crates/vox-ops-ludus` + PostgreSQL schema `ludus_community.*` on `helios-postgres`) | Multi-user guilds, global leaderboards, community raid targets, and peer jury voting are inherently centralized multi-user state and cannot function truthfully inside a single-user local SQLite file. |
| **Tier 3: Solo Local Companion, Quests, Battles & FSRS Discovery** | `LudusProfile`, `Companion`, `Quest`, `Battle`, `StreakTracker`, `AchievementTracker`, `shop`, `sprite_svg`, and FSRS spaced-repetition CLI/GUI discovery (`discovery/{fsrs,rank,ledger,scientia_kudos}.rs`) | **Public `crates/vox-gamify`** + local `VoxDb` (100% offline/local) | Solo progression, companion sprites, and FSRS command discovery work 100% offline with zero network calls and zero multi-user server dependencies. |
| **Shadow Telemetry Elimination** | Unconsented parallel telemetry tables formerly inside `vox-gamify` (`gamify_hint_telemetry` in `db/hint_telemetry.rs`, `gamify_policy_snapshots` in `db/kpi.rs`, `lsp_telemetry.rs`) | **Removed from `vox-gamify`**; folded into `vox-telemetry` (`TelemetryEvent`) | Eliminates shadow telemetry tables outside `vox-telemetry` so 100% of telemetry obeys the `VOX_TELEMETRY=off` kill-switch and `collection-taxonomy.v1.json`. |

---

## 10. Codebase & Git-History Scrubbing Contract (`SCRUB-01..04`)

To ensure public `vox` contains zero dead server prototypes, moved social/community automation adapters, moved multi-user Ludus modules, or shadow telemetry tables—both in the current working tree and across repository history:

1. **Working-Tree Scrub (`SCRUB-01`)**:
   - Remove `server/telemetry/` (prototype ingest/aggregator crate).
   - Remove the 9 live social media & community syndication adapters from `crates/vox-publisher/src/adapters/` (`twitter`, `reddit`, `youtube`, `discord`, `bluesky`, `mastodon`, `linkedin`, `hacker_news`, `opencollective`).
   - Remove moved multi-user Ludus modules (`collegium.rs`, `arena.rs`, `leaderboards.rs`, `disputes.rs`, `ludus/auth.rs`, `ludus/sync.rs`, `ludus/collegium.rs`, `ludus/arena.rs`, `ludus/disputes.rs`) and shadow gamify telemetry (`db/hint_telemetry.rs`, `lsp_telemetry.rs`) from public `vox`.
2. **CI Scrub Guard (`SCRUB-02`)**:
   - Codify all moved paths, retired SQL table identifiers (`gamify_collegium`, `gamify_arena_events`, `gamify_disputes`, `gamify_hint_telemetry`), and banned server/syndication symbols in `contracts/ci/moved-data-scrub-manifest.v1.json`.
   - Enforce via automated test / CI check that zero references to moved files or shadow telemetry surfaces re-enter the public `vox` workspace, while preserving transparent opt-in `vox-telemetry` (`vox telemetry {status,preview,enable,disable,upload}`) and `telemetry-trust-ssot.md`.
3. **Git-History & Commit-Log Scrub Specification (`SCRUB-03`, `SCRUB-04`)**:
   - Provide `contracts/ci/git-history-scrub.v1.json` (`invert_paths` list + `message_replacements` regex rules) and a deterministic `git filter-repo` verification script (`scripts/verify-history-scrub.vox`) that strips moved `server/telemetry/`, moved social adapter files, moved multi-user Ludus files, and legacy unconsented telemetry commit-log references during maintenance-window history rewrite on `main`.

---

## 11. Dynamic Zero-Cheating Leaderboard & `voxlang.org` / `vox-gui` Surfaces (`LEADERBOARD-01..05`)

To prevent client-side score forgery (`sqlite3 .vox/store.db "UPDATE gamify_profiles SET xp = ..."` or replayed local events) while delivering a live, transparent community & model ranking experience across **`voxlang.org`** and **`vox-gui`**:

### 11.1 Server-Authoritative Anti-Cheat & BLAKE3 Hash-Chained Ledger (`vox-ops-ludus`)

1. **Zero Client Score Trust (`LEADERBOARD-01`)**:
   - Public/global leaderboards **never** accept client-submitted XP, Lumens, or local SQLite profile dumps.
   - Only `vox-ops-ludus` (connecting as `vox_ops_app` to `ludus_community.*` on `helios-postgres`) can mint leaderboard points, and only from cryptographically verified server-side receipts:
     - **Verified GitHub Contributions:** HMAC-SHA256 verified GitHub webhooks (`X-Hub-Signature-256` via `vox-crypto`) and server-side GitHub API verification for merged PRs (`PullRequestMerged` with green CI check-runs), independent peer reviews (`PullRequestReviewApproved`), issue resolutions with linked commits (`IssueClosedWithCommit`), and verified `.vox` doctest/corpus pairs (`VerifiedCorpusContribution`).
     - **Verified 5-Spoke Model & Adapter Evaluations:** Signed `MensBeforeAfterEvaluationV1` benchmark receipts (`VerifiedFiveSpokeBenchRun`) across the 25-problem 5-spoke suite (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`).
2. **Append-Only BLAKE3 Hash Chain & Anti-Gaming Policy (`LEADERBOARD-02`)**:
   - Table `ludus_community.verified_contributions` enforces `UNIQUE (upstream_provider, upstream_event_id)` (replay immunity) and links every award via `row_chain_hash_hex = BLAKE3(prev_chain_hash_hex || upstream_event_id || contributor_id || category || awarded_lumens || awarded_xp || evidence_blake3_hex)`.
   - `AntiCheatPolicyV1` rejects self-merged PRs lacking green CI or review, trivial diffs (`< 10` AST tokens), rapid issue open/close loops, and applies a daily per-category diminishing-returns curve (`1.0x` for 1–3, `0.5x` for 4–6, `0.1x` for `7+`) so spam grinding cannot overtake genuine engineering or research contributions.
3. **Ed25519-Signed Dynamic Snapshot (`VerifiedLeaderboardSnapshotV1`)**:
   - Served at `GET /v1/ludus/leaderboard` (`ETag`, `stale-while-revalidate`) and `GET /v1/ludus/leaderboard/stream` (SSE), signed with the Vox Foundation Ed25519 key (`vox-crypto`), with strongly-typed `#[serde(deny_unknown_fields)]` schema validation.

### 11.2 v0 / Claude Design GUI Surfaces (`voxlang.org` & `vox-gui`)

Both `voxlang.org` (`docs-astro`) and the desktop `vox-gui` (`GamifyView.tsx`) share a cohesive **v0 / Claude Design** visual language (editorial typography, warm-ivory/obsidian contrast, `1px` subtle borders, `tabular-nums` monospace telemetry cells, and interactive cryptographic proof drawers):

| Surface | Component & Route | Capabilities |
|---|---|---|
| **1. `voxlang.org` Public Web Surface (`LEADERBOARD-03`)** | `docs-astro/src/components/VoxLeaderboardArena.astro` (`<vox-leaderboard-arena>`) mounted at `docs/src/community/leaderboard.md` (`/community/leaderboard/`) | Dynamic fetch from `/v1/ludus/leaderboard` with build-time signed snapshot fallback (instant zero-CLS load + works offline), 4 interactive lenses (`Stewardship & Lumens`, `5-Spoke Model & Mens Arena`, `Collegium Guilds`, `Proof Chain Inspector`), time-window filters (`7D`, `30D`, `Season`, `All-Time`), search, and an expandable **Cryptographic Proof Drawer** showing `BLAKE3` receipt chains and `Ed25519` verification status. |
| **2. `vox-gui` Desktop Surface (`LEADERBOARD-04`)** | `crates/vox-gui/ui/src/components/surfaces/Gamify/LeaderboardArenaPanel.tsx` embedded in `GamifyView.tsx` | Replaces the raw 10-row list in `GamifyView.tsx` with the v0/Claude-styled `LeaderboardArenaPanel`, explicitly separating **Verified Global Arena** (Ed25519-verified `VerifiedLeaderboardSnapshotV1` + interactive anti-cheat receipt drawer) from **Solo Local Mastery** (badged `LOCAL SANDBOX · UNRANKED` so local offline progression is never confused with global anti-cheat standings). |
| **3. End-to-End Anti-Cheat & Surface Verification (`LEADERBOARD-05`)** | Rust + Vitest + Astro build verification tests | Proves tampered scores, forged client payloads, self-merged unreviewed PRs, replayed `upstream_event_id`s, and broken `BLAKE3`/`Ed25519` signatures are rejected (`100%` anti-cheat rejection rate), and verifies both `voxlang.org` and `vox-gui` leaderboard components render all 4 lenses and proof drawers cleanly. |
