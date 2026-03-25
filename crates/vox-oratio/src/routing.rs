//! Two-way routing for transcript-driven loops (tool / chat / orchestrator).
//!
//! Uses deterministic intent classification with optional confidence gating aligned to
//! [`crate::runtime_config::OratioRuntimeConfig`].

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::runtime_config::OratioRuntimeConfig;

/// Classified intent for tool routing (deterministic scaffold; LLM disambiguation is out-of-crate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntentKind {
    None,
    OratioStatus,
}

fn classify_intent(transcript: &str) -> (IntentKind, f32) {
    let t = transcript.trim();
    if t.is_empty() {
        return (IntentKind::None, 0.0);
    }
    let lower = t.to_ascii_lowercase();
    let compact: String = lower.chars().filter(|c| !c.is_whitespace()).collect();

    let status_needles = [
        "voxoratiostatus",
        "oratiostatus",
        "vox oratio status",
        "oratio status",
    ];
    for n in status_needles {
        let n_compact: String = n.chars().filter(|c| !c.is_whitespace()).collect();
        if compact.contains(&n_compact) || lower.contains(n) {
            return (IntentKind::OratioStatus, 0.92);
        }
    }
    if lower == "status" || compact == "status" {
        return (IntentKind::OratioStatus, 0.78);
    }

    // Token-density hint: "status" near "oratio" / "vox"
    let tokens: Vec<&str> = lower.split_whitespace().collect();
    let has_status = tokens.iter().any(|w| *w == "status");
    let has_oratio = tokens.iter().any(|w| w.contains("oratio"));
    let has_vox = tokens.iter().any(|w| *w == "vox");
    if has_status && has_oratio {
        return (IntentKind::OratioStatus, 0.72);
    }
    if has_status && has_vox {
        return (IntentKind::OratioStatus, 0.55);
    }

    (IntentKind::None, 0.35)
}

fn chat_sessions() -> &'static Mutex<HashMap<String, Vec<String>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Vec<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn user_turn_counts() -> &'static Mutex<HashMap<String, usize>> {
    static CACHE: OnceLock<Mutex<HashMap<String, usize>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn last_user_transcript() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Route modes for transcript-driven execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RouteMode {
    /// No route action.
    #[default]
    None,
    /// Tool-loop route (safe local actions).
    Tool,
    /// Conversational route with session memory.
    Chat,
    /// Orchestrator envelope route.
    Orchestrator,
}

/// Route response envelope shared by CLI and MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResponse {
    /// Chosen route mode.
    pub mode: RouteMode,
    /// Stable action identifier when one is selected.
    pub action: String,
    /// Human-readable status.
    pub status: String,
    /// Structured payload for downstream consumers.
    pub payload: serde_json::Value,
}

/// Route with explicit confidence + runtime policy (CLI / MCP parity).
#[must_use]
pub fn route_transcript_with_options(
    mode: RouteMode,
    session_id: &str,
    transcript: &str,
    transcript_confidence: f32,
    runtime: &OratioRuntimeConfig,
) -> RouteResponse {
    let empty = transcript.trim().is_empty();
    if empty && !matches!(mode, RouteMode::None) {
        return RouteResponse {
            mode: RouteMode::None,
            action: "none".to_string(),
            status: "no_op_empty_transcript".to_string(),
            payload: serde_json::json!({ "note": "Empty transcript; route suppressed" }),
        };
    }

    if let Ok(mut last) = last_user_transcript().lock() {
        if let Some(prev) = last.get(session_id) {
            if !transcript.trim().is_empty() && prev == transcript {
                return RouteResponse {
                    mode: RouteMode::None,
                    action: "none".to_string(),
                    status: "repetition_breaker".to_string(),
                    payload: serde_json::json!({
                        "note": "Identical consecutive transcript for session",
                        "session_id": session_id,
                    }),
                };
            }
        }
        if !transcript.trim().is_empty() {
            last.insert(session_id.to_string(), transcript.to_string());
        }
    }

    match mode {
        RouteMode::None => RouteResponse {
            mode,
            action: "none".to_string(),
            status: "no_route".to_string(),
            payload: serde_json::json!({ "note": "No route action requested" }),
        },
        RouteMode::Tool => {
            let min_c = runtime.routing.tool_route_min_confidence.clamp(0.0, 1.0);
            if transcript_confidence < min_c {
                return RouteResponse {
                    mode: RouteMode::None,
                    action: "none".to_string(),
                    status: "below_tool_confidence".to_string(),
                    payload: serde_json::json!({
                        "note": "Transcript confidence below tool route threshold",
                        "transcript_confidence": transcript_confidence,
                        "min_confidence": min_c,
                    }),
                };
            }
            let (intent, route_conf) = classify_intent(transcript);
            if matches!(intent, IntentKind::OratioStatus) {
                let blended = (transcript_confidence * 0.5 + route_conf * 0.5).clamp(0.0, 1.0);
                if blended < min_c {
                    return RouteResponse {
                        mode: RouteMode::None,
                        action: "none".to_string(),
                        status: "below_blended_tool_confidence".to_string(),
                        payload: serde_json::json!({
                            "note": "Blended ASR + intent confidence below threshold",
                            "blended_confidence": blended,
                            "min_confidence": min_c,
                        }),
                    };
                }
                return RouteResponse {
                    mode,
                    action: "oratio.status".to_string(),
                    status: "executed".to_string(),
                    payload: serde_json::json!({
                        "summary": crate::transcript_status(),
                        "candle": crate::candle_backend_status_json(),
                        "intent_confidence": route_conf,
                        "blended_confidence": blended,
                    }),
                };
            }
            RouteResponse {
                mode,
                action: "none".to_string(),
                status: "no_match".to_string(),
                payload: serde_json::json!({
                    "note": "No safe tool action matched transcript",
                    "intent_confidence": route_conf,
                }),
            }
        }
        RouteMode::Chat => {
            let cap = runtime.routing.chat_max_messages.max(4);
            let max_turns = runtime.routing.route_max_user_turns.max(1);
            let mut turns = user_turn_counts()
                .lock()
                .expect("user turn mutex poisoned");
            let n = turns.entry(session_id.to_string()).or_insert(0);
            *n += 1;
            if *n > max_turns {
                return RouteResponse {
                    mode: RouteMode::None,
                    action: "none".to_string(),
                    status: "guard_max_user_turns".to_string(),
                    payload: serde_json::json!({
                        "session_id": session_id,
                        "user_turns": *n,
                        "max_user_turns": max_turns,
                    }),
                };
            }

            let mut cache = chat_sessions().lock().expect("chat session mutex poisoned");
            let history = cache.entry(session_id.to_string()).or_default();
            history.push(format!("user:{transcript}"));
            if history.len() > cap {
                let drain = history.len() - cap;
                history.drain(0..drain);
            }
            let response = format!("Session {session_id}: received '{}'", transcript);
            history.push(format!("assistant:{response}"));
            if history.len() > cap {
                let drain = history.len() - cap;
                history.drain(0..drain);
            }
            RouteResponse {
                mode,
                action: "chat.turn".to_string(),
                status: "executed".to_string(),
                payload: serde_json::json!({
                    "session_id": session_id,
                    "turns": history.len(),
                    "response": response,
                    "user_turn": *n,
                }),
            }
        }
        RouteMode::Orchestrator => {
            let min_o = runtime
                .routing
                .orchestrator_min_confidence
                .clamp(0.0, 1.0);
            let low = transcript_confidence < min_o;
            RouteResponse {
                mode,
                action: "orchestrator.enqueue".to_string(),
                status: if low {
                    "queued_low_confidence".to_string()
                } else {
                    "queued".to_string()
                },
                payload: serde_json::json!({
                    "session_id": session_id,
                    "intent_text": transcript,
                    "workflow_status": "queued",
                    "transcript_confidence": transcript_confidence,
                    "orchestrator_min_confidence": min_o,
                    "note": "Transcript routed to orchestrator-compatible envelope",
                }),
            }
        }
    }
}

/// Back-compat wrapper: uses [`crate::runtime_config::resolved_runtime_config`] and assumes
/// saturated transcript confidence (callers pre-gating tool mode should prefer
/// [`route_transcript_with_options`]).
#[must_use]
pub fn route_transcript(mode: RouteMode, session_id: &str, transcript: &str) -> RouteResponse {
    route_transcript_with_options(
        mode,
        session_id,
        transcript,
        1.0,
        crate::runtime_config::resolved_runtime_config(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_mode_matches_status_with_confidence() {
        let rt = OratioRuntimeConfig::default();
        let out = route_transcript_with_options(
            RouteMode::Tool,
            "s1",
            "oratio status",
            0.9,
            &rt,
        );
        assert_eq!(out.action, "oratio.status");
        let out_low = route_transcript_with_options(
            RouteMode::Tool,
            "s2",
            "oratio status",
            0.1,
            &rt,
        );
        assert_eq!(out_low.status, "below_tool_confidence");
    }

    #[test]
    fn repetition_breaker_triggers() {
        let rt = OratioRuntimeConfig::default();
        let _ = route_transcript_with_options(RouteMode::Chat, "rpt", "hello", 1.0, &rt);
        let out2 = route_transcript_with_options(RouteMode::Chat, "rpt", "hello", 1.0, &rt);
        assert_eq!(out2.status, "repetition_breaker");
    }

    #[test]
    fn chat_max_user_turns_guard() {
        let mut rt = OratioRuntimeConfig::default();
        rt.routing.route_max_user_turns = 2;
        let sid = "max-turn-unit-test";
        let _ = route_transcript_with_options(RouteMode::Chat, sid, "a", 1.0, &rt);
        let _ = route_transcript_with_options(RouteMode::Chat, sid, "b", 1.0, &rt);
        let out = route_transcript_with_options(RouteMode::Chat, sid, "c", 1.0, &rt);
        assert_eq!(out.status, "guard_max_user_turns");
    }
}
