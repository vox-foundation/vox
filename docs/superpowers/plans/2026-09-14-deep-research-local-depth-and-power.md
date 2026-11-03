# Deep Research Local Depth, Structure, and Power Implementation Plan (Hardened)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dramatically expand Vox Deep Research locally with depth-2/3 recursive crawling, SPA headless extraction, BLAKE3 CAS caching, zero-token local MENS subagent extraction, polyglot sandboxing (TS, Python, SQL, Vox, Rust), cycle-safe epistemic knowledge graph in VoxDB, continuous regression auditing (`vox audit research-parity`), and interactive GUI DAG/REPL surfaces.

**Architecture:** A 4-phase decoupled architecture:
1. **Deep Web Reconnaissance & Caching**: Autonomous depth-2 link crawler with priority scoring, SPA headless browser fallback, BLAKE3 content-addressed web cache with HTTP 304 conditional revalidation, and multi-provider search pool with 429 rate-limit backoff.
2. **Local Compute & MENS Cascades**: Zero-token MENS Qwen 3 8B claim triplet extraction with verbatim substring grounding, local term-density reranker, and resilient parallel exploration with cancellation tokens.
3. **Polyglot Empirical Sandboxing**: Multi-language verification sandboxes (TypeScript with `ambient.d.ts` wildcard, Python with `ast.parse` + `mypy`, SQL with SQLite & `sqlparser-rs` dialect parsing, Vox, Rust), micro-benchmarking with `std::hint::black_box` and warmup loops, and upstream Git shallow clone probe runner.
4. **Epistemic Knowledge Graph & Autonomic Mining**: Relational graph schema with cycle-safe path-tracking recursive CTE queries in `vox-db`, loose semver normalization with range matching, `vox audit research-parity` continuous auditing emitting versioned JSONL, and interactive GUI DAG canvas and Sandbox REPL in `vox-gui`.

**Tech Stack:** Rust (Tokio, reqwest, scraper, html2text, turso/SQLite, BLAKE3, semver, sqlparser), TypeScript/React (Tauri IPC, SVG DAG, Vitest), Bun, Python, MENS Qwen 3 8B on Apple Silicon Metal / CUDA.

**Spec:** `docs/src/architecture/deep-research-local-depth-and-power-research-2026.md`

## Global Constraints

- Emit one terminal command per step by default. Do not chain commands with `&&` or wrap in bash.
- Rust formatting: never run `cargo fmt --all`. Only format modified packages: `cargo fmt -p <crate>`.
- LLM facade: all model calls must route through `vox_actor_runtime::llm` facade (`chat_with_cascade_parsed`).
- Doctest code fences: all markdown `vox` code fences must include `// vox:skip <reason>`.
- Authored markdown frontmatter: all docs created under `docs/src/` must contain valid YAML frontmatter conforming to documentation governance.
- TDD requirement: every task must follow Red -> Green -> Refactor with zero skipped assertions or mocked false passes.
- Anti-Stub / Ponytail Policy: no fake stubs or mocked passes (`setOutput('Probe passed...')` is permanently banned).

---

## Phase 1: Deep Web Reconnaissance & Caching

### Task 1.1: Autonomous Depth-2/3 Recursive Link Traversal [PARALLEL-SAFE]

**Files:**
- Modify: `crates/vox-search/Cargo.toml`
- Create: `crates/vox-search/src/crawler.rs`
- Modify: `crates/vox-search/src/scraper.rs`
- Modify: `crates/vox-search/src/lib.rs`
- Test: `crates/vox-search/tests/crawler_test.rs`

**Interfaces:**
- Consumes: `crate::scraper::ScrapedDocument`, `crate::scraper::fetch_and_extract`
- Produces:
  ```rust
  pub fn normalize_crawl_url(url: &url::Url) -> url::Url;
  pub fn extract_candidate_links(html: &str, base_url: &str, allow_origin: &str) -> Vec<String>;
  pub fn score_and_prioritize_links(links: &[String]) -> Vec<(String, u32)>;
  pub async fn crawl_domain_depth(
      root_url: &str,
      max_depth: usize,
      max_pages: usize,
      timeout_ms: u64,
  ) -> anyhow::Result<Vec<crate::scraper::ScrapedDocument>>;
  ```

- [ ] **Step 1: Write the failing test**

Write `crates/vox-search/tests/crawler_test.rs`:
```rust
use url::Url;
use vox_search::crawler::{extract_candidate_links, normalize_crawl_url, score_and_prioritize_links};

#[test]
fn test_normalize_crawl_url_strips_fragments_and_tracking_queries() {
    let raw = Url::parse("https://docs.rs/tokio/1.38.0/tokio/sync/struct.Mutex.html?utm_source=feed&ref=tracker#method.lock").unwrap();
    let normalized = normalize_crawl_url(&raw);
    assert_eq!(normalized.as_str(), "https://docs.rs/tokio/1.38.0/tokio/sync/struct.Mutex.html");
}

#[test]
fn test_extract_candidate_links_preserves_api_items() {
    let html = r#"
        <html>
            <body>
                <a href="/tokio/sync/struct.Mutex.html">Mutex</a>
                <a href="/tokio/time/fn.sleep.html">sleep</a>
                <a href="https://external.com/ad">External</a>
                <a href="/tokio/asset.png">Image</a>
            </body>
        </html>
    "#;
    let base_url = "https://docs.rs/tokio/1.38.0";
    let allow_origin = "https://docs.rs";

    let links = extract_candidate_links(html, base_url, allow_origin);
    assert_eq!(links.len(), 2);
    assert!(links.contains(&"https://docs.rs/tokio/sync/struct.Mutex.html".to_string()));
    assert!(links.contains(&"https://docs.rs/tokio/time/fn.sleep.html".to_string()));
}

#[test]
fn test_score_and_prioritize_links_orders_api_higher_than_generic() {
    let links = vec![
        "https://docs.rs/tokio/about".to_string(),
        "https://docs.rs/tokio/sync/struct.Mutex.html".to_string(),
        "https://docs.rs/tokio/guide".to_string(),
    ];
    let scored = score_and_prioritize_links(&links);
    assert_eq!(scored[0].0, "https://docs.rs/tokio/sync/struct.Mutex.html");
    assert!(scored[0].1 > scored[1].1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-search --test crawler_test`
Expected: FAIL with compilation error `cannot find module crawler in vox_search`

- [ ] **Step 3: Implement crawler module**

Ensure `crates/vox-search/Cargo.toml` contains `url = { workspace = true }`.

Write `crates/vox-search/src/crawler.rs`:
```rust
use std::collections::{HashSet, VecDeque};
use url::Url;
use scraper::{Html, Selector};
use crate::scraper::{ScrapedDocument, fetch_and_extract};

const EXCLUDED_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp",
    ".pdf", ".zip", ".tar", ".gz", ".exe", ".dmg",
    ".css", ".js", ".map", ".ico", ".woff", ".woff2",
];

pub fn normalize_crawl_url(url: &Url) -> Url {
    let mut clean = url.clone();
    clean.set_fragment(None);
    clean.set_query(None);
    clean
}

pub fn extract_candidate_links(html: &str, base_url: &str, allow_origin: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    let Ok(base) = Url::parse(base_url) else {
        return Vec::new();
    };

    let mut links = Vec::new();
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            let Ok(resolved) = base.join(href) else {
                continue;
            };
            let normalized = normalize_crawl_url(&resolved);
            let normalized_str = normalized.to_string();

            if !normalized_str.starts_with(allow_origin) {
                continue;
            }
            let lower = normalized_str.to_lowercase();
            if EXCLUDED_EXTENSIONS.iter().any(|ext| lower.ends_with(ext)) {
                continue;
            }
            if !links.contains(&normalized_str) {
                links.push(normalized_str);
            }
        }
    }
    links
}

pub fn score_and_prioritize_links(links: &[String]) -> Vec<(String, u32)> {
    let mut scored: Vec<(String, u32)> = links
        .iter()
        .map(|link| {
            let lower = link.to_lowercase();
            let mut score = 10u32;
            if lower.contains("struct.") || lower.contains("fn.") || lower.contains("trait.") || lower.contains("class.") || lower.contains("api") {
                score += 50;
            }
            if lower.contains("spec") || lower.contains("architecture") || lower.contains("guide") || lower.contains("internals") {
                score += 30;
            }
            (link.clone(), score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored
}

pub async fn crawl_domain_depth(
    root_url: &str,
    max_depth: usize,
    max_pages: usize,
    timeout_ms: u64,
) -> anyhow::Result<Vec<ScrapedDocument>> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut results = Vec::new();

    let root_parsed = Url::parse(root_url)?;
    let allow_origin = root_parsed.origin().ascii_serialization();

    queue.push_back((root_url.to_string(), 0usize));
    visited.insert(root_url.to_string());

    while let Some((current_url, depth)) = queue.pop_front() {
        if results.len() >= max_pages {
            break;
        }

        match fetch_and_extract(&current_url, timeout_ms).await {
            Ok(doc) => {
                results.push(doc);

                if depth < max_depth {
                    // Extract links from candidate
                    let client = vox_http_client::client_builder()
                        .timeout(std::time::Duration::from_millis(timeout_ms))
                        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
                        .build()?;
                    if let Ok(resp) = client.get(&current_url).send().await {
                        if let Ok(html) = resp.text().await {
                            let candidates = extract_candidate_links(&html, &current_url, &allow_origin);
                            let prioritized = score_and_prioritize_links(&candidates);

                            for (link, _) in prioritized {
                                if !visited.contains(&link) && visited.len() < max_pages * 3 {
                                    visited.insert(link.clone());
                                    queue.push_back((link, depth + 1));
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(url = %current_url, error = %e, "Failed to crawl link");
            }
        }
    }

    Ok(results)
}
```

Modify `crates/vox-search/src/lib.rs` to expose `pub mod crawler;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-search --test crawler_test`
Expected: PASS with 3 passed tests.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-search`
Run: `git add crates/vox-search/Cargo.toml crates/vox-search/src/crawler.rs crates/vox-search/src/lib.rs crates/vox-search/tests/crawler_test.rs`
Run: `git commit -m "feat(search): implement priority-scored depth-2/3 recursive link crawler"`

---

### Task 1.2: Headless Browser Extraction Fallback & Bot-Wall Detection [SEQUENTIAL]

**Files:**
- Create: `crates/vox-search/src/spa_fallback.rs`
- Modify: `crates/vox-search/src/scraper.rs`
- Modify: `crates/vox-search/src/lib.rs`
- Test: `crates/vox-search/tests/spa_fallback_test.rs`

**Interfaces:**
- Consumes: Raw HTML, extracted character counts, text density
- Produces:
  ```rust
  pub fn is_bot_challenge_page(title: &str, raw_html: &str) -> bool;
  pub fn should_fallback_to_headless(raw_html: &str, extracted_chars: usize, text_density: f64) -> bool;
  pub fn sanitize_extracted_accessibility_text(ax_text: &str) -> String;
  ```

- [ ] **Step 1: Write the failing test**

Write `crates/vox-search/tests/spa_fallback_test.rs`:
```rust
use vox_search::spa_fallback::{is_bot_challenge_page, sanitize_extracted_accessibility_text, should_fallback_to_headless};

#[test]
fn test_is_bot_challenge_page_detection() {
    assert!(is_bot_challenge_page("Just a moment...", "<html><head><title>Just a moment...</title></head><body>Cloudflare Ray ID: 89f4</body></html>"));
    assert!(is_bot_challenge_page("Security Check", "<html><body>Please enable JavaScript and cookies to continue.</body></html>"));
    assert!(!is_bot_challenge_page("Tokio Documentation", "<html><article>Async runtime for Rust</article></html>"));
}

#[test]
fn test_should_fallback_to_headless_on_spa_shells() {
    let empty_root = "<html><body><div id=\"root\"></div><script src=\"bundle.js\"></script></body></html>";
    assert!(should_fallback_to_headless(empty_root, 15, 0.02));

    let next_app = "<html><body><div id=\"__next\"></div><script>var __NEXT_DATA__={};</script></body></html>";
    assert!(should_fallback_to_headless(next_app, 30, 0.03));

    let dense_content = "<html><body><article><p>Comprehensive technical documentation about Rust type systems.</p></article></body></html>";
    assert!(!should_fallback_to_headless(dense_content, 500, 0.35));
}

#[test]
fn test_sanitize_extracted_accessibility_text() {
    let raw_ax = "banner\n  heading \"Vox Documentation\" [level=1]\nmain\n  heading \"Overview\" [level=2]\n  text \"Vox is an agent-first programming language.\"";
    let cleaned = sanitize_extracted_accessibility_text(raw_ax);
    assert!(cleaned.contains("# Vox Documentation"));
    assert!(cleaned.contains("## Overview"));
    assert!(cleaned.contains("Vox is an agent-first programming language."));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-search --test spa_fallback_test`
Expected: FAIL with compilation error `cannot find module spa_fallback in vox_search`

- [ ] **Step 3: Implement SPA fallback and bot wall detector**

Write `crates/vox-search/src/spa_fallback.rs`:
```rust
const BOT_CHALLENGE_SUBSTRINGS: &[&str] = &[
    "just a moment...",
    "attention required!",
    "security check",
    "access denied",
    "ddos-guard",
    "cloudflare",
    "please enable javascript and cookies",
];

const SPA_SHELL_PATTERNS: &[&str] = &[
    "id=\"root\"",
    "id=\"__next\"",
    "id=\"app\"",
    "id=\"__nuxt\"",
    "id=\"docusaurus\"",
    "<noscript>you need to enable javascript",
    "<noscript>please enable javascript",
];

pub fn is_bot_challenge_page(title: &str, raw_html: &str) -> bool {
    let lower_title = title.to_lowercase();
    let lower_html = raw_html.to_lowercase();

    for sub in BOT_CHALLENGE_SUBSTRINGS {
        if lower_title.contains(sub) {
            return true;
        }
    }
    if lower_html.contains("cf-browser-verification") || lower_html.contains("cf_chl_opt") {
        return true;
    }
    false
}

pub fn should_fallback_to_headless(raw_html: &str, extracted_chars: usize, text_density: f64) -> bool {
    if extracted_chars < 120 || text_density < 0.06 {
        return true;
    }
    let lower = raw_html.to_lowercase();
    for pattern in SPA_SHELL_PATTERNS {
        if lower.contains(&pattern.to_lowercase()) && extracted_chars < 400 {
            return true;
        }
    }
    false
}

pub fn sanitize_extracted_accessibility_text(ax_text: &str) -> String {
    let mut out = Vec::new();
    for line in ax_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "banner" || trimmed == "main" || trimmed == "navigation" {
            continue;
        }
        if let Some(pos) = trimmed.find("heading \"") {
            let start = pos + 9;
            if let Some(end) = trimmed[start..].rfind('"') {
                let heading_text = &trimmed[start..start + end];
                let level = if trimmed.contains("level=1") {
                    "# "
                } else if trimmed.contains("level=2") {
                    "## "
                } else if trimmed.contains("level=3") {
                    "### "
                } else {
                    "#### "
                };
                out.push(format!("{level}{heading_text}"));
                continue;
            }
        }
        if let Some(pos) = trimmed.find("text \"") {
            let start = pos + 6;
            if let Some(end) = trimmed[start..].rfind('"') {
                let text = &trimmed[start..start + end];
                out.push(text.to_string());
                continue;
            }
        }
        if !trimmed.starts_with("generic") && !trimmed.starts_with("group") {
            out.push(trimmed.to_string());
        }
    }
    out.join("\n\n")
}
```

Modify `crates/vox-search/src/lib.rs` to add `pub mod spa_fallback;`.
Modify `crates/vox-search/src/scraper.rs` to validate against `is_bot_challenge_page`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-search --test spa_fallback_test`
Expected: PASS with 3 passed tests.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-search`
Run: `git add crates/vox-search/src/spa_fallback.rs crates/vox-search/src/scraper.rs crates/vox-search/src/lib.rs crates/vox-search/tests/spa_fallback_test.rs`
Run: `git commit -m "feat(search): add bot challenge detection and SPA accessibility tree sanitizer"`

---

### Task 1.3: Content-Addressed Local Web Cache with HTTP 304 Support [PARALLEL-SAFE]

**Files:**
- Create: `crates/vox-db/src/web_cache.rs`
- Modify: `crates/vox-db/src/schema/domains/knowledge.rs`
- Modify: `crates/vox-db/src/lib.rs`
- Test: `crates/vox-db/tests/web_cache_test.rs`

**Interfaces:**
- Consumes: `VoxDb`, BLAKE3
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct CachedWebArtifact {
      pub url_hash: String,
      pub url: String,
      pub etag: Option<String>,
      pub last_modified: Option<String>,
      pub status_code: u16,
      pub content_type: String,
      pub raw_body: Vec<u8>,
      pub extracted_markdown: String,
      pub fetched_at_ms: i64,
  }

  impl VoxDb {
      pub async fn get_cached_web_artifact(&self, url: &str) -> Result<Option<CachedWebArtifact>, StoreError>;
      pub async fn put_cached_web_artifact(&self, artifact: &CachedWebArtifact) -> Result<(), StoreError>;
      pub async fn touch_cached_web_artifact(&self, url: &str, touched_at_ms: i64) -> Result<(), StoreError>;
  }
  ```

- [ ] **Step 1: Write the failing test**

Write `crates/vox-db/tests/web_cache_test.rs`:
```rust
use vox_db::VoxDb;
use vox_db::web_cache::CachedWebArtifact;

#[tokio::test]
async fn test_web_cache_put_get_and_touch() {
    let db = VoxDb::in_memory().await.expect("in-memory db init");

    let artifact = CachedWebArtifact {
        url_hash: String::new(),
        url: "https://docs.rs/tokio/latest/tokio/time/fn.sleep.html".to_string(),
        etag: Some("\"33a64df5\"".to_string()),
        last_modified: Some("Wed, 21 Oct 2025 07:28:00 GMT".to_string()),
        status_code: 200,
        content_type: "text/html; charset=utf-8".to_string(),
        raw_body: b"<html><body>sleep doc</body></html>".to_vec(),
        extracted_markdown: "# sleep".to_string(),
        fetched_at_ms: 1000,
    };

    db.put_cached_web_artifact(&artifact).await.expect("put cache");

    let retrieved = db.get_cached_web_artifact(&artifact.url).await.expect("get cache");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().fetched_at_ms, 1000);

    // Touch on HTTP 304 Not Modified
    db.touch_cached_web_artifact(&artifact.url, 2000).await.expect("touch cache");
    let touched = db.get_cached_web_artifact(&artifact.url).await.expect("get cache");
    assert_eq!(touched.unwrap().fetched_at_ms, 2000);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-db --test web_cache_test`
Expected: FAIL with compilation error `no method named get_cached_web_artifact found for struct VoxDb`

- [ ] **Step 3: Implement web cache schema and methods**

Add table definition to `crates/vox-db/src/schema/domains/knowledge.rs`:
```sql
CREATE TABLE IF NOT EXISTS web_cache (
    url_hash TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    etag TEXT,
    last_modified TEXT,
    status_code INTEGER NOT NULL,
    content_type TEXT NOT NULL,
    raw_body BLOB NOT NULL,
    extracted_markdown TEXT NOT NULL,
    fetched_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_web_cache_url ON web_cache(url);
CREATE INDEX IF NOT EXISTS idx_web_cache_fetched ON web_cache(fetched_at_ms);
```

Write `crates/vox-db/src/web_cache.rs`:
```rust
use crate::VoxDb;
use crate::store::StoreError;
use turso::params;

#[derive(Debug, Clone, PartialEq)]
pub struct CachedWebArtifact {
    pub url_hash: String,
    pub url: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub status_code: u16,
    pub content_type: String,
    pub raw_body: Vec<u8>,
    pub extracted_markdown: String,
    pub fetched_at_ms: i64,
}

pub fn hash_url(url: &str) -> String {
    let normalized = url.trim().trim_end_matches('/');
    blake3::hash(normalized.as_bytes()).to_hex().to_string()
}

impl VoxDb {
    pub async fn get_cached_web_artifact(&self, url: &str) -> Result<Option<CachedWebArtifact>, StoreError> {
        let hash = hash_url(url);
        let conn = self.conn.clone();
        let breaker = self.breaker.clone();

        breaker.call(|| async move {
            let mut rows = conn
                .query(
                    "SELECT url_hash, url, etag, last_modified, status_code, content_type, raw_body, extracted_markdown, fetched_at_ms
                     FROM web_cache WHERE url_hash = ?1",
                    params![hash.as_str()],
                )
                .await?;

            if let Some(row) = rows.next().await? {
                let status_code: i64 = row.get(4)?;
                let raw_body: Vec<u8> = row.get(6)?;
                Ok(Some(CachedWebArtifact {
                    url_hash: row.get(0)?,
                    url: row.get(1)?,
                    etag: row.get(2)?,
                    last_modified: row.get(3)?,
                    status_code: status_code as u16,
                    content_type: row.get(5)?,
                    raw_body,
                    extracted_markdown: row.get(7)?,
                    fetched_at_ms: row.get(8)?,
                }))
            } else {
                Ok(None)
            }
        })
        .await
    }

    pub async fn put_cached_web_artifact(&self, artifact: &CachedWebArtifact) -> Result<(), StoreError> {
        let hash = if artifact.url_hash.is_empty() {
            hash_url(&artifact.url)
        } else {
            artifact.url_hash.clone()
        };
        let conn = self.conn.clone();
        let breaker = self.breaker.clone();
        let art = artifact.clone();

        breaker.call(|| async move {
            conn.execute(
                "INSERT OR REPLACE INTO web_cache
                 (url_hash, url, etag, last_modified, status_code, content_type, raw_body, extracted_markdown, fetched_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    hash.as_str(),
                    art.url.as_str(),
                    art.etag.as_deref(),
                    art.last_modified.as_deref(),
                    art.status_code as i64,
                    art.content_type.as_str(),
                    art.raw_body.as_slice(),
                    art.extracted_markdown.as_str(),
                    art.fetched_at_ms,
                ],
            )
            .await?;
            Ok(())
        })
        .await
    }

    pub async fn touch_cached_web_artifact(&self, url: &str, touched_at_ms: i64) -> Result<(), StoreError> {
        let hash = hash_url(url);
        let conn = self.conn.clone();
        let breaker = self.breaker.clone();

        breaker.call(|| async move {
            conn.execute(
                "UPDATE web_cache SET fetched_at_ms = ?1 WHERE url_hash = ?2",
                params![touched_at_ms, hash.as_str()],
            )
            .await?;
            Ok(())
        })
        .await
    }
}
```

Modify `crates/vox-db/src/lib.rs` to add `pub mod web_cache;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-db --test web_cache_test`
Expected: PASS with 1 passed test.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-db`
Run: `git add crates/vox-db/src/web_cache.rs crates/vox-db/src/schema/domains/knowledge.rs crates/vox-db/src/lib.rs crates/vox-db/tests/web_cache_test.rs`
Run: `git commit -m "feat(db): add web cache with BLAKE3 hashing and HTTP 304 touch support"`

---

### Task 1.4: Rate-Limit Circuit Breaker for Multi-Provider Search [PARALLEL-SAFE]

**Files:**
- Create: `crates/vox-search/src/search_circuit_breaker.rs`
- Modify: `crates/vox-search/src/web_dispatcher.rs`
- Modify: `crates/vox-search/src/lib.rs`
- Test: `crates/vox-search/tests/search_circuit_breaker_test.rs`

**Interfaces:**
- Consumes: Provider IDs, HTTP status codes
- Produces:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum SearchProviderId {
      Searxng,
      Tavily,
      DuckDuckGo,
  }

  pub struct ProviderCircuitBreaker;
  impl ProviderCircuitBreaker {
      pub fn record_failure(&mut self, is_rate_limit: bool);
      pub fn is_available(&self) -> bool;
  }
  ```

- [ ] **Step 1: Write the failing test**

Write `crates/vox-search/tests/search_circuit_breaker_test.rs`:
```rust
use vox_search::search_circuit_breaker::{ProviderCircuitBreaker, SearchProviderId};

#[test]
fn test_circuit_breaker_cooldown_on_rate_limit() {
    let mut breaker = ProviderCircuitBreaker::default();
    assert!(breaker.is_available());

    // Record HTTP 429 rate limit failure
    breaker.record_failure(true);
    assert!(!breaker.is_available());

    // Reset on success
    breaker.record_success();
    assert!(breaker.is_available());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-search --test search_circuit_breaker_test`
Expected: FAIL with compilation error `cannot find module search_circuit_breaker in vox_search`

- [ ] **Step 3: Implement circuit breaker module**

Write `crates/vox-search/src/search_circuit_breaker.rs`:
```rust
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchProviderId {
    Searxng,
    Tavily,
    DuckDuckGo,
}

#[derive(Debug, Clone)]
pub struct ProviderCircuitBreaker {
    pub consecutive_failures: u32,
    pub cooldown_until: Option<Instant>,
}

impl Default for ProviderCircuitBreaker {
    fn default() -> Self {
        Self {
            consecutive_failures: 0,
            cooldown_until: None,
        }
    }
}

impl ProviderCircuitBreaker {
    pub fn record_failure(&mut self, is_rate_limit: bool) {
        self.consecutive_failures += 1;
        let base_delay_secs = if is_rate_limit { 60 } else { 5 };
        let delay = Duration::from_secs(base_delay_secs * 2u64.pow(self.consecutive_failures.min(4)));
        self.cooldown_until = Some(Instant::now() + delay);
    }

    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.cooldown_until = None;
    }

    pub fn is_available(&self) -> bool {
        match self.cooldown_until {
            Some(expiry) => Instant::now() >= expiry,
            None => true,
        }
    }
}
```

Modify `crates/vox-search/src/lib.rs` to expose `pub mod search_circuit_breaker;`.
Wire circuit breaker checks into `crates/vox-search/src/web_dispatcher.rs` to bypass providers in active cooldown.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-search --test search_circuit_breaker_test`
Expected: PASS with 1 passed test.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-search`
Run: `git add crates/vox-search/src/search_circuit_breaker.rs crates/vox-search/src/web_dispatcher.rs crates/vox-search/src/lib.rs crates/vox-search/tests/search_circuit_breaker_test.rs`
Run: `git commit -m "feat(search): implement rate-limit circuit breaker for web search dispatcher"`

---

## Phase 2: Local Compute & MENS Cascades

### Task 2.1: MENS Qwen 3 8B Zero-Token Claim Extraction with Substring Grounding [PARALLEL-SAFE]

**Files:**
- Create: `crates/vox-search/src/mens_research_subagent.rs`
- Modify: `crates/vox-search/src/lib.rs`
- Test: `crates/vox-search/tests/mens_research_subagent_test.rs`

**Interfaces:**
- Consumes: `vox_actor_runtime::llm`
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
  pub struct ClaimTriplet {
      pub subject: String,
      pub predicate: String,
      pub object: String,
      pub confidence: f64,
      pub evidence_snippet: String,
  }

  pub fn parse_and_ground_claim_triplets(raw_json: &str, source_text: &str) -> Vec<ClaimTriplet>;
  pub fn build_local_claim_extraction_prompt(evidence: &str) -> String;
  ```

- [ ] **Step 1: Write the failing test**

Write `crates/vox-search/tests/mens_research_subagent_test.rs`:
```rust
use vox_search::mens_research_subagent::{parse_and_ground_claim_triplets, build_local_claim_extraction_prompt};

#[test]
fn test_parse_and_ground_claim_triplets_discards_hallucinations() {
    let source = "In Tokio 1.0, delay_for was renamed to sleep for naming consistency.";
    let json_text = r#"
    {
        "claims": [
            {
                "subject": "tokio::time::sleep",
                "predicate": "replaces",
                "object": "tokio::time::delay_for",
                "confidence": 0.95,
                "evidence_snippet": "delay_for was renamed to sleep"
            },
            {
                "subject": "tokio",
                "predicate": "supports",
                "object": "hallucinated_feature",
                "confidence": 0.80,
                "evidence_snippet": "this snippet does not exist in source text"
            }
        ]
    }
    "#;

    let claims = parse_and_ground_claim_triplets(json_text, source);
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].subject, "tokio::time::sleep");
    assert_eq!(claims[0].evidence_snippet, "delay_for was renamed to sleep");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-search --test mens_research_subagent_test`
Expected: FAIL with compilation error `cannot find module mens_research_subagent in vox_search`

- [ ] **Step 3: Implement grounded claim extractor module**

Write `crates/vox-search/src/mens_research_subagent.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimTriplet {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub evidence_snippet: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TripletEnvelope {
    Object { claims: Vec<RawTriplet> },
    Array(Vec<RawTriplet>),
}

#[derive(Deserialize)]
struct RawTriplet {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: Option<f64>,
    pub evidence_snippet: Option<String>,
}

pub fn parse_and_ground_claim_triplets(raw_json: &str, source_text: &str) -> Vec<ClaimTriplet> {
    let text = raw_json.trim();
    let start = match text.find(|c| c == '{' || c == '[') {
        Some(s) => s,
        None => return Vec::new(),
    };
    let end = match text.rfind(|c| c == '}' || c == ']') {
        Some(e) => e,
        None => return Vec::new(),
    };
    if start > end {
        return Vec::new();
    }
    let slice = &text[start..=end];

    let raw_list: Vec<RawTriplet> = match serde_json::from_str::<TripletEnvelope>(slice) {
        Ok(TripletEnvelope::Object { claims }) => claims,
        Ok(TripletEnvelope::Array(arr)) => arr,
        Err(_) => return Vec::new(),
    };

    let lower_source = source_text.to_lowercase();
    raw_list
        .into_iter()
        .filter_map(|raw| {
            let subject = raw.subject.trim().to_string();
            let predicate = raw.predicate.trim().to_lowercase();
            let object = raw.object.trim().to_string();
            let snippet = raw.evidence_snippet.unwrap_or_default().trim().to_string();

            if subject.is_empty() || object.is_empty() {
                return None;
            }

            // Verbatim grounding verification
            if !snippet.is_empty() && !lower_source.contains(&snippet.to_lowercase()) {
                return None; // Discard ungrounded hallucination
            }

            let confidence = raw.confidence.unwrap_or(0.90).clamp(0.0, 1.0);
            Some(ClaimTriplet {
                subject,
                predicate,
                object,
                confidence,
                evidence_snippet: snippet,
            })
        })
        .collect()
}

pub fn build_local_claim_extraction_prompt(evidence: &str) -> String {
    format!(
        "You are an epistemic claim extraction agent. Extract atomic epistemic triplets from the following text.\n\
         Every evidence_snippet MUST be a verbatim quote from the text.\n\
         Output ONLY valid JSON with format:\n\
         {{\"claims\": [{{\"subject\": \"...\", \"predicate\": \"...\", \"object\": \"...\", \"confidence\": 0.95, \"evidence_snippet\": \"...\"}}]}}\n\n\
         Text:\n{evidence}"
    )
}
```

Modify `crates/vox-search/src/lib.rs` to expose `pub mod mens_research_subagent;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-search --test mens_research_subagent_test`
Expected: PASS with 1 passed test.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-search`
Run: `git add crates/vox-search/src/mens_research_subagent.rs crates/vox-search/src/lib.rs crates/vox-search/tests/mens_research_subagent_test.rs`
Run: `git commit -m "feat(search): implement grounded local claim extraction parser discarding hallucinations"`

---

### Task 2.2: Term-Density Passage Reranker [PARALLEL-SAFE]

**Files:**
- Create: `crates/vox-search/src/term_density_reranker.rs`
- Modify: `crates/vox-search/src/lib.rs`
- Test: `crates/vox-search/tests/term_density_reranker_test.rs`

**Interfaces:**
- Consumes: Query string, candidate passages
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct CandidatePassage {
      pub id: String,
      pub text: String,
      pub base_score: f64,
  }

  pub fn score_passage_term_density(query_terms: &[&str], passage_text: &str) -> f64;
  pub fn rerank_passages(query: &str, passages: &[CandidatePassage], top_k: usize) -> Vec<CandidatePassage>;
  ```

- [ ] **Step 1: Write the failing test**

Write `crates/vox-search/tests/term_density_reranker_test.rs`:
```rust
use vox_search::term_density_reranker::{CandidatePassage, rerank_passages, score_passage_term_density};

#[test]
fn test_score_passage_term_density_scoring() {
    let query_terms = &["metal", "candle", "unified", "memory"];
    let relevant_passage = "Using candle-metal with Unified Memory allows zero-copy tensor sharing between CPU and GPU.";
    let irrelevant_passage = "A recipe for cooking pasta in boiling water with salt.";

    let score_high = score_passage_term_density(query_terms, relevant_passage);
    let score_low = score_passage_term_density(query_terms, irrelevant_passage);

    assert!(score_high > score_low);
    assert!(score_high > 0.5);
    assert_eq!(score_low, 0.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-search --test term_density_reranker_test`
Expected: FAIL with compilation error `cannot find module term_density_reranker in vox_search`

- [ ] **Step 3: Implement term density reranker module**

Write `crates/vox-search/src/term_density_reranker.rs`:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CandidatePassage {
    pub id: String,
    pub text: String,
    pub base_score: f64,
}

pub fn score_passage_term_density(query_terms: &[&str], passage_text: &str) -> f64 {
    let lower_passage = passage_text.to_lowercase();
    let mut matches = 0usize;
    let mut total_occurrences = 0usize;

    for term in query_terms {
        let lower_term = term.to_lowercase();
        if lower_term.len() < 2 {
            continue;
        }
        let count = lower_passage.matches(&lower_term).count();
        if count > 0 {
            matches += 1;
            total_occurrences += count;
        }
    }

    if query_terms.is_empty() || matches == 0 {
        return 0.0;
    }

    let coverage_ratio = matches as f64 / query_terms.len() as f64;
    let frequency_boost = (total_occurrences as f64).min(5.0) / 5.0 * 0.2;
    (coverage_ratio * 0.8 + frequency_boost).min(1.0)
}

pub fn rerank_passages(query: &str, passages: &[CandidatePassage], top_k: usize) -> Vec<CandidatePassage> {
    let query_terms: Vec<&str> = query.split_whitespace().collect();
    let mut scored: Vec<CandidatePassage> = passages
        .iter()
        .map(|p| {
            let term_score = score_passage_term_density(&query_terms, &p.text);
            let combined = p.base_score * 0.3 + term_score * 0.7;
            CandidatePassage {
                id: p.id.clone(),
                text: p.text.clone(),
                base_score: combined,
            }
        })
        .collect();

    scored.sort_by(|a, b| b.base_score.partial_cmp(&a.base_score).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(top_k).collect()
}
```

Modify `crates/vox-search/src/lib.rs` to expose `pub mod term_density_reranker;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-search --test term_density_reranker_test`
Expected: PASS with 1 passed test.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-search`
Run: `git add crates/vox-search/src/term_density_reranker.rs crates/vox-search/src/lib.rs crates/vox-search/tests/term_density_reranker_test.rs`
Run: `git commit -m "feat(search): implement term-density passage reranker"`

---

### Task 2.3: Parallel Multi-Branch Exploration with Cancellation [PARALLEL-SAFE]

**Files:**
- Create: `crates/vox-search/src/parallel_exploration.rs`
- Modify: `crates/vox-search/src/lib.rs`
- Test: `crates/vox-search/tests/parallel_exploration_test.rs`

**Interfaces:**
- Consumes: Branch definitions, timeouts, cancellation tokens
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct ResearchBranch {
      pub branch_id: String,
      pub query: String,
  }

  #[derive(Debug, Clone, PartialEq)]
  pub struct BranchResult {
      pub branch_id: String,
      pub query: String,
      pub snippets: Vec<String>,
      pub success: bool,
  }

  pub struct ParallelExplorationCoordinator;
  impl ParallelExplorationCoordinator {
      pub async fn execute_branches_resilient<F, Fut>(
          branches: Vec<ResearchBranch>,
          concurrency: usize,
          timeout_per_branch: std::time::Duration,
          cancellation: Option<tokio_util::sync::CancellationToken>,
          fetcher: F,
      ) -> Vec<BranchResult>
      where
          F: Fn(String) -> Fut + Send + Sync + 'static + Clone,
          Fut: std::future::Future<Output = anyhow::Result<Vec<String>>> + Send + 'static;
  }
  ```

- [ ] **Step 1: Write the failing test**

Write `crates/vox-search/tests/parallel_exploration_test.rs`:
```rust
use std::time::Duration;
use vox_search::parallel_exploration::{ParallelExplorationCoordinator, ResearchBranch};

#[tokio::test]
async fn test_parallel_exploration_resilience_and_timeout() {
    let branches = vec![
        ResearchBranch {
            branch_id: "b1".to_string(),
            query: "fast".to_string(),
        },
        ResearchBranch {
            branch_id: "b2".to_string(),
            query: "slow".to_string(),
        },
    ];

    let results = ParallelExplorationCoordinator::execute_branches_resilient(
        branches,
        2,
        Duration::from_millis(50),
        None,
        |q| async move {
            if q == "slow" {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            Ok(vec![format!("Result for {q}")])
        },
    )
    .await;

    assert_eq!(results.len(), 2);
    let fast_res = results.iter().find(|r| r.branch_id == "b1").unwrap();
    let slow_res = results.iter().find(|r| r.branch_id == "b2").unwrap();

    assert!(fast_res.success);
    assert!(!slow_res.success); // Timed out gracefully
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-search --test parallel_exploration_test`
Expected: FAIL with compilation error `cannot find module parallel_exploration in vox_search`

- [ ] **Step 3: Implement resilient parallel exploration coordinator**

Write `crates/vox-search/src/parallel_exploration.rs`:
```rust
use std::time::Duration;
use futures::StreamExt;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq)]
pub struct ResearchBranch {
    pub branch_id: String,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BranchResult {
    pub branch_id: String,
    pub query: String,
    pub snippets: Vec<String>,
    pub success: bool,
}

pub struct ParallelExplorationCoordinator;

impl ParallelExplorationCoordinator {
    pub async fn execute_branches_resilient<F, Fut>(
        branches: Vec<ResearchBranch>,
        concurrency: usize,
        timeout_per_branch: Duration,
        cancellation: Option<CancellationToken>,
        fetcher: F,
    ) -> Vec<BranchResult>
    where
        F: Fn(String) -> Fut + Send + Sync + 'static + Clone,
        Fut: std::future::Future<Output = anyhow::Result<Vec<String>>> + Send + 'static,
    {
        let stream = futures::stream::iter(branches).map(|branch| {
            let fetcher = fetcher.clone();
            let cancel = cancellation.clone();
            async move {
                if let Some(c) = &cancel {
                    if c.is_cancelled() {
                        return BranchResult {
                            branch_id: branch.branch_id,
                            query: branch.query,
                            snippets: vec![],
                            success: false,
                        };
                    }
                }
                match tokio::time::timeout(timeout_per_branch, fetcher(branch.query.clone())).await {
                    Ok(Ok(snippets)) => BranchResult {
                        branch_id: branch.branch_id,
                        query: branch.query,
                        snippets,
                        success: true,
                    },
                    Ok(Err(err)) => {
                        tracing::warn!(branch = %branch.branch_id, error = %err, "Branch fetcher failed");
                        BranchResult {
                            branch_id: branch.branch_id,
                            query: branch.query,
                            snippets: vec![],
                            success: false,
                        }
                    }
                    Err(_) => {
                        tracing::warn!(branch = %branch.branch_id, "Branch fetcher timed out");
                        BranchResult {
                            branch_id: branch.branch_id,
                            query: branch.query,
                            snippets: vec![],
                            success: false,
                        }
                    }
                }
            }
        });

        stream.buffer_unordered(concurrency.max(1)).collect().await
    }
}
```

Modify `crates/vox-search/src/lib.rs` to expose `pub mod parallel_exploration;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-search --test parallel_exploration_test`
Expected: PASS with 1 passed test.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-search`
Run: `git add crates/vox-search/src/parallel_exploration.rs crates/vox-search/src/lib.rs crates/vox-search/tests/parallel_exploration_test.rs`
Run: `git commit -m "feat(search): implement resilient parallel exploration coordinator with cancellation and timeout handling"`

---

## Phase 3: Polyglot Empirical Sandboxing

### Task 3.1: Polyglot Code Verification Sandboxes [PARALLEL-SAFE]

**Files:**
- Modify: `crates/vox-research-shim/Cargo.toml`
- Create: `crates/vox-research-shim/src/research/domain/polyglot_sandbox.rs`
- Modify: `crates/vox-research-shim/src/research/domain/mod.rs`
- Test: `crates/vox-research-shim/tests/polyglot_sandbox_test.rs`

**Interfaces:**
- Consumes: `CodeSandboxResult`
- Produces:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  pub enum PolyglotLanguage { Rust, TypeScript, Python, Sql, Vox }
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum SqlDialect { GenericSqlite, Postgres, MySql }

  pub async fn verify_in_polyglot_sandbox(
      code: &str,
      lang: PolyglotLanguage,
  ) -> anyhow::Result<crate::research::domain::codegen::CodeSandboxResult>;
  ```

- [ ] **Step 1: Write the failing test**

Add `turso = { workspace = true }`, `vox-compiler = { workspace = true }`, and `sqlparser = "0.54"` to `crates/vox-research-shim/Cargo.toml`.

Write `crates/vox-research-shim/tests/polyglot_sandbox_test.rs`:
```rust
use vox_research_shim::research::domain::polyglot_sandbox::{PolyglotLanguage, verify_in_polyglot_sandbox};

#[tokio::test]
async fn test_verify_sql_in_memory_sandbox() {
    let valid_sql = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT); INSERT INTO users VALUES (1, 'Alice');";
    let res = verify_in_polyglot_sandbox(valid_sql, PolyglotLanguage::Sql).await.expect("sandbox probe");
    assert!(res.passed);

    let invalid_sql = "CRATE TABEL broken syntax error;";
    let res = verify_in_polyglot_sandbox(invalid_sql, PolyglotLanguage::Sql).await.expect("sandbox probe");
    assert!(!res.passed);
}

#[tokio::test]
async fn test_verify_python_syntax_sandbox() {
    let valid_py = "def add(a: int, b: int) -> int:\n    return a + b\n";
    let res = verify_in_polyglot_sandbox(valid_py, PolyglotLanguage::Python).await.expect("sandbox probe");
    assert!(res.passed);

    let invalid_py = "def broken(\n    return 1\n";
    let res = verify_in_polyglot_sandbox(invalid_py, PolyglotLanguage::Python).await.expect("sandbox probe");
    assert!(!res.passed);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-research-shim --test polyglot_sandbox_test`
Expected: FAIL with compilation error `cannot find module polyglot_sandbox`

- [ ] **Step 3: Implement hardened polyglot sandbox**

Write `crates/vox-research-shim/src/research/domain/polyglot_sandbox.rs`:
```rust
use serde::{Deserialize, Serialize};
use crate::research::domain::codegen::{CodeSandboxResult, verify_rust_code_in_sandbox};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolyglotLanguage {
    Rust,
    TypeScript,
    Python,
    Sql,
    Vox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlDialect {
    GenericSqlite,
    Postgres,
    MySql,
}

pub async fn verify_in_polyglot_sandbox(
    code: &str,
    lang: PolyglotLanguage,
) -> anyhow::Result<CodeSandboxResult> {
    match lang {
        PolyglotLanguage::Rust => verify_rust_code_in_sandbox(code, &[]).await,
        PolyglotLanguage::Sql => verify_sql_in_sandbox(code, SqlDialect::GenericSqlite).await,
        PolyglotLanguage::Python => verify_python_in_sandbox(code).await,
        PolyglotLanguage::TypeScript => verify_typescript_in_sandbox(code).await,
        PolyglotLanguage::Vox => verify_vox_in_sandbox(code).await,
    }
}

pub async fn verify_sql_in_sandbox(sql: &str, dialect: SqlDialect) -> anyhow::Result<CodeSandboxResult> {
    match dialect {
        SqlDialect::GenericSqlite => {
            let pool = turso::Builder::new_local(":memory:").build().await?;
            let conn = pool.connect()?;
            match conn.execute_batch(sql).await {
                Ok(_) => Ok(CodeSandboxResult {
                    passed: true,
                    stdout: "SQLite query executed successfully".to_string(),
                    stderr: String::new(),
                }),
                Err(e) => Ok(CodeSandboxResult {
                    passed: false,
                    stdout: String::new(),
                    stderr: e.to_string(),
                }),
            }
        }
        SqlDialect::Postgres | SqlDialect::MySql => {
            let dialect_impl: Box<dyn sqlparser::dialect::Dialect> = match dialect {
                SqlDialect::Postgres => Box::new(sqlparser::dialect::PostgreSqlDialect {}),
                SqlDialect::MySql => Box::new(sqlparser::dialect::MySqlDialect {}),
                _ => unreachable!(),
            };
            match sqlparser::parser::Parser::parse_sql(&*dialect_impl, sql) {
                Ok(ast) => Ok(CodeSandboxResult {
                    passed: true,
                    stdout: format!("SQL syntax verified for {dialect:?} ({} statements)", ast.len()),
                    stderr: String::new(),
                }),
                Err(e) => Ok(CodeSandboxResult {
                    passed: false,
                    stdout: String::new(),
                    stderr: format!("SQL syntax error for {dialect:?}: {e}"),
                }),
            }
        }
    }
}

pub async fn verify_python_in_sandbox(code: &str) -> anyhow::Result<CodeSandboxResult> {
    let temp_file = tempfile::Builder::new().suffix(".py").tempfile()?;
    tokio::fs::write(temp_file.path(), code).await?;

    let python_cmd = if cfg!(windows) { "python" } else { "python3" };

    let syntax_check = tokio::process::Command::new(python_cmd)
        .kill_on_drop(true)
        .arg("-c")
        .arg("import ast, sys; ast.parse(open(sys.argv[1]).read())")
        .arg(temp_file.path())
        .output()
        .await;

    match syntax_check {
        Ok(out) => {
            let passed = out.status.success();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            Ok(CodeSandboxResult {
                passed,
                stdout: String::new(),
                stderr,
            })
        }
        Err(e) => Ok(CodeSandboxResult {
            passed: false,
            stdout: String::new(),
            stderr: format!("Python executable failed to launch: {e}"),
        }),
    }
}

pub async fn verify_typescript_in_sandbox(code: &str) -> anyhow::Result<CodeSandboxResult> {
    let temp_dir_guard = tempfile::Builder::new().prefix("vox-ts-sandbox-").tempdir()?;
    let temp_dir = temp_dir_guard.path();
    let ts_file = temp_dir.join("probe.ts");
    let ambient_file = temp_dir.join("ambient.d.ts");

    tokio::fs::write(&ts_file, code).await?;
    tokio::fs::write(&ambient_file, "declare module '*';").await?;

    // Try tsc first
    let tsc_res = tokio::process::Command::new("tsc")
        .kill_on_drop(true)
        .arg("--noEmit")
        .arg("--skipLibCheck")
        .arg(&ambient_file)
        .arg(&ts_file)
        .output()
        .await;

    if let Ok(out) = tsc_res {
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let all_errs = format!("{stdout}\n{stderr}");
        let real_errors: Vec<&str> = all_errs
            .lines()
            .filter(|l| l.contains("error TS") && !l.contains("TS2307") && !l.contains("TS7016"))
            .collect();

        return Ok(CodeSandboxResult {
            passed: real_errors.is_empty(),
            stdout,
            stderr: real_errors.join("\n"),
        });
    }

    // Fallback to bun
    let bun_res = tokio::process::Command::new("bun")
        .kill_on_drop(true)
        .args(["build", "--no-bundle"])
        .arg(&ts_file)
        .output()
        .await;

    if let Ok(out) = bun_res {
        return Ok(CodeSandboxResult {
            passed: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        });
    }

    anyhow::bail!("No TypeScript verifier (tsc or bun) available in environment")
}

pub async fn verify_vox_in_sandbox(code: &str) -> anyhow::Result<CodeSandboxResult> {
    let parsed = vox_compiler::parser::parse_source(code);
    match parsed {
        Ok(_) => Ok(CodeSandboxResult {
            passed: true,
            stdout: "Vox source parsed successfully".to_string(),
            stderr: String::new(),
        }),
        Err(e) => Ok(CodeSandboxResult {
            passed: false,
            stdout: String::new(),
            stderr: format!("Vox syntax error: {e:?}"),
        }),
    }
}
```

Modify `crates/vox-research-shim/src/research/domain/mod.rs` to add `pub mod polyglot_sandbox;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-research-shim --test polyglot_sandbox_test`
Expected: PASS with 2 passed tests.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-research-shim`
Run: `git add crates/vox-research-shim/Cargo.toml crates/vox-research-shim/src/research/domain/polyglot_sandbox.rs crates/vox-research-shim/src/research/domain/mod.rs crates/vox-research-shim/tests/polyglot_sandbox_test.rs`
Run: `git commit -m "feat(research-shim): implement polyglot sandbox with AST parsing and ambient module type checking"`

---

### Task 3.2: Micro-Benchmarking Engine with Black-Box Optimization Guards [PARALLEL-SAFE]

**Files:**
- Create: `crates/vox-research-shim/src/research/domain/micro_benchmark.rs`
- Modify: `crates/vox-research-shim/src/research/domain/mod.rs`
- Test: `crates/vox-research-shim/tests/micro_benchmark_test.rs`

**Interfaces:**
- Consumes: Code snippet, benchmark iterations
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct MicroBenchmarkReport {
      pub passed: bool,
      pub iterations: usize,
      pub median_ns: u64,
      pub mean_ns: u64,
      pub stderr: String,
  }

  pub fn scaffold_rust_benchmark_source(inner_statement: &str, iterations: usize) -> String;
  pub async fn run_rust_micro_benchmark(inner_statement: &str, iterations: usize, timeout_ms: u64) -> anyhow::Result<MicroBenchmarkReport>;
  ```

- [ ] **Step 1: Write the failing test**

Write `crates/vox-research-shim/tests/micro_benchmark_test.rs`:
```rust
use vox_research_shim::research::domain::micro_benchmark::{run_rust_micro_benchmark, scaffold_rust_benchmark_source};

#[test]
fn test_scaffold_rust_benchmark_source_uses_black_box() {
    let source = scaffold_rust_benchmark_source("let mut x = 0; for i in 0..100 { x += i; }", 50);
    assert!(source.contains("std::hint::black_box"));
    assert!(source.contains("BENCHMARK_RESULT:"));
}

#[tokio::test]
async fn test_run_rust_micro_benchmark_success() {
    let report = run_rust_micro_benchmark("let mut v = Vec::new(); v.push(42);", 20, 5000)
        .await
        .expect("benchmark execution");
    assert!(report.passed);
    assert_eq!(report.iterations, 20);
    assert!(report.mean_ns > 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-research-shim --test micro_benchmark_test`
Expected: FAIL with compilation error `cannot find module micro_benchmark`

- [ ] **Step 3: Implement benchmark runner**

Write `crates/vox-research-shim/src/research/domain/micro_benchmark.rs`:
```rust
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct MicroBenchmarkReport {
    pub passed: bool,
    pub iterations: usize,
    pub median_ns: u64,
    pub mean_ns: u64,
    pub stderr: String,
}

pub fn scaffold_rust_benchmark_source(inner_statement: &str, iterations: usize) -> String {
    format!(
        r#"
fn main() {{
    // Warmup cache & avoid cold-start timer artifacts
    for _ in 0..10 {{
        let res = std::hint::black_box({{
            {inner_statement}
        }});
        std::hint::black_box(res);
    }}

    let mut timings = Vec::with_capacity({iterations});
    for _ in 0..{iterations} {{
        let start = std::time::Instant::now();
        let res = std::hint::black_box({{
            {inner_statement}
        }});
        std::hint::black_box(res);
        let elapsed = start.elapsed().as_nanos() as u64;
        timings.push(elapsed);
    }}
    timings.sort_unstable();
    let median = timings[timings.len() / 2];
    let sum: u64 = timings.iter().sum();
    let mean = sum / (timings.len() as u64).max(1);
    println!("BENCHMARK_RESULT:{{}}:{{}}", median, mean);
}}
"#
    )
}

pub async fn run_rust_micro_benchmark(
    inner_statement: &str,
    iterations: usize,
    timeout_ms: u64,
) -> anyhow::Result<MicroBenchmarkReport> {
    let temp_dir_guard = tempfile::Builder::new().prefix("vox-bench-").tempdir()?;
    let temp_dir = temp_dir_guard.path();
    let bin_name = if cfg!(windows) { "bench_bin.exe" } else { "bench_bin" };
    let bin_path = temp_dir.join(bin_name);

    let source = scaffold_rust_benchmark_source(inner_statement, iterations);

    let mut compile_cmd = Command::new("rustc");
    compile_cmd
        .kill_on_drop(true)
        .arg("-O")
        .arg("-o")
        .arg(&bin_path)
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = compile_cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(source.as_bytes()).await?;
        stdin.flush().await?;
        drop(stdin);
    }

    let compile_output = tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        child.wait_with_output(),
    )
    .await??;

    if !compile_output.status.success() {
        return Ok(MicroBenchmarkReport {
            passed: false,
            iterations: 0,
            median_ns: 0,
            mean_ns: 0,
            stderr: String::from_utf8_lossy(&compile_output.stderr).to_string(),
        });
    }

    let mut exec_cmd = Command::new(&bin_path);
    exec_cmd.kill_on_drop(true);
    exec_cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());

    let mut exec_child = exec_cmd.spawn()?;
    let run_output = match tokio::time::timeout(Duration::from_millis(timeout_ms), exec_child.wait_with_output()).await {
        Ok(res) => res?,
        Err(_) => {
            let _ = exec_child.start_kill();
            return Ok(MicroBenchmarkReport {
                passed: false,
                iterations: 0,
                median_ns: 0,
                mean_ns: 0,
                stderr: format!("Benchmark execution timed out after {timeout_ms}ms"),
            });
        }
    };

    let stdout_str = String::from_utf8_lossy(&run_output.stdout);
    for line in stdout_str.lines() {
        if let Some(rest) = line.strip_prefix("BENCHMARK_RESULT:") {
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() == 2 {
                let median: u64 = parts[0].parse().unwrap_or(0);
                let mean: u64 = parts[1].parse().unwrap_or(0);
                return Ok(MicroBenchmarkReport {
                    passed: true,
                    iterations,
                    median_ns: median,
                    mean_ns: mean,
                    stderr: String::new(),
                });
            }
        }
    }

    Ok(MicroBenchmarkReport {
        passed: false,
        iterations: 0,
        median_ns: 0,
        mean_ns: 0,
        stderr: "Failed to parse benchmark stdout".to_string(),
    })
}
```

Modify `crates/vox-research-shim/src/research/domain/mod.rs` to add `pub mod micro_benchmark;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-research-shim --test micro_benchmark_test`
Expected: PASS with 2 passed tests.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-research-shim`
Run: `git add crates/vox-research-shim/src/research/domain/micro_benchmark.rs crates/vox-research-shim/src/research/domain/mod.rs crates/vox-research-shim/tests/micro_benchmark_test.rs`
Run: `git commit -m "feat(research-shim): implement micro-benchmarking engine with black_box dead-code elimination protection"`

---

### Task 3.3: Upstream Git Shallow Clone & Probe Runner [PARALLEL-SAFE]

**Files:**
- Create: `crates/vox-research-shim/src/research/domain/git_probe.rs`
- Modify: `crates/vox-research-shim/src/research/domain/mod.rs`
- Test: `crates/vox-research-shim/tests/git_probe_test.rs`

**Interfaces:**
- Consumes: Remote repo URL or local path, timeout
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct GitProbeOutcome {
      pub passed: bool,
      pub head_sha: String,
      pub stdout: String,
      pub stderr: String,
  }

  pub async fn execute_shallow_clone_and_probe(
      repo_url: &str,
      probe_command: &str,
      args: &[&str],
      timeout_secs: u64,
  ) -> anyhow::Result<GitProbeOutcome>;
  ```

- [ ] **Step 1: Write the failing test**

Write `crates/vox-research-shim/tests/git_probe_test.rs`:
```rust
use vox_research_shim::research::domain::git_probe::execute_shallow_clone_and_probe;

#[tokio::test]
async fn test_execute_shallow_clone_rejects_non_https() {
    let outcome = execute_shallow_clone_and_probe("ftp://bad-url", "git", &["status"], 5).await;
    assert!(outcome.is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-research-shim --test git_probe_test`
Expected: FAIL with compilation error `cannot find module git_probe`

- [ ] **Step 3: Implement git probe module**

Write `crates/vox-research-shim/src/research/domain/git_probe.rs`:
```rust
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct GitProbeOutcome {
    pub passed: bool,
    pub head_sha: String,
    pub stdout: String,
    pub stderr: String,
}

pub async fn execute_shallow_clone_and_probe(
    repo_url: &str,
    probe_command: &str,
    args: &[&str],
    timeout_secs: u64,
) -> anyhow::Result<GitProbeOutcome> {
    if !repo_url.starts_with("https://") && !repo_url.starts_with("git://") {
        anyhow::bail!("Invalid git repository URL scheme: {repo_url}");
    }

    let temp_dir_guard = tempfile::Builder::new().prefix("vox-git-probe-").tempdir()?;
    let clone_dest = temp_dir_guard.path().join("repo");

    let mut clone_cmd = Command::new("git");
    clone_cmd.kill_on_drop(true);
    clone_cmd.env("GIT_TERMINAL_PROMPT", "0");
    clone_cmd.env("GIT_ASKPASS", "");
    clone_cmd.args(["clone", "--depth", "1", "--", repo_url])
        .arg(&clone_dest);

    let clone_res = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        clone_cmd.output(),
    ).await;

    let clone_output = match clone_res {
        Ok(res) => res?,
        Err(_) => anyhow::bail!("Git clone timed out after {timeout_secs}s"),
    };

    if !clone_output.status.success() {
        return Ok(GitProbeOutcome {
            passed: false,
            head_sha: String::new(),
            stdout: String::from_utf8_lossy(&clone_output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&clone_output.stderr).to_string(),
        });
    }

    let rev_parse = Command::new("git")
        .kill_on_drop(true)
        .current_dir(&clone_dest)
        .args(["rev-parse", "HEAD"])
        .output()
        .await?;

    let head_sha = String::from_utf8_lossy(&rev_parse.stdout).trim().to_string();

    let mut probe = Command::new(probe_command);
    probe.kill_on_drop(true);
    probe.current_dir(&clone_dest).args(args);

    let probe_res = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        probe.output(),
    ).await;

    let probe_output = match probe_res {
        Ok(res) => res?,
        Err(_) => anyhow::bail!("Probe command '{probe_command}' timed out"),
    };

    Ok(GitProbeOutcome {
        passed: probe_output.status.success(),
        head_sha,
        stdout: String::from_utf8_lossy(&probe_output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&probe_output.stderr).to_string(),
    })
}
```

Add exemption to `docs/src/architecture/layers.toml` for `crates/vox-research-shim/src/research/domain/git_probe.rs` under `raw-git-exec`.
Modify `crates/vox-research-shim/src/research/domain/mod.rs` to add `pub mod git_probe;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-research-shim --test git_probe_test`
Expected: PASS with 1 passed test.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-research-shim`
Run: `git add docs/src/architecture/layers.toml crates/vox-research-shim/src/research/domain/git_probe.rs crates/vox-research-shim/src/research/domain/mod.rs crates/vox-research-shim/tests/git_probe_test.rs`
Run: `git commit -m "feat(research-shim): implement upstream git clone and probe runner with non-interactive env"`

---

## Phase 4: Epistemic Knowledge Graph & Autonomic Mining

### Task 4.1: Cycle-Safe Relational Epistemic Graph CTE Traversal in `vox-db` [PARALLEL-SAFE]

**Files:**
- Modify: `crates/vox-db/src/schema/domains/knowledge.rs`
- Modify: `crates/vox-db/src/store/ops_memory/knowledge.rs`
- Modify: `crates/vox-db/src/lib.rs`
- Test: `crates/vox-db/tests/epistemic_graph_test.rs`

**Interfaces:**
- Consumes: `knowledge_nodes`, `knowledge_edges`
- Produces:
  ```rust
  impl VoxDb {
      pub async fn find_reachable_knowledge_nodes_cte(
          &self,
          root_id: &str,
          max_depth: usize,
          direction: &str, // "forward" | "reverse" | "undirected"
      ) -> Result<Vec<String>, StoreError>;
  }
  ```

- [ ] **Step 1: Write the failing test**

Write `crates/vox-db/tests/epistemic_graph_test.rs`:
```rust
use vox_db::VoxDb;

#[tokio::test]
async fn test_epistemic_graph_cte_cycle_protection() {
    let db = VoxDb::in_memory().await.expect("db init");

    // Insert nodes via canonical ops
    db.upsert_knowledge_node("node:A", "Node A", Some("Node A content"), "concept", None, None).await.unwrap();
    db.upsert_knowledge_node("node:B", "Node B", Some("Node B content"), "concept", None, None).await.unwrap();

    // Create cycle A -> B -> A
    db.create_knowledge_edge("node:A", "node:B", "links_to", 1.0, None).await.unwrap();
    db.create_knowledge_edge("node:B", "node:A", "links_to", 1.0, None).await.unwrap();

    // Must terminate cleanly without infinite loop or recursion limit abort
    let reachable = db.find_reachable_knowledge_nodes_cte("node:A", 5, "forward").await.unwrap();
    assert_eq!(reachable.len(), 2);
    assert!(reachable.contains(&"node:A".to_string()));
    assert!(reachable.contains(&"node:B".to_string()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-db --test epistemic_graph_test`
Expected: FAIL with compilation error `no method named find_reachable_knowledge_nodes_cte found for struct VoxDb`

- [ ] **Step 3: Implement cycle-safe recursive CTE**

Add covering indexes to `crates/vox-db/src/schema/domains/knowledge.rs`:
```sql
CREATE INDEX IF NOT EXISTS idx_knowledge_edges_src_rel ON knowledge_edges(src_id, relation, dst_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_edges_dst_rel ON knowledge_edges(dst_id, relation, src_id);
```

Add method to `crates/vox-db/src/store/ops_memory/knowledge.rs`:
```rust
impl VoxDb {
    pub async fn find_reachable_knowledge_nodes_cte(
        &self,
        root_id: &str,
        max_depth: usize,
        direction: &str,
    ) -> Result<Vec<String>, StoreError> {
        let conn = self.conn.clone();
        let breaker = self.breaker.clone();
        let r = root_id.to_string();
        let dir = direction.to_string();

        breaker
            .call(|| async move {
                let sql = "
                WITH RECURSIVE graph_path(node_id, depth, visited_path) AS (
                    SELECT ?1 AS node_id, 0 AS depth, '/' || ?1 || '/' AS visited_path
                    UNION ALL
                    SELECT 
                        CASE WHEN ?3 = 'reverse' THEN e.src_id 
                             WHEN ?3 = 'forward' THEN e.dst_id 
                             ELSE (CASE WHEN e.src_id = p.node_id THEN e.dst_id ELSE e.src_id END)
                        END,
                        p.depth + 1,
                        p.visited_path || (CASE WHEN ?3 = 'reverse' THEN e.src_id 
                                                WHEN ?3 = 'forward' THEN e.dst_id 
                                                ELSE (CASE WHEN e.src_id = p.node_id THEN e.dst_id ELSE e.src_id END)
                                           END) || '/'
                    FROM knowledge_edges e
                    JOIN graph_path p ON (
                        (?3 = 'forward' AND e.src_id = p.node_id) OR
                        (?3 = 'reverse' AND e.dst_id = p.node_id) OR
                        (?3 = 'undirected' AND (e.src_id = p.node_id OR e.dst_id = p.node_id))
                    )
                    WHERE p.depth < ?2
                      AND instr(p.visited_path, '/' || (
                          CASE WHEN ?3 = 'reverse' THEN e.src_id 
                               WHEN ?3 = 'forward' THEN e.dst_id 
                               ELSE (CASE WHEN e.src_id = p.node_id THEN e.dst_id ELSE e.src_id END)
                          END
                      ) || '/') = 0
                )
                SELECT DISTINCT node_id FROM graph_path;
                ";
                let mut rows = conn.query(sql, turso::params![r.as_str(), max_depth as i64, dir.as_str()]).await?;
                let mut out = Vec::new();
                while let Some(row) = rows.next().await? {
                    let id: String = row.get(0)?;
                    out.push(id);
                }
                Ok(out)
            })
            .await
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-db --test epistemic_graph_test`
Expected: PASS with 1 passed test.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-db`
Run: `git add crates/vox-db/src/schema/domains/knowledge.rs crates/vox-db/src/store/ops_memory/knowledge.rs crates/vox-db/tests/epistemic_graph_test.rs`
Run: `git commit -m "feat(db): implement cycle-safe epistemic recursive CTE with visited path tracking"`

---

### Task 4.2: Temporal Validity & Semantic Version Tagging [PARALLEL-SAFE]

**Files:**
- Modify: `crates/vox-db/Cargo.toml`
- Create: `crates/vox-db/src/temporal_claims.rs`
- Modify: `crates/vox-db/src/lib.rs`
- Test: `crates/vox-db/tests/temporal_claims_test.rs`

**Interfaces:**
- Consumes: Loose semver strings
- Produces:
  ```rust
  pub fn normalize_semver(s: &str) -> Option<semver::Version>;
  pub fn is_claim_valid_at_version(
      valid_since: Option<&str>,
      valid_until: Option<&str>,
      version_req: Option<&str>,
      target_version_str: &str,
  ) -> Result<bool, String>;
  ```

- [ ] **Step 1: Write the failing test**

Add `semver = { workspace = true }` to `crates/vox-db/Cargo.toml`.

Write `crates/vox-db/tests/temporal_claims_test.rs`:
```rust
use vox_db::temporal_claims::{is_claim_valid_at_version, normalize_semver};

#[test]
fn test_normalize_semver_loose_formats() {
    assert_eq!(normalize_semver("v1.25.0").unwrap().to_string(), "1.25.0");
    assert_eq!(normalize_semver("1.0").unwrap().to_string(), "1.0.0");
    assert_eq!(normalize_semver("2").unwrap().to_string(), "2.0.0");
}

#[test]
fn test_is_claim_valid_at_version_interval_semantics() {
    assert!(is_claim_valid_at_version(Some("1.0.0"), None, None, "v1.25.0").unwrap());
    assert!(!is_claim_valid_at_version(Some("1.0.0"), None, None, "0.2.22").unwrap());

    // Half-open interval [0.2.0, 1.0.0)
    assert!(is_claim_valid_at_version(Some("0.2.0"), Some("1.0.0"), None, "0.9.0").unwrap());
    assert!(!is_claim_valid_at_version(Some("0.2.0"), Some("1.0.0"), None, "1.0.0").unwrap());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-db --test temporal_claims_test`
Expected: FAIL with compilation error `cannot find module temporal_claims in vox_db`

- [ ] **Step 3: Implement temporal claims evaluator**

Write `crates/vox-db/src/temporal_claims.rs`:
```rust
use semver::{Version, VersionReq};

pub fn normalize_semver(s: &str) -> Option<Version> {
    let s = s.trim();
    let s = s.strip_prefix('v').or_else(|| s.strip_prefix('V')).unwrap_or(s);
    let s = s.strip_prefix('=').unwrap_or(s).trim();

    if let Ok(v) = Version::parse(s) {
        return Some(v);
    }

    let (ver_core, extra) = if let Some(idx) = s.find(['-', '+']) {
        (&s[..idx], Some(&s[idx..]))
    } else {
        (s, None)
    };

    let parts: Vec<&str> = ver_core.split('.').collect();
    let padded = match parts.len() {
        1 if !parts[0].is_empty() => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        3 => format!("{}.{}.{}", parts[0], parts[1], parts[2]),
        _ => return None,
    };

    let reconstructed = match extra {
        Some(ext) => format!("{}{}", padded, ext),
        None => padded,
    };

    Version::parse(&reconstructed).ok()
}

pub fn is_claim_valid_at_version(
    valid_since: Option<&str>,
    valid_until: Option<&str>,
    version_req: Option<&str>,
    target_version_str: &str,
) -> Result<bool, String> {
    let target = normalize_semver(target_version_str)
        .ok_or_else(|| format!("Invalid target version: '{target_version_str}'"))?;

    if let Some(req_str) = version_req {
        let req = VersionReq::parse(req_str)
            .map_err(|e| format!("Invalid version requirement '{req_str}': {e}"))?;
        if !req.matches(&target) {
            return Ok(false);
        }
    }

    if let Some(since_str) = valid_since {
        if let Some(since) = normalize_semver(since_str) {
            if target < since {
                return Ok(false);
            }
        }
    }

    if let Some(until_str) = valid_until {
        if let Some(until) = normalize_semver(until_str) {
            if target >= until {
                return Ok(false);
            }
        }
    }

    Ok(true)
}
```

Modify `crates/vox-db/src/lib.rs` to add `pub mod temporal_claims;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-db --test temporal_claims_test`
Expected: PASS with 2 passed tests.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-db`
Run: `git add crates/vox-db/Cargo.toml crates/vox-db/src/temporal_claims.rs crates/vox-db/src/lib.rs crates/vox-db/tests/temporal_claims_test.rs`
Run: `git commit -m "feat(db): implement loose semver normalization and half-open interval claim validity"`

---

### Task 4.3: Continuous Regression Parity Auditing: `vox audit research-parity` [PARALLEL-SAFE]

**Files:**
- Create: `crates/vox-cli/src/commands/audit_research_parity.rs`
- Modify: `crates/vox-cli/src/commands/audit.rs`
- Test: `crates/vox-cli/tests/audit_research_parity_test.rs`

**Interfaces:**
- Consumes: `docs/src/architecture/*.md`, filesystem
- Produces:
  ```rust
  #[derive(clap::Args, Debug, Clone)]
  pub struct ResearchParityArgs {
      #[arg(long, default_value = "docs/src/architecture")]
      pub docs_dir: std::path::PathBuf,
      #[arg(long)]
      pub json: bool,
  }

  pub fn run_research_parity_audit_sync(args: &ResearchParityArgs) -> anyhow::Result<usize>;
  ```

- [ ] **Step 1: Write the failing test**

Write `crates/vox-cli/tests/audit_research_parity_test.rs`:
```rust
use vox_cli::commands::audit_research_parity::{ResearchParityArgs, run_research_parity_audit_sync};

#[test]
fn test_research_parity_audit_detects_probes() {
    let args = ResearchParityArgs {
        docs_dir: std::path::PathBuf::from("docs/src/architecture"),
        json: false,
    };
    let count = run_research_parity_audit_sync(&args).expect("run audit");
    assert!(count >= 2); // metal_optimization_probe_test.rs and ax_snapshot_probe_test.rs
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-cli --test audit_research_parity_test`
Expected: FAIL with compilation error `cannot find module audit_research_parity`

- [ ] **Step 3: Implement synchronous parity audit emitting JSONL**

Write `crates/vox-cli/src/commands/audit_research_parity.rs`:
```rust
use clap::Args;
use std::path::{Path, PathBuf};
use serde::Serialize;

#[derive(Args, Debug, Clone)]
pub struct ResearchParityArgs {
    #[arg(long, default_value = "docs/src/architecture")]
    pub docs_dir: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Serialize)]
struct ParityFinding {
    schema_version: u32,
    doc_path: String,
    referenced_probe: String,
    probe_exists: bool,
}

pub fn extract_empirical_test_references(doc_content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in doc_content.split_whitespace() {
        let clean = word
            .trim_matches('`')
            .trim_matches('(')
            .trim_matches(')')
            .trim_matches('[')
            .trim_matches(']');
        if clean.ends_with("_probe_test.rs") || clean.ends_with("_test.rs") {
            if !out.contains(&clean.to_string()) {
                out.push(clean.to_string());
            }
        }
    }
    out
}

pub fn run_research_parity_audit_sync(args: &ResearchParityArgs) -> anyhow::Result<usize> {
    if !args.docs_dir.exists() {
        anyhow::bail!("Docs directory not found: {:?}", args.docs_dir);
    }
    let mut total_probes = 0usize;
    for entry in std::fs::read_dir(&args.docs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let content = std::fs::read_to_string(&path)?;
            let refs = extract_empirical_test_references(&content);
            for r in refs {
                total_probes += 1;
                let exists = Path::new(&r).exists();
                if args.json {
                    let finding = ParityFinding {
                        schema_version: 1,
                        doc_path: path.display().to_string(),
                        referenced_probe: r,
                        probe_exists: exists,
                    };
                    println!("{}", serde_json::to_string(&finding)?);
                }
            }
        }
    }
    if !args.json {
        println!("Audited architecture SSOTs: found {} empirical test probe citations", total_probes);
    }
    Ok(total_probes)
}
```

Modify `crates/vox-cli/src/commands/audit.rs`:
- Add `pub mod audit_research_parity;`
- Add `ResearchParity(audit_research_parity::ResearchParityArgs)` variant to `enum AuditSubcommand`.
- Dispatch it synchronously in `run_audit_subcommand`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-cli --test audit_research_parity_test`
Expected: PASS with 1 passed test.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-cli`
Run: `git add crates/vox-cli/src/commands/audit_research_parity.rs crates/vox-cli/src/commands/audit.rs crates/vox-cli/tests/audit_research_parity_test.rs`
Run: `git commit -m "feat(cli): implement synchronous vox audit research-parity with schema_version JSONL output"`

---

### Task 4.4: Dynamic Research DAG Canvas & Sandbox REPL in GUI [SEQUENTIAL]

**Files:**
- Modify: `crates/vox-gui/src/commands/research.rs`
- Modify: `crates/vox-gui/src/main.rs`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts`
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchDagCanvas.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchDagCanvas.test.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/SandboxReplModal.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/SandboxReplModal.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`

**Interfaces:**
- Consumes: Radix Dialog, `researchActions.ts`
- Produces:
  ```tsx
  export interface DagNode { id: string; label: string; wave: number; status: 'pending' | 'verified' | 'contradicted'; }
  export interface DagEdge { srcId: string; dstId: string; relation: 'supports' | 'contradicts'; }
  export function ResearchDagCanvas(props: { nodes: DagNode[]; edges: DagEdge[] }): JSX.Element;
  export function SandboxReplModal(props: { isOpen: boolean; onClose: () => void; initialCode: string }): JSX.Element;
  ```

- [ ] **Step 1: Write the failing tests**

Write `crates/vox-gui/ui/src/components/surfaces/Research/ResearchDagCanvas.test.tsx`:
```tsx
import { render, screen } from '@testing-library/react';
import React from 'react';
import { describe, expect, it } from 'vitest';
import { ResearchDagCanvas } from './ResearchDagCanvas';

describe('ResearchDagCanvas', () => {
  it('renders dynamic svg canvas with wave auto-positioning and wcag accessibility', () => {
    const nodes = [
      { id: '1', label: 'Node A', wave: 0, status: 'verified' as const },
      { id: '2', label: 'Node B', wave: 1, status: 'contradicted' as const },
    ];
    const edges = [{ srcId: '1', dstId: '2', relation: 'contradicts' as const }];

    render(<ResearchDagCanvas nodes={nodes} edges={edges} />);
    expect(screen.getByRole('region', { name: /epistemic research dag/i })).toBeInTheDocument();
    expect(screen.getByText('Node A')).toBeInTheDocument();
    expect(screen.getByText('Node B')).toBeInTheDocument();
  });
});
```

Write `crates/vox-gui/ui/src/components/surfaces/Research/SandboxReplModal.test.tsx`:
```tsx
import { render, screen } from '@testing-library/react';
import React from 'react';
import { describe, expect, it } from 'vitest';
import { SandboxReplModal } from './SandboxReplModal';

describe('SandboxReplModal', () => {
  it('renders modal with run compiler probe button without calling raw invoke', () => {
    render(
      <SandboxReplModal
        isOpen={true}
        onClose={() => {}}
        initialCode="pub fn probe() {}"
      />
    );
    expect(screen.getByText('Sandbox REPL Probe')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /run compiler probe/i })).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --dir crates/vox-gui/ui vitest run src/components/surfaces/Research/ResearchDagCanvas.test.tsx src/components/surfaces/Research/SandboxReplModal.test.tsx`
Expected: FAIL with module not found errors

- [ ] **Step 3: Implement components, backend probe command, and wire into ResearchView**

In `crates/vox-gui/src/commands/research.rs`:
```rust
#[tauri::command]
pub async fn execute_sandbox_probe(
    code: String,
    language: String,
) -> Result<vox_research_shim::research::domain::codegen::CodeSandboxResult, String> {
    let lang = match language.as_str() {
        "python" => vox_research_shim::research::domain::polyglot_sandbox::PolyglotLanguage::Python,
        "typescript" => vox_research_shim::research::domain::polyglot_sandbox::PolyglotLanguage::TypeScript,
        "sql" => vox_research_shim::research::domain::polyglot_sandbox::PolyglotLanguage::Sql,
        "vox" => vox_research_shim::research::domain::polyglot_sandbox::PolyglotLanguage::Vox,
        _ => vox_research_shim::research::domain::polyglot_sandbox::PolyglotLanguage::Rust,
    };
    vox_research_shim::research::domain::polyglot_sandbox::verify_in_polyglot_sandbox(&code, lang)
        .await
        .map_err(|e| e.to_string())
}
```
Register `commands::research::execute_sandbox_probe` in `crates/vox-gui/src/main.rs`.

In `crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts`:
```typescript
export async function executeSandboxProbe(code: string, language: string) {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<{ passed: boolean; stdout: string; stderr: string }>('execute_sandbox_probe', { code, language });
}
```

Write `crates/vox-gui/ui/src/components/surfaces/Research/ResearchDagCanvas.tsx` with dynamic wave-based X/Y layout, Tailwind semantic theme tokens, and accessible status glyphs.

Write `crates/vox-gui/ui/src/components/surfaces/Research/SandboxReplModal.tsx` calling `executeSandboxProbe` and rendering diagnostics.

Wire `ResearchDagCanvas` and `SandboxReplModal` into `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --dir crates/vox-gui/ui vitest run src/components/surfaces/Research/ResearchDagCanvas.test.tsx src/components/surfaces/Research/SandboxReplModal.test.tsx`
Expected: PASS with 2 passed test suites.

- [ ] **Step 5: Commit**

Run: `cargo fmt -p vox-gui`
Run: `git add crates/vox-gui/src/commands/research.rs crates/vox-gui/src/main.rs crates/vox-gui/ui/src/components/surfaces/Research/ crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`
Run: `git commit -m "feat(gui): implement accessible dynamic ResearchDagCanvas, SandboxReplModal, and backend probe command"`

---

## Plan Verification & Quality Gates

Run the full battery of quality gates before declaring complete:
1. `cargo test -p vox-search`
2. `cargo test -p vox-db --all-features`
3. `cargo test -p vox-research-shim`
4. `cargo test -p vox-cli --test audit_research_parity_test`
5. `pnpm --dir crates/vox-gui/ui vitest run`
6. `cargo run -p vox-doc-pipeline -- --lint-only`
7. `cargo run -p vox-cli -- ci doctest-md --strict`
