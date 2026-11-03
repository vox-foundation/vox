# Tab 3: Dual-Lane Policy, Dynamic Fan-Out, True RRF & Wiremock CI Suite

> **Harness & Execution Directives (Gemini Flash 3.8 under Antigravity Desktop Harness):**
> - **Atomic Green Commits:** Every task must compile, pass tests, and end **GREEN** before committing.
> - **Verify-Before-Use:** Before modifying or importing any symbol, execute a pre-flight `rg` command to verify the actual in-repo signature.
> - **Single Command Discipline:** Emit exactly **one terminal command per step**. Never chain with `&&`, `|`, `;`, or wrap in `bash -lc` (causes allowlist parsing failures and orphaned processes on Windows/PowerShell).
> - **Scoped Rustfmt:** Run `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`. **NEVER** run `cargo fmt --all` (causes Windows command-line overflow error 206).
> - **Strict File Disjointness:** Files modified in Tab 3 are localized to `crates/vox-search/`.
> - **Two-Strike Circuit Breaker:** If a build or test fails twice consecutively on the same step, STOP and report immediately. Never weaken CI flags or alter `layers.toml`.

---

## 1. Handoff Contract

- **Upstream Dependencies:** Requires Tab 1 deliverables (`openalex.rs`, `arxiv.rs`, `quota.rs`).
- **Downstream Deliverables:**
  1. `crates/vox-search/src/policy.rs`: `ResearchLane` (`Fast` vs `Deep`), timeouts (`fast: 1500ms`, `deep: 4000ms`), and endpoint overrides (`wikipedia_api_url`, `openalex_api_url`, `arxiv_api_url`, `tavily_api_url`).
  2. `crates/vox-search/src/web_dispatcher.rs`: `search_with_lane`, parallel fan-out join, DuckDuckGo pruning, and True RRF scoring with source authority weights.
  3. `crates/vox-search/tests/deterministic_lanes_ci_test.rs`: 100% offline Wiremock test suite.
- **Handoff Consumer:** Tab 4 (Orchestrator) invokes `search_with_lane` with `ResearchLane`.

---

## 2. Context & Technical Specification

### 2.1 Dual-Lane Policy
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ResearchLane {
    #[default]
    Fast,
    Deep,
}
```
`SearchPolicy` Configuration:
- `default_lane: ResearchLane::Fast`
- `fast_timeout_ms: 1500`
- `deep_timeout_ms: 4000`
- `enable_wikipedia: true`
- `enable_openalex: true`
- `enable_arxiv: true`
- Wiremock Endpoint Overrides: `wikipedia_api_url`, `openalex_api_url`, `arxiv_api_url`, `tavily_api_url`.

### 2.2 Parallel Fan-Out & True RRF Scoring
The dispatcher joins results from all enabled providers using `join_all` bounded by `tokio::time::timeout(lane_timeout)`:
$$\text{RRF}(d \in \mathcal{D}) = \sum_{m \in M} \frac{1}{k + r_m(d)} \cdot \omega_m$$
Where $k = 60$, $r_m(d)$ is the 1-based rank, and $\omega_m$ is the domain authority multiplier:
- $\omega_{\text{arxiv}} = 1.20$
- $\omega_{\text{openalex}} = 1.10$
- $\omega_{\text{wiki}} = 1.00$
- $\omega_{\text{tavily}} = 1.00$
- $\omega_{\text{searxng}} = 1.00$

### 2.3 Pruning Dead Weight
`api.duckduckgo.com` is removed from active fan-out. Legacy types in `duckduckgo.rs` and circuit breaker enums are retained as stubs so legacy tests compile.

---

## 3. Step-by-Step Implementation

### Step 1: Pre-flight Verification
Run `rg` to verify existing `SearchPolicy` in `vox-search`:
```bash
rg "pub struct SearchPolicy" crates/vox-search/src/policy.rs
```

### Step 2: Write Failing Unit Test for Dual-Lane Policy
Create `crates/vox-search/tests/dual_lane_policy_test.rs`:
```rust
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

### Step 3: Run Failing Policy Test
```bash
cargo test -p vox-search --test dual_lane_policy_test
```
Expected: FAIL (missing `ResearchLane` and lane timeout fields).

### Step 4: Update `policy.rs` and `lib.rs`
1. In `crates/vox-search/src/policy.rs`, add `ResearchLane` and new fields with `#[serde(default)]`.
2. In `crates/vox-search/src/lib.rs`, export `pub mod openalex;` and `pub mod arxiv;`.

### Step 5: Verify Policy Test Passes
```bash
cargo test -p vox-search --test dual_lane_policy_test
```
Expected: PASS.

### Step 6: Write Failing Wiremock Deterministic CI Suite
Create `crates/vox-search/tests/deterministic_lanes_ci_test.rs`:
```rust
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::WebSearchDispatcher;

#[tokio::test]
async fn test_fast_lane_deadline_enforcement() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(2500)))
        .mount(&server)
        .await;

    let mut policy = SearchPolicy::default();
    policy.fast_timeout_ms = 500;
    policy.openalex_api_url = Some(format!("{}/slow", server.uri()));

    let dispatcher = WebSearchDispatcher::new();
    let hits = dispatcher.search_with_lane("test query", ResearchLane::Fast, &policy).await.expect("search");
    // Slow provider timed out cleanly without failing the batch
    assert!(hits.is_empty() || hits.iter().all(|h| h.engine.as_deref() != Some("openalex")));
}
```

### Step 7: Run Failing Dispatcher Tests
```bash
cargo test -p vox-search --test deterministic_lanes_ci_test
```
Expected: FAIL (`search_with_lane` missing).

### Step 8: Implement Dynamic Fan-Out and True RRF in `web_dispatcher.rs`
1. Implement `search_with_lane(query, lane, policy)`.
2. Fan out to Wikipedia, OpenAlex, arXiv, Tavily, SearXNG with `tokio::time::timeout`.
3. Drop DuckDuckGo from dispatch.
4. Merge using `canonical_url_key` and True RRF with authority weights.

### Step 9: Verify Deterministic Tests Pass
```bash
cargo test -p vox-search --test deterministic_lanes_ci_test
```
Expected: PASS.

### Step 10: Format Code
```bash
cargo fmt -p vox-search
```

### Step 11: Atomic Commit
```bash
git add crates/vox-search/src/policy.rs crates/vox-search/src/lib.rs crates/vox-search/tests/dual_lane_policy_test.rs crates/vox-search/src/web_dispatcher.rs crates/vox-search/tests/deterministic_lanes_ci_test.rs
git commit -m "feat(search): implement dual-lane dynamic fan-out with True RRF and Wiremock CI suite"
```
