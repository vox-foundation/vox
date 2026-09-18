# Research Multi-Source Integration, Dual Lanes, and Free API Key Governance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement robust multi-source research retrieval in Vox with a guaranteed zero-key baseline (Wikipedia, OpenAlex, arXiv), a dual-lane execution model (Fast sub-second vs. Deep multi-hop CRAG), dynamic quota visualization for free tiers, and in-GUI source controls with direct validated links to acquire free API keys.

**Architecture:** Refactor `vox-search` from a fragile sequential cascade into an intent-aware parallel dispatcher with per-provider timeouts, canonical deduplication, and Reciprocal Rank Fusion (RRF). Wire dual lanes (`Fast` vs. `Deep`) through `vox-research-shim` and `vox-orchestrator`, tracking monthly usage quotas locally and via upstream APIs. Expose active status, lane controls, and free-key onboarding cards directly in the Axis GUI research bar and slide-out drawer.

**Tech Stack:** Rust (Tokio, Reqwest, Serde JSON, Futures), TypeScript (React, Tailwind CSS, Tauri IPC, Playwright).

**Spec:** [`docs/superpowers/specs/2026-09-17-research-multisource-lanes-and-free-keys-design.md`](docs/superpowers/specs/2026-09-17-research-multisource-lanes-and-free-keys-design.md)

## Global Constraints

- Zero-key baseline: Vox research MUST work out of the box with zero API keys on any topic.
- Prune dead weight: Do not call `api.duckduckgo.com` (deprecated Instant Answer JSON).
- Strict fail-open: Network timeouts or errors from an individual provider must never crash or block the search pipeline.
- Latency ceilings: Fast Lane per-engine timeout is 1,500 ms; Deep Lane per-engine timeout is 4,000 ms.
- Secrets SSOT: All API keys must be written and resolved via `vox-secrets` (Clavis vault).
- VoxScript/Rustfmt: Use `vox run scripts/fmt.vox` for dirty `.rs` formatting; never run `cargo fmt --all`.

---

### Task 1: Core Keyless Search Providers: OpenAlex & arXiv Clients [PARALLEL-SAFE]

**Files:**
- Create: `crates/vox-search/src/openalex.rs`
- Create: `crates/vox-search/src/arxiv.rs`
- Modify: `crates/vox-search/src/lib.rs`
- Test: `crates/vox-search/tests/keyless_providers_test.rs`

**Interfaces:**
- Consumes: `vox_http_client::client()`, `crate::searxng::SearxngResult`
- Produces:
  - `crate::openalex::OpenAlexClient::search(query: &str, limit: usize) -> anyhow::Result<Vec<crate::searxng::SearxngResult>>`
  - `crate::arxiv::ArXivClient::search(query: &str, limit: usize) -> anyhow::Result<Vec<crate::searxng::SearxngResult>>`

- [ ] **Step 1: Write the failing tests for OpenAlex and arXiv clients**

```rust
// crates/vox-search/tests/keyless_providers_test.rs
use vox_search::arxiv::ArXivClient;
use vox_search::openalex::OpenAlexClient;

#[test]
fn test_openalex_abstract_reconstruction() {
    let mut inverted = std::collections::HashMap::new();
    inverted.insert("Rust".to_string(), vec![0]);
    inverted.insert("is".to_string(), vec![1]);
    inverted.insert("safe.".to_string(), vec![2]);

    let reconstructed = vox_search::openalex::reconstruct_abstract(&inverted);
    assert_eq!(reconstructed, "Rust is safe.");
}

#[test]
fn test_arxiv_atom_parsing() {
    let sample_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <id>http://arxiv.org/abs/2206.05503v1</id>
    <title>Rust: Safety and Performance</title>
    <summary>A study on Rust memory safety.</summary>
    <link href="https://arxiv.org/abs/2206.05503v1" rel="alternate" type="text/html"/>
  </entry>
</feed>"#;

    let hits = ArXivClient::parse_atom_xml(sample_xml, 5).expect("parse xml");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Rust: Safety and Performance");
    assert_eq!(hits[0].content, "A study on Rust memory safety.");
    assert_eq!(hits[0].url, "https://arxiv.org/abs/2206.05503v1");
    assert_eq!(hits[0].engine.as_deref(), Some("arxiv"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-search --test keyless_providers_test`
Expected: FAIL (modules `openalex` and `arxiv` do not exist).

- [ ] **Step 3: Implement OpenAlex and arXiv clients**

In `crates/vox-search/src/openalex.rs`:
```rust
use std::collections::HashMap;
use serde::Deserialize;
use tracing::debug;
use crate::searxng::SearxngResult;

#[derive(Deserialize)]
struct OpenAlexSearchResponse {
    results: Option<Vec<OpenAlexWorkItem>>,
}

#[derive(Deserialize)]
struct OpenAlexWorkItem {
    id: Option<String>,
    display_name: Option<String>,
    doi: Option<String>,
    abstract_inverted_index: Option<HashMap<String, Vec<usize>>>,
    primary_location: Option<OpenAlexLocation>,
}

#[derive(Deserialize)]
struct OpenAlexLocation {
    landing_page_url: Option<String>,
}

pub fn reconstruct_abstract(inverted: &HashMap<String, Vec<usize>>) -> String {
    let mut indexed: Vec<(usize, &str)> = Vec::new();
    for (word, positions) in inverted {
        for &pos in positions {
            indexed.push((pos, word.as_str()));
        }
    }
    indexed.sort_by_key(|&(pos, _)| pos);
    indexed.into_iter().map(|(_, w)| w).collect::<Vec<_>>().join(" ")
}

pub struct OpenAlexClient;

impl OpenAlexClient {
    pub fn parse_search_json(json_str: &str, limit: usize) -> anyhow::Result<Vec<SearxngResult>> {
        let parsed: OpenAlexSearchResponse = serde_json::from_str(json_str)?;
        let items = parsed.results.unwrap_or_default();
        let results = items
            .into_iter()
            .take(limit)
            .map(|item| {
                let title = item.display_name.unwrap_or_default();
                let content = item
                    .abstract_inverted_index
                    .as_ref()
                    .map(reconstruct_abstract)
                    .unwrap_or_else(|| title.clone());
                let url = item
                    .doi
                    .or_else(|| item.primary_location.and_then(|l| l.landing_page_url))
                    .or(item.id)
                    .unwrap_or_default();
                SearxngResult {
                    url,
                    title,
                    content,
                    engine: Some("openalex".to_string()),
                    score: Some(0.88),
                }
            })
            .filter(|r| !r.url.is_empty())
            .collect();
        Ok(results)
    }

    pub async fn search(query: &str, limit: usize) -> anyhow::Result<Vec<SearxngResult>> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let client = vox_http_client::client();
        let url = format!(
            "https://api.openalex.org/works?search={}&per_page={}",
            urlencoding::encode(query.trim()),
            limit.clamp(1, 50)
        );
        debug!(url = %url, query = query, "Firing OpenAlex works search");
        let resp = client
            .get(&url)
            .header("User-Agent", "VoxResearchBot/1.0 (mailto:research@vox.dev)")
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("OpenAlex API returned status {}", resp.status()));
        }
        let text = resp.text().await?;
        Self::parse_search_json(&text, limit)
    }
}
```

In `crates/vox-search/src/arxiv.rs`:
```rust
use crate::searxng::SearxngResult;
use regex::Regex;
use tracing::debug;

pub struct ArXivClient;

impl ArXivClient {
    pub fn parse_atom_xml(xml: &str, limit: usize) -> anyhow::Result<Vec<SearxngResult>> {
        let entry_re = Regex::new(r"(?s)<entry>(.*?)</entry>")?;
        let title_re = Regex::new(r"(?s)<title>(.*?)</title>")?;
        let summary_re = Regex::new(r"(?s)<summary>(.*?)</summary>")?;
        let id_re = Regex::new(r"(?s)<id>(.*?)</id>")?;

        let mut results = Vec::new();
        for cap in entry_re.captures_iter(xml).take(limit) {
            let entry_body = &cap[1];
            let title = title_re
                .captures(entry_body)
                .map(|c| c[1].trim().replace('\n', " "))
                .unwrap_or_default();
            let summary = summary_re
                .captures(entry_body)
                .map(|c| c[1].trim().replace('\n', " "))
                .unwrap_or_default();
            let id = id_re
                .captures(entry_body)
                .map(|c| c[1].trim().to_string())
                .unwrap_or_default();

            if !id.is_empty() {
                results.push(SearxngResult {
                    url: id,
                    title,
                    content: summary,
                    engine: Some("arxiv".to_string()),
                    score: Some(0.86),
                });
            }
        }
        Ok(results)
    }

    pub async fn search(query: &str, limit: usize) -> anyhow::Result<Vec<SearxngResult>> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let client = vox_http_client::client();
        let url = format!(
            "https://export.arxiv.org/api/query?search_query=all:{}&start=0&max_results={}",
            urlencoding::encode(query.trim()),
            limit.clamp(1, 50)
        );
        debug!(url = %url, query = query, "Firing arXiv API query");
        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("arXiv API returned status {}", resp.status()));
        }
        let text = resp.text().await?;
        Self::parse_atom_xml(&text, limit)
    }
}
```

In `crates/vox-search/src/lib.rs`:
Export `pub mod openalex;` and `pub mod arxiv;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p vox-search --test keyless_providers_test`
Expected: PASS (2 tests pass).

- [ ] **Step 5: Commit**

```bash
git add crates/vox-search/src/openalex.rs crates/vox-search/src/arxiv.rs crates/vox-search/src/lib.rs crates/vox-search/tests/keyless_providers_test.rs
git commit -m "feat(search): add keyless OpenAlex and arXiv clients"
```

---

### Task 2: Dual-Lane Policy, Granular Source Toggles & Quota Tracker [SEQUENTIAL]

**Files:**
- Modify: `crates/vox-search/src/policy.rs`
- Modify: `crates/vox-search/src/tavily_budget.rs`
- Modify: `crates/vox-secrets/src/spec/registry/platform.rs`
- Test: `crates/vox-search/tests/dual_lane_policy_test.rs`

**Interfaces:**
- Produces:
  - `pub enum ResearchLane { Fast, Deep }`
  - `SearchPolicy` lane fields (`default_lane`, `fast_timeout_ms`, `deep_timeout_ms`, `enable_wikipedia`, `enable_openalex`, `enable_arxiv`)
  - `TavilySessionBudget::monthly_usage_and_remaining() -> (usize, usize)`

- [ ] **Step 1: Write failing test for policy and budget tracking**

```rust
// crates/vox-search/tests/dual_lane_policy_test.rs
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::tavily_budget::TavilySessionBudget;

#[test]
fn test_default_policy_lane_and_sources() {
    let policy = SearchPolicy::default();
    assert_eq!(policy.default_lane, ResearchLane::Fast);
    assert_eq!(policy.fast_timeout_ms, 1500);
    assert_eq!(policy.deep_timeout_ms, 4000);
    assert!(policy.enable_wikipedia);
    assert!(policy.enable_openalex);
    assert!(policy.enable_arxiv);
}

#[test]
fn test_monthly_budget_remaining_calculation() {
    let budget = TavilySessionBudget::new(1000);
    assert!(budget.record_charge(150));
    let (used, remaining) = budget.usage_and_remaining();
    assert_eq!(used, 150);
    assert_eq!(remaining, 850);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-search --test dual_lane_policy_test`
Expected: FAIL (missing `ResearchLane`, missing policy fields).

- [ ] **Step 3: Implement lane policy and usage tracking**

In `crates/vox-search/src/policy.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchLane {
    Fast,
    Deep,
}

impl Default for ResearchLane {
    fn default() -> Self {
        Self::Fast
    }
}
```
Add to `SearchPolicy`:
```rust
    pub default_lane: ResearchLane,
    pub fast_timeout_ms: u64,
    pub deep_timeout_ms: u64,
    pub enable_wikipedia: bool,
    pub enable_openalex: bool,
    pub enable_arxiv: bool,
```
Initialize defaults: `default_lane: ResearchLane::Fast`, `fast_timeout_ms: 1500`, `deep_timeout_ms: 4000`, `enable_wikipedia: true`, `enable_openalex: true`, `enable_arxiv: true`.

In `crates/vox-search/src/tavily_budget.rs`:
Add `pub fn usage_and_remaining(&self) -> (usize, usize)`:
```rust
    pub fn usage_and_remaining(&self) -> (usize, usize) {
        let used = self.spent.load(std::sync::atomic::Ordering::Relaxed);
        let remaining = self.limit.saturating_sub(used);
        (used, remaining)
    }
```

In `crates/vox-secrets/src/spec/registry/platform.rs`:
Update `TavilyApiKey` spec remediation text to explicitly include direct URL:
`"Tavily web search API key. Free 1,000 requests/mo at https://app.tavily.com/sign-up"`

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-search --test dual_lane_policy_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-search/src/policy.rs crates/vox-search/src/tavily_budget.rs crates/vox-secrets/src/spec/registry/platform.rs crates/vox-search/tests/dual_lane_policy_test.rs
git commit -m "feat(search): add dual-lane policy fields and budget remaining tracking"
```

---

### Task 3: Dynamic Dispatcher, Parallel Fan-Out, DDG Pruning & RRF Fusion [SEQUENTIAL]

**Files:**
- Modify: `crates/vox-search/src/web_dispatcher.rs`
- Modify: `crates/vox-search/src/duckduckgo.rs`
- Test: `crates/vox-search/tests/web_dispatcher_fanout_test.rs`

**Interfaces:**
- Consumes: `OpenAlexClient`, `ArXivClient`, `WikipediaClient`, `TavilySearchClient`, `SearchPolicy`, `ResearchLane`
- Produces: `WebSearchDispatcher::search_with_lane(query: &str, lane: ResearchLane, policy: &SearchPolicy) -> anyhow::Result<Vec<HybridSearchHit>>`

- [ ] **Step 1: Write failing test for parallel fan-out and zero-key baseline**

```rust
// crates/vox-search/tests/web_dispatcher_fanout_test.rs
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::WebSearchDispatcher;

#[tokio::test]
async fn test_web_dispatcher_zero_key_baseline() {
    let mut policy = SearchPolicy::default();
    policy.tavily_enabled = false;
    policy.searxng_url = None;
    policy.enable_wikipedia = true;
    policy.enable_openalex = true;
    policy.enable_arxiv = true;

    // Fast lane with keyless engines
    let hits = WebSearchDispatcher::search_with_lane("Rust compiler borrow checker", ResearchLane::Fast, &policy)
        .await
        .expect("dispatcher search");

    assert!(!hits.is_empty(), "Keyless search must return hits for valid topic");
    assert!(hits.iter().any(|h| h.provenance.iter().any(|p| p.contains("engine:wikipedia") || p.contains("engine:openalex") || p.contains("engine:arxiv"))));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-search --test web_dispatcher_fanout_test`
Expected: FAIL (`search_with_lane` not implemented).

- [ ] **Step 3: Implement parallel fan-out with per-provider timeouts and RRF merge**

In `crates/vox-search/src/web_dispatcher.rs`:
- Deprecate calling `DuckDuckGoClient::search` (Instant Answer).
- Classify subquery intent:
  - If contains `"paper" | "survey" | "algorithm" | "proof" | "benchmark" | "formal"`, flag `is_academic = true`.
- Compute lane timeout:
  - `lane_timeout = match lane { ResearchLane::Fast => policy.fast_timeout_ms, ResearchLane::Deep => policy.deep_timeout_ms };`
- Spawn parallel tasks with `tokio::time::timeout(Duration::from_millis(lane_timeout), ...)`:
  - Task 1: `WikipediaClient::search` (if `policy.enable_wikipedia`)
  - Task 2: `OpenAlexClient::search` (if `policy.enable_openalex` and (`is_academic` or zero-key fallback))
  - Task 3: `ArXivClient::search` (if `policy.enable_arxiv` and `is_academic`)
  - Task 4: `TavilySearchClient::search` (if `policy.tavily_enabled` and key present)
  - Task 5: `SearxngSearchClient::search` (if `policy.searxng_url.is_some()`)
- Collect results across all tasks via `futures::future::join_all`.
- Flatten results into `Vec<SearxngResult>`.
- Deduplicate by `canonical_url_key`.
- Apply position-based RRF fusion + `source_authority_score`.
- Convert to `Vec<HybridSearchHit>`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-search --test web_dispatcher_fanout_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-search/src/web_dispatcher.rs crates/vox-search/src/duckduckgo.rs crates/vox-search/tests/web_dispatcher_fanout_test.rs
git commit -m "feat(search): implement parallel multi-source dispatcher with RRF fusion and DDG pruning"
```

---

### Task 4: Orchestrator Integration, Chat Research Isolation & Low-Evidence Guard [SEQUENTIAL]

**Files:**
- Modify: `crates/vox-research-shim/src/research/types.rs`
- Modify: `crates/vox-research-shim/src/research/orchestrator/web_gather.rs`
- Modify: `crates/vox-orchestrator/src/orchestrator/core/mod.rs`
- Modify: `crates/vox-secrets/src/spec/ids.rs`
- Modify: `crates/vox-secrets/src/spec/registry/config.rs`
- Test: `crates/vox-research-shim/tests/lane_orchestrator_test.rs`

**Interfaces:**
- Produces:
  - `ResearchQuery.lane: ResearchLane`
  - `SecretId::VoxChatResearchEnabled`
  - Fast Lane single-hop gather vs. Deep Lane multi-hop gather branching

- [ ] **Step 1: Write failing test for orchestrator lane branching**

```rust
// crates/vox-research-shim/tests/lane_orchestrator_test.rs
use vox_research_shim::research::types::{ResearchQuery, ResearchScope};
use vox_search::policy::ResearchLane;

#[test]
fn test_query_lane_defaults_to_fast() {
    let q = ResearchQuery {
        query: "What is quantum annealing?".to_string(),
        scope: ResearchScope::All,
        site_scope: None,
        lane: ResearchLane::Fast,
    };
    assert_eq!(q.lane, ResearchLane::Fast);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-research-shim --test lane_orchestrator_test`
Expected: FAIL (`field lane does not exist on ResearchQuery`).

- [ ] **Step 3: Implement lane handling and chat research setting**

In `crates/vox-research-shim/src/research/types.rs`:
Add `pub lane: vox_search::policy::ResearchLane` to `ResearchQuery`.

In `crates/vox-research-shim/src/research/orchestrator/web_gather.rs`:
Branch on `query.lane`:
- If `query.lane == ResearchLane::Fast`:
  - Run exactly 1 gather hop directly using `WebSearchDispatcher::search_with_lane(..., ResearchLane::Fast, policy)`.
  - Skip iterative query expansion and skip heavy scraping.
- If `query.lane == ResearchLane::Deep`:
  - Run full multi-hop CRAG loop with query decomposition.

In `crates/vox-secrets/src/spec/ids.rs` and `config.rs`:
Register `VoxChatResearchEnabled` (`canonical_env: "VOX_CHAT_RESEARCH_ENABLED"`, default `true`, description `"Enable autonomous research in chat interactions"`).

In `crates/vox-orchestrator/src/orchestrator/core/mod.rs`:
Check `vox_secrets::resolve_secret(SecretId::VoxChatResearchEnabled)` before initiating autonomous research during chat turns. If false, bypass research retrieval and answer directly from LLM context.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-research-shim --test lane_orchestrator_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-research-shim/src/research/types.rs crates/vox-research-shim/src/research/orchestrator/web_gather.rs crates/vox-secrets/src/spec/ids.rs crates/vox-secrets/src/spec/registry/config.rs crates/vox-orchestrator/src/orchestrator/core/mod.rs crates/vox-research-shim/tests/lane_orchestrator_test.rs
git commit -m "feat(orchestrator): wire ResearchLane and add VOX_CHAT_RESEARCH_ENABLED setting"
```

---

### Task 5: Backend Tauri IPC Commands for Engine Status, Quotas & Free Key Links [SEQUENTIAL]

**Files:**
- Modify: `crates/vox-gui/src/commands/search_probe.rs`
- Modify: `crates/vox-gui/src/main.rs`
- Test: `crates/vox-gui/tests/search_probe_test.rs`

**Interfaces:**
- Produces Tauri commands:
  - `get_research_engine_status() -> Result<ResearchEngineStatusDto, String>`
  - `save_research_engine_config(config: ResearchEngineConfigDto) -> Result<(), String>`
  - Updated `probe_search_provider(provider, query)` supporting `openalex` and `arxiv`

- [ ] **Step 1: Write failing test for engine status and new providers in probe**

```rust
// In crates/vox-gui/tests/search_probe_test.rs
#[tokio::test]
async fn test_probe_openalex_and_arxiv_accepted() {
    let res_oa = vox_gui::commands::search_probe::probe_search_provider("openalex".to_string(), "rust".to_string()).await;
    assert!(res_oa.is_ok());

    let res_ax = vox_gui::commands::search_probe::probe_search_provider("arxiv".to_string(), "quantum".to_string()).await;
    assert!(res_ax.is_ok());
}

#[tokio::test]
async fn test_get_research_engine_status() {
    let status = vox_gui::commands::search_probe::get_research_engine_status().await;
    assert!(status.is_ok());
    let status = status.unwrap();
    assert!(status.providers.iter().any(|p| p.id == "openalex" && p.is_keyless));
    assert!(status.free_key_offers.iter().any(|o| o.provider_id == "tavily" && o.signup_url.contains("tavily.com")));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-gui --test search_probe_test`
Expected: FAIL (unknown providers and missing commands).

- [ ] **Step 3: Implement Tauri DTOs and commands**

In `crates/vox-gui/src/commands/search_probe.rs`:
```rust
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatusDto {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub is_keyless: bool,
    pub has_key: bool,
    pub quota_display: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FreeKeyOfferDto {
    pub provider_id: String,
    pub name: String,
    pub free_tier_description: String,
    pub signup_url: String,
    pub secret_env: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResearchEngineStatusDto {
    pub active_lane: String,
    pub fast_timeout_ms: u64,
    pub deep_timeout_ms: u64,
    pub providers: Vec<ProviderStatusDto>,
    pub free_key_offers: Vec<FreeKeyOfferDto>,
}

#[tauri::command]
pub async fn get_research_engine_status() -> Result<ResearchEngineStatusDto, String> {
    // Return active lane, timeouts, provider statuses with remaining quota, and validated free key offers
}

#[tauri::command]
pub async fn save_research_engine_config(active_lane: String, fast_timeout_ms: u64, deep_timeout_ms: u64) -> Result<(), String> {
    // Persist runtime updates
}
```
Update `probe_search_provider` to handle `"openalex"` and `"arxiv"`.
Register new commands in `crates/vox-gui/src/main.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-gui --test search_probe_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/src/commands/search_probe.rs crates/vox-gui/src/main.rs crates/vox-gui/tests/search_probe_test.rs
git commit -m "feat(gui): add get_research_engine_status command and wire openalex/arxiv probes"
```

---

### Task 6: Frontend Research View Bar: Lane Switcher, Quota Badges & Re-Run Guard [PARALLEL-SAFE]

**Files:**
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts`
- Test: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx`

**Interfaces:**
- Consumes: `get_research_engine_status`, `startResearchAsync(..., lane)`
- Produces:
  - Segmented lane switch (`[⚡ Fast]` vs `[🔬 Deep Research]`)
  - Active source badges with quota displays (`✨ Tavily: 840 left`, `✓ Wikipedia`, `✓ OpenAlex`, `✓ arXiv`)
  - `[⚙ Sources & Keys]` button to trigger the drawer
  - Low-evidence re-run badge (`⚠️ Low evidence. Re-run in Deep Research`)

- [ ] **Step 1: Write frontend unit tests for lane switch and badges**

In `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx`:
Add tests verifying:
1. Lane switcher renders with "Fast" active by default.
2. Clicking "Deep Research" switches the lane state.
3. Active source badges render with green keyless pills and Tavily quota pill.
4. If results return with low confidence, the re-run button appears.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @vox/ui test crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx`
Expected: FAIL (missing lane switcher and badges).

- [ ] **Step 3: Implement lane switcher and badge strip in ResearchView.tsx**

- Add `lane: 'fast' | 'deep'` state, defaulting to `'fast'`.
- Render segmented buttons above/beside the query input:
  - `<button onClick={() => setLane('fast')}>⚡ Fast</button>`
  - `<button onClick={() => setLane('deep')}>🔬 Deep Research</button>`
- Fetch `get_research_engine_status` on mount and render active badges:
  - `{engineStatus?.providers.map(p => <Badge key={p.id}>{p.name} {p.quotaDisplay}</Badge>)}`
- Pass `lane` into `startResearchAsync(query, lane)`.
- If `detail?.confidence_tier === 'Light' || detail?.claims?.length === 0`, render the `[🔬 Re-run in Deep Research]` banner.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @vox/ui test crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx
git commit -m "feat(ui): add segmented lane switcher, quota badges, and low-evidence rerun banner to ResearchView"
```

---

### Task 7: Frontend Research Engine Slide-Out Drawer & Free Key Acquisition Hub [PARALLEL-SAFE]

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx`
- Test: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx`

**Interfaces:**
- Consumes: `get_research_engine_status`, `save_research_engine_config`, `set_secret`, `open_url`
- Produces:
  - Slide-out drawer with Zero-Key Guarantee banner, source checkboxes, timeout sliders, and direct free key acquisition cards
  - Chat research toggle (`VOX_CHAT_RESEARCH_ENABLED`) in SettingsView

- [ ] **Step 1: Write unit tests for the ResearchEngineDrawer**

In `crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx`:
Add tests verifying:
1. Drawer renders zero-key guarantee banner.
2. Direct acquisition button for Tavily calls external browser launcher (`open_url`).
3. Entering an API key and clicking Save calls `set_secret`.
4. Chat research toggle in Settings reflects and updates `VOX_CHAT_RESEARCH_ENABLED`.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @vox/ui test crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx`
Expected: FAIL (`ResearchEngineDrawer` does not exist).

- [ ] **Step 3: Implement ResearchEngineDrawer and Settings toggle**

In `crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.tsx`:
- Render slide-out drawer on `isOpen` with a backdrop.
- Section 1: "✓ Zero-Key Guarantee" banner explaining built-in keyless sources.
- Section 2: Lane sliders (Fast: 500–3000ms, Deep: 2000–10000ms) and source checkboxes.
- Section 3: "Enhance Research with Free API Keys" cards for Tavily, Google Gemini, OpenRouter, and Semantic Scholar with direct links calling `open_url(offer.signupUrl)` and inline key entry with `set_secret`.

In `crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx`:
- Under Chat / Agent Settings, add `VOX_CHAT_RESEARCH_ENABLED` toggle switch.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @vox/ui test crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.tsx crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx
git commit -m "feat(ui): add ResearchEngineDrawer with free key acquisition hub and chat research setting"
```

---

### Task 8: End-to-End Benchmarking & Verification Suite [SEQUENTIAL]

**Files:**
- Create: `crates/vox-search/tests/live_lane_benchmarks.rs` (marked `#[ignore]`)
- Modify: `crates/vox-gui/ui/e2e/browser-surface.spec.ts`
- Run: Full CI regression tests and benchmark verification

**Interfaces:**
- Produces: Comparative benchmark report measuring latency, hit count, domain diversity, and completeness between Fast and Deep lanes.

- [ ] **Step 1: Write live comparative benchmark test**

```rust
// crates/vox-search/tests/live_lane_benchmarks.rs
use std::time::Instant;
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::WebSearchDispatcher;

#[tokio::test]
#[ignore]
async fn benchmark_fast_vs_deep_lane_live() {
    let policy = SearchPolicy::default();
    let query = "Formal verification of Rust type systems and borrow checker";

    // Measure Fast Lane
    let start_fast = Instant::now();
    let hits_fast = WebSearchDispatcher::search_with_lane(query, ResearchLane::Fast, &policy)
        .await
        .expect("fast lane search");
    let elapsed_fast = start_fast.elapsed();

    // Measure Deep Lane
    let start_deep = Instant::now();
    let hits_deep = WebSearchDispatcher::search_with_lane(query, ResearchLane::Deep, &policy)
        .await
        .expect("deep lane search");
    let elapsed_deep = start_deep.elapsed();

    println!("\n=== RESEARCH LANE PERFORMANCE SCOREBOARD ===");
    println!("Fast Lane: {} ms | {} hits | Domains: {:?}",
        elapsed_fast.as_millis(),
        hits_fast.len(),
        hits_fast.iter().map(|h| &h.path).collect::<Vec<_>>()
    );
    println!("Deep Lane: {} ms | {} hits | Domains: {:?}",
        elapsed_deep.as_millis(),
        hits_deep.len(),
        hits_deep.iter().map(|h| &h.path).collect::<Vec<_>>()
    );

    assert!(elapsed_fast.as_millis() < 2500, "Fast lane must stay responsive");
    assert!(!hits_fast.is_empty(), "Fast lane must produce hits");
    assert!(!hits_deep.is_empty(), "Deep lane must produce hits");
}
```

- [ ] **Step 2: Run deterministic workspace checks**

Run: `cargo test -p vox-search -p vox-research-shim -p vox-gui`
Expected: PASS.

- [ ] **Step 3: Run live benchmark probe**

Run: `cargo test -p vox-search --test live_lane_benchmarks -- --ignored --nocapture`
Expected: Output performance scoreboard showing Fast lane $< 1,500\,\text{ms}$ and Deep lane multi-domain hits.

- [ ] **Step 4: Commit**

```bash
git add crates/vox-search/tests/live_lane_benchmarks.rs
git commit -m "test(bench): add live comparative lane benchmark test"
```
