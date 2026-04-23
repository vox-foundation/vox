//! Minimal Qdrant REST search client (`qdrant-vector` feature).
//!
//! - Request shape matches [Qdrant search points](https://api.qdrant.tech/api-reference/search/points)
//!   (`vector` as plain array or `{ "name", "vector" }` for named vectors).
//! - Optional `VOX_SEARCH_QDRANT_API_KEY` → `api-key` header (Qdrant Cloud / secured nodes).

use reqwest::Client;
use serde::Deserialize;

/// ANN search against a Qdrant collection.
#[derive(Debug, Clone)]
pub struct QdrantSemanticClient {
    client: Client,
    base_url: String,
    collection: String,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    result: Vec<ScoredPoint>,
}

#[derive(Debug, Deserialize)]
struct ScoredPoint {
    id: serde_json::Value,
    score: f64,
    payload: Option<serde_json::Value>,
}

fn truncate_note(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        format!("{}…", t.chars().take(max).collect::<String>())
    }
}

fn snippet_from_payload(payload: &serde_json::Value) -> Option<String> {
    let obj = payload.as_object()?;
    for key in [
        "text",
        "snippet",
        "body",
        "content",
        "source_uri",
        "path",
        "uri",
    ] {
        if let Some(v) = obj.get(key)
            && let Some(s) = v.as_str()
        {
            let t = s.trim();
            if !t.is_empty() {
                return Some(truncate_note(t, 160));
            }
        }
    }
    None
}

impl QdrantSemanticClient {
    /// New client (base e.g. `http://127.0.0.1:6333`, collection per policy).
    pub fn new(base_url: impl Into<String>, collection: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            collection: collection.into(),
        }
    }

    /// Returns `(point_id, score, optional payload snippet)`.
    ///
    /// `vector_name`: when `Some`, serializes `vector` as a [`NamedVector`](https://api.qdrant.tech/api-reference/search/points) object.
    /// `trace_id`: when set, sends `X-Vox-Trace-Id` for operator correlation (ignored by stock Qdrant).
    pub async fn search_vectors(
        &self,
        vector: &[f32],
        limit: usize,
        vector_name: Option<&str>,
        trace_id: Option<&str>,
    ) -> Result<Vec<(String, f32, Option<String>)>, String> {
        let url = format!(
            "{}/collections/{}/points/search",
            self.base_url, self.collection
        );
        let vector_val =
            serde_json::to_value(vector).map_err(|e| format!("qdrant request encode: {e}"))?;
        let vector_field = if let Some(name) = vector_name.map(str::trim).filter(|n| !n.is_empty())
        {
            serde_json::json!({ "name": name, "vector": vector_val })
        } else {
            vector_val
        };
        let body = serde_json::json!({
            "vector": vector_field,
            "limit": limit.max(1),
            "with_payload": true
        });

        let mut req = self.client.post(&url).json(&body);
        if let Some(key) = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxSearchQdrantApiKey)
            .expose()
            .map(str::trim)
            .filter(|k| !k.is_empty())
        {
            req = req.header("api-key", key);
        }
        if let Some(t) = trace_id.map(str::trim).filter(|s| !s.is_empty()) {
            req = req.header("X-Vox-Trace-Id", t);
        }

        let res = req.send().await.map_err(|e| e.to_string())?;
        let status = res.status();
        if !status.is_success() {
            let txt = res.text().await.unwrap_or_default();
            return Err(truncate_note(
                &format!("qdrant HTTP {}: {}", status, txt),
                512,
            ));
        }
        let parsed: SearchResponse = res.json().await.map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for p in parsed.result {
            let id = match p.id {
                serde_json::Value::String(s) => s,
                serde_json::Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };
            let snip = p.payload.as_ref().and_then(snippet_from_payload);
            out.push((id, p.score as f32, snip));
        }
        Ok(out)
    }
}
