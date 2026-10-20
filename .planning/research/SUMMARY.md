# Project Research Summary: Milestone v1.7 — Centralized Postgres Telemetry, Private `vox-ops` Extraction & Community Intelligence Flywheel

**Date:** 2026-10-19
**Canonical Architecture SSOT:** [`docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md`](../../docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md)

## Key Findings

### 1. Stack & Datastore Consolidation (`helios-postgres` over ClickHouse)
- **Replace ClickHouse with Shared Postgres 17 (`helios-postgres`):** The June 2026 `server/telemetry/` prototype used `clickhouse = "0.13.3"`. Across all production services on `helios` (`ramada` ADR-0008, `ngi.video`, `brFinancier`, `gigme`), the house standard is **one shared PostgreSQL 17 instance (`q40ck888ggkkcsosgcwgc0oo`, `pgvector/pgvector:pg17`)** (~7 MB marginal disk, ~0 extra RAM vs. 150–250 MB RSS for a separate database container).
- **Three-Role Privilege Fencing (`vox_ops` database):**
  - `vox_ops_owner` (`NOLOGIN`): owns schemas `telemetry`, `community`, `syndication`, all base tables (`telemetry.events_raw`, `community.intelligence_snapshots`, `syndication.outbox`, `syndication.attempt_ledger`), and all `SECURITY DEFINER` views.
  - `vox_ingest_app` (`LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS CONNECTION LIMIT 10`, `statement_timeout = '5s'`): `INSERT`-only on `telemetry.events_raw`; zero `SELECT`/`UPDATE`/`DELETE`/`TRUNCATE` privileges anywhere.
  - `vox_ops_app` (`LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS CONNECTION LIMIT 15`, `statement_timeout = '15s'`): reads telemetry exclusively through owner-owned `SECURITY DEFINER` (`security_barrier = true`) views enforcing `HAVING count(DISTINCT install_id) >= 20` (`k_anonymity = 20`), with `INSERT/UPDATE/DELETE` revoked on views (closing the `ramada` D-36 auto-updatable view gap), plus read/write grants on `syndication.*` and `community.*`.
  - Provisioned idempotently via `deploy/helios/vox-ops-db-setup` using client-side `scram-verifier.py` (`SCRAM-SHA-256`, `SET log_min_error_statement = panic`), `REVOKE CONNECT, TEMPORARY ON DATABASE vox_ops FROM PUBLIC`, Coolify runtime-only `DATABASE_URL` injection via `php artisan tinker`, and a terminal privilege-verification row (`t|f|f|...`).

### 2. Telemetry Surface Audit Across `vox`
- **`vox-telemetry-otlp::upload::upload_pending_otlp`** (`crates/vox-telemetry-otlp/src/upload.rs`) is currently a stub (`Ok(0)`). Must be wired to batch pending redacted OTLP JSON files into `/v1/logs` with `governor` rate limiting and Clavis-backed URL/token resolution.
- **`vox-orchestrator-d` and `vox-gui` missing recorder registration:** `set_global_recorder` is only invoked in `vox-cli` (`crates/vox-cli/src/lib.rs:808`), leaving all daemon and desktop GUI telemetry emissions as silent no-ops.
- **`ResearchMetricsSink` incomplete variant mapping:** `crates/vox-db/src/telemetry_sink.rs` only persists 6 of 21 `TelemetryEvent` variants locally, dropping all Track E/F product and autonomic events (`CommandUsage`, `SkillActivation`, `EditPattern`, `HarnessUsage`, `ErrorSurface`, `DefaultDecision`, `ModelPrompt`, `LintFinding`, `RepairAttempt`, `RepairOutcome`, `AuditRun`, `SelectionDecision`, `ModelDiscovery`, `ModelClassification`, `ConfidencePromotion`, `DoctorProjectCheck`).
- **Server DDL/Ingest column drop:** `server/telemetry/src/ingest.rs` (`EventRow`) hardcoded 19 fields and silently dropped 20+ taxonomy-allowlisted columns. The new Postgres DDL and `sqlx`/`tokio-postgres` ingest in `~/dev/vox-ops` will be 100% generated/verified against `contracts/telemetry/collection-taxonomy.v1.json`.
- **Transparency & Control UX:** Add `vox telemetry preview` (inspect exact outgoing OTLP payload without network IO) and GUI telemetry transparency/consent controls in `SettingsView.tsx` so users can inspect, toggle, or completely disable telemetry at any time.

### 3. Private Repo Split (`~/dev/vox-ops` / `brbrainerd/vox-ops`)
- **Remove `server/telemetry/` from public `vox`** (`Cargo.toml` exclude list + directory removal).
- **Extract live social media syndication & community automation** (`vox-publisher` social adapters: X/Twitter, Reddit, YouTube, Discord, Bluesky, Mastodon, LinkedIn, Hacker News, OpenCollective + `social_retry.rs` + scheduled syndication outbox/workers) into `~/dev/vox-ops`, backed by Postgres `syndication.outbox` (`FOR UPDATE SKIP LOCKED`) and `syndication.attempt_ledger`.
- **Retain local scholarly/academic manifest tooling in public `vox`:** `CITATION.cff`, BibTeX, Crossref/Zenodo/arXiv/OpenReview local bundle preparation remain in `vox-publisher`, with an optional non-blocking webhook/HTTP handoff to `vox-ops` when configured via Clavis.

### 4. Community-Collated Intelligence Flywheel (Offline-First)
- `vox-ops` publishes an Ed25519-signed `CommunityIntelligenceBundleV1` (`GET /v1/intelligence/bundle.json`) derived strictly from $k \ge 20$ views: empirical model latency/error-rate/cost priors, OpenRouter free-tier slug liveness, empirical `default_decision` recommended constants, and council-approved `Confirmed` `ModelPromptProfile` segments.
- `vox` clients operate 100% standalone/offline using compiled-in SSOT defaults and local `VoxDb`. When online and allowed (`VOX_COMMUNITY_INTEL != off` and `is_master_enabled()`), `vox` optionally fetches, Ed25519-verifies (`vox-crypto`), and caches `~/.vox/cache/community-intelligence.v1.json` to enrich routing and prompt selection without ever blocking or failing when offline.

## Implications for Roadmap

- **Phase 89 (`TELEM-AUDIT`):** Complete the client-side telemetry audit fixes in `vox` (`ResearchMetricsSink` 21-variant coverage, `vox-orchestrator-d` + `vox-gui` recorder registration, `vox-telemetry-otlp::upload` implementation, `vox telemetry preview` & GUI transparency controls) and remove `server/telemetry/` from public `vox`.
- **Phase 90 (`PG-FENCE`):** Bootstrap `~/dev/vox-ops` (`brbrainerd/vox-ops`) and implement the three-role fenced PostgreSQL 17 schema (`vox_ops_owner`, `vox_ingest_app`, `vox_ops_app`), taxonomy-driven DDL + `SECURITY DEFINER` $k \ge 20$ views, OTLP `/v1/logs` ingest server, and `deploy/helios/vox-ops-db-setup` SCRAM provisioning + Coolify pull-timer deployment.
- **Phase 91 (`SOCIAL-OPS`):** Extract live social media publication & community automation adapters from `vox-publisher` into `~/dev/vox-ops` with a Postgres-backed durable syndication outbox (`FOR UPDATE SKIP LOCKED`), keeping local scholarly/academic manifest prep and optional webhook handoff in public `vox`.
- **Phase 92 (`COMM-INTEL`):** Ship the Ed25519-signed `CommunityIntelligenceBundleV1` endpoint in `vox-ops` and the strictly offline-safe, non-blocking client hydration in `vox-orchestrator` / `vox-config` / `vox-cli` (`vox telemetry intelligence status|sync|inspect`), plus end-to-end verification and CI gate closure.

## Sources

- [`docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md`](../../docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md)
- [`docs/src/architecture/telemetry-trust-ssot.md`](../../docs/src/architecture/telemetry-trust-ssot.md)
- [`docs/src/adr/023-optional-telemetry-remote-upload.md`](../../docs/src/adr/023-optional-telemetry-remote-upload.md)
- [`contracts/telemetry/collection-taxonomy.v1.json`](../../contracts/telemetry/collection-taxonomy.v1.json)
- `~/dev/ramada/docs/decisions/0008-one-shared-postgres-two-roles-per-app.md`
- `~/dev/ngi.video/deploy/helios/ngi-video-db-setup` & `sql/ngi_video_early_access.sql`
