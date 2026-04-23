use crate::context::SearchRuntimeContext;
use crate::policy::SearchPolicy;

#[cfg(feature = "qdrant-vector")]
use crate::vector_qdrant::QdrantSemanticClient;

/// Scans the project for symbol proximity (split-brain detection) based on the query.
pub async fn scan_symbol_proximity(
    _ctx: &SearchRuntimeContext,
    query: &str,
    policy: &SearchPolicy,
    query_vector: Option<&[f32]>,
) -> Vec<String> {
    let mut hits = Vec::new();
    let mut qdrant_results = Vec::new();

    #[cfg(feature = "qdrant-vector")]
    {
        if let Some(url) = policy.qdrant_url.as_deref().filter(|u| !u.is_empty()) {
            if let Some(qv) = query_vector.filter(|v| !v.is_empty()) {
                let client = QdrantSemanticClient::new(url, policy.qdrant_collection.as_str());
                let trace = _ctx
                    .trace_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty());

                match client
                    .search_vectors(qv, 5, policy.qdrant_vector_name.as_deref(), trace)
                    .await
                {
                    Ok(results) => qdrant_results = results,
                    Err(e) => tracing::warn!(
                        "Qdrant vector search failed during symbol proximity scan. Falling back to text stub. Error: {:?}",
                        e
                    ),
                }
            }
        }
    }

    // 1. Load retired surfaces schema (fallback if not found)
    let workspace_root = std::env::var("VOX_REPO_ROOT").unwrap_or_else(|_| ".".into());
    let schema_path = std::path::Path::new(&workspace_root)
        .join("contracts")
        .join("proximity")
        .join("retired-surfaces.v1.json");

    let surfaces_json = match std::fs::read_to_string(&schema_path) {
        Ok(json) => json,
        Err(_) => return hits, // Return early if schema can't be loaded
    };

    let schema: serde_json::Value = match serde_json::from_str(&surfaces_json) {
        Ok(v) => v,
        Err(_) => return hits,
    };

    let surfaces = schema
        .get("surfaces")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // 2. Iterate through canonical vs retired pairs
    for surface in surfaces {
        let retired = surface
            .get("retired_symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let canonical = surface
            .get("canonical_replacement")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if retired.is_empty() || canonical.is_empty() {
            continue;
        }

        let query_lower = query.to_lowercase();
        let retired_lower = retired.to_lowercase();

        // Exact substring match gives high confidence
        let mut max_ratio =
            if query_lower.contains(&retired_lower) || retired_lower.contains(&query_lower) {
                1.0
            } else {
                // Levenshtein approximation for partial matches
                // let diff = similar::TextDiff::from_chars(&query_lower, &retired_lower);
                // diff.ratio() as f64
                0.5 // Stub
            };

        // Fuse with Qdrant score if available
        for (id, sc, _) in &qdrant_results {
            if id.to_lowercase().contains(&retired_lower)
                || retired_lower.contains(&id.to_lowercase())
            {
                max_ratio = (max_ratio + *sc as f64) / 2.0;
            }
        }

        if max_ratio > 0.65 {
            hits.push(format!(
                "[proximity:{} score:{:.3}] `{}` shares semantic overlap with retired symbol `{}`. Canonical replacement is `{}`.", 
                retired, max_ratio, query, retired, canonical
            ));
        }
    }

    hits
}
