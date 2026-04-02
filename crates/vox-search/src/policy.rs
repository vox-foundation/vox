//! Versioned search policy loaded from defaults with `VOX_SEARCH_*` environment overrides.

use serde::{Deserialize, Serialize};

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
}

impl Default for SearchPolicy {
    fn default() -> Self {
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
        p
    }

    /// Effective fusion weight clamped to `[0, 1]`.
    #[must_use]
    pub fn clamped_memory_vector_weight(&self) -> f32 {
        self.memory_vector_fusion_weight.clamp(0.0, 1.0)
    }
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
