//! Arca Replay: Extracts supervised multi-turn strings and generic tool traces directly
//! from live Arca `agent_events` and `a2a_messages` tables for self-training loops.

#[cfg(feature = "database")]
use vox_db::VoxDb;

/// Container for extracted real-world corpus rows from Arca telemetry.
#[derive(Debug, serde::Serialize)]
pub struct ReplayRow {
    pub prompt: String,
    pub response: String,
    pub category: String,
    pub record_type: String,
    #[serde(default)]
    pub chatml: bool,
}

#[cfg(feature = "database")]
pub async fn extract_arca_pairs(db: &VoxDb, limit: i64, chatml: bool, _min_score: f64) -> anyhow::Result<Vec<ReplayRow>> {
    let mut rows = Vec::new();

    // Replay 1: A2A Messages
    // Extract recent A2A interaction payloads using payload shapes from actual telemetry.
    let sql_a2a = "
        SELECT sender, receiver, msg_type, payload
        FROM a2a_messages
        WHERE payload IS NOT NULL AND msg_type != ''
          AND (code_version >= '0.3.0' OR code_version IS NULL)
          AND timestamp > datetime('now', '-30 days')
        ORDER BY id DESC
        LIMIT ?1
    ";
    
    match db.query_all(sql_a2a, turso::params![limit]).await {
        Ok(results) => {
            for row in results {
                let sender = row.get::<String>(0).unwrap_or_default();
                let _receiver = row.get::<String>(1).unwrap_or_default();
                let msg_type = row.get::<String>(2).unwrap_or_default();
                let payload = row.get::<String>(3).unwrap_or_default();

                // Simple SFT: If it's a known A2A format, we map it into an instruction pair.
                // Normally we'd do ChatML unpacking here based on JSON structure
                if let Ok(_json) = serde_json::from_str::<serde_json::Value>(&payload) {
                    rows.push(ReplayRow {
                        prompt: format!("Process A2A {} message from {}", msg_type, sender),
                        response: payload.clone(),
                        category: msg_type,
                        record_type: "a2a_trace".to_string(),
                        chatml: false,
                    });
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to query a2a_messages for replay: {}", e);
        }
    }

    // Replay 2: Agent Events (Tool Trace / LLM Turns / Multi-turn sessions)
    let sql_events = "
        SELECT event_type, payload
        FROM agent_events
        WHERE payload IS NOT NULL 
          AND timestamp > datetime('now', '-30 days')
        ORDER BY id ASC
        LIMIT ?1
    ";

    let mut sessions: std::collections::HashMap<String, Vec<serde_json::Value>> = std::collections::HashMap::new();

    match db.query_all(sql_events, turso::params![limit]).await {
        Ok(results) => {
            for row in results {
                let event_type = row.get::<String>(0).unwrap_or_default();
                let payload = row.get::<String>(1).unwrap_or_default();

                if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&payload) {
                    // Ensure event type is captured in json if missing
                    if !json.as_object().unwrap().contains_key("type") {
                        json.as_object_mut().unwrap().insert("type".to_string(), serde_json::Value::String(event_type.clone()));
                    }

                    if let Some(session_id) = json.get("session_id").and_then(|v| v.as_str()) {
                        sessions.entry(session_id.to_string()).or_default().push(json.clone());
                    } else if !chatml {
                        if event_type == "tool_call" {
                            // Fallback for flat tool calls
                            let tool_name = json.get("tool").and_then(|v| v.as_str()).unwrap_or("unknown_tool");
                            let args = json.get("args").and_then(|v| v.as_str()).unwrap_or("{}");
                            
                            rows.push(ReplayRow {
                                prompt: format!("Call tool {}", tool_name),
                                response: args.to_string(),
                                category: tool_name.to_string(),
                                record_type: "tool_trace".to_string(),
                                chatml: false,
                            });
                        } else if event_type == "llm_turn" {
                            // Fallback for flat LLM turns
                            if let Some(prompt) = json.get("prompt").and_then(|v| v.as_str()) {
                                if let Some(resp) = json.get("response").and_then(|v| v.as_str()) {
                                    rows.push(ReplayRow {
                                        prompt: prompt.to_string(),
                                        response: resp.to_string(),
                                        category: "llm_turn".to_string(),
                                        record_type: "llm_turn".to_string(),
                                        chatml: false, 
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // Compile multi-turn ChatML sessions if requested or opportunistically
            if chatml {
                for (session_id, events) in sessions {
                    // Placeholder for min_score integration if scores were tracked per session in metrics
                    // if min_score > 0.0 && !check_session_score(session_id) { continue; }
                    if let Some(chatml_row) = compile_chatml_session(&session_id, &events) {
                        rows.push(chatml_row);
                    }
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to query agent_events for replay: {}", e);
        }
    }

    Ok(rows)
}

/// Reconstructs a full multi-turn trajectory from grouped agent events into a proper ChatML trace.
///
/// Untrusted strings from the event payload are passed through [`sanitize_chatml`] before
/// being embedded as role content so injected `<|im_start|>` / `<|im_end|>` tokens cannot
/// corrupt the role-boundary structure expected by the training loss mask.
fn compile_chatml_session(session_id: &str, events: &[serde_json::Value]) -> Option<ReplayRow> {
    let mut chatml_buffer = String::new();
    let mut initial_task = String::new();

    for ev in events {
        let ty = ev.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match ty {
            "TaskSubmitted" | "TaskStarted" => {
                if initial_task.is_empty() {
                    if let Some(desc) = ev
                        .get("description")
                        .or_else(|| ev.get("task"))
                        .and_then(|v| v.as_str())
                    {
                        initial_task = desc.to_string();
                        chatml_buffer.push_str(&format!(
                            "<|im_start|>user\n{}<|im_end|>\n",
                            sanitize_chatml(&initial_task)
                        ));
                    }
                }
            }
            "ActivityStarted" => {
                if let Some(act) = ev.get("activity").and_then(|v| v.as_str()) {
                    chatml_buffer.push_str(&format!(
                        "<|im_start|>system\n[Orchestrator Step: {}]<|im_end|>\n",
                        sanitize_chatml(act)
                    ));
                }
            }
            "llm_turn" => {
                if let Some(resp) = ev.get("response").and_then(|v| v.as_str()) {
                    chatml_buffer.push_str(&format!(
                        "<|im_start|>assistant\n{}<|im_end|>\n",
                        sanitize_chatml(resp)
                    ));
                }
            }
            "tool_call" => {
                let tool = sanitize_chatml(ev.get("tool").and_then(|v| v.as_str()).unwrap_or("unknown"));
                let args = sanitize_chatml(ev.get("args").and_then(|v| v.as_str()).unwrap_or("{}"));
                chatml_buffer.push_str(&format!(
                    "<|im_start|>assistant\n<|tool_call|>{{\"name\":\"{tool}\", \"args\":{args}}}<|tool_end|><|im_end|>\n"
                ));
            }
            // Emit the tool's environment feedback so the model learns the full
            // request→call→result loop rather than just seeing the invocation.
            "tool_result" | "ToolResult" => {
                let tool = sanitize_chatml(
                    ev.get("tool")
                        .or_else(|| ev.get("tool_name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown"),
                );
                let result = sanitize_chatml(
                    ev.get("result")
                        .or_else(|| ev.get("output"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                );
                if !result.is_empty() {
                    chatml_buffer.push_str(&format!(
                        "<|im_start|>tool\n[{tool}]: {result}<|im_end|>\n"
                    ));
                }
            }
            "TaskCompleted" => {
                // System-controlled string — no sanitization needed.
                chatml_buffer.push_str("<|im_start|>system\n[Task Completed]<|im_end|>\n");
            }
            _ => {}
        }
    }

    if chatml_buffer.is_empty() || initial_task.is_empty() {
        return None;
    }

    Some(ReplayRow {
        prompt: chatml_buffer,
        response: String::new(), // Full trace serialized into prompt field.
        category: "multi_turn_session".to_string(),
        record_type: format!("chatml_session_{session_id}"),
        chatml: true,
    })
}

/// Escape reserved ChatML control tokens in untrusted content.
///
/// Replaces `<|im_start|>` and `<|im_end|>` with bracket-quoted equivalents so
/// injected text cannot escape its assigned role slot or corrupt the loss mask.
#[inline]
fn sanitize_chatml(s: &str) -> String {
    s.replace("<|im_start|>", "[im_start]")
        .replace("<|im_end|>", "[im_end]")
}
