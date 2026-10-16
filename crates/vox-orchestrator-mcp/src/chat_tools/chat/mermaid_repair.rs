//! Autonomous Mermaid diagram observation, MCP validation tool (`vox_validate_mermaid`),
//! and budgeted model-selected self-repair (`MERMAID-01..03`).
//!
//! Works across two complementary surfaces:
//! 1. **Explicit / Dispatched MCP Tool (`vox_validate_mermaid`)**: validates standalone Mermaid
//!    code or Markdown containing ` ```mermaid ` blocks, returning line-accurate diagnostics and
//!    a repaired version.
//! 2. **Automatic Agent-Turn Observation (`observe_and_repair_turn_mermaid`)**: intercepts
//!    assistant responses containing ` ```mermaid ` fences before turn completion, dispatches
//!    `vox_validate_mermaid`, checks [`vox_orchestrator::budget::BudgetSignal`], routes repair
//!    model selection through [`vox_orchestrator::models::SelectionIntent::mermaid_repair`], and
//!    repairs broken diagrams automatically.

use serde::{Deserialize, Serialize};
use vox_actor_runtime::llm::{LlmChatMessage, LlmConfig};
use vox_actor_runtime::{ActivityOptions, ActivityResult};

use crate::params::ToolResult;
use crate::server_state::ServerState;

/// Canonical supported Mermaid diagram headers across Vox docs and chat surfaces.
pub const SUPPORTED_DIAGRAM_HEADERS: &[&str] = &[
    "flowchart",
    "graph",
    "sequenceDiagram",
    "stateDiagram-v2",
    "stateDiagram",
    "classDiagram",
    "erDiagram",
    "xychart-beta",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MermaidDiagnostic {
    /// 1-indexed line number inside the Mermaid block (or file when extracted from Markdown).
    pub line: usize,
    /// Stable rule code (e.g. `mermaid/unquoted-edge-label`).
    pub code: String,
    /// Human- and LLM-readable diagnostic message.
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MermaidValidationResult {
    pub valid: bool,
    pub diagram_type: Option<String>,
    pub diagnostics: Vec<MermaidDiagnostic>,
    /// Deterministically repaired Mermaid source when all detected issues are auto-repairable.
    pub repaired_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MermaidBlock {
    /// 1-indexed line number of the opening ` ```mermaid ` fence in the source file.
    pub fence_start_line: usize,
    /// 1-indexed line number of the closing ` ``` ` fence in the source file.
    pub fence_end_line: usize,
    /// Raw Mermaid content inside the fence.
    pub code: String,
}

/// Extract all fenced ` ```mermaid ` blocks from a Markdown/MDX document.
#[must_use]
pub fn extract_mermaid_blocks(markdown: &str) -> Vec<MermaidBlock> {
    let mut blocks = Vec::new();
    let mut in_fence = false;
    let mut is_mermaid = false;
    let mut fence_backticks = 0usize;
    let mut start_line = 0usize;
    let mut buf = String::new();

    for (idx, line) in markdown.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim_start();
        let ticks = trimmed.chars().take_while(|&c| c == '`').count();
        let after = trimmed[ticks..].trim();
        let is_fence_line = ticks >= 3
            && (after.is_empty()
                || after
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ',' || c == '.'));

        if !in_fence {
            if is_fence_line {
                in_fence = true;
                fence_backticks = ticks;
                let lang = after.split(',').next().unwrap_or("").trim();
                is_mermaid = lang.eq_ignore_ascii_case("mermaid");
                start_line = line_no;
                buf.clear();
            }
        } else if is_fence_line && ticks >= fence_backticks && after.is_empty() {
            if is_mermaid {
                blocks.push(MermaidBlock {
                    fence_start_line: start_line,
                    fence_end_line: line_no,
                    code: buf.clone(),
                });
            }
            in_fence = false;
            is_mermaid = false;
            buf.clear();
        } else if is_mermaid {
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf.push_str(line);
        }
    }

    blocks
}

/// Validate a single Mermaid block and compute a deterministic repair if needed.
#[must_use]
pub fn validate_and_repair_mermaid(code: &str) -> MermaidValidationResult {
    let mut diagnostics = Vec::new();
    let mut repaired_lines: Vec<String> = Vec::new();
    let mut changed = false;

    let mut diagram_type: Option<String> = None;
    let mut subgraph_depth: isize = 0;

    for (idx, raw_line) in code.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim();

        if trimmed.is_empty() || trimmed.starts_with("%%") {
            repaired_lines.push(raw_line.to_string());
            continue;
        }

        if diagram_type.is_none() {
            let first_token = trimmed.split_whitespace().next().unwrap_or("");
            if SUPPORTED_DIAGRAM_HEADERS.contains(&first_token) {
                diagram_type = Some(first_token.to_string());
                repaired_lines.push(raw_line.to_string());
                continue;
            }
            diagnostics.push(MermaidDiagnostic {
                line: line_no,
                code: "mermaid/unsupported-or-missing-header".to_string(),
                message: format!(
                    "Mermaid diagram must start with a supported header ({}), found {:?}",
                    SUPPORTED_DIAGRAM_HEADERS.join(", "),
                    first_token
                ),
            });
            repaired_lines.push(raw_line.to_string());
            continue;
        }

        let first_token = trimmed.split_whitespace().next().unwrap_or("");
        if first_token == "subgraph" {
            subgraph_depth += 1;
        } else if trimmed == "end" || trimmed.starts_with("end ") {
            subgraph_depth -= 1;
            if subgraph_depth < 0 {
                diagnostics.push(MermaidDiagnostic {
                    line: line_no,
                    code: "mermaid/unbalanced-subgraph-end".to_string(),
                    message: "Encountered `end` without a matching `subgraph`".to_string(),
                });
                subgraph_depth = 0;
            }
        }

        let is_flow_or_graph = matches!(diagram_type.as_deref(), Some("flowchart" | "graph"));
        let mut working_line = raw_line.to_string();

        if is_flow_or_graph {
            if let Some(fixed) = repair_unquoted_edge_label(&working_line) {
                diagnostics.push(MermaidDiagnostic {
                    line: line_no,
                    code: "mermaid/unquoted-edge-label".to_string(),
                    message: "Edge label `-- ... -->` contains special characters `()[]{}` or `<>` and must be quoted (`-- \"...\" -->`)".to_string(),
                });
                working_line = fixed;
                changed = true;
            }

            if let Some(fixed) = repair_unquoted_node_labels(&working_line) {
                diagnostics.push(MermaidDiagnostic {
                    line: line_no,
                    code: "mermaid/unquoted-node-label".to_string(),
                    message: "Node label contains unquoted parentheses, braces, or HTML tags; quote as `id[\"Label\"]`".to_string(),
                });
                working_line = fixed;
                changed = true;
            }

            if has_unbalanced_brackets(&working_line) {
                diagnostics.push(MermaidDiagnostic {
                    line: line_no,
                    code: "mermaid/unbalanced-brackets".to_string(),
                    message: "Line has unbalanced brackets or quotes in node/edge definition"
                        .to_string(),
                });
            }
        }

        repaired_lines.push(working_line);
    }

    if diagram_type.is_none() {
        diagnostics.push(MermaidDiagnostic {
            line: 1,
            code: "mermaid/empty-diagram".to_string(),
            message: "Mermaid code block is empty".to_string(),
        });
    }

    if subgraph_depth > 0 {
        diagnostics.push(MermaidDiagnostic {
            line: code.lines().count().max(1),
            code: "mermaid/unclosed-subgraph".to_string(),
            message: format!(
                "{subgraph_depth} unclosed `subgraph` block(s); add matching `end` line(s)"
            ),
        });
        for _ in 0..subgraph_depth {
            repaired_lines.push("    end".to_string());
        }
        changed = true;
    }

    let repaired_candidate = repaired_lines.join("\n");
    let repaired_code = if changed {
        Some(repaired_candidate)
    } else {
        None
    };

    MermaidValidationResult {
        valid: diagnostics.is_empty(),
        diagram_type,
        diagnostics,
        repaired_code,
    }
}

/// Validate and repair all ` ```mermaid ` blocks in a Markdown document.
#[must_use]
pub fn validate_and_repair_markdown_mermaid(markdown: &str) -> (String, Vec<MermaidDiagnostic>) {
    let blocks = extract_mermaid_blocks(markdown);
    if blocks.is_empty() {
        return (markdown.to_string(), Vec::new());
    }

    let mut file_diagnostics = Vec::new();
    let lines: Vec<&str> = markdown.lines().collect();
    let mut out_lines: Vec<String> = Vec::with_capacity(lines.len());
    let mut cursor = 0usize;

    for block in blocks {
        let res = validate_and_repair_mermaid(&block.code);
        for d in &res.diagnostics {
            file_diagnostics.push(MermaidDiagnostic {
                line: block.fence_start_line + d.line,
                code: d.code.clone(),
                message: d.message.clone(),
            });
        }
        let open_idx = block.fence_start_line - 1;
        let close_idx = block.fence_end_line - 1;
        while cursor <= open_idx && cursor < lines.len() {
            out_lines.push(lines[cursor].to_string());
            cursor += 1;
        }
        if let Some(repaired) = res.repaired_code {
            for rline in repaired.lines() {
                out_lines.push(rline.to_string());
            }
        } else {
            while cursor < close_idx && cursor < lines.len() {
                out_lines.push(lines[cursor].to_string());
                cursor += 1;
            }
        }
        cursor = close_idx;
        if cursor < lines.len() {
            out_lines.push(lines[cursor].to_string());
            cursor += 1;
        }
    }

    while cursor < lines.len() {
        out_lines.push(lines[cursor].to_string());
        cursor += 1;
    }

    let mut rebuilt = out_lines.join("\n");
    if markdown.ends_with('\n') && !rebuilt.ends_with('\n') {
        rebuilt.push('\n');
    }
    (rebuilt, file_diagnostics)
}

fn repair_unquoted_edge_label(line: &str) -> Option<String> {
    let start_idx = line.find(" -- ")?;
    let after_start = &line[start_idx + 4..];
    let arrow_offset = after_start.find(" -->")?;
    let label = after_start[..arrow_offset].trim();
    if label.is_empty() || (label.starts_with('"') && label.ends_with('"')) {
        return None;
    }
    let has_special = label
        .chars()
        .any(|c| matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>'));
    if !has_special {
        return None;
    }
    let escaped = label.replace('"', "'");
    let mut out = String::with_capacity(line.len() + 4);
    out.push_str(&line[..start_idx]);
    out.push_str(" -- \"");
    out.push_str(&escaped);
    out.push_str("\" -->");
    out.push_str(&after_start[arrow_offset + 4..]);
    Some(out)
}

fn repair_unquoted_node_labels(line: &str) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len() + 8);
    let mut i = 0usize;
    let mut changed = false;

    while i < chars.len() {
        if chars[i] == '['
            && i > 0
            && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_')
            && chars.get(i + 1).copied() != Some('"')
            && chars.get(i + 1).copied() != Some('[')
            && chars.get(i + 1).copied() != Some('(')
        {
            let mut depth = 1usize;
            let mut j = i + 1;
            let mut in_quotes = false;
            while j < chars.len() && depth > 0 {
                if chars[j] == '"' {
                    in_quotes = !in_quotes;
                } else if !in_quotes {
                    if chars[j] == '[' {
                        depth += 1;
                    } else if chars[j] == ']' {
                        depth -= 1;
                    }
                }
                if depth > 0 {
                    j += 1;
                }
            }
            if depth == 0 && j > i + 1 {
                let inner: String = chars[i + 1..j].iter().collect();
                let needs_quote = inner
                    .chars()
                    .any(|c| matches!(c, '(' | ')' | '{' | '}' | '<' | '>'));
                if needs_quote {
                    let cleaned = inner
                        .replace(" <br/> ", " - ")
                        .replace(" <br> ", " - ")
                        .replace("<br/>", " - ")
                        .replace("<br>", " - ")
                        .replace('"', "'");
                    out.push_str("[\"");
                    out.push_str(cleaned.trim());
                    out.push_str("\"]");
                    changed = true;
                    i = j + 1;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }

    if changed { Some(out) } else { None }
}

fn has_unbalanced_brackets(line: &str) -> bool {
    let mut in_quotes = false;
    let mut sq = 0isize;
    let mut paren = 0isize;
    let mut curly = 0isize;
    for c in line.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            match c {
                '[' => sq += 1,
                ']' => sq -= 1,
                '(' => paren += 1,
                ')' => paren -= 1,
                '{' => curly += 1,
                '}' => curly -= 1,
                _ => {}
            }
        }
    }
    in_quotes || sq != 0 || paren != 0 || curly != 0
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidateMermaidParams {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub markdown: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidateMermaidResponse {
    pub valid: bool,
    pub diagram_type: Option<String>,
    pub count: usize,
    pub diagnostics: Vec<MermaidDiagnostic>,
    pub repaired_code: Option<String>,
    pub repaired_markdown: Option<String>,
}

/// MCP handler for `vox_validate_mermaid`.
pub async fn vox_validate_mermaid(args: serde_json::Value) -> String {
    let params: ValidateMermaidParams = match serde_json::from_value(args) {
        Ok(p) => p,
        Err(e) => {
            return ToolResult::<ValidateMermaidResponse>::err(format!(
                "invalid vox_validate_mermaid arguments: {e}"
            ))
            .to_json();
        }
    };

    if let Some(code) = params.code {
        let res = validate_and_repair_mermaid(&code);
        return ToolResult::ok(ValidateMermaidResponse {
            valid: res.valid,
            diagram_type: res.diagram_type,
            count: res.diagnostics.len(),
            diagnostics: res.diagnostics,
            repaired_code: res.repaired_code,
            repaired_markdown: None,
        })
        .to_json();
    }

    if let Some(md) = params.markdown {
        let (repaired_md, diags) = validate_and_repair_markdown_mermaid(&md);
        let changed = repaired_md != md;
        return ToolResult::ok(ValidateMermaidResponse {
            valid: diags.is_empty(),
            diagram_type: None,
            count: diags.len(),
            diagnostics: diags,
            repaired_code: None,
            repaired_markdown: changed.then_some(repaired_md),
        })
        .to_json();
    }

    ToolResult::<ValidateMermaidResponse>::err(
        "vox_validate_mermaid requires either `code` or `markdown` string parameter".to_string(),
    )
    .to_json()
}

const MERMAID_REPAIR_SYSTEM_PROMPT: &str = "You are a Mermaid diagram syntax repair tool. \
Fix the syntax errors reported in the Mermaid diagram while preserving all nodes, edges, and labels. \
Quote any node or edge labels containing parentheses, brackets, braces, or angle brackets (`id[\"Label (info)\"]` or `A -- \"Yes (1)\" --> B`), \
remove HTML tags like `<br/>`, and balance `subgraph` / `end` blocks. \
Return ONLY the raw Mermaid diagram code (no markdown fences, no commentary).";

/// Attempt an LLM-assisted repair using the model chosen by [`vox_orchestrator::models::SelectionIntent::mermaid_repair`].
/// Strictly bounded in latency (8s) and output tokens (256) so it never stalls a chat turn.
async fn llm_repair_mermaid_block(
    code: &str,
    diagnostics: &[MermaidDiagnostic],
    model_id: &str,
) -> Option<String> {
    let diag_summary = diagnostics
        .iter()
        .map(|d| format!("line {}: {} ({})", d.line, d.message, d.code))
        .collect::<Vec<_>>()
        .join("\n");

    let messages = vec![
        LlmChatMessage {
            role: "system".into(),
            content: MERMAID_REPAIR_SYSTEM_PROMPT.to_string(),
            ..Default::default()
        },
        LlmChatMessage {
            role: "user".into(),
            content: format!("Diagnostics:\n{diag_summary}\n\nMermaid code:\n{code}"),
            ..Default::default()
        },
    ];

    let llm_config = LlmConfig {
        provider: "auto".into(),
        model: model_id.to_string(),
        cost_per_1k: None,
        base_url: None,
        api_key: None,
        temperature: Some(0.0),
        top_p: None,
        max_tokens: Some(256),
        response_format: None,
        tools: None,
        tool_choice: None,
        timeout_ms: Some(8_000),
        telemetry_session_id: None,
        telemetry_user_id: None,
        telemetry_task_category: Some("MermaidRepair".into()),
        telemetry_strength_tag: None,
        telemetry_trace_id: None,
        telemetry_attempt_number: Some(1),
        telemetry_skip_interaction: true,
        seed: None,
    };

    let activity_options =
        // drift-allow(timeout-literal,duplicate-numeric-literal): bounded Mermaid repair helper timeout
        ActivityOptions::default().with_timeout(std::time::Duration::from_secs(8));

    let res =
        vox_actor_runtime::llm::infer_with_retry(&activity_options, messages, vec![llm_config])
            .await;

    let (resp, _) = match res {
        ActivityResult::Ok(Ok(pair)) => pair,
        _ => return None,
    };

    let raw = resp.content.trim();
    let stripped = raw
        .strip_prefix("```mermaid")
        .or_else(|| raw.strip_prefix("```"))
        .unwrap_or(raw)
        .trim()
        .strip_suffix("```")
        .unwrap_or(raw)
        .trim();

    let check = validate_and_repair_mermaid(stripped);
    if check.valid {
        Some(stripped.to_string())
    } else {
        check
            .repaired_code
            .filter(|c| validate_and_repair_mermaid(c).valid)
    }
}

/// Automatically observe ` ```mermaid ` blocks in an assistant response, dispatch the
/// `vox_validate_mermaid` MCP tool for verification + cryptographic receipt, and run
/// budgeted repair using [`vox_orchestrator::models::SelectionIntent::mermaid_repair`].
pub async fn observe_and_repair_turn_mermaid(
    state: &ServerState,
    text: &str,
    _permission_mode: Option<&str>,
    tool_calls_made: &mut usize,
    events: &mut Vec<serde_json::Value>,
    telemetry: &mut std::collections::HashMap<String, serde_json::Value>,
) -> String {
    if !text.contains("```mermaid") {
        return text.to_string();
    }

    // 1. Invoke `vox_validate_mermaid` and record its cryptographic tool receipt.
    let validate_args = serde_json::json!({ "markdown": text });
    let receipt_id = state
        .orchestrator
        .issue_tool_receipt(
            vox_orchestrator::types::AgentId(0),
            "vox_validate_mermaid",
            &validate_args.to_string(),
        )
        .ok();
    let tool_res = vox_validate_mermaid(validate_args).await;
    *tool_calls_made += 1;

    if let Some(id) = receipt_id {
        let fulfilled = state.orchestrator.fulfill_tool_receipt(&id, &tool_res);
        events.push(serde_json::json!({
            "kind": "tool_receipt",
            "receipt_id": id,
            "tool_name": "vox_validate_mermaid",
            "fulfilled": fulfilled,
            "verified": state.orchestrator.verify_tool_receipt(&id),
        }));
    }

    let (mut repaired_md, diags) =
        match serde_json::from_str::<ToolResult<ValidateMermaidResponse>>(&tool_res)
            .ok()
            .and_then(|tr| tr.data)
        {
            Some(data) if data.valid => return text.to_string(),
            Some(data) => (
                data.repaired_markdown.unwrap_or_else(|| text.to_string()),
                data.diagnostics,
            ),
            None => validate_and_repair_markdown_mermaid(text),
        };

    if diags.is_empty() {
        return text.to_string();
    }

    // 2. Select the repair model through the canonical selector using `SelectionIntent::mermaid_repair()`.
    let repair_intent = vox_orchestrator::models::SelectionIntent::mermaid_repair();
    let selected_model = vox_orchestrator::models::select_with_default_registry(&repair_intent)
        .map(|o| o.model_id)
        .unwrap_or_else(|| "google/gemini-2.5-flash".to_string());

    telemetry.insert(
        "mermaid_observed_errors".to_string(),
        serde_json::Value::Number(serde_json::Number::from(diags.len())),
    );
    telemetry.insert(
        "mermaid_repair_model".to_string(),
        serde_json::Value::String(selected_model.clone()),
    );

    // 3. Check if any residual errors remain after deterministic repair; if so, check budget and invoke LLM repair.
    let (_, residual_diags) = validate_and_repair_markdown_mermaid(&repaired_md);
    if !residual_diags.is_empty() {
        let budget_signal = {
            let bm = state.orchestrator.budget_manager_handle();
            vox_orchestrator::sync_lock::rw_read(&*bm)
                .agent_budget_signal(vox_orchestrator::types::AgentId(0))
        };
        let budget_allows_llm = !matches!(
            budget_signal,
            vox_orchestrator::budget::BudgetSignal::CostExceeded { .. }
                | vox_orchestrator::budget::BudgetSignal::Critical { .. }
        );
        if budget_allows_llm {
            let blocks = extract_mermaid_blocks(&repaired_md);
            for block in blocks {
                let block_res = validate_and_repair_mermaid(&block.code);
                if !block_res.valid
                    && let Some(llm_fixed) = llm_repair_mermaid_block(
                        &block.code,
                        &block_res.diagnostics,
                        &selected_model,
                    )
                    .await
                {
                    let target_fence = format!("```mermaid\n{}\n```", block.code);
                    let replacement_fence = format!("```mermaid\n{llm_fixed}\n```");
                    repaired_md = repaired_md.replacen(&target_fence, &replacement_fence, 1);
                }
            }
        }
    }

    let repaired = repaired_md != text;
    telemetry.insert(
        "mermaid_repaired".to_string(),
        serde_json::Value::Bool(repaired),
    );
    events.push(serde_json::json!({
        "kind": "mermaid_repaired",
        "diagnostics_found": diags.len(),
        "repaired": repaired,
        "model": selected_model,
    }));

    repaired_md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_unquoted_edge_and_node_labels() {
        let md = "Here is the flow:\n```mermaid\nflowchart TD\n    A[Step 1 (Init) <br/> Ready] -- Yes (Iter < Max) --> B[Done]\n```\n";
        let (repaired, diags) = validate_and_repair_markdown_mermaid(md);
        assert_eq!(diags.len(), 2);
        assert!(repaired.contains("A[\"Step 1 (Init) - Ready\"]"));
        assert!(repaired.contains("-- \"Yes (Iter < Max)\" -->"));
        let (_, after) = validate_and_repair_markdown_mermaid(&repaired);
        assert!(
            after.is_empty(),
            "repaired markdown must validate clean: {after:?}"
        );
    }

    #[tokio::test]
    async fn vox_validate_mermaid_tool_returns_structured_diagnostics_and_repair() {
        let raw = vox_validate_mermaid(serde_json::json!({
            "code": "flowchart TD\n    subgraph S\n    A -- Yes (1) --> B"
        }))
        .await;
        let parsed: ToolResult<ValidateMermaidResponse> =
            serde_json::from_str(&raw).expect("valid ToolResult JSON");
        assert!(parsed.success);
        let data = parsed.data.expect("data payload");
        assert!(!data.valid);
        assert_eq!(data.count, 2);
        let repaired = data.repaired_code.expect("repaired code");
        assert!(repaired.contains("-- \"Yes (1)\" -->"));
        assert!(repaired.ends_with("end"));
    }
}
