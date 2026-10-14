//! Mnemosyne (memory) surface for the Vox GUI, served by the daemon so it works against a
//! local or a remote Vox server: corpus counts from `GET /api/v2/knowledge/corpus-counts`,
//! recall from `GET /api/v2/knowledge/recall`, reindex via the `vox_memory_reindex` op (the
//! daemon owns `MEMORY.md` and the memory database). Shard tiles and the recent-recall list
//! are presentation state and stay GUI-side.

use serde::Serialize;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tauri::State;
use vox_orchestrator_mcp::gui_ops::dto::knowledge::CorpusCountsDto;
pub use vox_orchestrator_mcp::gui_ops::dto::knowledge::UiHitResult;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query, tool_data};

const REINDEX_TOOL: &str = "vox_memory_reindex";
const RECENT_RECALLS_CAP: usize = 5;

static RECENT_RECALLS: Mutex<VecDeque<RecentRecallPayload>> = Mutex::new(VecDeque::new());

#[derive(Serialize)]
pub struct MemoryStatusPayload {
    pub corpus_counts: HashMap<String, u32>,
    pub shards: Vec<ShardPayload>,
    pub recent_recalls: Vec<RecentRecallPayload>,
    /// Embedding vector dimension when known (from workspace config / index).
    pub embedding_dim: Option<u32>,
}

#[derive(Serialize)]
pub struct ShardPayload {
    pub id: String,
    pub depth: u32,
    pub entries: u32,
    pub hot: bool,
    pub dirty: bool,
    pub spark: Vec<f32>,
}

#[derive(Serialize, Clone)]
pub struct RecentRecallPayload {
    pub q: String,
    pub n: usize,
    pub when: String,
}

fn shard(id: &str, depth: u32, entries: u32, hot: bool, dirty: bool) -> ShardPayload {
    ShardPayload {
        id: id.to_string(),
        depth,
        entries,
        hot,
        dirty,
        spark: Vec::new(),
    }
}

fn status_from_counts(c: CorpusCountsDto, recent: Vec<RecentRecallPayload>) -> MemoryStatusPayload {
    let corpus_counts = HashMap::from([
        ("proj".to_string(), c.proj),
        ("docs".to_string(), c.docs),
        ("chats".to_string(), c.chats),
        ("rules".to_string(), c.rules),
        ("web".to_string(), c.web),
    ]);
    let shards = vec![
        shard("K-01", 2, c.docs, c.recent_memories > 0, false),
        shard("M-01", 1, c.chats, true, true),
        shard("W-01", 3, c.proj, false, false),
        shard("C-01", 2, c.rules, false, false),
    ];
    MemoryStatusPayload {
        corpus_counts,
        shards,
        recent_recalls: recent,
        embedding_dim: Some(1024),
    }
}

fn record_recall(q: String, n: usize) {
    let mut recent = RECENT_RECALLS.lock().unwrap_or_else(|p| p.into_inner());
    recent.push_front(RecentRecallPayload {
        q,
        n,
        when: "just now".to_string(),
    });
    recent.truncate(RECENT_RECALLS_CAP);
}

fn recent_recalls() -> Vec<RecentRecallPayload> {
    let recent = RECENT_RECALLS.lock().unwrap_or_else(|p| p.into_inner());
    recent.iter().cloned().collect()
}

pub(crate) async fn memory_status_impl<B: BackendCalls>(
    b: &B,
) -> Result<MemoryStatusPayload, String> {
    let counts: CorpusCountsDto = api_data(b.api_v2_get("/knowledge/corpus-counts").await?)?;
    Ok(status_from_counts(counts, recent_recalls()))
}

pub(crate) async fn recall_impl<B: BackendCalls>(
    b: &B,
    query: &str,
    limit: usize,
) -> Result<Vec<UiHitResult>, String> {
    let path = path_with_query(
        "/knowledge/recall",
        &[
            ("q", Some(query.to_string())),
            ("limit", Some(limit.to_string())),
        ],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn reindex_impl<B: BackendCalls>(b: &B) -> Result<(), String> {
    let _: String = tool_data(REINDEX_TOOL, b.call_tool(REINDEX_TOOL, json!({})).await?)?;
    Ok(())
}

#[tauri::command]
pub async fn get_memory_status(
    backend: State<'_, Arc<BackendState>>,
) -> Result<MemoryStatusPayload, String> {
    memory_status_impl(&backend.current().await).await
}

#[tauri::command]
pub async fn mnemosyne_recall(
    backend: State<'_, Arc<BackendState>>,
    query: String,
    _scope: String,
    limit: usize,
) -> Result<Vec<UiHitResult>, String> {
    let hits = recall_impl(&backend.current().await, &query, limit).await?;
    record_recall(query, hits.len());
    Ok(hits)
}

#[tauri::command]
pub async fn mnemosyne_reindex(backend: State<'_, Arc<BackendState>>) -> Result<(), String> {
    reindex_impl(&backend.current().await).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    #[tokio::test]
    async fn status_maps_daemon_counts_into_corpus_tiles_and_shards() {
        let b = FakeBackend::new().on_get(
            "/knowledge/corpus-counts",
            Ok(json!({ "v": 1, "data": {
                "proj": 7, "docs": 3, "chats": 2, "rules": 1, "web": 9, "recent_memories": 4
            }})),
        );
        let s = memory_status_impl(&b).await.unwrap();
        assert_eq!(s.corpus_counts["docs"], 3);
        assert_eq!(s.corpus_counts["web"], 9);
        assert_eq!(s.corpus_counts.len(), 5);
        let k = &s.shards[0];
        assert_eq!((k.id.as_str(), k.entries, k.hot), ("K-01", 3, true));
        assert_eq!(s.shards[2].entries, 7);
    }

    #[test]
    fn k01_is_cold_without_recent_memories() {
        let s = status_from_counts(CorpusCountsDto::default(), Vec::new());
        assert!(!s.shards[0].hot);
        assert!(s.shards[1].hot && s.shards[1].dirty);
    }

    #[tokio::test]
    async fn recall_and_reindex_go_through_the_daemon() {
        let b = FakeBackend::new()
            .on_get(
                "/knowledge/recall?q=what%20is&limit=3",
                Ok(json!({ "v": 1, "data": [
                    { "src": "MEMORY.md", "line": 0, "score": 0.5, "kind": "chat", "text": "t" }
                ]})),
            )
            .on_tool(
                REINDEX_TOOL,
                Ok(json!({ "success": true, "data": "reindexed" })),
            );
        let hits = recall_impl(&b, "what is", 3).await.unwrap();
        assert_eq!(hits.len(), 1);
        reindex_impl(&b).await.unwrap();
        let keys: Vec<String> = b.calls().into_iter().map(|(k, _)| k).collect();
        assert_eq!(
            keys,
            vec![
                "get:/knowledge/recall?q=what%20is&limit=3".to_string(),
                format!("tool:{REINDEX_TOOL}")
            ]
        );
    }

    #[test]
    fn recent_recalls_are_capped() {
        for i in 0..(RECENT_RECALLS_CAP + 3) {
            record_recall(format!("q{i}"), i);
        }
        let r = recent_recalls();
        assert_eq!(r.len(), RECENT_RECALLS_CAP);
        assert_eq!(r[0].q, format!("q{}", RECENT_RECALLS_CAP + 2));
    }

    #[test]
    fn memory_module_no_longer_touches_the_db_directly() {
        let src = include_str!("memory.rs");
        assert!(!src.contains(&["vox_", "db::"].concat()));
        assert!(!src.contains(&["turso", "::"].concat()));
    }
}
