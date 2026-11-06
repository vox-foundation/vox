use super::*;

/// Extract the clean user question from a context-assembled `user_message`
/// (stripping `[CONVERSATION HISTORY]`, `[AUTONOMOUS RESEARCH]`, `[ACTIVE FILE]`,
/// and `Objectives (treat as a single set...` boilerplate) so spoke classification
/// and BM25 tool ranking score only the current turn's utterance (`SOTA-PROMPT-03`).
#[must_use]
pub(crate) fn extract_raw_utterance_for_tool_ranking(user_message: &str) -> &str {
    // If canonicalized or context-prefixed, the current utterance sits at the tail after the last double newline
    // or after "1. " in the Objectives block.
    let tail = if let Some(idx) = user_message.rfind("\n\nObjectives (treat as a single set") {
        &user_message[idx + 2..]
    } else if user_message.starts_with('[')
        && let Some(idx) = user_message.rfind("\n\n")
    {
        &user_message[idx + 2..]
    } else {
        user_message
    };
    if let Some(after_obj) = tail
        .strip_prefix("Objectives (treat as a single set; order does not imply priority):\n\n1. ")
    {
        after_obj.trim()
    } else {
        tail.trim()
    }
}

/// Phase 150 (`SOTA-INTRATURN-01`): Maximum character length of a single tool result
/// before head+tail+digest capping is applied inside `run_agent_turn`.
pub(crate) const MAX_SINGLE_TOOL_OUTPUT_CHARS: usize = 6_000;

/// Phase 150 (`SOTA-INTRATURN-01`): Bound an oversized single tool result while preserving
/// both its head (3,200 chars), tail (2,200 chars — including inline verification blocks),
/// and a `blake3` content digest. Returns `(bounded_content, estimated_tokens_saved)`.
#[must_use]
pub(crate) fn cap_and_summarize_tool_output(tool_name: &str, content: &str) -> (String, usize) {
    // T3 Wave 4 (`T3-LOOP-CHECKPOINT-02`): Fold `\r` terminal progress-bar overwrites first,
    // and apply 3-pass DOM snapshot shedding (`<= 20 KiB`) when output is a DOM/HTML snapshot.
    let normalized = if content.contains('\r') {
        vox_orchestrator::command_output_hub::normalize_terminal_output(content)
    } else {
        content.to_string()
    };
    let dom_shed = if tool_name.contains("snapshot")
        || tool_name.contains("html")
        || tool_name.contains("dom")
        || (normalized.len() > crate::mcp_tool_access::MAX_DOM_SNAPSHOT_BYTES
            && (normalized.contains("data:image/") || normalized.contains("<html")))
    {
        crate::mcp_tool_access::bound_snapshot_metadata(&normalized)
    } else {
        normalized
    };
    let content = dom_shed.as_str();
    if content.len() <= MAX_SINGLE_TOOL_OUTPUT_CHARS {
        return (dom_shed, 0);
    }
    let before_tokens = vox_orchestrator::compaction::CompactionEngine::estimate_tokens(content)
        .max(content.len() / 4);
    let mut head_end = 3_200.min(content.len());
    while head_end > 0 && !content.is_char_boundary(head_end) {
        head_end -= 1;
    }
    let mut tail_start = content.len().saturating_sub(2_200).max(head_end);
    while tail_start < content.len() && !content.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    let omitted_slice = &content[head_end..tail_start];
    let omitted = omitted_slice.len();
    let digest = vox_crypto::hex_encode(&vox_crypto::secure_hash(content.as_bytes()));
    let short_digest = &digest[..16.min(digest.len())];
    let has_code_sigs = [concat!(" ", "f", "n "), " struct ", " enum ", " interface "]
        .iter()
        .any(|kw| omitted_slice.contains(kw));
    let ast_skeleton_block = if has_code_sigs {
        let folded = super::ast_slice_and_preflight::fold_code_to_ast_skeleton(omitted_slice, 900);
        format!("\n[OMITTED REGION AST SKELETON]:\n{}", folded.trim())
    } else {
        String::new()
    };
    let capped = format!(
        "{}\n...[INTRA-TURN TOOL OUTPUT CAPPED: tool={tool_name}, omitted_chars={omitted}, blake3={short_digest}]{ast_skeleton_block}...\n{}",
        &content[..head_end],
        &content[tail_start..]
    );
    let after_tokens = vox_orchestrator::compaction::CompactionEngine::estimate_tokens(&capped)
        .max(capped.len() / 4);
    (capped, before_tokens.saturating_sub(after_tokens))
}

/// Phase 150 (`SOTA-INTRATURN-02`): Replace verbose `role == "tool"` messages from
/// `>= 2` iterations ago with compact `blake3`-addressed observation placeholders
/// so multi-step tool loops do not re-send multi-kilobyte payloads on every step.
pub(crate) fn mask_stale_intraturn_tool_messages(
    messages: &mut [LlmChatMessage],
    tool_msg_iterations: &[(usize, usize, String)],
    current_iteration: usize,
) -> (usize, usize) {
    let mut masked_count = 0usize;
    let mut tokens_saved = 0usize;
    for (msg_idx, iter_idx, tool_name) in tool_msg_iterations {
        if *iter_idx + 2 > current_iteration {
            continue;
        }
        let Some(msg) = messages.get_mut(*msg_idx) else {
            continue;
        };
        if msg.role != "tool"
            || msg.content.len() <= 360
            || msg
                .content
                .starts_with("[INTRA-TURN TOOL OBSERVATION MASKED:")
        {
            continue;
        }
        let before_tokens =
            vox_orchestrator::compaction::CompactionEngine::estimate_tokens(&msg.content)
                .max(msg.content.len() / 4);
        let digest = vox_crypto::hex_encode(&vox_crypto::secure_hash(msg.content.as_bytes()));
        let short_digest = &digest[..16.min(digest.len())];
        let mut preview_end = 140.min(msg.content.len());
        while preview_end > 0 && !msg.content.is_char_boundary(preview_end) {
            preview_end -= 1;
        }
        let preview = msg.content[..preview_end].replace('\n', " ");
        let replacement = format!(
            "[INTRA-TURN TOOL OBSERVATION MASKED: tool={tool_name}, iter={iter_idx}, blake3={short_digest}] {preview}..."
        );
        let after_tokens =
            vox_orchestrator::compaction::CompactionEngine::estimate_tokens(&replacement)
                .max(replacement.len() / 4);
        if before_tokens > after_tokens {
            msg.content = replacement;
            masked_count += 1;
            tokens_saved += before_tokens - after_tokens;
        }
    }
    (masked_count, tokens_saved)
}

/// Phase 151 (`SOTA-SCHEMA-01`): Number of top-ranked primary tools that keep full verbose
/// property descriptions; secondary tools (`rank >= 8`) strip recursive property description/title metadata.
pub(crate) const PRIMARY_FULL_SCHEMA_TOOLS_CAP: usize = 8;

/// Recursively strip `"description"`, `"title"`, `"examples"`, and `"$schema"` metadata from a
/// JSON Schema value while preserving all structural type, required, enum, and nested object/array constraints.
pub(crate) fn strip_schema_descriptions_recursive(val: &mut serde_json::Value) {
    match val {
        serde_json::Value::Object(map) => {
            map.remove("description");
            map.remove("title");
            map.remove("examples");
            map.remove("$schema");
            map.remove("additionalProperties");
            for child in map.values_mut() {
                strip_schema_descriptions_recursive(child);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                strip_schema_descriptions_recursive(item);
            }
        }
        _ => {}
    }
}

/// Phase 151 (`SOTA-SCHEMA-01`): Build a two-tier progressive `LlmToolDef` based on rank.
#[must_use]
pub(crate) fn compact_tool_def_for_rank(
    rank: usize,
    entry: &vox_mcp_registry::McpToolRegistryEntry,
    raw_schema: serde_json::Map<String, serde_json::Value>,
) -> LlmToolDef {
    if rank < PRIMARY_FULL_SCHEMA_TOOLS_CAP {
        return LlmToolDef {
            name: entry.name.to_string(),
            description: Some(entry.description.to_string()),
            parameters: serde_json::Value::Object(raw_schema),
        };
    }
    let mut parameters = serde_json::Value::Object(raw_schema);
    strip_schema_descriptions_recursive(&mut parameters);
    let first_sentence = entry
        .description
        .split(". ")
        .next()
        .unwrap_or(entry.description)
        .trim_end_matches('.');
    let mut end = 64.min(first_sentence.len());
    while end > 0 && !first_sentence.is_char_boundary(end) {
        end -= 1;
    }
    let short_desc = &first_sentence[..end];
    LlmToolDef {
        name: entry.name.to_string(),
        description: Some(short_desc.to_string()),
        parameters,
    }
}

/// Phase 160 (`SOTA-PATH-MEMO-01`): Normalize tool argument JSON for deterministic memo keys
/// by stripping trailing `/` or `\` on path strings and serializing with sorted keys.
#[must_use]
pub(crate) fn canonicalize_tool_args_json(val: &serde_json::Value) -> String {
    fn normalize_val(v: &serde_json::Value, is_path_key: bool) -> serde_json::Value {
        match v {
            serde_json::Value::Object(map) => {
                let mut sorted: std::collections::BTreeMap<String, serde_json::Value> =
                    std::collections::BTreeMap::new();
                for (k, child) in map {
                    let path_key = matches!(
                        k.as_str(),
                        "path" | "file_path" | "target" | "source_path" | "dest_path"
                    );
                    sorted.insert(k.clone(), normalize_val(child, path_key));
                }
                serde_json::to_value(sorted).unwrap_or_default()
            }
            serde_json::Value::Array(arr) => serde_json::Value::Array(
                arr.iter()
                    .map(|item| normalize_val(item, is_path_key))
                    .collect(),
            ),
            serde_json::Value::String(s) if is_path_key => {
                let trimmed = s.trim();
                let stripped = trimmed
                    .strip_prefix("./")
                    .or_else(|| trimmed.strip_prefix(".\\"))
                    .unwrap_or(trimmed)
                    .trim_end_matches(['/', '\\']);
                serde_json::Value::String(if stripped.is_empty() {
                    trimmed.to_string()
                } else {
                    stripped.to_string()
                })
            }
            other => other.clone(),
        }
    }
    serde_json::to_string(&normalize_val(val, false)).unwrap_or_else(|_| val.to_string())
}

/// Extract normalized path targets from a tool call's arguments JSON.
#[must_use]
pub(crate) fn extract_normalized_paths_from_args(args: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(obj) = args.as_object() else {
        return out;
    };
    for key in [
        "path",
        "file_path",
        "target",
        "source_path",
        "dest_path",
        "files",
        "paths",
    ] {
        if let Some(val) = obj.get(key) {
            if let Some(s) = val.as_str() {
                let clean = s
                    .trim()
                    .strip_prefix("./")
                    .unwrap_or(s.trim())
                    .trim_end_matches(['/', '\\']);
                if !clean.is_empty() {
                    out.push(clean.to_string());
                }
            } else if let Some(arr) = val.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        let clean = s
                            .trim()
                            .strip_prefix("./")
                            .unwrap_or(s.trim())
                            .trim_end_matches(['/', '\\']);
                        if !clean.is_empty() {
                            out.push(clean.to_string());
                        }
                    }
                }
            }
        }
    }
    out
}

/// Phase 160 (`SOTA-PATH-MEMO-02..03`): Path-scoped invalidation of `read_tool_memo` when a write tool executes.
/// Evicts only memo entries referencing the written path(s) (or non-path-scoped workspace tools like `vox_repo_status`),
/// preserving memoized reads of unrelated files. Falls back to full clear when the write tool has no extractable path.
pub(crate) fn invalidate_read_memo_for_write(
    memo: &mut std::collections::HashMap<(String, String), (usize, String, String)>,
    write_args: &serde_json::Value,
) -> usize {
    if memo.is_empty() {
        return 0;
    }
    let write_paths = extract_normalized_paths_from_args(write_args);
    if write_paths.is_empty() {
        let count = memo.len();
        memo.clear();
        return count;
    }
    let before = memo.len();
    memo.retain(|(_tool_name, canon_args_str), _| {
        let Ok(parsed_args) = serde_json::from_str::<serde_json::Value>(canon_args_str) else {
            return false;
        };
        let read_paths = extract_normalized_paths_from_args(&parsed_args);
        if read_paths.is_empty() {
            // Non-file-scoped read tool (e.g. `vox_repo_status` with `{}`): evict on any write
            return false;
        }
        !read_paths.iter().any(|rp| {
            write_paths
                .iter()
                .any(|wp| rp == wp || rp.ends_with(wp) || wp.ends_with(rp))
        })
    });
    before.saturating_sub(memo.len())
}

/// Phase 161 & 163 (`SOTA-FAST-PREAMBLE-01..02`, `SOTA-WRITE-PRIORITY-03`): Build a compact
/// line-numbered `[TARGET FILE CONTEXT WINDOW]` around a failed edit's target file so the model
/// can self-repair on the next iteration without an extra read round-trip.
#[must_use]
pub(crate) fn build_edit_failure_recovery_window(
    workspace_root: Option<&std::path::Path>,
    tool_name: &str,
    args: &serde_json::Value,
) -> Option<String> {
    if !matches!(
        tool_name,
        "vox_apply_structured_edit"
            | "vox_generate_code"
            | "vox_edit_file"
            | "vox_write_file"
            | "vox_multi_edit"
            | "vox_patch_file"
    ) {
        return None;
    }
    let paths = extract_normalized_paths_from_args(args);
    let rel_or_abs = paths.first()?;
    let candidate_path = std::path::Path::new(rel_or_abs);
    let resolved = if candidate_path.is_absolute() {
        candidate_path.to_path_buf()
    } else if let Some(root) = workspace_root {
        root.join(candidate_path)
    } else {
        candidate_path.to_path_buf()
    };
    let raw = std::fs::read_to_string(&resolved).ok()?;
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() {
        return None;
    }
    let needle = args
        .get("target_content")
        .or_else(|| args.get("old_str"))
        .or_else(|| args.get("target"))
        .or_else(|| args.get("search"))
        .or_else(|| args.get("replacement_code"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.lines().find(|l| !l.trim().is_empty()))
        .map(str::trim);
    let center_idx = needle
        .and_then(|n| lines.iter().position(|line| line.contains(n)))
        .unwrap_or(0);
    let start = center_idx.saturating_sub(15);
    let end = (start + 40).min(lines.len());
    let mut snippet = String::new();
    for (idx, line) in lines[start..end].iter().enumerate() {
        let line_no = start + idx + 1;
        snippet.push_str(&format!("{line_no:>4} | {line}\n"));
    }
    Some(format!(
        "\n[TARGET FILE CONTEXT WINDOW (`{rel_or_abs}`, lines {}..={})]:\n{}",
        start + 1,
        end,
        snippet.trim_end()
    ))
}

/// Phase 164 (`SOTA-PREFIX-IMMUTABLE-02`): Overflow token threshold above which stale intra-turn
/// tool outputs are masked even on prefix-KV-cached frontier providers.
pub(crate) const KV_CACHE_OVERFLOW_MASK_THRESHOLD_TOKENS: u64 = 24_000;

/// Phase 164 (`SOTA-PREFIX-IMMUTABLE-01`): Detect whether the configured provider/model uses
/// automatic prefix KV-caching (Anthropic, Gemini, DeepSeek, OpenAI, or OpenRouter routing to
/// those model families). When `true` and under `KV_CACHE_OVERFLOW_MASK_THRESHOLD_TOKENS`,
/// prior-iteration tool messages are kept byte-identical so the prefix KV-cache is never invalidated.
#[must_use]
pub(crate) fn provider_uses_prefix_kv_cache(cfg: &LlmConfig) -> bool {
    let p = cfg.provider.trim().to_ascii_lowercase();
    let m = cfg.model.trim().to_ascii_lowercase();
    if matches!(
        p.as_str(),
        "anthropic" | "gemini" | "google" | "deepseek" | "openai"
    ) {
        return true;
    }
    if p == "openrouter" {
        return m.contains("claude")
            || m.contains("gemini")
            || m.contains("deepseek")
            || m.contains("gpt-")
            || m.contains("o1")
            || m.contains("o3")
            || m.contains("o4");
    }
    false
}

/// Phase 168 (`SOTA-CASCADE-DEESC-01`): Determine whether a speculative repair escalation
/// (`repair_escalated == true`) should de-escalate back to the fast primary model after a clean
/// iteration in which every dispatched tool call succeeded with zero inline verification failures.
#[must_use]
pub(crate) const fn should_deescalate_repair_cascade(
    was_escalated_at_iter_start: bool,
    iter_all_calls_clean: bool,
) -> bool {
    was_escalated_at_iter_start && iter_all_calls_clean
}
