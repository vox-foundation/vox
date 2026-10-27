//! Playwright-style actionability: before acting on an element, scroll it into
//! view and wait until it is attached, visible, stable, enabled and (for clicks)
//! the topmost element at its center point.

use std::time::{Duration, Instant};

use chromiumoxide_cdp::cdp::browser_protocol::dom::{BackendNodeId, ResolveNodeParams};
use chromiumoxide_cdp::cdp::js_protocol::runtime::{CallArgument, CallFunctionOnParams};
use serde::Deserialize;

use crate::engine::BrowserEngine;

pub(crate) const DEFAULT_ACTION_TIMEOUT_MS: u64 = 5_000;
pub(crate) const MAX_ACTION_TIMEOUT_MS: u64 = 60_000;
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Runs in the page with `this` bound to the target node. Two animation frames
/// separate the rect samples so in-flight transitions read as unstable.
const CHECK_JS: &str = r##"async function () {
  const el = this.nodeType === 1 ? this : this.parentElement;
  if (!el || !el.isConnected) return { state: "detached" };
  el.scrollIntoView({ block: "center", inline: "center", behavior: "instant" });
  const raf = () => new Promise((r) => requestAnimationFrame(() => r()));
  const a = el.getBoundingClientRect();
  await raf(); await raf();
  const b = el.getBoundingClientRect();
  const stable = a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;
  const style = getComputedStyle(el);
  const visible = b.width > 0 && b.height > 0 && style.visibility !== "hidden" && style.display !== "none";
  const enabled = !el.matches(":disabled") && !el.closest('[aria-disabled="true"]');
  const editable = el.isContentEditable || (["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName) && !el.readOnly);
  const x = b.x + b.width / 2, y = b.y + b.height / 2;
  let hit = document.elementFromPoint(x, y);
  while (hit && hit.shadowRoot) {
    const inner = hit.shadowRoot.elementFromPoint(x, y);
    if (!inner || inner === hit) break;
    hit = inner;
  }
  let n = hit, receives = false;
  while (n) { if (n === el) { receives = true; break; } n = n.parentNode || n.host; }
  const label = (e) => e ? e.tagName.toLowerCase() + (e.id ? "#" + e.id : "") : null;
  return { state: "attached", stable, visible, enabled, editable, receives, x, y, hit: label(hit) };
}"##;

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct ElementCheck {
    pub(crate) state: String,
    pub(crate) stable: bool,
    pub(crate) visible: bool,
    pub(crate) enabled: bool,
    pub(crate) editable: bool,
    pub(crate) receives: bool,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) hit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionKind {
    Click,
    Fill,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Actionability {
    Ready {
        x: f64,
        y: f64,
    },
    NotActionable {
        reason: &'static str,
        check: ElementCheck,
    },
}

/// The first unmet condition, or `None` when the element can be acted on.
pub(crate) fn blocking_reason(check: &ElementCheck, kind: ActionKind) -> Option<&'static str> {
    if check.state != "attached" {
        return Some("detached");
    }
    if !check.visible {
        return Some("not_visible");
    }
    if !check.stable {
        return Some("not_stable");
    }
    if !check.enabled {
        return Some("disabled");
    }
    match kind {
        ActionKind::Click if !check.receives => Some("covered"),
        ActionKind::Fill if !check.editable => Some("not_editable"),
        _ => None,
    }
}

pub(crate) fn clamp_timeout_ms(requested: Option<u64>) -> u64 {
    requested
        .unwrap_or(DEFAULT_ACTION_TIMEOUT_MS)
        .min(MAX_ACTION_TIMEOUT_MS)
}

pub(crate) fn not_actionable_response(
    ref_id: &str,
    reason: &str,
    check: &ElementCheck,
) -> serde_json::Value {
    let hint = match reason {
        "covered" => "another element is on top; check the snapshot for a dialog or overlay",
        "disabled" => "the element is disabled; complete the prerequisite steps first",
        "not_editable" => "the element does not accept text; pick an input or textarea",
        _ => "take a fresh snapshot; the element may have moved or been hidden",
    };
    serde_json::json!({
        "ok": false,
        "reason": "not_actionable",
        "detail": reason,
        "ref": ref_id,
        "hit": check.hit,
        "hint": hint,
    })
}

impl BrowserEngine {
    /// Call `function_declaration` with `this` bound to the backend DOM node and
    /// return its JSON result. `Err` when the node no longer resolves.
    pub(crate) async fn call_on_backend_node(
        &self,
        page: &chromiumoxide::Page,
        backend_dom_node_id: i64,
        function_declaration: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let resolved = page
            .execute(
                ResolveNodeParams::builder()
                    .backend_node_id(BackendNodeId::new(backend_dom_node_id))
                    .build(),
            )
            .await
            .map_err(Self::map_page_err)?;
        let object_id = resolved
            .result
            .object
            .object_id
            .clone()
            .ok_or_else(|| "resolved node has no object id".to_string())?;
        let arguments = args
            .into_iter()
            .map(|value| CallArgument::builder().value(value).build())
            .collect::<Vec<_>>();
        let params = CallFunctionOnParams::builder()
            .function_declaration(function_declaration)
            .object_id(object_id)
            .arguments(arguments)
            .await_promise(true)
            .return_by_value(true)
            .build()
            .map_err(|e| e.to_string())?;
        let out = page.execute(params).await.map_err(Self::map_page_err)?;
        if let Some(details) = &out.result.exception_details {
            return Err(format!("page function threw: {}", details.text));
        }
        Ok(out.result.result.value.clone().unwrap_or_default())
    }

    async fn check_element(
        &self,
        page: &chromiumoxide::Page,
        backend_dom_node_id: i64,
    ) -> Result<ElementCheck, String> {
        let value = self
            .call_on_backend_node(page, backend_dom_node_id, CHECK_JS, Vec::new())
            .await?;
        serde_json::from_value(value).map_err(|e| format!("actionability result: {e}"))
    }

    /// Poll until the element is actionable or `timeout_ms` elapses. `Err` means
    /// the node could not be resolved at all (stale ref).
    pub(crate) async fn wait_actionable(
        &self,
        page: &chromiumoxide::Page,
        backend_dom_node_id: i64,
        kind: ActionKind,
        timeout_ms: u64,
    ) -> Result<Actionability, String> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            let check = self.check_element(page, backend_dom_node_id).await?;
            let reason = blocking_reason(&check, kind);
            match reason {
                None => {
                    return Ok(Actionability::Ready {
                        x: check.x,
                        y: check.y,
                    });
                }
                Some("detached") => return Err("detached".to_string()),
                Some(reason) if Instant::now() >= deadline => {
                    return Ok(Actionability::NotActionable { reason, check });
                }
                Some(_) => tokio::time::sleep(POLL_INTERVAL).await,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> ElementCheck {
        ElementCheck {
            state: "attached".into(),
            stable: true,
            visible: true,
            enabled: true,
            editable: true,
            receives: true,
            x: 10.0,
            y: 20.0,
            hit: Some("button".into()),
        }
    }

    #[test]
    fn ready_element_has_no_blocking_reason() {
        assert_eq!(blocking_reason(&ready(), ActionKind::Click), None);
        assert_eq!(blocking_reason(&ready(), ActionKind::Fill), None);
    }

    #[test]
    fn conditions_are_reported_in_priority_order() {
        let mut c = ready();
        c.receives = false;
        assert_eq!(blocking_reason(&c, ActionKind::Click), Some("covered"));
        c.enabled = false;
        assert_eq!(blocking_reason(&c, ActionKind::Click), Some("disabled"));
        c.stable = false;
        assert_eq!(blocking_reason(&c, ActionKind::Click), Some("not_stable"));
        c.visible = false;
        assert_eq!(blocking_reason(&c, ActionKind::Click), Some("not_visible"));
        c.state = "detached".into();
        assert_eq!(blocking_reason(&c, ActionKind::Click), Some("detached"));
    }

    #[test]
    fn fill_requires_editable_but_not_hit_target() {
        let mut c = ready();
        c.receives = false;
        assert_eq!(blocking_reason(&c, ActionKind::Fill), None);
        c.editable = false;
        assert_eq!(blocking_reason(&c, ActionKind::Fill), Some("not_editable"));
        assert_eq!(blocking_reason(&c, ActionKind::Click), Some("covered"));
    }

    #[test]
    fn timeout_defaults_and_is_capped() {
        assert_eq!(clamp_timeout_ms(None), DEFAULT_ACTION_TIMEOUT_MS);
        assert_eq!(clamp_timeout_ms(Some(250)), 250);
        assert_eq!(clamp_timeout_ms(Some(u64::MAX)), MAX_ACTION_TIMEOUT_MS);
    }

    #[test]
    fn check_result_parses_from_page_json() {
        let json = serde_json::json!({
            "state": "attached", "stable": true, "visible": true, "enabled": true,
            "editable": false, "receives": false, "x": 1.5, "y": 2.5, "hit": "div#overlay"
        });
        let check: ElementCheck = serde_json::from_value(json).unwrap();
        assert_eq!(blocking_reason(&check, ActionKind::Click), Some("covered"));
        let out = not_actionable_response("e3", "covered", &check);
        assert_eq!(out["hit"], "div#overlay");
        assert_eq!(out["reason"], "not_actionable");
    }

    #[test]
    fn detached_payload_parses_with_defaults() {
        let check: ElementCheck =
            serde_json::from_value(serde_json::json!({ "state": "detached" })).unwrap();
        assert_eq!(blocking_reason(&check, ActionKind::Fill), Some("detached"));
    }
}
