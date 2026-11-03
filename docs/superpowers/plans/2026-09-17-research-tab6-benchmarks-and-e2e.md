# Tab 6: Comparative Live Benchmarks & Playwright E2E Verification Suite

> **Harness & Execution Directives (Gemini Flash 3.8 under Antigravity Desktop Harness):**
> - **Atomic Green Commits:** Every task must compile, pass tests, and end **GREEN** before committing.
> - **Verify-Before-Use:** Before modifying or importing any symbol, execute a pre-flight `rg` command to verify the actual in-repo signature.
> - **Single Command Discipline:** Emit exactly **one terminal command per step**. Never chain with `&&`, `|`, `;`, or wrap in `bash -lc` (causes allowlist parsing failures and orphaned processes on Windows/PowerShell).
> - **Scoped Rustfmt:** Run `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`. **NEVER** run `cargo fmt --all` (causes Windows command-line overflow error 206).
> - **Strict File Disjointness:** Files modified in Tab 6 are localized to integration benchmarks and Playwright E2E specs.
> - **Two-Strike Circuit Breaker:** If a build or test fails twice consecutively on the same step, STOP and report immediately. Never weaken CI flags or alter `layers.toml`.

---

## 1. Handoff Contract

- **Upstream Dependencies:** Requires completion of all preceding tabs (Tab 1 through Tab 5C).
- **Downstream Deliverables:**
  1. `crates/vox-search/tests/live_lane_benchmarks.rs`: Live comparative benchmark suite (marked `#[ignore]`) emitting formatted scoreboards comparing Fast and Deep lanes.
  2. `crates/vox-search/tests/partial_harvest_resilience_test.rs`: Offline wiremock test asserting that slow providers (4,000 ms) never block fast providers (40 ms) within the 1,500 ms Fast Lane deadline.
  3. `crates/vox-gui/ui/e2e/hitl/deep-research-honesty.spec.ts`: End-to-end Playwright test verifying lane switching, quota badge rendering, drawer layering, and honesty sentry compliance.
- **Handoff Consumer:** Final verification gate before merging into `main`.

---

## 2. Context & Technical Specification

### 2.1 Comparative Benchmark Scoreboard
The live benchmark (`live_lane_benchmarks.rs`) measures:
- **Speed**: Wall-clock latency (ms) for Fast Lane ($\le 1,500\,\text{ms}$) vs. Deep Lane.
- **Authority**: Mean trust score across returned citations.
- **Completeness**: Total hit count and distinct authority domain count.
- Sample queries tested:
  1. *"What are the primary tradeoffs of LSM-trees vs B-trees in database storage engines?"* (Technical)
  2. *"Quantum error correction surface codes threshold"* (Academic / Science)
  3. *"History of the Peloponnesian War Sicilian expedition"* (Humanities / History)

### 2.2 Partial Harvest Resilience
A slow or hanging search provider must never cause the entire search batch to fail or stall past the lane deadline:
- When a mock provider responds in 4,000 ms and a keyless provider responds in 40 ms, the Fast Lane MUST harvest the 40 ms provider and return within 1,500 ms with zero panic.

### 2.3 Playwright E2E Suite (`deep-research-honesty.spec.ts`)
Validates the full GUI user flow:
1. Opens `ResearchView` and verifies Fast Lane is active by default.
2. Clicks `🔬 Deep Research` and asserts lane switch toggle updates.
3. Clicks `[ ⚙️ Configure Sources & Free Keys ]` and asserts `ResearchEngineDrawer` opens at `z-50`.
4. Presses `Escape` and asserts `ResearchEngineDrawer` closes while `InspectorDrawer` (if open) remains intact.
5. Injects low-evidence mock and verifies `[data-testid="empty-results-notice"]` renders, satisfying `sentryHonesty.ts`.

---

## 3. Step-by-Step Implementation

### Step 1: Pre-flight Verification
Run `cargo test` across all affected crates to verify a green baseline before writing benchmark tests:
```bash
cargo test -p vox-search -p vox-research-shim -p vox-gui
```
Expected: PASS.

### Step 2: Implement Partial Harvest Resilience Test
Create `crates/vox-search/tests/partial_harvest_resilience_test.rs`:
```rust
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::WebSearchDispatcher;

#[tokio::test]
async fn test_partial_harvest_survives_slow_provider() {
    let fast_server = MockServer::start().await;
    let slow_server = MockServer::start().await;

    // Fast provider: 40ms
    Mock::given(method("GET"))
        .and(path("/works"))
        .respond_with(ResponseTemplate::new(200)
            .set_delay(Duration::from_millis(40))
            .set_body_json(serde_json::json!({
                "results": [{
                    "id": "https://openalex.org/W1",
                    "title": "Fast Keyless Study",
                    "doi": "https://doi.org/10.1000/182",
                    "primary_location": { "landing_page_url": "https://example.com/fast" },
                    "abstract_inverted_index": { "Fast": [0], "paper": [1] }
                }]
            })))
        .mount(&fast_server)
        .await;

    // Slow provider: 4000ms (exceeds fast lane deadline)
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(4000)))
        .mount(&slow_server)
        .await;

    let mut policy = SearchPolicy::default();
    policy.fast_timeout_ms = 1000;
    policy.openalex_api_url = Some(format!("{}/works", fast_server.uri()));
    policy.arxiv_api_url = Some(format!("{}/slow", slow_server.uri()));

    let start = std::time::Instant::now();
    let dispatcher = WebSearchDispatcher::new();
    let hits = dispatcher.search_with_lane("test query", ResearchLane::Fast, &policy).await.expect("search");

    let elapsed = start.elapsed().as_millis();
    assert!(elapsed < 1500, "Fast lane must not wait for slow provider (took {}ms)", elapsed);
    assert!(!hits.is_empty(), "Fast provider results must be harvested");
    assert_eq!(hits[0].title, "Fast Keyless Study");
}
```

### Step 3: Run Partial Harvest Resilience Test
```bash
cargo test -p vox-search --test partial_harvest_resilience_test
```
Expected: PASS.

### Step 4: Implement Live Comparative Benchmark Suite
Create `crates/vox-search/tests/live_lane_benchmarks.rs`:
```rust
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::WebSearchDispatcher;

#[tokio::test]
#[ignore = "Requires live internet connectivity"]
async fn test_live_lane_comparative_scoreboard() {
    let policy = SearchPolicy::default();
    let dispatcher = WebSearchDispatcher::new();
    let queries = [
        "Tradeoffs of LSM-trees vs B-trees in database storage",
        "Quantum error correction surface codes threshold",
    ];

    println!("\n{:=^80}", " LIVE RESEARCH LANE COMPARATIVE SCOREBOARD ");
    println!("{:<35} | {:<8} | {:<8} | {:<10}", "Query", "Lane", "Latency", "Hits");
    println!("{:-^80}", "");

    for q in queries {
        // Fast Lane
        let t0 = std::time::Instant::now();
        let fast_hits = dispatcher.search_with_lane(q, ResearchLane::Fast, &policy).await.unwrap_or_default();
        let fast_ms = t0.elapsed().as_millis();

        // Deep Lane
        let t1 = std::time::Instant::now();
        let deep_hits = dispatcher.search_with_lane(q, ResearchLane::Deep, &policy).await.unwrap_or_default();
        let deep_ms = t1.elapsed().as_millis();

        println!("{:<35} | {:<8} | {:>6}ms | {:>8}", q.chars().take(35).collect::<String>(), "FAST", fast_ms, fast_hits.len());
        println!("{:<35} | {:<8} | {:>6}ms | {:>8}", "", "DEEP", deep_ms, deep_hits.len());
        println!("{:-^80}", "");
    }
}
```

### Step 5: Implement Playwright E2E Honesty Spec
Create `crates/vox-gui/ui/e2e/hitl/deep-research-honesty.spec.ts`:
- Exercises lane switching (`lane-switch-fast`, `lane-switch-deep`).
- Verifies `ResearchEngineDrawer` opens at `z-50` and closes on `Escape`.
- Asserts `[data-testid="empty-results-notice"]` renders on low evidence.

### Step 6: Run Playwright E2E Suite
```bash
pnpm --dir crates/vox-gui/ui exec playwright test e2e/hitl/deep-research-honesty.spec.ts --project=chromium
```
Expected: PASS.

### Step 7: Atomic Commit
```bash
git add crates/vox-search/tests/partial_harvest_resilience_test.rs crates/vox-search/tests/live_lane_benchmarks.rs crates/vox-gui/ui/e2e/hitl/deep-research-honesty.spec.ts
git commit -m "test(bench): add partial harvest resilience, live comparative benchmarks, and Playwright honesty E2E"
```
