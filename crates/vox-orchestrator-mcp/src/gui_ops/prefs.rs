//! GUI preference ops for Axis local mode (plan 07 Task 5).
//!
//! `internal` exposure: served only to loopback callers under `dev`, so a remote
//! server never serves them (D1 keeps `gui.*` client-side in remote mode).
use axum::Router;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::gui_ops::{GUI_USER_ID, require_db};
use crate::http_gateway::GatewayState;
use crate::params::ToolResult;
use crate::server_state::ServerState;

pub const GUI_PREFERENCE_PREFIX: &str = "gui.";

/// Only `gui.*` keys, so these ops can never touch orchestrator-owned preferences
/// such as `socrates_gate_enforced` or `attention_enabled`.
pub fn is_gui_preference_key(key: &str) -> bool {
    key.starts_with(GUI_PREFERENCE_PREFIX) && key.len() <= 128
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GuiPreferenceGetParams {
    pub key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GuiPreferenceSetParams {
    pub key: String,
    pub value: String,
}

fn refuse_key(key: &str) -> String {
    ToolResult::<String>::err_with_remediation(
        format!("key '{key}' is not a GUI preference"),
        "GUI preferences are namespaced `gui.*`; use the owning tool for other keys.",
    )
    .to_json()
}

pub async fn gui_preference_get(state: &ServerState, p: GuiPreferenceGetParams) -> String {
    if !is_gui_preference_key(&p.key) {
        return refuse_key(&p.key);
    }
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<Option<String>>::err(e).to_json(),
    };
    match db.get_user_preference(GUI_USER_ID, &p.key).await {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => ToolResult::<Option<String>>::err(e.to_string()).to_json(),
    }
}

pub async fn gui_preference_set(state: &ServerState, p: GuiPreferenceSetParams) -> String {
    if !is_gui_preference_key(&p.key) {
        return refuse_key(&p.key);
    }
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<String>::err(e).to_json(),
    };
    match db.set_user_preference(GUI_USER_ID, &p.key, &p.value).await {
        Ok(()) => ToolResult::ok(p.key).to_json(),
        Err(e) => ToolResult::<String>::err(e.to_string()).to_json(),
    }
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
}

#[cfg(test)]
mod tests {
    use vox_mcp_registry::{
        ActiveProfile, BootReport, CallContext, CallerRole, ProfileDenial, Subsystem,
        authorize_tool,
    };

    use super::*;
    use crate::gui_ops::test_support::{gateway_with_memory_db, tool_json};

    #[test]
    fn router_builds() {
        let _ = api_v2_router();
    }

    fn set(key: &str, value: &str) -> GuiPreferenceSetParams {
        GuiPreferenceSetParams {
            key: key.into(),
            value: value.into(),
        }
    }

    fn get(key: &str) -> GuiPreferenceGetParams {
        GuiPreferenceGetParams { key: key.into() }
    }

    fn attention_enabled(st: &ServerState) -> bool {
        st.orchestrator
            .config_handle()
            .read()
            .expect("config lock")
            .attention_enabled
    }

    #[tokio::test]
    async fn set_refuses_orchestrator_keys_and_accepts_gui_keys() {
        let (gs, db) = gateway_with_memory_db().await;
        let st = gs.server_state();
        let attention_before = attention_enabled(st);

        for key in ["socrates_gate_enforced", "attention_enabled"] {
            let out = tool_json(&gui_preference_set(st, set(key, "false")).await);
            assert_eq!(out["success"], false, "{key}: {out}");
            assert!(
                db.get_user_preference(GUI_USER_ID, key)
                    .await
                    .expect("read")
                    .is_none(),
                "{key} row must not be written"
            );
        }
        assert_eq!(attention_enabled(st), attention_before);

        let ok = tool_json(&gui_preference_set(st, set("gui.theme", "dark")).await);
        assert_eq!(ok["success"], true, "{ok}");
        let back = tool_json(&gui_preference_get(st, get("gui.theme")).await);
        assert_eq!(back["data"], "dark");
    }

    #[tokio::test]
    async fn get_returns_null_for_missing_gui_keys_and_refuses_others() {
        let (gs, _db) = gateway_with_memory_db().await;
        let st = gs.server_state();
        let missing = tool_json(&gui_preference_get(st, get("gui.nope")).await);
        assert_eq!(missing["success"], true, "{missing}");
        assert!(missing["data"].is_null());
        let refused = tool_json(&gui_preference_get(st, get("attention_enabled")).await);
        assert_eq!(refused["success"], false, "{refused}");
    }

    #[test]
    fn key_rule_requires_prefix_and_bounded_length() {
        assert!(is_gui_preference_key("gui.keybinds"));
        assert!(!is_gui_preference_key("guix"));
        assert!(!is_gui_preference_key(&format!("gui.{}", "a".repeat(200))));
    }

    #[test]
    fn prefs_ops_are_loopback_only_and_outside_named_profiles() {
        let all_up = BootReport {
            booted: Subsystem::ALL.iter().copied().collect(),
            failed: Default::default(),
        };
        let off_loopback = CallContext {
            role: CallerRole::Write,
            loopback: false,
        };
        for tool in ["vox_gui_preference_get", "vox_gui_preference_set"] {
            assert_eq!(
                authorize_tool(&ActiveProfile::dev(), &all_up, off_loopback, tool).map(|_| ()),
                Err(ProfileDenial::ExposureDenied),
                "{tool}"
            );
            assert!(
                authorize_tool(
                    &ActiveProfile::dev(),
                    &all_up,
                    CallContext::LOCAL_WRITE,
                    tool
                )
                .is_ok(),
                "{tool}"
            );
            let models = ActiveProfile::resolve(&["models"]).expect("models profile");
            assert!(!models.contains_tool(tool), "{tool}");
        }
    }
}
