//! Desktop computer-use MCP tools (`vox_computer_*`).
//!
//! Dispatches through vox-plugin-host to the `computer` code plugin's `ComputerControl`
//! extension point. Every call blocks on OS accessibility / capture APIs, so it runs inside
//! `tokio::task::spawn_blocking`. Plugin refusals (`{"ok":false,"reason":..}`) are successful
//! tool results: the model needs the reason to recover, not a transport error.

use crate::caller_role::{CallerRole, trusted_caller_role};
use crate::params::{
    ComputerActParams, ComputerScreenshotParams, ComputerSnapshotParams, ComputerWindowsParams,
    ToolResult,
};
use crate::server_state::ServerState;
use vox_plugin_host::loader::LoadedCodePlugin;

const MIN_REVISION: u32 = 1;

fn computer_plugin() -> anyhow::Result<&'static LoadedCodePlugin> {
    let plugin = vox_plugin_host::cached_code_plugin("computer")
        .map_err(|e| anyhow::anyhow!("computer plugin load: {e}"))?;
    let control = plugin
        .plugin
        .as_computer_control()
        .into_option()
        .ok_or_else(|| anyhow::anyhow!("computer plugin loaded without a ComputerControl"))?;
    let actual = control.revision();
    if actual < MIN_REVISION {
        anyhow::bail!(
            "computer plugin revision {actual} is too old; revision {MIN_REVISION} is required. \
             Rebuild/reinstall the computer plugin."
        );
    }
    Ok(plugin)
}

/// `{"pid":..,"app":..}` from the flattened MCP fields; `None` when neither is set.
fn target_json(app: Option<String>, pid: Option<i64>) -> Option<serde_json::Value> {
    if app.is_none() && pid.is_none() {
        return None;
    }
    let mut target = serde_json::Map::new();
    if let Some(pid) = pid {
        target.insert("pid".into(), pid.into());
    }
    if let Some(app) = app {
        target.insert("app".into(), app.into());
    }
    Some(serde_json::Value::Object(target))
}

fn snapshot_options(p: ComputerSnapshotParams) -> serde_json::Value {
    let mut o = serde_json::Map::new();
    if let Some(t) = target_json(p.app, p.pid) {
        o.insert("target".into(), t);
    }
    if let Some(d) = p.max_depth {
        o.insert("max_depth".into(), d.into());
    }
    if let Some(n) = p.max_nodes {
        o.insert("max_nodes".into(), n.into());
    }
    if p.interactive_only {
        o.insert("interactive_only".into(), true.into());
    }
    serde_json::Value::Object(o)
}

fn screenshot_options(p: ComputerScreenshotParams) -> serde_json::Value {
    let mut o = serde_json::Map::new();
    if let Some(t) = target_json(p.app, p.pid) {
        o.insert("target".into(), t);
    }
    if let Some(r) = p.region {
        o.insert("region".into(), serde_json::json!(r));
    }
    if let Some(m) = p.max_long_edge {
        o.insert("max_long_edge".into(), m.into());
    }
    serde_json::Value::Object(o)
}

/// Plugin action JSON: `app`/`pid` move under `target`, and sensitive-field refusal is
/// decided by the trusted caller role, never by the model.
fn act_request(p: ComputerActParams, respect_sensitive: bool) -> serde_json::Value {
    let mut v = serde_json::to_value(&p).unwrap_or_else(|_| serde_json::json!({}));
    if let Some(obj) = v.as_object_mut() {
        let app = obj
            .remove("app")
            .and_then(|a| a.as_str().map(str::to_string));
        let pid = obj.remove("pid").and_then(|p| p.as_i64());
        if let Some(t) = target_json(app, pid) {
            obj.insert("target".into(), t);
        }
        obj.insert("respect_sensitive".into(), respect_sensitive.into());
    }
    v
}

async fn run_blocking<F>(op: &'static str, f: F) -> String
where
    F: FnOnce(&LoadedCodePlugin) -> Result<String, String> + Send + 'static,
{
    let joined = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let plugin = computer_plugin()?;
        let raw = f(plugin).map_err(|e| anyhow::anyhow!("computer {op}: {e}"))?;
        serde_json::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("computer {op}: invalid plugin JSON: {e}"))
    })
    .await;
    match joined {
        Ok(Ok(value)) => ToolResult::ok(value).to_json(),
        Ok(Err(e)) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(format!("spawn_blocking: {e}")).to_json(),
    }
}

macro_rules! control {
    ($plugin:expr) => {
        $plugin
            .plugin
            .as_computer_control()
            .into_option()
            .ok_or_else(|| "ComputerControl accessor vanished".to_string())?
    };
}

pub async fn computer_permissions(_state: &ServerState, _p: serde_json::Value) -> String {
    run_blocking("permissions", |plugin| {
        control!(plugin)
            .permissions()
            .into_result()
            .map(|s| s.into_string())
            .map_err(|e| e.to_string())
    })
    .await
}

pub async fn computer_list_apps(_state: &ServerState, _p: serde_json::Value) -> String {
    run_blocking("list_apps", |plugin| {
        control!(plugin)
            .list_apps()
            .into_result()
            .map(|s| s.into_string())
            .map_err(|e| e.to_string())
    })
    .await
}

pub async fn computer_list_windows(_state: &ServerState, p: ComputerWindowsParams) -> String {
    let options = serde_json::to_string(&p).unwrap_or_else(|_| "{}".into());
    run_blocking("list_windows", move |plugin| {
        control!(plugin)
            .list_windows(options.as_str().into())
            .into_result()
            .map(|s| s.into_string())
            .map_err(|e| e.to_string())
    })
    .await
}

pub async fn computer_snapshot(_state: &ServerState, p: ComputerSnapshotParams) -> String {
    let options = snapshot_options(p).to_string();
    run_blocking("snapshot", move |plugin| {
        control!(plugin)
            .snapshot(options.as_str().into())
            .into_result()
            .map(|s| s.into_string())
            .map_err(|e| e.to_string())
    })
    .await
}

pub async fn computer_screenshot(_state: &ServerState, p: ComputerScreenshotParams) -> String {
    let options = screenshot_options(p).to_string();
    run_blocking("screenshot", move |plugin| {
        control!(plugin)
            .screenshot(options.as_str().into())
            .into_result()
            .map(|s| s.into_string())
            .map_err(|e| e.to_string())
    })
    .await
}

pub async fn computer_act(_state: &ServerState, p: ComputerActParams) -> String {
    let request = act_request(p, trusted_caller_role() != CallerRole::Human).to_string();
    run_blocking("act", move |plugin| {
        control!(plugin)
            .act(request.as_str().into())
            .into_result()
            .map(|s| s.into_string())
            .map_err(|e| e.to_string())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(json: serde_json::Value) -> ComputerActParams {
        serde_json::from_value(json).expect("valid params")
    }

    #[test]
    fn act_request_nests_target_and_forces_respect_sensitive() {
        let req = act_request(
            act(serde_json::json!({"app": "Calculator", "kind": "press", "ref": "e4"})),
            true,
        );
        assert_eq!(
            req,
            serde_json::json!({
                "target": {"app": "Calculator"},
                "kind": "press",
                "ref": "e4",
                "require_background": false,
                "respect_sensitive": true,
            })
        );
    }

    #[test]
    fn act_params_reject_model_supplied_respect_sensitive() {
        let parsed = serde_json::from_value::<ComputerActParams>(serde_json::json!({
            "app": "Notes", "kind": "type", "text": "x", "respect_sensitive": false
        }));
        assert!(
            parsed.is_err(),
            "the model must not be able to waive the secure-field guard"
        );
    }

    #[test]
    fn act_request_keeps_pid_target_and_coordinates() {
        let req = act_request(
            act(serde_json::json!({
                "pid": 42, "kind": "drag", "x": 1.0, "y": 2.0, "to_x": 3.0, "to_y": 4.0,
                "coords": "screen", "tier": "event"
            })),
            false,
        );
        assert_eq!(req["target"], serde_json::json!({"pid": 42}));
        assert_eq!(req["coords"], "screen");
        assert_eq!(req["tier"], "event");
        assert_eq!(req["to_y"], 4.0);
        assert_eq!(req["respect_sensitive"], false);
        assert!(req.get("app").is_none() && req.get("pid").is_none());
    }

    #[test]
    fn snapshot_and_screenshot_options_omit_unset_fields() {
        let snap = snapshot_options(
            serde_json::from_value(serde_json::json!({"app": "Finder", "interactive_only": true}))
                .expect("params"),
        );
        assert_eq!(
            snap,
            serde_json::json!({"target": {"app": "Finder"}, "interactive_only": true})
        );
        let shot =
            screenshot_options(serde_json::from_value(serde_json::json!({})).expect("params"));
        assert_eq!(shot, serde_json::json!({}));
        let zoom = screenshot_options(
            serde_json::from_value(serde_json::json!({"pid": 7, "region": [0, 0, 10, 20]}))
                .expect("params"),
        );
        assert_eq!(zoom["target"], serde_json::json!({"pid": 7}));
        assert_eq!(zoom["region"], serde_json::json!([0.0, 0.0, 10.0, 20.0]));
    }
}
