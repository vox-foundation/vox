# Research Multi-Source Integration, Dual Lanes, and Free API Key Governance: Implementation & Handoff Plan

> **Harness & Execution Directives (Gemini Flash 3.8 under Antigravity Desktop Harness):**
> - **Atomic Green Commits:** Every task must compile, pass tests, and end **GREEN** before committing.
> - **Verify-Before-Use:** Before modifying or importing any symbol, execute a pre-flight `rg` command to verify the actual in-repo signature.
> - **Single Command Discipline:** Emit exactly **one terminal command per step**. Never chain with `&&`, `|`, `;`, or wrap in `bash -lc` (causes allowlist parsing failures and orphaned processes on Windows/PowerShell).
> - **Scoped Rustfmt:** Run `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`. **NEVER** run `cargo fmt --all` (causes Windows command-line overflow error 206).
> - **Strict File Disjointness:** Tasks executed in parallel in the same wave must have strictly non-overlapping file sets ($F_A \cap F_B = \emptyset$). Hub files (`Cargo.toml`, `Cargo.lock`, `layers.toml`, `where-things-live.md`, `mod.rs`, contract indexes) are strictly **`[SEQUENTIAL]`**.
> - **Two-Strike Circuit Breaker:** If a build or test fails twice consecutively on the same step, STOP and report immediately. Never weaken CI flags or alter `layers.toml`.
> - **Integration Handoff Contract:** Coordinates directly with the Axis GUI Visual Debugger and Deep Research Inspection Lab (`feat/axis-gui-debugger-deep-research`). `ResearchEngineDrawer` is layered at `z-50` with circular focus trapping and stops `Escape` propagation to prevent closing `InspectorDrawer` (`z-45`).

**Goal:** Provide zero-key, high-accuracy research retrieval across any subject (Wikipedia, OpenAlex, arXiv), a dual-lane execution model (`Fast` sub-second vs. `Deep` multi-hop CRAG), dynamic quota visualization for free tiers, in-GUI source controls with direct validated links for free API key acquisition, and an isolated chat research killswitch.

**Architecture:** Intent-aware parallel dispatch in `vox-search` using `tokio::time::timeout` and True Reciprocal Rank Fusion (RRF); endpoint-injectable keyless clients for deterministic Wiremock testing in CI; SQLite quota persistence in `vox_db`; dual-lane routing in `vox-research-shim`; chat research killswitch in `vox-orchestrator`; and an unoccluded Axis GUI with honesty-sentry compliance.

**Tech Stack:** Rust 2024 (Tokio, Reqwest, Serde JSON/YAML, Wiremock, Futures), TypeScript 5.5+ (React 19, Tailwind CSS, Dockview, Playwright, Vitest).

**Spec SSOT:** [`docs/superpowers/specs/2026-09-17-research-multisource-lanes-and-free-keys-design.md`](docs/superpowers/specs/2026-09-17-research-multisource-lanes-and-free-keys-design.md)  
**Companion Spec:** [`docs/superpowers/specs/2026-09-17-axis-gui-visual-debugger-deep-research-design.md`](docs/superpowers/specs/2026-09-17-axis-gui-visual-debugger-deep-research-design.md)

---

## Global Constraints & Architectural Invariants

1. **Zero-Key Baseline:** Research retrieval must function out of the box with zero API keys on any domain (code, science, math, history, general facts).
2. **Prune Dead Weight:** Completely deprecate calling `api.duckduckgo.com` (Instant Answer endpoint returning empty topic lists). Retain legacy types in `duckduckgo.rs` and `SearchProviderId::DuckDuckGo` in `search_circuit_breaker.rs` as stubs so legacy tests stay green.
3. **Endpoint Injection for CI:** `SearchPolicy` must provide optional base URLs (`wikipedia_api_url`, `openalex_api_url`, `arxiv_api_url`, `tavily_api_url`) so CI tests run 100% deterministically via Wiremock with zero live network calls.
4. **Epistemic Invariant:** If `all_hits.is_empty()`, the pipeline must halt immediately with `ResearchStage::Failed`. Never synthesize answers from "internal knowledge only" when external sources were requested.
5. **Honesty Sentry Invariant:** Fast Lane low-evidence results ($< 0.35$) must carry `data-testid="empty-results-notice"` to satisfy `sentryHonesty.ts` without triggering a `fake_success` defect.
6. **Chat Research Killswitch:** `VOX_CHAT_RESEARCH_ENABLED` (default: `true`) must allow completely bypassing research retrieval in chat turns for isolated debugging without altering the chat UI.
7. **Secrets SSOT:** All API keys and direct acquisition URLs must be ledgered in `vox-secrets` (Clavis vault). Raw keys are write-only and never leaked into DOM states or client DTOs.

---

## File Manifest & Concurrency Map

| Wave | Task ID | Execution Mode | Target Files | Primary Responsibility |
| :--- | :--- | :--- | :--- | :--- |
| **Wave 1** | **Task 1** | `[PARALLEL-SAFE]` | `crates/vox-search/src/openalex.rs`<br/>`crates/vox-search/src/arxiv.rs`<br/>`crates/vox-search/tests/keyless_providers_test.rs` | OpenAlex & arXiv clients with abstract reconstruction and endpoint injection |
| **Wave 1** | **Task 2** | `[PARALLEL-SAFE]` | `crates/vox-db/src/store/quota.rs`<br/>`crates/vox-search/src/tavily_budget.rs`<br/>`crates/vox-search/tests/quota_tracker_test.rs` | SQLite persistence for monthly quota tracking and upstream usage reconciliation |
| **Wave 1** | **Task 3** | `[PARALLEL-SAFE]` | `crates/vox-secrets/src/spec/ids.rs`<br/>`crates/vox-secrets/src/spec/registry/config.rs`<br/>`crates/vox-secrets/src/spec/registry/platform.rs`<br/>`crates/vox-secrets/src/spec/free_tier.rs` | Register `VoxChatResearchEnabled` and structured `FreeTierOffer` metadata |
| **Wave 2** | **Task 4** | `[SEQUENTIAL]` | `crates/vox-search/src/policy.rs`<br/>`crates/vox-search/src/lib.rs`<br/>`crates/vox-search/tests/dual_lane_policy_test.rs` | Export modules, add `ResearchLane`, lane timeouts, source toggles, and endpoint overrides |
| **Wave 3** | **Task 5** | `[SEQUENTIAL]` | `crates/vox-search/src/web_dispatcher.rs`<br/>`crates/vox-search/src/duckduckgo.rs`<br/>`crates/vox-search/src/safety_governor.rs`<br/>`crates/vox-search/tests/web_dispatcher_fanout_test.rs`<br/>`crates/vox-search/tests/deterministic_lanes_ci_test.rs` | Parallel fan-out, DDG pruning, True RRF rank fusion, and Wiremock CI suite |
| **Wave 4** | **Task 6** | `[SEQUENTIAL]` | `crates/vox-research-shim/src/research/types.rs`<br/>`crates/vox-research-shim/src/research/orchestrator/pipeline.rs`<br/>`crates/vox-research-shim/src/research/orchestrator/web_gather.rs`<br/>`crates/vox-research-shim/src/research/orchestrator/stages.rs`<br/>`crates/vox-orchestrator/src/orchestrator/core/mod.rs`<br/>`crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`<br/>`crates/vox-research-shim/tests/lane_orchestrator_test.rs` | Fast/Deep lane orchestration, low-evidence metadata, chat killswitch, and epistemic zero-hit halting |
| **Wave 5** | **Task 7** | `[SEQUENTIAL]` | `crates/vox-gui/src/commands/search_probe.rs`<br/>`crates/vox-gui/src/commands/research.rs`<br/>`crates/vox-gui/src/main.rs`<br/>`crates/vox-gui/tests/search_probe_test.rs` | Tauri IPC commands (`get_research_engine_status`, `save_research_engine_config`, `probe_search_provider`, `start_research_async` lane forwarding) |
| **Wave 6** | **Task 8** | `[PARALLEL-SAFE]` | `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`<br/>`crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts`<br/>`crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx` | ResearchView segmented lane switch, quota badges, honesty-compliant low-evidence guard |
| **Wave 6** | **Task 9** | `[PARALLEL-SAFE]` | `crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.tsx`<br/>`crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx`<br/>`crates/vox-gui/ui/src/config/settingsIndex.ts`<br/>`crates/vox-gui/ui/src/debugger/usePipelineStepper.ts`<br/>`crates/vox-gui/ui/e2e/lib/tauriMockShared.ts`<br/>`crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx` | Slide-out drawer (`z-50`), focus trap, free key cards, settings toggle, stepper & mock updates |
| **Wave 7** | **Task 10** | `[SEQUENTIAL]` | `crates/vox-search/tests/live_lane_benchmarks.rs`<br/>`crates/vox-search/tests/partial_harvest_resilience_test.rs`<br/>`crates/vox-gui/ui/e2e/hitl/deep-research-honesty.spec.ts` | Live comparative benchmark scoreboard (`--ignored`), partial harvest resilience, Playwright E2E |

---

## Tasks

### Task 1: Core Keyless Search Providers: OpenAlex & arXiv Clients `[PARALLEL-SAFE]`

**Files:**
- Create: `crates/vox-search/src/openalex.rs`
- Create: `crates/vox-search/src/arxiv.rs`
- Test: `crates/vox-search/tests/keyless_providers_test.rs`

**Interfaces:**
- Produces:
  - `OpenAlexClient::search(query: &str, limit: usize, base_url: Option<&str>, api_key: Option<&str>) -> anyhow::Result<Vec<SearxngResult>>`
  - `ArXivClient::search(query: &str, limit: usize, base_url: Option<&str>) -> anyhow::Result<Vec<SearxngResult>>`
  - `reconstruct_abstract(inverted: &HashMap<String, Vec<usize>>, max_chars: usize) -> String`

**Pre-flight Verification:**
Run: `rg "pub struct SearxngResult" crates/vox-search/src/searxng.rs` to verify result structure.

- [ ] **Step 1: Write failing tests for OpenAlex and arXiv clients**

```rust
// crates/vox-search/tests/keyless_providers_test.rs
use std::collections::HashMap;
use vox_search::arxiv::ArXivClient;
use vox_search::openalex::{reconstruct_abstract, OpenAlexClient};

#[test]
fn test_openalex_abstract_reconstruction_bounds() {
    let mut inverted = HashMap::new();
    inverted.insert("Rust".to_string(), vec![0]);
    inverted.insert("memory".to_string(), vec![1]);
    inverted.insert("safety.".to_string(), vec![2]);

    let reconstructed = reconstruct_abstract(&inverted, 500);
    assert_eq!(reconstructed, "Rust memory safety.");

    let truncated = reconstruct_abstract(&inverted, 6);
    assert!(truncated.ends_with("..."));
}

#[test]
fn test_arxiv_atom_parsing_isolated_entries() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title type="html">arXiv Query: search_query=all:Rust</title>
  <entry>
    <id>http://arxiv.org/abs/2206.05503v1</id>
    <title>Rust: Safety and Performance</title>
    <summary>A study on Rust memory safety without garbage collection.</summary>
    <link href="http://arxiv.org/abs/2206.05503v1" rel="alternate" type="text/html"/>
    <link title="pdf" href="http://arxiv.org/pdf/2206.05503v1" rel="related" type="application/pdf"/>
  </entry>
</feed>"#;

    let hits = ArXivClient::parse_atom_xml(xml, 5).expect("parse xml");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Rust: Safety and Performance");
    assert_eq!(hits[0].content, "A study on Rust memory safety without garbage collection.");
    assert_eq!(hits[0].url, "https://arxiv.org/abs/2206.05503");
    assert_eq!(hits[0].engine.as_deref(), Some("arxiv"));
}
```

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p vox-search --test keyless_providers_test`  
Expected: FAIL (unresolved modules).

- [ ] **Step 3: Implement `openalex.rs` and `arxiv.rs`**
In `crates/vox-search/src/openalex.rs`:
Implement `reconstruct_abstract` with sorting and length clamping. Implement `parse_search_json` extracting in priority: `open_access.oa_url` $\rightarrow$ `primary_location.landing_page_url` $\rightarrow$ `doi` $\rightarrow$ `id`. Support `base_url` and `api_key` overrides.
In `crates/vox-search/src/arxiv.rs`:
Implement `parse_atom_xml` extracting only `<entry>` blocks (ignoring feed `<title>`). Normalize unversioned HTTPS URLs (`https://arxiv.org/abs/{id}`). Support `base_url` overrides.

- [ ] **Step 4: Verify test passes**
Run: `cargo test -p vox-search --test keyless_providers_test`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-search/src/openalex.rs crates/vox-search/src/arxiv.rs crates/vox-search/tests/keyless_providers_test.rs
git commit -m "feat(search): add endpoint-injectable OpenAlex and arXiv clients"
```

---

### Task 2: SQLite Quota Persistence & Upstream Usage Reconciliation `[PARALLEL-SAFE]`

**Files:**
- Create: `crates/vox-db/src/store/quota.rs`
- Modify: `crates/vox-search/src/tavily_budget.rs`
- Test: `crates/vox-search/tests/quota_tracker_test.rs`

**Interfaces:**
- Produces:
  - `provider_quota_usage` table in SQLite
  - `TavilySessionBudget::sync_with_upstream(api_key: &str) -> anyhow::Result<(usize, usize)>`
  - `TavilySessionBudget::usage_and_remaining() -> (usize, usize)`

**Pre-flight Verification:**
Run: `rg "TavilySessionBudget" crates/vox-search/src/tavily_budget.rs` to review existing struct.

- [ ] **Step 1: Write failing test for quota tracking and month rollover**

```rust
// crates/vox-search/tests/quota_tracker_test.rs
use vox_search::tavily_budget::TavilySessionBudget;

#[test]
fn test_budget_spend_and_remaining_calculation() {
    let budget = TavilySessionBudget::new(1000);
    assert!(budget.try_consume(200));
    let (used, remaining) = budget.usage_and_remaining();
    assert_eq!(used, 200);
    assert_eq!(remaining, 800);
}

#[test]
fn test_budget_exhaustion_rejects_consumption() {
    let budget = TavilySessionBudget::new(10);
    assert!(budget.try_consume(10));
    assert!(!budget.try_consume(1), "Exhausted budget must reject consumption");
    let (used, remaining) = budget.usage_and_remaining();
    assert_eq!(used, 10);
    assert_eq!(remaining, 0);
}
```

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p vox-search --test quota_tracker_test`  
Expected: FAIL (`try_consume` / `usage_and_remaining` not matching).

- [ ] **Step 3: Implement SQLite quota persistence and reconciliation**
In `crates/vox-db/src/store/quota.rs`:
Create `provider_quota_usage (provider, period_key TEXT, units_spent INTEGER, units_limit INTEGER, last_synced_at TEXT)`.
In `crates/vox-search/src/tavily_budget.rs`:
Store atomic counters initialized from current UTC calendar month (`YYYY-MM`). Implement `sync_with_upstream` calling `GET https://api.tavily.com/usage` with 15-minute caching.

- [ ] **Step 4: Verify test passes**
Run: `cargo test -p vox-search --test quota_tracker_test`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-db/src/store/quota.rs crates/vox-search/src/tavily_budget.rs crates/vox-search/tests/quota_tracker_test.rs
git commit -m "feat(search): add persistent monthly quota tracking and Tavily usage reconciliation"
```

---

### Task 3: Free Tier Metadata Catalog & Chat Research Killswitch Registration `[PARALLEL-SAFE]`

**Files:**
- Create: `crates/vox-secrets/src/spec/free_tier.rs`
- Modify: `crates/vox-secrets/src/spec/ids.rs`
- Modify: `crates/vox-secrets/src/spec/registry/config.rs`
- Modify: `crates/vox-secrets/src/spec/registry/platform.rs`

**Interfaces:**
- Produces:
  - `SecretId::VoxChatResearchEnabled`
  - `pub struct FreeTierOffer { provider_id, name, signup_url, free_tier_description, quota_summary, requires_credit_card, secret_id }`
  - `pub fn list_free_tier_offers() -> Vec<FreeTierOffer>`

**Pre-flight Verification:**
Run: `rg "SecretId::VoxSearchTavilyEnabled" crates/vox-secrets/src/spec/ids.rs` to review enum conventions.

- [ ] **Step 1: Write unit test asserting free tier offer catalog completeness**

In `crates/vox-secrets/src/tests.rs`:
```rust
#[test]
fn test_free_tier_catalog_contains_verified_providers() {
    let offers = vox_secrets::spec::free_tier::list_free_tier_offers();
    assert!(offers.iter().any(|o| o.provider_id == "tavily" && o.signup_url == "https://app.tavily.com/sign-up" && !o.requires_credit_card));
    assert!(offers.iter().any(|o| o.provider_id == "gemini" && o.signup_url == "https://aistudio.google.com/app/apikey"));
    assert!(offers.iter().any(|o| o.provider_id == "openrouter" && o.signup_url == "https://openrouter.ai/keys"));
    assert!(offers.iter().any(|o| o.provider_id == "semantic_scholar" && o.signup_url.contains("semanticscholar.org")));
}
```

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p vox-secrets test_free_tier_catalog_contains_verified_providers`  
Expected: FAIL (module `free_tier` does not exist).

- [ ] **Step 3: Implement `free_tier.rs` and register `VoxChatResearchEnabled`**
In `crates/vox-secrets/src/spec/free_tier.rs`:
Define `FreeTierOffer` and export `list_free_tier_offers()` with validated URLs for Tavily, Google Gemini, OpenRouter, and Semantic Scholar.
In `crates/vox-secrets/src/spec/ids.rs` and `config.rs`:
Register `SecretId::VoxChatResearchEnabled` with canonical env `"VOX_CHAT_RESEARCH_ENABLED"`, default `true`.
In `crates/vox-secrets/src/spec/registry/platform.rs`:
Update `TavilyApiKey` remediation to `"Tavily web search API key. Free 1,000 requests/mo at https://app.tavily.com/sign-up"`.

- [ ] **Step 4: Verify test passes**
Run: `cargo test -p vox-secrets test_free_tier_catalog_contains_verified_providers`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-secrets/src/spec/free_tier.rs crates/vox-secrets/src/spec/ids.rs crates/vox-secrets/src/spec/registry/config.rs crates/vox-secrets/src/spec/registry/platform.rs
git commit -m "feat(secrets): add FreeTierOffer catalog and register VoxChatResearchEnabled"
```

---

### Task 4: Dual-Lane Policy & Module Exports `[SEQUENTIAL]`

**Files:**
- Modify: `crates/vox-search/src/policy.rs`
- Modify: `crates/vox-search/src/lib.rs`
- Test: `crates/vox-search/tests/dual_lane_policy_test.rs`

**Interfaces:**
- Produces:
  - `pub enum ResearchLane { Fast, Deep }`
  - `SearchPolicy` lane fields (`default_lane`, `fast_timeout_ms: 1500`, `deep_timeout_ms: 4000`)
  - Source toggles: `enable_wikipedia: true`, `enable_openalex: true`, `enable_arxiv: true`
  - Endpoint overrides: `wikipedia_api_url`, `openalex_api_url`, `arxiv_api_url`, `tavily_api_url`
  - Export `openalex` and `arxiv` modules in `lib.rs`

**Pre-flight Verification:**
Run: `rg "pub struct SearchPolicy" crates/vox-search/src/policy.rs` to review fields.

- [ ] **Step 1: Write failing test for policy lane defaults and endpoint overrides**

```rust
// crates/vox-search/tests/dual_lane_policy_test.rs
use vox_search::policy::{ResearchLane, SearchPolicy};

#[test]
fn test_default_policy_lane_and_sources() {
    let policy = SearchPolicy::default();
    assert_eq!(policy.default_lane, ResearchLane::Fast);
    assert_eq!(policy.fast_timeout_ms, 1500);
    assert_eq!(policy.deep_timeout_ms, 4000);
    assert!(policy.enable_wikipedia);
    assert!(policy.enable_openalex);
    assert!(policy.enable_arxiv);
    assert!(policy.wikipedia_api_url.is_none());
    assert!(policy.openalex_api_url.is_none());
    assert!(policy.arxiv_api_url.is_none());
}
```

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p vox-search --test dual_lane_policy_test`  
Expected: FAIL (missing fields).

- [ ] **Step 3: Update `policy.rs` and `lib.rs`**
Add `ResearchLane` and new fields with `#[serde(default)]`. Export `pub mod openalex;` and `pub mod arxiv;` in `lib.rs`.

- [ ] **Step 4: Verify test passes**
Run: `cargo test -p vox-search --test dual_lane_policy_test`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-search/src/policy.rs crates/vox-search/src/lib.rs crates/vox-search/tests/dual_lane_policy_test.rs
git commit -m "feat(search): add ResearchLane, lane timeouts, and test endpoint overrides to SearchPolicy"
```

---

### Task 5: Dynamic Dispatcher, Parallel Fan-Out, DDG Pruning & Wiremock CI Suite `[SEQUENTIAL]`

**Files:**
- Modify: `crates/vox-search/src/web_dispatcher.rs`
- Modify: `crates/vox-search/src/duckduckgo.rs`
- Modify: `crates/vox-search/src/safety_governor.rs`
- Test: `crates/vox-search/tests/web_dispatcher_fanout_test.rs`
- Test: `crates/vox-search/tests/deterministic_lanes_ci_test.rs`

**Interfaces:**
- Produces:
  - `WebSearchDispatcher::search_with_lane(query: &str, lane: ResearchLane, policy: &SearchPolicy) -> anyhow::Result<Vec<HybridSearchHit>>`
  - True RRF fusion function `rrf_fuse_results(lists, limit, k)`
  - Canonical normalizer `canonical_url_key` supporting unversioned arXiv IDs and DOIs

**Pre-flight Verification:**
Run: `rg "pub async fn search" crates/vox-search/src/web_dispatcher.rs` to review current method signature.

- [ ] **Step 1: Write failing Wiremock deterministic CI tests**
Copy the complete Wiremock test suite from Track 5 audit into `crates/vox-search/tests/deterministic_lanes_ci_test.rs` testing RRF multi-source fusion, Fast Lane 1,500 ms deadline compliance, Deep Lane 4,000 ms allowance, and Quota exhaustion fail-open.

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p vox-search --test deterministic_lanes_ci_test`  
Expected: FAIL (`search_with_lane` not implemented).

- [ ] **Step 3: Implement parallel fan-out and True RRF**
In `crates/vox-search/src/web_dispatcher.rs`:
1. Classify query intent into `is_academic`, `is_technical`, `is_general`.
2. Compute `lane_timeout = match lane { ResearchLane::Fast => policy.fast_timeout_ms, ResearchLane::Deep => policy.deep_timeout_ms };`.
3. Spawn parallel tasks with `tokio::time::timeout(Duration::from_millis(lane_timeout), ...)` for:
   - Wikipedia (`policy.enable_wikipedia`)
   - OpenAlex (`policy.enable_openalex` if `is_academic || is_technical || zero_key_mode`)
   - arXiv (`policy.enable_arxiv` if `is_academic`)
   - Tavily (if keyed and `policy.tavily_enabled`)
   - SearXNG (if configured)
4. Prune DDG from parallel fan-out.
5. Join tasks via `futures::future::join_all`.
6. Apply enhanced `canonical_url_key` and True RRF scoring with `source_authority_score`.

- [ ] **Step 4: Verify test passes**
Run: `cargo test -p vox-search --test deterministic_lanes_ci_test`  
Expected: PASS (all 4 deterministic Wiremock tests pass).

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-search/src/web_dispatcher.rs crates/vox-search/src/duckduckgo.rs crates/vox-search/src/safety_governor.rs crates/vox-search/tests/deterministic_lanes_ci_test.rs
git commit -m "feat(search): implement parallel multi-source dispatcher with True RRF and Wiremock CI suite"
```

---

### Task 6: Orchestrator Dual Lanes, Chat Killswitch & Epistemic Halting `[SEQUENTIAL]`

**Files:**
- Modify: `crates/vox-research-shim/src/research/types.rs`
- Modify: `crates/vox-research-shim/src/research/orchestrator/pipeline.rs`
- Modify: `crates/vox-research-shim/src/research/orchestrator/web_gather.rs`
- Modify: `crates/vox-research-shim/src/research/orchestrator/stages.rs`
- Modify: `crates/vox-orchestrator/src/orchestrator/core/mod.rs`
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`
- Test: `crates/vox-research-shim/tests/lane_orchestrator_test.rs`

**Interfaces:**
- Produces:
  - `ResearchQuery.lane: ResearchLane`
  - `ResearchMetadata.low_grounding_evidence: bool`
  - `ResearchMetadata.suggested_lane: Option<ResearchLane>`
  - `vox_orchestrator::is_chat_research_enabled() -> bool`
  - Zero-evidence hard halt in `pipeline.rs`

**Pre-flight Verification:**
Run: `rg "pub struct ResearchQuery" crates/vox-research-shim/src/research/types.rs` to review fields.

- [ ] **Step 1: Write failing test for orchestrator lane routing and low-evidence metadata**

```rust
// crates/vox-research-shim/tests/lane_orchestrator_test.rs
use vox_research_shim::research::types::{ResearchQuery, ResearchScope};
use vox_search::policy::ResearchLane;

#[test]
fn test_query_lane_defaults_and_metadata_serialization() {
    let q = ResearchQuery {
        query: "What is quantum annealing?".to_string(),
        scope: ResearchScope::All,
        max_sources: 5,
        persist_to_docs: false,
        verify_claims: true,
        site_scope: None,
        domain_mode: Default::default(),
        waves: 1,
        lane: ResearchLane::Fast,
    };
    assert_eq!(q.lane, ResearchLane::Fast);
}
```

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p vox-research-shim --test lane_orchestrator_test`  
Expected: FAIL (`lane` field missing on `ResearchQuery`).

- [ ] **Step 3: Implement orchestrator lane routing, killswitch, and epistemic halting**
1. Add `lane: ResearchLane` to `ResearchQuery`.
2. In `pipeline.rs`, bypass LLM decomposition if `lane == Fast`.
3. In `web_gather.rs`, execute 1 hop directly with `search_with_lane` if `lane == Fast`.
4. Enforce universal zero-evidence halt: if `all_hits.is_empty()`, fail immediately with `ResearchStage::Failed` across all scopes.
5. In `stages.rs`, remove dummy `"Answering from internal knowledge only"` fallback.
6. In `pipeline.rs`, set `metadata.low_grounding_evidence = true` when confidence $< 0.35$.
7. In `vox-orchestrator`, implement `is_chat_research_enabled()` checking `VOX_CHAT_RESEARCH_ENABLED`, and short-circuit research dispatch in chat turns.

- [ ] **Step 4: Verify test passes**
Run: `cargo test -p vox-research-shim --test lane_orchestrator_test`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-research-shim/src/research/types.rs crates/vox-research-shim/src/research/orchestrator/pipeline.rs crates/vox-research-shim/src/research/orchestrator/web_gather.rs crates/vox-research-shim/src/research/orchestrator/stages.rs crates/vox-orchestrator/src/orchestrator/core/mod.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs crates/vox-research-shim/tests/lane_orchestrator_test.rs
git commit -m "feat(orchestrator): add Fast/Deep lane routing, epistemic zero-hit halt, and chat research killswitch"
```

---

### Task 7: Tauri IPC Commands for Engine Status, Config & Probe Updates `[SEQUENTIAL]`

**Files:**
- Modify: `crates/vox-gui/src/commands/search_probe.rs`
- Modify: `crates/vox-gui/src/commands/research.rs`
- Modify: `crates/vox-gui/src/main.rs`
- Test: `crates/vox-gui/tests/search_probe_test.rs`

**Interfaces:**
- Produces:
  - `get_research_engine_status() -> Result<ResearchEngineStatusDto, String>`
  - `save_research_engine_config(config: ResearchEngineConfigDto) -> Result<(), String>`
  - Updated `probe_search_provider` and `probe_all_search_providers` (supporting `openalex` and `arxiv`, removing `duckduckgo`)
  - Updated `start_research_async(query, lane, scope, ...)` passing `lane`

**Pre-flight Verification:**
Run: `rg "probe_search_provider" crates/vox-gui/src/commands/search_probe.rs` to review command.

- [ ] **Step 1: Write failing test for new IPC commands and probe handlers**

```rust
// In crates/vox-gui/tests/search_probe_test.rs
#[tokio::test]
async fn test_probe_openalex_and_arxiv_accepted() {
    let res_oa = vox_gui::commands::search_probe::probe_search_provider("openalex".into(), "rust".into()).await;
    assert!(res_oa.is_ok());
    let res_ax = vox_gui::commands::search_probe::probe_search_provider("arxiv".into(), "rust".into()).await;
    assert!(res_ax.is_ok());
}

#[tokio::test]
async fn test_get_research_engine_status_payload() {
    let status = vox_gui::commands::search_probe::get_research_engine_status().await.unwrap();
    assert!(status.providers.iter().any(|p| p.id == "openalex" && p.is_keyless));
    assert!(status.free_key_offers.iter().any(|o| o.provider_id == "tavily" && o.signup_url.contains("tavily.com")));
}
```

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p vox-gui --test search_probe_test`  
Expected: FAIL (missing commands and probe match arms).

- [ ] **Step 3: Implement IPC commands and wire `lane` in `research.rs`**
In `crates/vox-gui/src/commands/search_probe.rs`:
Implement `ProviderStatusDto`, `FreeKeyOfferDto`, `ResearchEngineStatusDto`, `ResearchEngineConfigDto`. Implement `get_research_engine_status` and `save_research_engine_config`.
In `crates/vox-gui/src/commands/research.rs`:
Add `lane: Option<String>` to `start_research_async` and forward into `dei_method::RESEARCH_RUN`.
Register commands in `crates/vox-gui/src/main.rs`.

- [ ] **Step 4: Verify test passes**
Run: `cargo test -p vox-gui --test search_probe_test`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-gui/src/commands/search_probe.rs crates/vox-gui/src/commands/research.rs crates/vox-gui/src/main.rs crates/vox-gui/tests/search_probe_test.rs
git commit -m "feat(gui): implement get_research_engine_status, save_research_engine_config, and lane forwarding"
```

---

### Task 8: Frontend Research View Bar: Lane Switcher, Quota Badges & Honesty Guard `[PARALLEL-SAFE]`

**Files:**
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts`
- Test: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx`

**Interfaces:**
- Consumes: `getResearchEngineStatus`, `startResearchAsync(..., lane)`
- Produces:
  - Segmented lane switch (`⚡ Fast` vs `🔬 Deep Research`)
  - Active source badge strip with dynamic quota
  - Honesty-sentry-compliant low-evidence guard (`data-testid="empty-results-notice"`)

**Pre-flight Verification:**
Run: `rg "startResearchAsync" crates/vox-gui/ui/src/components/surfaces/Research/` to check call site.

- [ ] **Step 1: Write Vitest unit tests for lane switch and honesty sentry compliance**

In `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx`:
Add tests asserting:
1. Fast lane is active by default.
2. Clicking Deep Research changes active state and forwards `lane: 'deep'`.
3. Low-evidence guard renders with `data-testid="empty-results-notice"` when confidence is low.

- [ ] **Step 2: Run test to verify failure**
Run: `pnpm --filter @vox/ui test crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx`  
Expected: FAIL.

- [ ] **Step 3: Implement lane switch, quota badges, and low-evidence banner**
Update `researchActions.ts` to accept `lane?: 'fast' | 'deep'`. Update `ResearchView.tsx` with segmented lane switch in the subheader row, standard-flow badge strip, and the `[data-testid="empty-results-notice"]` re-run banner.

- [ ] **Step 4: Verify test passes**
Run: `pnpm --filter @vox/ui test crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx
git commit -m "feat(ui): add segmented lane switcher, quota badges, and honesty sentry low-evidence guard"
```

---

### Task 9: Frontend Slide-Out Drawer & Free Key Acquisition Hub `[PARALLEL-SAFE]`

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx`
- Modify: `crates/vox-gui/ui/src/config/settingsIndex.ts`
- Modify: `crates/vox-gui/ui/src/debugger/usePipelineStepper.ts`
- Modify: `crates/vox-gui/ui/e2e/lib/tauriMockShared.ts`
- Test: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx`

**Interfaces:**
- Produces:
  - `ResearchEngineDrawer` mounted at `z-50` with circular focus trap and event stop-propagation on Escape
  - Free API key cards with direct validated links (`open_url`)
  - `VOX_CHAT_RESEARCH_ENABLED` toggle under `section === 'orchestrator'` in `SettingsView.tsx`

**Pre-flight Verification:**
Run: `rg "InspectorDrawer" crates/vox-gui/ui/src/debugger/` to verify overlay styling and z-index.

- [ ] **Step 1: Write unit tests for `ResearchEngineDrawer`**
In `crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx`:
Test that the drawer renders the Zero-Key Guarantee banner, external signup links call `open_url`, key entry calls `set_secret`, and hitting Escape closes the drawer without bubbling.

- [ ] **Step 2: Run test to verify failure**
Run: `pnpm --filter @vox/ui test crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx`  
Expected: FAIL (component does not exist).

- [ ] **Step 3: Implement `ResearchEngineDrawer.tsx`, settings toggle, and mock updates**
1. Implement `ResearchEngineDrawer.tsx` at `z-50` with full-screen backdrop (`fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex justify-end`).
2. Add circular focus trap and `e.stopPropagation()` on Escape.
3. In `SettingsView.tsx`, add `Autonomous chat research` toggle under `section === 'orchestrator'`. Register in `settingsIndex.ts`.
4. In `usePipelineStepper.ts` and `tauriMockShared.ts`, update mock providers to `['wikipedia', 'openalex', 'arxiv', 'tavily', 'searxng']` and add mock handlers for `get_research_engine_status` and `save_research_engine_config`.

- [ ] **Step 4: Verify test passes**
Run: `pnpm --filter @vox/ui test crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.tsx crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx crates/vox-gui/ui/src/config/settingsIndex.ts crates/vox-gui/ui/src/debugger/usePipelineStepper.ts crates/vox-gui/ui/e2e/lib/tauriMockShared.ts crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx
git commit -m "feat(ui): add ResearchEngineDrawer at z-50, chat research setting, and updated test mocks"
```

---

### Task 10: Comparative Live Benchmarks & End-to-End Verification `[SEQUENTIAL]`

**Files:**
- Create: `crates/vox-search/tests/live_lane_benchmarks.rs` (marked `#[ignore]`)
- Create: `crates/vox-search/tests/partial_harvest_resilience_test.rs`
- Modify: `crates/vox-gui/ui/e2e/hitl/deep-research-honesty.spec.ts`

**Interfaces:**
- Produces: Comparative scoreboard measuring Speed (ms), Accuracy (trust score), and Completeness (hits, domain diversity) across Fast and Deep lanes.

**Pre-flight Verification:**
Run: `cargo test -p vox-search --test deterministic_lanes_ci_test` to confirm CI suite is green before live benchmarking.

- [ ] **Step 1: Implement partial harvest resilience and live benchmark suite**
In `crates/vox-search/tests/partial_harvest_resilience_test.rs`:
Implement wiremock test verifying slow provider (4s delay) does not block harvesting fast provider (40ms) within the 1,500 ms deadline.
In `crates/vox-search/tests/live_lane_benchmarks.rs`:
Implement live comparative benchmark across 3 canonical queries emitting the formatted scoreboard.

- [ ] **Step 2: Run deterministic workspace checks**
Run: `cargo test -p vox-search -p vox-research-shim -p vox-gui`  
Expected: PASS.

- [ ] **Step 3: Run live benchmark probe**
Run: `cargo test -p vox-search --test live_lane_benchmarks -- --ignored --nocapture`  
Expected: Emits comparative scoreboard showing Fast Lane $\le 1,500\,\text{ms}$ and Deep Lane domain diversity $\ge 2$.

- [ ] **Step 4: Run Playwright E2E test**
Run: `pnpm --filter @vox/ui test:e2e crates/vox-gui/ui/e2e/hitl/deep-research-honesty.spec.ts`  
Expected: PASS (zero invariant sentry violations).

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-search/tests/live_lane_benchmarks.rs crates/vox-search/tests/partial_harvest_resilience_test.rs crates/vox-gui/ui/e2e/hitl/deep-research-honesty.spec.ts
git commit -m "test(bench): add partial harvest resilience and live comparative lane benchmark scoreboard"
```
