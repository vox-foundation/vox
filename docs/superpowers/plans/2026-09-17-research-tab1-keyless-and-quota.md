# Tab 1: Core Keyless Search Providers (OpenAlex & arXiv) & SQLite Quota Persistence

> **Harness & Execution Directives (Gemini Flash 3.8 under Antigravity Desktop Harness):**
> - **Atomic Green Commits:** Every task must compile, pass tests, and end **GREEN** before committing.
> - **Verify-Before-Use:** Before modifying or importing any symbol, execute a pre-flight `rg` command to verify the actual in-repo signature.
> - **Single Command Discipline:** Emit exactly **one terminal command per step**. Never chain with `&&`, `|`, `;`, or wrap in `bash -lc` (causes allowlist parsing failures and orphaned processes on Windows/PowerShell).
> - **Scoped Rustfmt:** Run `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`. **NEVER** run `cargo fmt --all` (causes Windows command-line overflow error 206).
> - **Strict File Disjointness:** Files modified in Tab 1 must be strictly disjoint from all concurrent tabs.
> - **Two-Strike Circuit Breaker:** If a build or test fails twice consecutively on the same step, STOP and report immediately. Never weaken CI flags or alter `layers.toml`.

---

## 1. Handoff Contract

- **Upstream Dependencies:** None (Foundational backend wave; parallel-safe with Tab 2).
- **Downstream Deliverables:**
  1. `crates/vox-search/src/openalex.rs`: Keyless academic paper search with inverted abstract reconstruction and endpoint injection.
  2. `crates/vox-search/src/arxiv.rs`: Keyless preprint search parsing Atom XML entries into `SearxngResult` with endpoint injection.
  3. `crates/vox-db/src/store/quota.rs`: SQLite persistence for monthly quota tracking (`provider_quota_usage`).
  4. `crates/vox-search/src/tavily_budget.rs`: Upstream usage reconciliation (`sync_with_upstream`) calling `GET https://api.tavily.com/usage`.
- **Handoff Consumer:** Tab 3 (Policy & Dispatcher) imports `OpenAlexClient`, `ArXivClient`, and quota tracking.

---

## 2. Context & Technical Specification

### 2.1 OpenAlex Client
- **API Endpoint:** `https://api.openalex.org/works` (configurable via `base_url`).
- **Free Limit:** 100,000 requests/day keyless without authentication.
- **Abstract Reconstruction:** OpenAlex stores abstracts as an inverted index (`{"word": [pos1, pos2]}`). `reconstruct_abstract` inverts the mapping, sorts by token position, joins words with spaces, and truncates to `max_chars` with an ellipsis if exceeded.
- **Landing Page Resolution:** Selects URLs in order of priority: `open_access.oa_url` $\rightarrow$ `primary_location.landing_page_url` $\rightarrow$ `doi` $\rightarrow$ `id`.

### 2.2 arXiv Client
- **API Endpoint:** `https://export.arxiv.org/api/query` (configurable via `base_url`).
- **Data Format:** Atom 1.0 XML.
- **Parsing Invariant:** Must extract `<entry>` nodes only, ignoring the feed-level `<title>`. Normalizes versioned IDs (`v1`, `v2`) to canonical HTTPS landing pages (`https://arxiv.org/abs/{id}`).

### 2.3 Quota Persistence & Reconciliation
- **Table:** `provider_quota_usage (provider, period_key TEXT, units_spent INTEGER, units_limit INTEGER, last_synced_at TEXT)`.
- **Period Key:** UTC calendar month (`YYYY-MM`). Rollover occurs automatically at month boundary.
- **Upstream Reconciliation:** `TavilySessionBudget::sync_with_upstream(api_key)` queries `https://api.tavily.com/usage` with a 15-minute memory cache to prevent rate limit consumption.

---

## 3. Step-by-Step Implementation

### Step 1: Pre-flight Verification
Run `rg` to verify `SearxngResult` structure in `vox-search`:
```bash
rg "pub struct SearxngResult" crates/vox-search/src/searxng.rs
```

### Step 2: Write Failing Unit Tests for Keyless Providers
Create `crates/vox-search/tests/keyless_providers_test.rs`:
```rust
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

### Step 3: Run Failing Provider Tests
```bash
cargo test -p vox-search --test keyless_providers_test
```
Expected: FAIL (unresolved modules `openalex` and `arxiv`).

### Step 4: Implement `openalex.rs` and `arxiv.rs`
Create `crates/vox-search/src/openalex.rs` and `crates/vox-search/src/arxiv.rs`. Support custom `base_url` for deterministic Wiremock injection.

### Step 5: Verify Provider Tests Pass
```bash
cargo test -p vox-search --test keyless_providers_test
```
Expected: PASS.

### Step 6: Write Failing Unit Tests for Quota Tracking
Create `crates/vox-search/tests/quota_tracker_test.rs`:
```rust
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

### Step 7: Run Failing Quota Tests
```bash
cargo test -p vox-search --test quota_tracker_test
```
Expected: FAIL (`try_consume` / `usage_and_remaining` not matching).

### Step 8: Implement SQLite Quota Store & Tavily Upstream Reconciliation
Create `crates/vox-db/src/store/quota.rs` and update `crates/vox-search/src/tavily_budget.rs`.

### Step 9: Verify Quota Tests Pass
```bash
cargo test -p vox-search --test quota_tracker_test
```
Expected: PASS.

### Step 10: Format Code
```bash
cargo fmt -p vox-search -p vox-db
```

### Step 11: Atomic Commit
```bash
git add crates/vox-search/src/openalex.rs crates/vox-search/src/arxiv.rs crates/vox-search/tests/keyless_providers_test.rs crates/vox-db/src/store/quota.rs crates/vox-search/src/tavily_budget.rs crates/vox-search/tests/quota_tracker_test.rs
git commit -m "feat(search): implement keyless OpenAlex and arXiv clients with SQLite quota persistence"
```
