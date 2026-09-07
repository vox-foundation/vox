use std::collections::BTreeMap;

use chromiumoxide_cdp::cdp::browser_protocol::dom::{BackendNodeId, GetBoxModelParams};

use crate::engine::BrowserEngine;
use crate::snapshot::{
    AxBox, AxRef, AxSnapshot, SnapshotOptions, compact_ax_snapshot, wrap_snapshot_tree,
};

pub(crate) fn stale_ref_error(ref_id: &str) -> String {
    format!("stale_ref: {ref_id}")
}

fn lookup_ref<'a>(map: &'a BTreeMap<String, AxRef>, ref_id: &str) -> Result<&'a AxRef, String> {
    map.get(ref_id).ok_or_else(|| stale_ref_error(ref_id))
}

impl BrowserEngine {
    pub async fn snapshot(
        &self,
        page_id: &str,
        opts: SnapshotOptions,
    ) -> Result<AxSnapshot, String> {
        let nodes = self.ax_tree(page_id).await?;
        let nodes = nodes
            .as_array()
            .ok_or_else(|| "AXTree response was not an array".to_string())?;
        let mut compact = compact_ax_snapshot(nodes, &opts);

        if opts.include_boxes {
            let page = self.page_ref(page_id).await?;
            for ax_ref in compact.refs.values_mut() {
                ax_ref.box_css = box_for_backend_node(&page, ax_ref.backend_dom_node_id)
                    .await
                    .ok();
            }
        }

        let info = self.page_info(page_id).await?;
        let url = info
            .get("url")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let title = info
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();

        {
            let mut guard = self.host.lock().await;
            let host = guard.host_for_page_mut(page_id)?;
            host.ref_maps
                .insert(page_id.to_string(), compact.refs.clone());
        }

        Ok(AxSnapshot {
            page_id: page_id.to_string(),
            url,
            title,
            tree: wrap_snapshot_tree(&compact.tree),
            refs: compact.refs,
            truncated: compact.truncated,
        })
    }

    pub async fn click_ref(
        &self,
        page_id: &str,
        ref_id: &str,
        respect_sensitive: bool,
    ) -> Result<serde_json::Value, String> {
        let ax_ref = self.last_ref(page_id, ref_id).await?;
        if respect_sensitive && ax_ref.sensitive {
            return Ok(sensitive_ref_response(ref_id));
        }

        let page = self.page_ref(page_id).await?;
        let bounds = box_for_backend_node(&page, ax_ref.backend_dom_node_id)
            .await
            .map_err(|_| stale_ref_error(ref_id))?;
        self.click_xy(
            page_id,
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )
        .await?;
        Ok(serde_json::json!({ "ok": true, "ref": ref_id }))
    }

    pub async fn fill_ref(
        &self,
        page_id: &str,
        ref_id: &str,
        value: &str,
        respect_sensitive: bool,
    ) -> Result<serde_json::Value, String> {
        let ax_ref = self.last_ref(page_id, ref_id).await?;
        if respect_sensitive && ax_ref.sensitive {
            return Ok(sensitive_ref_response(ref_id));
        }

        let page = self.page_ref(page_id).await?;
        let bounds = box_for_backend_node(&page, ax_ref.backend_dom_node_id)
            .await
            .map_err(|_| stale_ref_error(ref_id))?;
        self.click_xy(
            page_id,
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )
        .await?;
        self.type_text(page_id, value).await?;
        Ok(serde_json::json!({ "ok": true, "ref": ref_id }))
    }

    async fn last_ref(&self, page_id: &str, ref_id: &str) -> Result<AxRef, String> {
        let guard = self.host.lock().await;
        let map = guard
            .host_for_page(page_id)
            .ok()
            .and_then(|host| host.ref_maps.get(page_id))
            .ok_or_else(|| stale_ref_error(ref_id))?;
        lookup_ref(map, ref_id).cloned()
    }

    /// Snapshot refs survive SPA mutations until the next snapshot. Navigation
    /// clears them so callers recover from `stale_ref` by taking a fresh snapshot.
    pub(crate) async fn clear_ref_map(&self, page_id: &str) {
        let mut guard = self.host.lock().await;
        if let Ok(host) = guard.host_for_page_mut(page_id) {
            host.ref_maps.remove(page_id);
        }
    }
}

async fn box_for_backend_node(
    page: &chromiumoxide::Page,
    backend_dom_node_id: i64,
) -> Result<AxBox, String> {
    let response = page
        .execute(
            GetBoxModelParams::builder()
                .backend_node_id(BackendNodeId::new(backend_dom_node_id))
                .build(),
        )
        .await
        .map_err(BrowserEngine::map_page_err)?;
    let points = response.result.model.border.inner();
    if points.len() < 8 {
        return Err("DOM.getBoxModel returned an invalid border quad".to_string());
    }
    let xs = [points[0], points[2], points[4], points[6]];
    let ys = [points[1], points[3], points[5], points[7]];
    let min_x = xs.into_iter().fold(f64::INFINITY, f64::min);
    let max_x = xs.into_iter().fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.into_iter().fold(f64::INFINITY, f64::min);
    let max_y = ys.into_iter().fold(f64::NEG_INFINITY, f64::max);
    Ok(AxBox {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    })
}

fn sensitive_ref_response(ref_id: &str) -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "needs_human": true,
        "reason": "password_field",
        "ref": ref_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_ref_empty_map_is_stale() {
        let map: BTreeMap<String, AxRef> = BTreeMap::new();
        assert_eq!(lookup_ref(&map, "e9").unwrap_err(), stale_ref_error("e9"));
    }

    #[test]
    fn stale_ref_error_is_only_constructor() {
        // compile-time: all miss paths must call stale_ref_error; this test
        // locks the string so MCP can match it.
        assert_eq!(stale_ref_error("e9"), "stale_ref: e9");
    }

    #[test]
    fn only_box_model_failures_map_to_stale_ref() {
        let source = include_str!("ref_actions.rs");
        let stale_mapping = [".map_err(|_| stale_ref_error(", "ref_id))?"].concat();
        assert_eq!(
            source.matches(&stale_mapping).count(),
            2,
            "click/type transport errors must remain intact"
        );
    }

    #[tokio::test]
    #[ignore = "slow; requires local Chrome/Chromium binary"]
    async fn snapshot_click_ref_on_data_html() {
        let engine = BrowserEngine::new();
        let html = "data:text/html,<html><body><button id='go'>Go</button></body></html>";
        let page_id = engine.open(html, true).await.unwrap();
        let snap = engine
            .snapshot(&page_id, SnapshotOptions::default())
            .await
            .unwrap();
        assert!(snap.refs.values().any(|r| r.name == "Go"));
        let go = snap
            .refs
            .values()
            .find(|r| r.name == "Go")
            .unwrap()
            .ref_id
            .clone();
        let out = engine.click_ref(&page_id, &go, true).await.unwrap();
        assert_eq!(out["ok"], true);
        engine.close(&page_id).await.unwrap();
    }
}
