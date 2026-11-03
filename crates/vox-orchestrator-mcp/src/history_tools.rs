//! `vox_search_history`: first-parent repo history for agents (same facade as `vox graph history`).
use crate::params::ToolResult;
use crate::server_state::ServerState;
use std::time::Duration;
use vox_graph_reader::history::api::{self, Answer, HistoryQuery, Paths};

/// Self-imposed budget, well under the 120 s dispatch timeout. An unfinished backfill stops,
/// releases the lock, keeps its rows, and reports `complete: false` so the agent calls again.
const BUDGET: Duration = Duration::from_secs(60);

/// Rows plus completeness in `data`; catch-up outcome and rendered text in `meta`.
pub(crate) fn envelope(a: &Answer) -> String {
    ToolResult::ok_with_meta(
        a.value.clone(),
        serde_json::json!({ "catch_up": format!("{:?}", a.catch_up), "text": a.text }),
    )
    .to_json()
}

pub async fn history(state: &ServerState, q: HistoryQuery) -> String {
    let root = state.repository.root.clone();
    let res = tokio::task::spawn_blocking(move || {
        let code_graph = vox_config::graphify::load_graphify_corpora(&root)
            .ok()
            .and_then(|r| r.corpora.into_iter().find(|c| c.id == "repo-code-graph"))
            .map(|c| root.join(c.graph_path));
        let paths = Paths {
            repo_root: &root,
            code_graph: code_graph.as_deref(),
        };
        api::answer(
            &paths,
            &q,
            chrono::Utc::now().timestamp(),
            Some(BUDGET),
            &mut |_, _| {},
        )
    })
    .await;
    match res {
        Ok(Ok(a)) => envelope(&a),
        Ok(Err(e)) => ToolResult::<serde_json::Value>::err_with_remediation(
            format!("history: {e}"),
            "Run from a git checkout with `origin/main` or `main`; see `vox graph history focus`.",
        )
        .to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err_with_remediation(
            format!("history task panicked: {e}"),
            "Report this with the tool arguments.",
        )
        .to_json(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_graph_reader::history::ingest::CatchUp;

    #[test]
    fn envelope_carries_completeness_in_data() {
        let a = Answer {
            value: serde_json::json!({"complete": false, "behind": 3, "rows": []}),
            text: "partial: 3 commits not yet ingested; call again.".into(),
            catch_up: CatchUp::OverBudget {
                ingested: 1,
                behind: 3,
            },
            complete: false,
            behind: 3,
        };
        let v: serde_json::Value = serde_json::from_str(&envelope(&a)).unwrap();
        assert_eq!(v["data"]["complete"], false);
        assert_eq!(v["data"]["behind"], 3);
        assert!(v["meta"]["text"].as_str().unwrap().starts_with("partial:"));
    }
}
