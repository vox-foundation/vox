//! Unified search, served by the daemon (`GET /api/v2/search/query`) so it runs against a
//! local or a remote Vox server. Opening a hit is host-side: see `commands::locator`.

use std::sync::Arc;

use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::knowledge::SearchResponseDto;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query};

/// Comma-joins a list query parameter; `Some(vec![])` stays an explicit empty value.
fn join_list(v: Option<Vec<String>>) -> Option<String> {
    v.map(|items| items.join(","))
}

pub(crate) async fn search_query_impl<B: BackendCalls>(
    b: &B,
    query: String,
    scope: Option<Vec<String>>,
    kinds: Option<Vec<String>>,
    path_glob: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<SearchResponseDto, String> {
    let path = path_with_query(
        "/search/query",
        &[
            ("q", Some(query)),
            ("scope", join_list(scope)),
            ("kinds", join_list(kinds)),
            ("path_glob", path_glob),
            ("limit", limit.map(|l| l.to_string())),
            ("offset", offset.map(|o| o.to_string())),
        ],
    );
    api_data(b.api_v2_get(&path).await?)
}

/// Perform a typed multi-corpus search and return structured hits with facets + pagination.
///
/// * `query`     — free-text search query (required, must be non-empty).
/// * `scope`     — optional corpus allow-list: `"memory"`, `"knowledge"`, `"chunk"`, `"repo"`,
///   `"web"`, `"chats"`. When `None` or empty, all corpora from the heuristic plan are used.
/// * `kinds`     — optional hit-kind filter (e.g. `["doc","code"]`).
/// * `path_glob` — optional glob pattern applied to `hit.path`.
/// * `limit`     — page size; defaults to 30.
/// * `offset`    — page start; defaults to 0.
#[tauri::command]
pub async fn vox_search_query(
    backend: State<'_, Arc<BackendState>>,
    query: String,
    scope: Option<Vec<String>>,
    kinds: Option<Vec<String>>,
    path_glob: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<SearchResponseDto, String> {
    search_query_impl(
        &backend.current().await,
        query,
        scope,
        kinds,
        path_glob,
        limit,
        offset,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;
    use serde_json::json;

    fn empty_response() -> serde_json::Value {
        json!({ "v": 1, "data": {
            "hits": [], "facets_by_source": [], "facets_by_kind": [], "total": 0,
            "next_cursor": null, "corpora": ["memory"], "repo_truncated": false
        }})
    }

    #[tokio::test]
    async fn search_query_hits_the_daemon_route_with_comma_lists() {
        let path = "/search/query?q=find%20me&scope=memory%2Cchats&kinds=doc&limit=5";
        let b = FakeBackend::new().on_get(path, Ok(empty_response()));
        let r = search_query_impl(
            &b,
            "find me".into(),
            Some(vec!["memory".into(), "chats".into()]),
            Some(vec!["doc".into()]),
            None,
            Some(5),
            None,
        )
        .await
        .unwrap();
        assert_eq!(r.corpora, vec!["memory"]);
        assert_eq!(b.calls()[0].0, format!("get:{path}"));
    }

    #[tokio::test]
    async fn search_query_surfaces_server_errors() {
        let b = FakeBackend::new().on_get(
            "/search/query?q=x",
            Ok(json!({ "v": 1, "error": { "code": "search_failed", "message": "boom" } })),
        );
        let e = search_query_impl(&b, "x".into(), None, None, None, None, None)
            .await
            .unwrap_err();
        assert!(e.contains("boom"), "{e}");
    }

    #[test]
    fn search_module_no_longer_touches_the_db_directly() {
        let src = include_str!("search.rs");
        let needle = ["vox_", "db::"].concat();
        assert!(!src.contains(&needle));
        assert!(!src.contains(&["Gui", "DbPool"].concat()));
    }
}
