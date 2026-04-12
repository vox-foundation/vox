//! Versioned search policy loaded from defaults with `VOX_SEARCH_*` environment overrides.
//!
//! SearXNG `engines` / `language` defaults are embedded from
//! [`contracts/scientia/searxng-query.defaults.v1.yaml`](../../../contracts/scientia/searxng-query.defaults.v1.yaml).

use serde::{Deserialize, Serialize};

use crate::searxng_defaults::embedded_searxng_query_defaults;

/// Policy version mirrored into [`vox_db::SearchDiagnostics::policy_version`] and notes.
pub const SEARCH_POLICY_DEFAULT_VERSION: u32 = 1;

/// Tunable retrieval weights and safety rails (replaces ad hoc literals in tool surfaces).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchPolicy {
    /// Monotonic policy schema version.
    pub version: u32,
    /// Weight on the vector leg in memory hybrid fusion (`fuse_hybrid_results`).
    pub memory_vector_fusion_weight: f32,
    /// Trigger automatic verification when coarse evidence quality falls below this threshold.
    pub verification_weak_evidence_threshold: f64,
    /// Weight of top fused score when estimating `evidence_quality`.
    pub evidence_quality_top_weight: f64,
    /// Weight of citation coverage when estimating `evidence_quality`.
    pub evidence_quality_coverage_weight: f64,
    /// Maximum repository files scanned for path-token inventory search per query.
    pub repo_inventory_max_files: usize,
    /// Repository directory names skipped during inventory walks.
    pub repo_inventory_skip_dirs: Vec<String>,
    /// Qdrant HTTP API base (`None` disables sidecar vector path).
    pub qdrant_url: Option<String>,
    /// Named Qdrant collection for repo/doc embeddings mirror.
    pub qdrant_collection: String,
    /// When the collection uses **named** dense vectors, set to the config name (e.g. `default`).
    pub qdrant_vector_name: Option<String>,
    /// Root directory for on-disk Tantivy indices (under `.vox/search/tantivy` by default).
    pub tantivy_index_root: Option<std::path::PathBuf>,
    /// Enable reciprocal rank fusion across corpus hit lists (`VOX_SEARCH_PREFER_RRF`).
    pub prefer_rrf_merge: bool,
    /// Master switch for live web retrieval.
    pub tavily_enabled: bool,
    /// API depth: basic or advanced.
    pub tavily_search_depth: String,
    /// Max results per search call.
    pub tavily_max_results: usize,
    /// Auto-fire when ALL local corpora empty.
    pub tavily_fire_on_empty: bool,
    /// Auto-fire when evidence_quality < threshold (CRAG mode).
    pub tavily_fire_on_weak: bool,
    /// Max credits per session (safety rail).
    pub tavily_credit_budget_per_session: usize,
    /// SearXNG base URL (`None` disables Tier 2).
    pub searxng_url: Option<String>,
    /// Max search results to request from SearXNG/DDG.
    pub searxng_max_results: usize,
    /// Max top hits to deep-scrape for markdown extraction.
    pub searxng_max_urls_to_scrape: usize,
    /// SearXNG `engines=` query parameter (comma-separated engine ids).
    pub searxng_engines: String,
    /// SearXNG `language=` query parameter (short language tag).
    pub searxng_language: String,
    /// Enable Tier 3 DuckDuckGo fallback when SearXNG is unavailable.
    pub duckduckgo_fallback_enabled: bool,
    /// Scraper fetch timeout.
    pub scraper_timeout_ms: u64,
    /// Honor robots.txt (experimental).
    pub scraper_robots_txt_respect: bool,
    /// Minimum text density to accept scraped content (noise reduction).
    pub scraper_min_text_density: f64,
    /// Max iterative search hops for multi-hop research.
    pub web_search_max_hops: u8,
}

impl Default for SearchPolicy {
    fn default() -> Self {
        let searxng_embedded = embedded_searxng_query_defaults();
        Self {
            version: SEARCH_POLICY_DEFAULT_VERSION,
            memory_vector_fusion_weight: 0.55,
            verification_weak_evidence_threshold: 0.55,
            evidence_quality_top_weight: 0.7,
            evidence_quality_coverage_weight: 0.3,
            repo_inventory_max_files: 20_000,
            repo_inventory_skip_dirs: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                ".next".to_string(),
                ".vox".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
            qdrant_url: std::env::var("VOX_SEARCH_QDRANT_URL")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string()),
            qdrant_collection: std::env::var("VOX_SEARCH_QDRANT_COLLECTION")
                .unwrap_or_else(|_| "vox_docs".to_string()),
            qdrant_vector_name: std::env::var("VOX_SEARCH_QDRANT_VECTOR_NAME")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            tantivy_index_root: std::env::var("VOX_SEARCH_TANTIVY_ROOT")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(std::path::PathBuf::from),
            prefer_rrf_merge: parse_truthy_env("VOX_SEARCH_PREFER_RRF"),
            tavily_enabled: parse_truthy_env("VOX_SEARCH_TAVILY_ENABLED"),
            tavily_search_depth: std::env::var("VOX_SEARCH_TAVILY_DEPTH")
                .unwrap_or_else(|_| "basic".to_string()),
            tavily_max_results: std::env::var("VOX_SEARCH_TAVILY_MAX_RESULTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            tavily_fire_on_empty: match std::env::var("VOX_SEARCH_TAVILY_ON_EMPTY") {
                Ok(v) => {
                    let v = v.trim();
                    v == "1"
                        || v.eq_ignore_ascii_case("true")
                        || v.eq_ignore_ascii_case("yes")
                        || v.eq_ignore_ascii_case("on")
                }
                Err(_) => true,
            },
            tavily_fire_on_weak: parse_truthy_env("VOX_SEARCH_TAVILY_ON_WEAK"),
            tavily_credit_budget_per_session: std::env::var("VOX_SEARCH_TAVILY_BUDGET")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            searxng_url: std::env::var("VOX_SEARCH_SEARXNG_URL")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            searxng_max_results: std::env::var("VOX_SEARCH_SEARXNG_MAX_RESULTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            searxng_max_urls_to_scrape: std::env::var("VOX_SEARCH_SEARXNG_MAX_SCRAPE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            searxng_engines: searxng_embedded.engines.clone(),
            searxng_language: searxng_embedded.language.clone(),
            duckduckgo_fallback_enabled: !parse_falsy_env("VOX_SEARCH_DDG_FALLBACK_DISABLED"),
            scraper_timeout_ms: std::env::var("VOX_SEARCH_SCRAPER_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000),
            scraper_robots_txt_respect: parse_truthy_env("VOX_SEARCH_SCRAPER_ROBOTS_RESPECT"),
            scraper_min_text_density: std::env::var("VOX_SEARCH_SCRAPER_MIN_DENSITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.15),
            web_search_max_hops: std::env::var("VOX_SEARCH_MAX_HOPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
        }
    }
}

impl SearchPolicy {
    /// Environment overrides merged onto [`Default::default`].
    #[must_use]
    pub fn from_env() -> Self {
        let mut p = Self::default();
        if let Ok(v) = std::env::var("VOX_SEARCH_POLICY_VERSION")
            && let Ok(n) = v.parse::<u32>()
        {
            p.version = n;
        }
        if let Ok(v) = std::env::var("VOX_SEARCH_MEMORY_VECTOR_WEIGHT")
            && let Ok(w) = v.parse::<f32>()
        {
            p.memory_vector_fusion_weight = w.clamp(0.0, 1.0);
        }
        if let Ok(v) = std::env::var("VOX_SEARCH_VERIFICATION_QUALITY_THRESHOLD")
            && let Ok(t) = v.parse::<f64>()
        {
            p.verification_weak_evidence_threshold = t.clamp(0.0, 1.0);
        }
        if let Ok(v) = std::env::var("VOX_SEARCH_REPO_MAX_FILES")
            && let Ok(n) = v.parse::<usize>()
        {
            p.repo_inventory_max_files = n.max(100);
        }
        if let Ok(raw) = std::env::var("VOX_SEARCH_REPO_SKIP_DIRS") {
            let dirs: Vec<String> = raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !dirs.is_empty() {
                p.repo_inventory_skip_dirs = dirs;
            }
        }
        if std::env::var("VOX_SEARCH_TAVILY_ENABLED").is_ok() {
            p.tavily_enabled = parse_truthy_env("VOX_SEARCH_TAVILY_ENABLED");
        }
        if let Ok(v) = std::env::var("VOX_SEARCH_TAVILY_DEPTH") {
            p.tavily_search_depth = v;
        }
        if let Ok(v) = std::env::var("VOX_SEARCH_TAVILY_MAX_RESULTS")
            && let Ok(n) = v.parse::<usize>()
        {
            p.tavily_max_results = n;
        }
        if std::env::var("VOX_SEARCH_TAVILY_ON_EMPTY").is_ok() {
            p.tavily_fire_on_empty = parse_truthy_env("VOX_SEARCH_TAVILY_ON_EMPTY");
        }
        if std::env::var("VOX_SEARCH_TAVILY_ON_WEAK").is_ok() {
            p.tavily_fire_on_weak = parse_truthy_env("VOX_SEARCH_TAVILY_ON_WEAK");
        }
        if let Ok(v) = std::env::var("VOX_SEARCH_TAVILY_BUDGET")
            && let Ok(n) = v.parse::<usize>()
        {
            p.tavily_credit_budget_per_session = n;
        }
        if let Ok(v) = std::env::var("VOX_SEARCH_MAX_HOPS")
            && let Ok(n) = v.parse::<u8>()
        {
            p.web_search_max_hops = n;
        }
        if let Ok(v) = std::env::var("VOX_SEARCH_SEARXNG_ENGINES") {
            if let Some(norm) = normalize_searxng_engines_csv(&v) {
                p.searxng_engines = norm;
            } else {
                tracing::warn!(
                    raw = %v,
                    "VOX_SEARCH_SEARXNG_ENGINES ignored (allowed: ASCII alnum, comma, hyphen, underscore)"
                );
            }
        }
        if let Ok(v) = std::env::var("VOX_SEARCH_SEARXNG_LANGUAGE") {
            if let Some(norm) = normalize_searxng_language_tag(&v) {
                p.searxng_language = norm;
            } else {
                tracing::warn!(
                    raw = %v,
                    "VOX_SEARCH_SEARXNG_LANGUAGE ignored (allowed: ASCII alnum and hyphen, max 16 chars)"
                );
            }
        }
        p
    }

    /// Effective fusion weight clamped to `[0, 1]`.
    #[must_use]
    pub fn clamped_memory_vector_weight(&self) -> f32 {
        self.memory_vector_fusion_weight.clamp(0.0, 1.0)
    }

    /// Sanitized `engines=` value for SearXNG requests.
    #[must_use]
    pub fn searxng_engines_csv(&self) -> &str {
        self.searxng_engines.as_str()
    }

    /// Sanitized `language=` value for SearXNG requests.
    #[must_use]
    pub fn searxng_language_tag(&self) -> &str {
        self.searxng_language.as_str()
    }
}

/// Restrict `engines=` to characters SearXNG engine bundles use (prevents query injection).
fn normalize_searxng_engines_csv(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() || t.len() > 256 {
        return None;
    }
    if !t
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ',' | '_' | '-'))
    {
        return None;
    }
    Some(t.to_string())
}

fn normalize_searxng_language_tag(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() || t.len() > 16 {
        return None;
    }
    if !t
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return None;
    }
    Some(t.to_string())
}

fn parse_truthy_env(key: &str) -> bool {
    match std::env::var(key) {
        Ok(v) => {
            let v = v.trim();
            v == "1"
                || v.eq_ignore_ascii_case("true")
                || v.eq_ignore_ascii_case("yes")
                || v.eq_ignore_ascii_case("on")
        }
        Err(_) => false,
    }
}

fn parse_falsy_env(key: &str) -> bool {
    match std::env::var(key) {
        Ok(v) => {
            let v = v.trim();
            v == "0"
                || v.eq_ignore_ascii_case("false")
                || v.eq_ignore_ascii_case("no")
                || v.eq_ignore_ascii_case("off")
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searxng_engine_csv_accepts_baseline() {
        let s = normalize_searxng_engines_csv("google,bing,ddg").expect("baseline engines");
        assert_eq!(s, "google,bing,ddg");
        assert!(normalize_searxng_engines_csv("bad;injection").is_none());
    }

    #[test]
    fn searxng_language_tag_accepts_en_us() {
        assert_eq!(
            normalize_searxng_language_tag("en-US").as_deref(),
            Some("en-US")
        );
        assert!(normalize_searxng_language_tag("en_US").is_none());
    }
}
