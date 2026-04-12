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
        use vox_clavis::{SecretId, resolve_secret};

        let qdrant_url = resolve_secret(SecretId::VoxSearchQdrantUrl)
            .expose()
            .trim()
            .to_string();
        let qdrant_url = if qdrant_url.is_empty() {
            None
        } else {
            Some(qdrant_url)
        };

        let qdrant_collection = resolve_secret(SecretId::VoxSearchQdrantCollection).expose();
        let qdrant_collection = if qdrant_collection.is_empty() {
            "vox_docs".to_string()
        } else {
            qdrant_collection
        };

        let qdrant_vector_name = resolve_secret(SecretId::VoxSearchQdrantVectorName)
            .expose()
            .trim()
            .to_string();
        let qdrant_vector_name = if qdrant_vector_name.is_empty() {
            None
        } else {
            Some(qdrant_vector_name)
        };

        let tantivy_index_root = resolve_secret(SecretId::VoxSearchTantivyRoot)
            .expose()
            .trim()
            .to_string();
        let tantivy_index_root = if tantivy_index_root.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(tantivy_index_root))
        };

        let search_pref_rrf = resolve_secret(SecretId::VoxSearchPreferRrf).expose();
        let prefer_rrf_merge = clavis_parse_bool(&search_pref_rrf, false);

        let tav_en = resolve_secret(SecretId::VoxSearchTavilyEnabled).expose();
        let tavily_enabled = clavis_parse_bool(&tav_en, false);

        let tav_depth = resolve_secret(SecretId::VoxSearchTavilyDepth).expose();
        let tavily_search_depth = if tav_depth.is_empty() {
            "basic".to_string()
        } else {
            tav_depth
        };

        let tav_max = resolve_secret(SecretId::VoxSearchTavilyMaxResults).expose();
        let tavily_max_results = tav_max.parse().unwrap_or(5);

        let tav_empty = resolve_secret(SecretId::VoxSearchTavilyOnEmpty).expose();
        let tavily_fire_on_empty = clavis_parse_bool(&tav_empty, true);

        let tav_weak = resolve_secret(SecretId::VoxSearchTavilyOnWeak).expose();
        let tavily_fire_on_weak = clavis_parse_bool(&tav_weak, false);

        let tav_budget = resolve_secret(SecretId::VoxSearchTavilyBudget).expose();
        let tavily_credit_budget_per_session = tav_budget.parse().unwrap_or(50);

        let sx_url = resolve_secret(SecretId::VoxSearchSearxngUrl).expose();
        let searxng_url = if sx_url.is_empty() {
            None
        } else {
            Some(sx_url)
        };

        let sx_max = resolve_secret(SecretId::VoxSearchSearxngMaxResults).expose();
        let searxng_max_results = sx_max.parse().unwrap_or(5);

        let sx_scrape = resolve_secret(SecretId::VoxSearchSearxngMaxScrape).expose();
        let searxng_max_urls_to_scrape = sx_scrape.parse().unwrap_or(3);

        let ddg_no = resolve_secret(SecretId::VoxSearchDdgFallbackDisabled).expose();
        let duckduckgo_fallback_enabled = !clavis_parse_bool(&ddg_no, false);

        let sc_to = resolve_secret(SecretId::VoxSearchScraperTimeout).expose();
        let scraper_timeout_ms = sc_to.parse().unwrap_or(5000);

        let rob_res = resolve_secret(SecretId::VoxSearchScraperRobotsRespect).expose();
        let scraper_robots_txt_respect = clavis_parse_bool(&rob_res, false);

        let min_dens = resolve_secret(SecretId::VoxSearchScraperMinDensity).expose();
        let scraper_min_text_density = min_dens.parse().unwrap_or(0.15);

        let max_hops = resolve_secret(SecretId::VoxSearchMaxHops).expose();
        let web_search_max_hops = max_hops.parse().unwrap_or(3);

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
            qdrant_url,
            qdrant_collection,
            qdrant_vector_name,
            tantivy_index_root,
            prefer_rrf_merge,
            tavily_enabled,
            tavily_search_depth,
            tavily_max_results,
            tavily_fire_on_empty,
            tavily_fire_on_weak,
            tavily_credit_budget_per_session,
            searxng_url,
            searxng_max_results,
            searxng_max_urls_to_scrape,
            searxng_engines: searxng_embedded.engines.clone(),
            searxng_language: searxng_embedded.language.clone(),
            duckduckgo_fallback_enabled,
            scraper_timeout_ms,
            scraper_robots_txt_respect,
            scraper_min_text_density,
            web_search_max_hops,
        }
    }
}

impl SearchPolicy {
    /// Environment overrides merged onto [`Default::default`].
    #[must_use]
    pub fn from_env() -> Self {
        let mut p = Self::default();
        use vox_clavis::{SecretId, resolve_secret};

        let bm25_k1 = resolve_secret(SecretId::VoxSearchBm25K1).expose();
        if !bm25_k1.is_empty() {
            // Note: bm25_k1 is not in SearchPolicy struct yet, but we resolve it for completeness if needed.
        }

        let bm25_b = resolve_secret(SecretId::VoxSearchBm25B).expose();
        if !bm25_b.is_empty() {
            // Note: bm25_b is not in SearchPolicy struct yet.
        }

        let rrf_k = resolve_secret(SecretId::VoxSearchRrfK).expose();
        if !rrf_k.is_empty() {
            // Note: rrf_k is not in SearchPolicy struct yet.
        }

        let sx_engines = resolve_secret(SecretId::VoxSearchSearxngEngines).expose();
        if !sx_engines.is_empty() {
            if let Some(norm) = normalize_searxng_engines_csv(&sx_engines) {
                p.searxng_engines = norm;
            }
        }

        let sx_lang = resolve_secret(SecretId::VoxSearchSearxngLanguage).expose();
        if !sx_lang.is_empty() {
            if let Some(norm) = normalize_searxng_language_tag(&sx_lang) {
                p.searxng_language = norm;
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

fn clavis_parse_bool(val: &str, default_val: bool) -> bool {
    let v = val.trim();
    if v.is_empty() {
        return default_val;
    }
    v == "1"
        || v.eq_ignore_ascii_case("true")
        || v.eq_ignore_ascii_case("yes")
        || v.eq_ignore_ascii_case("on")
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
