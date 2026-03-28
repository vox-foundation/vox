use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vox_orchestrator::MemorySearchEngine;
use vox_orchestrator::services::embeddings::EmbeddingService;
use vox_runtime::llm::LlmConfig;

use super::config::memory_config_for_state;
use crate::server::ServerState;

/// Why retrieval is being invoked for this turn/tool path.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalTriggerMode {
    /// Silent preamble enrichment for chat turns.
    AutoChatPreamble,
    /// Explicit user call through a retrieval/search tool.
    ExplicitToolQuery,
    /// Additional retrieval pass used for contradiction/risk verification.
    VerificationPass,
}

/// Structured retrieval metadata shared between MCP surfaces and Socrates telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetrievalEvidenceEnvelope {
    /// Trigger mode that initiated this retrieval pass.
    pub trigger: RetrievalTriggerMode,
    /// Effective execution tier: `hybrid`, `bm25`, `lexical_fallback`, or `none`.
    pub retrieval_tier: String,
    /// Number of memory hits returned by the selected retrieval tier.
    pub memory_hit_count: usize,
    /// Number of knowledge graph rows returned from VoxDb.
    pub knowledge_hit_count: usize,
    /// Ingested `search_document_chunks` hits (RAG corpus).
    #[serde(default)]
    pub chunk_hit_count: usize,
    /// Whether the vector leg contributed evidence.
    pub used_vector: bool,
    /// Whether BM25/keyword ranking contributed evidence.
    pub used_bm25: bool,
    /// Whether lexical fallback (substring scan) was used.
    pub used_lexical_fallback: bool,
    /// Contradiction hints detected in merged retrieval output.
    pub contradiction_count: usize,
    /// Highest fused score in returned memory hits.
    pub top_score: Option<f64>,
    /// Observed `PRAGMA journal_mode` when a DB was available (telemetry / routing hints).
    #[serde(default)]
    pub sqlite_journal_mode: Option<String>,
    /// Whether compile options suggested FTS5 (see [`vox_db::capabilities::SqliteProbeSnapshot`]).
    #[serde(default)]
    pub sqlite_fts5_reported: Option<bool>,
    /// Whether `PRAGMA foreign_keys` reported enforcement.
    #[serde(default)]
    pub sqlite_foreign_keys_on: Option<bool>,
}

/// Internal retrieval payload used by chat preamble and memory tools.
#[derive(Debug, Clone)]
pub struct RetrievalBundle {
    pub memory_lines: Vec<String>,
    pub knowledge_lines: Vec<String>,
    pub chunk_lines: Vec<String>,
    pub evidence: RetrievalEvidenceEnvelope,
}

fn embedding_config_from_env() -> Option<LlmConfig> {
    if let Some(token) = vox_config::inference::huggingface_hub_token() {
        return Some(LlmConfig {
            provider: "hf_router".to_string(),
            model: std::env::var("VOX_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "sentence-transformers/all-MiniLM-L6-v2".to_string()),
            base_url: Some("https://router.huggingface.co/v1/embeddings".to_string()),
            api_key: Some(token),
            temperature: None,
            max_tokens: None,
            response_format: None,
            timeout_ms: None,
        });
    }
    let openai_key = vox_clavis::resolve_secret(vox_clavis::SecretId::OpenAiApiKey)
        .expose()
        .unwrap_or_default()
        .to_string();
    if !openai_key.trim().is_empty() {
        return Some(LlmConfig {
            provider: "openai".to_string(),
            model: std::env::var("VOX_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".to_string()),
            base_url: Some("https://api.openai.com/v1/embeddings".to_string()),
            api_key: Some(openai_key),
            temperature: None,
            max_tokens: None,
            response_format: None,
            timeout_ms: None,
        });
    }
    let openrouter_key = vox_clavis::resolve_secret(vox_clavis::SecretId::OpenRouterApiKey)
        .expose()
        .unwrap_or_default()
        .to_string();
    if !openrouter_key.trim().is_empty() {
        return Some(LlmConfig {
            provider: "openrouter".to_string(),
            model: std::env::var("VOX_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".to_string()),
            base_url: Some("https://openrouter.ai/api/v1/embeddings".to_string()),
            api_key: Some(openrouter_key),
            temperature: None,
            max_tokens: None,
            response_format: None,
            timeout_ms: None,
        });
    }
    None
}

fn detect_vector_or_bm25(memory_lines: &[String]) -> (bool, bool) {
    let mut used_vector = false;
    let mut used_bm25 = false;
    for line in memory_lines {
        let l = line.to_lowercase();
        if l.contains("evidence:vector") || l.contains("evidence:hybrid") {
            used_vector = true;
        }
        if l.contains("evidence:fulltext") || l.contains("evidence:hybrid") || l.contains("bm25:") {
            used_bm25 = true;
        }
    }
    (used_vector, used_bm25)
}

/// Unified retrieval trigger used by chat preamble + explicit search tools.
pub async fn run_retrieval_bundle(
    state: &ServerState,
    query: &str,
    trigger: RetrievalTriggerMode,
    limit: usize,
) -> Result<RetrievalBundle, String> {
    let sqlite_cap = match (&state.sqlite_capabilities, state.db.as_ref()) {
        (Some(s), _) => Some(s.clone()),
        (None, Some(db)) => db.sqlite_capabilities_snapshot().await.ok(),
        _ => None,
    };

    let cfg = memory_config_for_state(state);
    let mut engine = MemorySearchEngine::new();
    engine.index_dir(&cfg.log_dir);
    if !cfg.memory_md_path.starts_with(&cfg.log_dir) {
        engine.index_file(&cfg.memory_md_path);
    }

    let mut lexical_fallback_used = false;
    let memory_lines: Vec<String> = if let Some(db) = state.db.clone() {
        let engine = engine.with_db(db.clone());
        let embedder =
            embedding_config_from_env().map(|llm_cfg| EmbeddingService::new(db, llm_cfg));
        let hybrid_hits = engine.hybrid_search(query, limit, embedder.as_ref()).await;
        if hybrid_hits.is_empty() {
            lexical_fallback_used = true;
            let mgr = vox_orchestrator::MemoryManager::new(cfg.clone())
                .map_err(|e| format!("memory init failed: {e}"))?;
            mgr.search(query)
                .map_err(|e| e.to_string())?
                .into_iter()
                .take(limit)
                .map(|h| format!("[{}:{}] {}", h.source, h.line, h.content))
                .collect()
        } else {
            hybrid_hits
                .into_iter()
                .map(|h| {
                    format!(
                        "[{}] {} (score {:.3}; provenance: {}; contradiction: {})",
                        h.path,
                        h.content_snippet.replace('\n', " "),
                        h.score,
                        h.provenance.join(", "),
                        h.potential_contradiction
                    )
                })
                .collect()
        }
    } else {
        let hybrid_hits = engine.hybrid_search(query, limit, None).await;
        if hybrid_hits.is_empty() {
            lexical_fallback_used = true;
            let mgr = vox_orchestrator::MemoryManager::new(cfg.clone())
                .map_err(|e| format!("memory init failed: {e}"))?;
            mgr.search(query)
                .map_err(|e| e.to_string())?
                .into_iter()
                .take(limit)
                .map(|h| format!("[{}:{}] {}", h.source, h.line, h.content))
                .collect()
        } else {
            hybrid_hits
                .into_iter()
                .map(|h| {
                    format!(
                        "[{}] {} (score {:.3}; provenance: {}; contradiction: {})",
                        h.path,
                        h.content_snippet.replace('\n', " "),
                        h.score,
                        h.provenance.join(", "),
                        h.potential_contradiction
                    )
                })
                .collect()
        }
    };

    let knowledge_lines = if let Some(db) = state.db.as_ref() {
        db.query_knowledge_nodes(query, limit as i64)
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(id, label, snippet)| {
                let snip = snippet.replace('\n', " ");
                format!("[node:{id}] {label} — {snip}")
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let chunk_lines = if let Some(db) = state.db.as_ref() {
        db.query_search_document_chunks(query, limit as i64)
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(chunk_id, doc_id, snippet, title)| {
                let snip = snippet.replace('\n', " ");
                format!("[chunk:{chunk_id} doc:{doc_id} title:{title}] {snip}")
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let (used_vector, used_bm25) = detect_vector_or_bm25(&memory_lines);
    let contradiction_count = memory_lines
        .iter()
        .filter(|line| line.to_lowercase().contains("contradiction: true"))
        .count();
    let top_score = memory_lines.iter().find_map(|line| {
        let marker = "score ";
        let idx = line.find(marker)?;
        let tail = &line[idx + marker.len()..];
        let score_str = tail.split(';').next()?.trim();
        score_str.parse::<f64>().ok()
    });
    let retrieval_tier = if used_vector && used_bm25 {
        "hybrid"
    } else if used_bm25 {
        "bm25"
    } else if lexical_fallback_used {
        "lexical_fallback"
    } else {
        "none"
    };

    let (sqlite_journal_mode, sqlite_fts5_reported, sqlite_foreign_keys_on) =
        sqlite_cap.map_or((None, None, None), |p| {
            (
                Some(p.journal_mode.clone()),
                Some(p.fts5_reported),
                Some(p.foreign_keys_on),
            )
        });

    Ok(RetrievalBundle {
        evidence: RetrievalEvidenceEnvelope {
            trigger,
            retrieval_tier: retrieval_tier.to_string(),
            memory_hit_count: memory_lines.len(),
            knowledge_hit_count: knowledge_lines.len(),
            chunk_hit_count: chunk_lines.len(),
            used_vector,
            used_bm25,
            used_lexical_fallback: lexical_fallback_used,
            contradiction_count,
            top_score,
            sqlite_journal_mode,
            sqlite_fts5_reported,
            sqlite_foreign_keys_on,
        },
        memory_lines,
        knowledge_lines,
        chunk_lines,
    })
}
