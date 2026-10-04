//! Adds the engine's own severity to each agent-event frame the GUI receives, and says whether the
//! event is one the activity log records, so no GUI code classifies events by string matching.

use serde::Deserialize;
use serde_json::Value;
use vox_orchestrator::AgentEventKind;

/// Tauri event emitted when the Activity view should re-query (an activity-loggable event or a replay frame).
pub const ACTIVITY_APPENDED_EVENT: &str = "vox://activity-appended";

/// Set `severity` on an agent-event frame and return whether the Activity view should refresh: true
/// for an activity-loggable event and for a replay frame (rows written while the stream was down).
/// A replay frame, or a frame whose `kind` is not an `AgentEventKind`, is left unchanged.
pub fn annotate_agent_event(frame: &mut Value) -> bool {
    let Some(obj) = frame.as_object_mut() else {
        return false;
    };
    if obj.get("replay").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    let Some(kind) = obj.get("kind") else {
        return false;
    };
    // Token frames are the high-frequency path: classify without parsing.
    if kind.get("type").and_then(Value::as_str) == Some("token_streamed") {
        obj.insert("severity".into(), Value::from("debug"));
        return false;
    }
    let Ok(kind) = AgentEventKind::deserialize(kind) else {
        return false;
    };
    if let Ok(severity) = serde_json::to_value(kind.severity()) {
        obj.insert("severity".into(), severity);
    }
    vox_orchestrator::activity::is_loggable(&kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_token_frame_is_debug_and_not_logged_without_being_parsed() {
        let mut frame = json!({ "id": 1, "timestamp_ms": 0, "kind": { "type": "token_streamed", "anything": true } });
        assert!(!annotate_agent_event(&mut frame));
        assert_eq!(frame["severity"], json!("debug"));
    }

    #[test]
    fn a_loggable_event_is_annotated_and_reported() {
        let mut frame = json!({ "id": 2, "timestamp_ms": 5, "kind": {
            "type": "lock_waiting", "resource_id": "db://x", "task_id": 7 } });
        assert!(
            annotate_agent_event(&mut frame),
            "LockWaiting is activity-loggable"
        );
        assert_eq!(frame["severity"], json!("info"));
    }

    #[test]
    fn an_error_event_carries_its_severity() {
        let mut frame = json!({ "id": 3, "timestamp_ms": 5, "kind": { "type": "injection_detected", "detail": "x" } });
        assert!(!annotate_agent_event(&mut frame));
        assert_eq!(frame["severity"], json!("error"));
    }

    #[test]
    fn the_token_fast_path_agrees_with_the_engine() {
        let kind: AgentEventKind =
            serde_json::from_value(json!({ "type": "token_streamed", "agent_id": 1, "text": "" }))
                .unwrap();
        assert_eq!(
            serde_json::to_value(kind.severity()).unwrap(),
            json!("debug")
        );
    }

    #[test]
    fn a_replay_frame_refreshes_the_activity_view_and_is_left_alone() {
        // Shape of `orch_daemon::replay_frame_value`: rows written while the stream was down.
        let mut frame = json!({ "replay": true, "op_id": 4, "agent_id": 1, "timestamp_ms": 0,
            "description": "d", "kind": null });
        let before = frame.clone();
        assert!(annotate_agent_event(&mut frame));
        assert_eq!(frame, before);
    }

    #[test]
    fn a_frame_that_is_not_an_agent_event_is_left_alone() {
        let mut frame = json!({ "offset": 9, "kind": { "type": "not_a_kind" } });
        let before = frame.clone();
        assert!(!annotate_agent_event(&mut frame));
        assert_eq!(frame, before);
    }
}
