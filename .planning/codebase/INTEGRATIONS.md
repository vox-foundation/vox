---
last_mapped_commit: 9a79f708de1ce8f8b3944dc99afb51a2877fb960
last_mapped_at: 2026-09-22
---
# External Integrations

**Analysis Date:** 2026-09-22

## APIs & External Services

**LLM Providers (all routed through the model-agnostic facade — see below):**

- Google Gemini - direct + via OpenRouter
  - Auth: `GEMINI_API_KEY` (aliases `VOX_GEMINI_API_KEY`), `GEMINI_DIRECT_MODEL`, `OPENROUTER_GEMINI_MODEL` — `crates/vox-secrets/src/spec/registry/llm.rs`
- OpenRouter (unified LLM access) - Auth: `OPENROUTER_API_KEY` (aliases `VOX_OPENROUTER_API_KEY`), `OPENROUTER_MODEL`
- OpenAI - Auth: `OpenaiApiKey`/`OpenaiModel`/`OpenaiBaseUrl` (`SecretId` in `crates/vox-secrets/src/spec/ids.rs`)
- Anthropic - Auth: `AnthropicApiKey`
- Ollama (local) - `OllamaUrl`, `OllamaModel`
- Hugging Face - `HuggingFaceToken` (model/dataset hub access, `hf-hub` crate)
- Groq, Cerebras, Mistral, DeepSeek, SambaNova, Together, custom OpenAI-compatible - each has a dedicated `*ApiKey` `SecretId`
- **Model-agnostic boundary (required):** all calls MUST go through `vox_actor_runtime::llm` (`infer_with_retry`, `llm_chat`, `llm_stream`, `llm_embed`); model selection lives in `vox-orchestrator::models::{registry, select, autonomic}` (`crates/vox-orchestrator/src/models/`). Direct vendor hostname calls are flagged as an `Error` by `vox-code-audit` detector `llm_provider_call` (`crates/vox-code-audit/src/detectors/llm_provider_call.rs`), which lists the banned hostnames: `openrouter.ai`, `api.anthropic.com`, `api.openai.com`, `cohere.ai`, `api.mistral.ai`, `api.together.xyz`, `api.replicate.com`, `huggingface.co/api`, `api.fireworks.ai`, `api.perplexity.ai`, `generativelanguage.googleapis.com`, `aiplatform.googleapis.com`.

**Web Search / Research:**

- Tavily (search API) - `tavily` crate `2.1.0`, Auth: `TAVILY_API_KEY`, `TAVILY_PROJECT` (`crates/vox-secrets/src/spec/registry/platform.rs`)

**Scholarly / Publishing APIs** (`crates/vox-secrets/src/spec/registry/scholarly.rs`, consumed by `crates/vox-research-shim/`):

- Zenodo - `ZENODO_ACCESS_TOKEN`, `VOX_ZENODO_SANDBOX`, `VOX_ZENODO_API_BASE`
- OpenReview - `VOX_OPENREVIEW_EMAIL`, `VOX_OPENREVIEW_ACCESS_TOKEN`, `VOX_OPENREVIEW_PASSWORD`, `VOX_OPENREVIEW_API_BASE`, `VOX_OPENREVIEW_INVITATION`, `VOX_OPENREVIEW_SIGNATURE`
- Crossref - `VOX_CROSSREF_PLUS_API_KEY`
- ORCID - `VOX_ORCID_CLIENT_ID`, `VOX_ORCID_CLIENT_SECRET`
- DataCite - `VOX_DATACITE_REPOSITORY`, `VOX_DATACITE_PASSWORD`
- OpenAlex - `VOX_OPENALEX_EMAIL`
- Semantic Scholar - `VoxSemanticScholarApiKey`
- OSF, arXiv, SWHID (Software Heritage) - `VoxOsfApiToken`, `VoxArxivAccessToken`/`VoxArxivAssistHandoffSecret`, `VoxSwhidApiToken`
- Per-user nanopublication signing (RSA/hex key) - `VoxNanopubSigningKeyHex`, `VoxUserRsaNanopubPrivateKeyB64`
- Kill switches: `VOX_SCHOLARLY_ADAPTER`, `VOX_SCHOLARLY_DISABLE`, `VOX_SCHOLARLY_DISABLE_LIVE`, `VOX_SCHOLARLY_DISABLE_ZENODO`, `VOX_SCHOLARLY_DISABLE_OPENREVIEW`

**Social / News Publishing** (`crates/vox-secrets/src/spec/registry/social.rs`):

- Bluesky - `bsky-sdk` `0.1` + `atrium-api` `0.25` crates; Auth: `VOX_SOCIAL_BLUESKY_HANDLE`, `VOX_SOCIAL_BLUESKY_PASSWORD`, `VOX_SOCIAL_BLUESKY_PDS_URL`
- Reddit - `VOX_SOCIAL_REDDIT_CLIENT_ID`/`_CLIENT_SECRET`/`_REFRESH_TOKEN`/`_USER_AGENT`/`_API_BASE`
- YouTube - `VOX_SOCIAL_YOUTUBE_CLIENT_ID`/`_CLIENT_SECRET`/`_REFRESH_TOKEN`/`_DEFAULT_CATEGORY_ID`
- Mastodon - `VOX_SOCIAL_MASTODON_TOKEN`, `VOX_SOCIAL_MASTODON_DOMAIN`
- LinkedIn - `VOX_SOCIAL_LINKEDIN_ACCESS_TOKEN`, `VOX_SOCIAL_LINKEDIN_AUTHOR_URN`, `VOX_SOCIAL_LINKEDIN_API_BASE`
- Twitter/X - `VOX_NEWS_TWITTER_TOKEN` (bearer), `VOX_SOCIAL_TWITTER_API_BASE`
- Discord - `VOX_SOCIAL_DISCORD_WEBHOOK` (outgoing webhook)
- Open Collective - `VOX_NEWS_OPENCOLLECTIVE_TOKEN`, `VOX_NEWS_OPENCOLLECTIVE_SLUG`
- Hacker News - `VOX_SOCIAL_HN_MODE`

**Version Control (as a data source, not glue automation):**

- GitHub - `gh` CLI (authenticated as `brbrainerd` per house rules), `VOX_GITHUB_TOKEN` (`.env.example`)
- `gix` `0.84` (git) and `jj-lib` `0.42` (jujutsu) - native VCS integration in `crates/vox-git/`, `crates/vox-vcs/`; repos may be colocated (`.jj/` + `.git/`)

## Data Storage

**Databases:**

- Turso (libSQL, embedded + synced-replica SQLite-compatible) - primary application/telemetry database
  - Connection: `VOX_DB_URL`, `VOX_DB_TOKEN`, `VOX_DB_PATH` (`.env.example`); also `VoxAppDbUrl` for a secondary app DB
  - Client: `turso` crate `0.6` (feature `sync`), wrapped in `crates/vox-db/` (`crates/vox-db/src/lib.rs`, `config.rs`, `sql_util.rs`, `research.rs`, `research_doc_io.rs`, `preferences.rs`, `web_cache.rs`, `mesh_locks.rs`, etc.)
  - Retired names: `TURSO_URL`/`VOX_TURSO_URL`/`VOX_TURSO_TOKEN` → canonical `VOX_DB_URL`/`VOX_DB_TOKEN` (see `AGENTS.md` §Retired Surfaces)
- ClickHouse - telemetry analytics store, isolated to `server/telemetry/` (standalone Cargo workspace, excluded from root); driver `clickhouse` crate `0.13.3`

**File Storage:**

- Local filesystem only for primary artifacts; `vox-bounded-fs` (`crates/vox-bounded-fs/`) provides sandboxed/capability-scoped filesystem access for scripts and plugins
- Hugging Face Hub used as a model/dataset artifact store (`hf-hub` crate, `HuggingFaceToken`)

**Caching:**

- `web_cache.rs` (`crates/vox-db/src/web_cache.rs`) - DB-backed web fetch cache
- Qdrant (vector search) - `VOX_SEARCH_QDRANT_API_KEY` (`VoxSearchQdrantApiKey`), used by `vox-search`'s hybrid retrieval stack alongside `tantivy` `0.22`

## Authentication & Identity

**Auth Provider:**

- Custom - no third-party identity provider (Auth0/Clerk/etc.) detected
  - Implementation: `crates/vox-identity/` - per-user Ed25519 keypair, signing challenges, trust ledger (`src/identity.rs`, `src/challenge.rs`, `src/trust.rs`), per-job ephemeral Ed25519 subkeys for result attestation (`src/ephemeral.rs`), per-pairing X25519 key derivation for JWE encryption (`src/pairing_x25519.rs`)
  - Mesh network auth: `VoxMeshToken`/`VoxMeshWorkerToken`/`VoxMeshSubmitterToken`/`VoxMeshAdminToken`, `VoxMeshJwtHmacSecret`, wire scheme selectable via `VoxMeshAuthScheme` (`ed25519-envelope` default | `jwt-hs256` | `both`) - `crates/vox-mesh-policy/`, `crates/vox-mesh-transport/` (built on `iroh` P2P)
  - Service-to-service: `VOX_API_KEY`, `VOX_BEARER_TOKEN`, `VOX_MCP_HTTP_BEARER_TOKEN`, `VOX_MCP_HTTP_READ_BEARER_TOKEN`
  - All application crypto (signing, hashing, AEAD, KDF) MUST go through `vox-crypto` (`crates/vox-crypto/`) - see `AGENTS.md` §Cryptography Policy

## Monitoring & Observability

**Error Tracking:**

- None detected (no Sentry/Bugsnag SDK found)

**Logs / Telemetry:**

- `tracing` + `tracing-subscriber` (env-filter, json) throughout the workspace
- First-party OTLP telemetry: `vox-telemetry` / `vox-telemetry-otlp` (`crates/vox-telemetry*`) emit events; optional explicit remote upload documented at `docs/src/adr/023-optional-telemetry-remote-upload.md` and controlled via `VoxTelemetryUploadUrl`/`VoxTelemetryUploadToken`
- Ingest target: `server/telemetry/` - standalone Axum `0.7` + ClickHouse `0.13.3` OTLP/HTTP ingest service, deployed via `.github/workflows/deploy-telemetry.yml` / `docker-telemetry.yml`
- Local CLI command: `vox telemetry`; trust/boundary map at `docs/src/architecture/telemetry-trust-ssot.md`

## CI/CD & Deployment

**Hosting:**

- Hetzner - `.github/workflows/deploy-hetzner.yml`
- Coolify - `.github/workflows/coolify-eval-sync.yml`; Auth: `COOLIFY_WEBHOOK_URL`, `COOLIFY_BASE_URL`, `COOLIFY_TOKEN`, `COOLIFY_READ_TOKEN`, `COOLIFY_APP_UUID`
- Docker - `Dockerfile` (production `vox-cli` image, `~50MB`, multi-stage), `Dockerfile.ci-runner` (self-hosted runner image)

**CI Pipeline:**

- GitHub Actions - `.github/workflows/` (40+ workflows: `ci.yml`, `codeql.yml`, `gitleaks.yml`, `cross-platform-check.yml`, `mobile-e2e-android.yml`, `mobile-e2e-ios.yml`, `mobile-eas-build.yml`, `docs-deploy.yml`, `bundle-release.yml`, etc.)
- Test runner: `cargo nextest` for Rust; `pnpm --dir crates/vox-gui/ui test:e2e` (Playwright) for GUI
- Local-first policy: fleet of self-hosted Docker runners is authoritative for most gates; GitHub-hosted `runs-on` requires a registered exception (`docs/src/ci/github-hosted-exceptions.md`) - see `AGENTS.md` §Local-First CI Verification Contract
- Runner-fleet coordination and check-status polling are gated by a hook (`vox ci queue --hook-guard`) - do not poll `gh pr checks`/`gh run watch` directly

## Environment Configuration

**Required env vars (per `.env.example`):**

- `GEMINI_API_KEY`, `OPENROUTER_API_KEY`, `VOX_DB_URL`, `VOX_DB_TOKEN`, `VOX_DB_PATH`, `VOX_GITHUB_TOKEN`, `PORT`

**Full secret surface:**

- 100+ `SecretId` variants enumerated in `crates/vox-secrets/src/spec/ids.rs`, each with canonical env var, aliases, deprecated aliases, and remediation text defined per-entry in `crates/vox-secrets/src/spec/registry/{llm,mesh,social,scholarly,platform,identity,config,core_ids,missing}.rs`
- `.env` file present at repo root - contains local environment configuration (contents not read per security policy)

**Secrets location:**

- Resolved centrally via `vox_secrets::resolve_secret(...)` (`crates/vox-secrets/src/resolver.rs`, sources in `crates/vox-secrets/src/sources/*`) - never read directly from `env::get` in consumer code
- OS keyring integration available (`keyring` crate `3`) for local credential storage
- After any secret-surface change: run `vox ci secret-env-guard` and `vox ci secrets-parity` (per `AGENTS.md` §Secret Management)

## Webhooks & Callbacks

**Incoming:**

- Webhook ingress - `VOX_WEBHOOK_INGRESS_TOKEN`, `VOX_WEBHOOK_SIGNING_SECRET` (`crates/vox-secrets/src/spec/registry/platform.rs`)
- MCP HTTP transport - bearer-authenticated (`VOX_MCP_HTTP_BEARER_TOKEN`, `VOX_MCP_HTTP_READ_BEARER_TOKEN`), registry in `crates/vox-mcp-registry/`
- Coolify deploy webhook - `COOLIFY_WEBHOOK_URL`

**Outgoing:**

- Discord webhook - `VOX_SOCIAL_DISCORD_WEBHOOK` (news/notifications)
- Coolify trigger-deploy - `COOLIFY_TOKEN` + `COOLIFY_BASE_URL`
- Social/scholarly publishing pushes (Bluesky, Reddit, YouTube, Mastodon, LinkedIn, Twitter, Zenodo, OpenReview, etc.) - see APIs & External Services above

---

*Integration audit: 2026-09-22*
