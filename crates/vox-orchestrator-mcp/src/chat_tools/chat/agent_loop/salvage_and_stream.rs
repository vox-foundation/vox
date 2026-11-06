use super::*;

/// Stream one iteration's model call via [`llm_stream_events_activity`],
/// parsing both `delta.content` and `delta.tool_calls` in a single HTTP pass,
/// enforcing `vox_llm_egress::RepetitionGuard` (`vox_crypto::fast_hash`) to abort
/// degenerate token/n-gram loops in `O(1)`, and computing live 3-tier cost provenance.
pub(super) async fn stream_final_answer(
    state: &ServerState,
    activity_options: &ActivityOptions,
    messages: &[LlmChatMessage],
    config: LlmConfig,
    session_id: Option<&str>,
) -> Option<LlmResponse> {
    use futures_util::StreamExt;

    let model = config.model.clone();
    let mut stream =
        match llm_stream_events_activity(activity_options, messages.to_vec(), config).await {
            vox_actor_runtime::ActivityResult::Ok(s) => s,
            vox_actor_runtime::ActivityResult::Failed(_)
            | vox_actor_runtime::ActivityResult::Cancelled => return None,
        };

    let bus = state.orchestrator.event_bus();
    let agent_id = AgentId(0); // chat's pseudo-agent id, matching message.rs's convention.
    let start = std::time::Instant::now();
    let mut ttft_ms: Option<u64> = None;
    let mut content = String::new();
    let stream_session_key = session_id
        .filter(|s| !s.is_empty())
        .unwrap_or("default")
        .to_string();
    {
        let mut streams = state.resumable_streams.lock();
        let entry = streams
            .entry(stream_session_key.clone())
            .or_insert_with(|| (String::new(), false));
        entry.1 = false;
    }
    {
        let mut reg = state.resumable_stream_registry.lock();
        if !reg.is_generating(&stream_session_key) {
            reg.start_turn(
                stream_session_key.clone(),
                format!("run-{}", crate::chat_tools::now_ts()),
            );
        }
    }
    let mut repetition_guard = vox_actor_runtime::llm::RepetitionGuard::new();
    #[derive(Default)]
    struct ToolCallAcc {
        id: String,
        name: String,
        arguments_raw: String,
        prefetched_args: Option<serde_json::Value>,
    }
    let mut tool_calls_by_index: std::collections::BTreeMap<usize, ToolCallAcc> =
        std::collections::BTreeMap::new();
    let mut streamed_prompt_tokens: Option<u32> = None;
    let mut streamed_completion_tokens: Option<u32> = None;
    let mut streamed_cache_read_tokens: Option<u32> = None;

    while let Some(item) = stream.next().await {
        match item {
            Ok(ev) => {
                if ev.prompt_tokens.is_some() {
                    streamed_prompt_tokens = ev.prompt_tokens;
                }
                if ev.completion_tokens.is_some() {
                    streamed_completion_tokens = ev.completion_tokens;
                }
                if ev.cache_read_tokens.is_some() {
                    streamed_cache_read_tokens = ev.cache_read_tokens;
                }
                if let Some(chunk) = ev.content
                    && !chunk.is_empty()
                {
                    if ttft_ms.is_none() {
                        ttft_ms = Some(start.elapsed().as_millis() as u64);
                    }
                    content.push_str(&chunk);
                    if repetition_guard
                        .observe_chunk_with_hasher(&chunk, vox_crypto::fast_hash)
                        .is_some()
                    {
                        break;
                    }
                    if !looks_like_streamed_tool_call_envelope(&content) {
                        if let Some((buf, _)) =
                            state.resumable_streams.lock().get_mut(&stream_session_key)
                        {
                            buf.push_str(&chunk);
                        }
                        state.resumable_stream_registry.lock().append_chunk(
                            &stream_session_key,
                            vox_orchestrator::models::spec::TurnStreamEventKind::TextDelta,
                            chunk.clone(),
                        );
                        bus.emit(AgentEventKind::TokenStreamed {
                            agent_id,
                            text: chunk,
                            session_id: session_id.map(str::to_string),
                        });
                    }
                }
                for tc in ev.tool_calls {
                    if ttft_ms.is_none() {
                        ttft_ms = Some(start.elapsed().as_millis() as u64);
                    }
                    let entry = tool_calls_by_index.entry(tc.index).or_default();
                    if let Some(id) = tc.id
                        && !id.is_empty()
                    {
                        entry.id = id;
                    }
                    if let Some(name) = tc.name
                        && !name.is_empty()
                    {
                        let was_empty = entry.name.is_empty();
                        entry.name.push_str(&name);
                        if was_empty {
                            state.resumable_stream_registry.lock().append_chunk(
                                &stream_session_key,
                                vox_orchestrator::models::spec::TurnStreamEventKind::ToolCallStart {
                                    id: entry.id.clone(),
                                    name: entry.name.clone(),
                                },
                                entry.name.clone(),
                            );
                        }
                    }
                    if let Some(args) = tc.arguments_chunk
                        && !args.is_empty()
                    {
                        entry.arguments_raw.push_str(&args);
                        if entry.prefetched_args.is_none()
                            && let Some(valid_args) =
                                try_parse_streaming_read_prefetch(&entry.name, &entry.arguments_raw)
                        {
                            for path in super::context_and_memo::extract_normalized_paths_from_args(
                                &valid_args,
                            ) {
                                let _ = std::fs::metadata(&path);
                            }
                            entry.prefetched_args = Some(valid_args);
                        }
                    }
                }
            }
            // Mid-stream transport failure: fall back rather than surfacing a
            // partial/garbled answer as final.
            Err(_) => return None,
        }
    }

    let assembled_tool_calls: Vec<serde_json::Value> = tool_calls_by_index
        .into_iter()
        .enumerate()
        .filter_map(|(fallback_idx, (_idx, acc))| {
            if acc.name.trim().is_empty() {
                return None;
            }
            let id = if acc.id.trim().is_empty() {
                format!("stream_call_{fallback_idx}")
            } else {
                acc.id
            };
            let arguments: serde_json::Value = if let Some(prefetched) = acc.prefetched_args {
                prefetched
            } else if acc.arguments_raw.trim().is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(&acc.arguments_raw).unwrap_or(serde_json::Value::Null)
            };
            Some(serde_json::json!({
                "id": id,
                "name": acc.name,
                "arguments": arguments,
            }))
        })
        .collect();

    if content.trim().is_empty() && assembled_tool_calls.is_empty() {
        return None;
    }
    if let Some((_, completed)) = state.resumable_streams.lock().get_mut(&stream_session_key) {
        *completed = assembled_tool_calls.is_empty();
    }
    if assembled_tool_calls.is_empty() {
        state.resumable_stream_registry.lock().append_chunk(
            &stream_session_key,
            vox_orchestrator::models::spec::TurnStreamEventKind::Finish {
                reason: "stop".to_string(),
            },
            String::new(),
        );
    }

    let latency_ms = start.elapsed().as_millis() as u64;
    let prompt_tokens = streamed_prompt_tokens.unwrap_or(0);
    let completion_tokens = streamed_completion_tokens.unwrap_or_else(|| {
        vox_orchestrator::compaction::CompactionEngine::estimate_tokens(&content) as u32
    });
    let cache_read_tokens = streamed_cache_read_tokens.unwrap_or(0);
    let tpot_ms = ttft_ms.and_then(|t| {
        (completion_tokens > 0)
            .then(|| latency_ms.saturating_sub(t) as f64 / completion_tokens as f64)
    });
    let tool_calls = if assembled_tool_calls.is_empty() {
        None
    } else {
        serde_json::from_value(serde_json::Value::Array(assembled_tool_calls)).ok()
    };
    let cost_usd = {
        let models_arc = state.orchestrator.models_handle();
        let registry = vox_orchestrator::sync_lock::rw_read(&models_arc);
        registry.get(&model).map(|spec| {
            let billable_prompt = prompt_tokens.saturating_sub(cache_read_tokens);
            let input_cost = (f64::from(billable_prompt) / 1_000.0) * spec.cost_per_1k_input
                + (f64::from(cache_read_tokens) / 1_000.0) * (spec.cost_per_1k_input * 0.10);
            let output_cost = (f64::from(completion_tokens) / 1_000.0) * spec.cost_per_1k_output;
            input_cost + output_cost
        })
    };
    Some(LlmResponse {
        content,
        prompt_tokens,
        completion_tokens,
        model,
        cost_usd,
        tool_calls,
        latency_ms,
        cache_read_tokens,
        ttft_ms,
        tpot_ms,
    })
}

/// Phase 181 (`STREAM-EXEC-01`): Detect when an in-flight streaming `delta.tool_calls` accumulator
/// has completed a valid JSON object for a read-only (`http_read_role_eligible`) tool call so the
/// harness can speculatively prefetch/validate it before the rest of the SSE stream finishes.
#[must_use]
pub fn try_parse_streaming_read_prefetch(
    tool_name: &str,
    arguments_raw: &str,
) -> Option<serde_json::Value> {
    let trimmed = arguments_raw.trim();
    if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
        return None;
    }
    if !super::batch_and_eval_gate::is_read_only_memoizable(tool_name.trim()) {
        return None;
    }
    let val: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    val.is_object().then_some(val)
}

/// Phase 168 (`SOTA-CASCADE-DEESC-02`): Detect when accumulated streaming text starts with a
/// raw tool-call JSON/XML envelope so `stream_final_answer` suppresses `TokenStreamed` UI emission
/// before the post-stream salvage pass converts it into structured `tool_calls`.
#[must_use]
pub(crate) fn looks_like_streamed_tool_call_envelope(accumulated_content: &str) -> bool {
    let trimmed = accumulated_content.trim_start();
    if trimmed.starts_with("<tool_call>") || trimmed.starts_with("```tool:") {
        return true;
    }
    let body = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```tool_call"))
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .trim_start();
    let body = if let Some(after_arr) = body.strip_prefix('[') {
        after_arr.trim_start()
    } else {
        body
    };
    body.starts_with(r#"{"name""#)
        || body.starts_with(r#"{ "name""#)
        || body.starts_with(r#"{"tool_name""#)
        || body.starts_with(r#"{ "tool_name""#)
        || body.starts_with(r#"{"tool""#)
        || body.starts_with(r#"{ "tool""#)
        || body.starts_with(r#"{"type":"function""#)
        || body.starts_with(r#"{"type": "function""#)
        || body.starts_with(r#"{"type":"tool_use""#)
        || body.starts_with(r#"{"type": "tool_use""#)
        || body.starts_with(r#"{"method":"tools/call""#)
        || body.starts_with(r#"{"method": "tools/call""#)
        || body.starts_with(r#"{"jsonrpc""#)
}

/// Phase 165 (`SOTA-UNIVERSAL-SALVAGE-01..03`): Normalize a single JSON object from any of the
/// 5 standard LLM tool-call dialects into the canonical `{"id", "name", "arguments"}` envelope,
/// resolving wire aliases via `crate::tool_aliases::canonical_tool_name` and unwrapping
/// stringified JSON `"arguments"` into `Value::Object`.
fn normalize_salvaged_tool_call_item(
    v: &serde_json::Value,
    tool_defs: &[LlmToolDef],
) -> Option<serde_json::Value> {
    let obj = v.as_object()?;
    let (raw_name, raw_args) = if let Some(fn_obj) = obj.get("function").and_then(|f| f.as_object())
    {
        // OpenAI `{"type":"function","function":{"name":...,"arguments":...}}`
        let name = fn_obj.get("name")?.as_str()?;
        let args = fn_obj
            .get("arguments")
            .or_else(|| fn_obj.get("parameters"))
            .or_else(|| fn_obj.get("input"));
        (name, args)
    } else if let Some(params_obj) = obj.get("params").and_then(|p| p.as_object()) {
        // MCP JSON-RPC `{"method":"tools/call","params":{"name":...,"arguments":...}}`
        let name = params_obj
            .get("name")
            .or_else(|| params_obj.get("tool"))
            .and_then(|n| n.as_str())?;
        let args = params_obj
            .get("arguments")
            .or_else(|| params_obj.get("input"))
            .or_else(|| params_obj.get("args"));
        (name, args)
    } else {
        // Standard `{"name","arguments"}`, MENS `{"tool_name","arguments"}`/`{"tool","args"}`,
        // or Anthropic `{"type":"tool_use","name","input"}`
        let name = obj
            .get("name")
            .or_else(|| obj.get("tool_name"))
            .or_else(|| obj.get("tool"))
            .and_then(|n| n.as_str())?;
        let args = obj
            .get("arguments")
            .or_else(|| obj.get("input"))
            .or_else(|| obj.get("args"))
            .or_else(|| obj.get("parameters"));
        (name, args)
    };

    let trimmed_name = raw_name.trim();
    let canonical = crate::tool_aliases::canonical_tool_name(trimmed_name);
    let ide_alias = match canonical {
        "edit_file" | "apply_edit" | "str_replace" | "str_replace_editor" => {
            "vox_apply_structured_edit"
        }
        other => other,
    };
    let prefixed = format!("vox_{trimmed_name}");
    let offered_name = tool_defs
        .iter()
        .find(|t| {
            t.name == canonical
                || t.name == ide_alias
                || t.name == trimmed_name
                || t.name == prefixed
        })
        .map(|t| t.name.clone())?;

    let arguments = match raw_args {
        Some(serde_json::Value::String(s)) => serde_json::from_str::<serde_json::Value>(s.trim())
            .unwrap_or_else(|_| serde_json::Value::String(s.clone())),
        Some(other) => other.clone(),
        None => serde_json::json!({}),
    };

    Some(serde_json::json!({
        "id": format!("salvaged-{}", uuid::Uuid::new_v4()),
        "name": offered_name,
        "arguments": arguments,
    }))
}

fn collect_salvaged_from_value(
    v: &serde_json::Value,
    tool_defs: &[LlmToolDef],
    out: &mut Vec<serde_json::Value>,
) {
    match v {
        serde_json::Value::Array(items) => {
            for item in items {
                collect_salvaged_from_value(item, tool_defs, out);
            }
        }
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Array(tc_arr)) = map.get("tool_calls") {
                for item in tc_arr {
                    collect_salvaged_from_value(item, tool_defs, out);
                }
            } else if let Some(norm) = normalize_salvaged_tool_call_item(v, tool_defs) {
                out.push(norm);
            }
        }
        _ => {}
    }
}

/// Phase 165 (`SOTA-UNIVERSAL-SALVAGE-01..03`): Multi-format and multi-call salvage parser.
/// Extracts all valid tool calls embedded in prose (`<tool_call>...</tool_call>`, fenced JSON
/// blocks, JSON arrays `[{...}, {...}]`, or bare JSON objects) across OpenAI, Anthropic, MENS,
/// and MCP JSON-RPC tool-call dialects.
#[must_use]
pub(crate) fn salvage_tool_calls_from_text(
    text: &str,
    tool_defs: &[LlmToolDef],
) -> Vec<serde_json::Value> {
    if text.trim().is_empty() || tool_defs.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();

    // Tier 1: `<tool_call>...</tool_call>` XML wrappers (Hermes / Qwen / DeepSeek / MENS)
    static XML_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let xml_re = XML_RE.get_or_init(|| {
        regex::Regex::new(r"(?s)<tool_call>\s*(.*?)\s*</tool_call>").expect("static regex")
    });
    for caps in xml_re.captures_iter(text) {
        let inner = caps[1]
            .trim()
            .strip_prefix("```json")
            .or_else(|| caps[1].trim().strip_prefix("```"))
            .unwrap_or(caps[1].trim())
            .trim_end_matches("```")
            .trim();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(inner) {
            collect_salvaged_from_value(&v, tool_defs, &mut out);
        }
    }
    if !out.is_empty() {
        return out;
    }

    // Tier 1.5 (`T3-SALVAGE-RULES-01`): `pi-t3chat` ```tool:<name>\n{...}\n``` blocks
    // (must match `looks_like_streamed_tool_call_envelope`'s `trimmed.starts_with("```tool:")`)
    static TOOL_COLON_FENCE_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let tool_colon_re = TOOL_COLON_FENCE_RE.get_or_init(|| {
        regex::Regex::new(r"(?s)```tool:([A-Za-z0-9_.:-]+)\s*\r?\n(.*?)\s*```")
            .expect("static regex")
    });
    for caps in tool_colon_re.captures_iter(text) {
        let tool_name = caps[1].trim();
        let inner = caps[2].trim();
        let parsed_args = serde_json::from_str::<serde_json::Value>(inner).or_else(|_| {
            inner
                .find('{')
                .and_then(|pos| enclosing_json_object(inner, pos))
                .ok_or_else(|| serde_json::Error::io(std::io::Error::other("no json object")))
                .and_then(serde_json::from_str::<serde_json::Value>)
        });
        if let Ok(args_val) = parsed_args {
            let synthetic = if args_val.get("name").is_some()
                || args_val.get("tool_name").is_some()
                || args_val.get("function").is_some()
            {
                args_val
            } else {
                serde_json::json!({
                    "name": tool_name,
                    "arguments": args_val,
                })
            };
            collect_salvaged_from_value(&synthetic, tool_defs, &mut out);
        }
    }
    if !out.is_empty() {
        return out;
    }

    // Tier 2: Fenced ```json / ``` blocks (single object, array of objects, or multiple blocks)
    static FENCE_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let fence_re = FENCE_RE.get_or_init(|| {
        regex::Regex::new(r"(?s)```(?:json|tool_call|xml)?\s*(.*?)\s*```").expect("static regex")
    });
    for caps in fence_re.captures_iter(text) {
        let inner = caps[1].trim();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(inner) {
            collect_salvaged_from_value(&v, tool_defs, &mut out);
        }
    }
    if !out.is_empty() {
        return out;
    }

    // Tier 3: Full trimmed text is valid JSON (e.g. bare array `[{...}, {...}]` or bare object)
    let trimmed = text.trim();
    if (trimmed.starts_with('[') || trimmed.starts_with('{'))
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed)
    {
        collect_salvaged_from_value(&v, tool_defs, &mut out);
        if !out.is_empty() {
            return out;
        }
    }

    // Tier 4: Bare JSON objects embedded in prose
    let bytes = text.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        if bytes[idx] == b'{' {
            if let Some(obj_src) = enclosing_json_object(text, idx) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(obj_src) {
                    collect_salvaged_from_value(&v, tool_defs, &mut out);
                }
                idx += obj_src.len().max(1);
                continue;
            }
        }
        idx += 1;
    }
    out
}

/// Given a byte offset known to sit inside a JSON object's key/value area, walk
/// backward to that object's opening `{` (skipping over any nested `{...}`
/// closed before reaching it) and forward to the matching closing `}`, and
/// return the enclosing `text` slice. `None` if the braces are unbalanced.
fn enclosing_json_object(text: &str, pos: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut start = None;
    let mut i = pos;
    loop {
        match bytes.get(i) {
            Some(b'}') => depth += 1,
            Some(b'{') => {
                if depth == 0 {
                    start = Some(i);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    let start = start?;
    let mut depth = 0i32;
    for (j, b) in bytes.iter().enumerate().skip(start) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return text.get(start..=j);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod t3_salvage_tests {
    use super::*;

    #[test]
    fn salvages_pi_t3chat_tool_colon_fence_matching_stream_suppression_envelope() {
        let raw =
            "I will read Cargo.toml now.\n```tool:vox_read_file\n{\"path\": \"Cargo.toml\"}\n```";
        assert!(looks_like_streamed_tool_call_envelope(
            "```tool:vox_read_file\n{\"path\": \"Cargo.toml\"}\n```"
        ));
        let tool_defs = vec![LlmToolDef {
            name: "vox_read_file".to_string(),
            description: Some("Read a file".to_string()),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let salvaged = salvage_tool_calls_from_text(raw, &tool_defs);
        assert_eq!(salvaged.len(), 1);
        assert_eq!(salvaged[0]["name"], "vox_read_file");
        assert_eq!(salvaged[0]["arguments"]["path"], "Cargo.toml");
    }
}
