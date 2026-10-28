//! Schemas for tool, workflow, and A2A traces distilled into supervised JSONL.

use serde::{Deserialize, Serialize};

/// One tool invocation suitable for SFT (prompt → tool → args → result).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTraceRecord {
    /// User or system text that triggered the tool call.
    pub task_prompt: String,
    /// Registered tool name (MCP or internal).
    pub tool_name: String,
    /// JSON-serialized arguments object.
    pub arguments_json: String,
    /// JSON-serialized tool result payload.
    pub result_json: String,
    /// Whether the invocation completed without error.
    pub success: bool,
    /// Optional assistant follow-up after the tool result.
    pub followup_text: Option<String>,
    /// Correlates multi-turn sessions when present.
    pub session_id: Option<String>,
}

/// Workflow step trace for training plan → skeleton expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTraceRecord {
    /// Natural-language workflow goal.
    pub intent: String,
    /// Resolved durable workflow name, if known.
    pub workflow_name: Option<String>,
    /// Truncated execution log for supervision.
    pub execution_log_excerpt: String,
    /// Model-produced Vox snippet, if captured.
    pub synthesized_vox: Option<String>,
    /// NNT small-world routing efficiency score (0.0-1.0).
    pub routing_efficiency: Option<f64>,
}

/// A2A message pair for coordination training.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATraceRecord {
    /// Sending agent id or role label.
    pub from_agent: String,
    /// Receiving agent id or role label.
    pub to_agent: String,
    /// Discriminator for routing / schema (e.g. `task`, `handoff`).
    pub message_type: String,
    /// JSON body of the A2A message.
    pub payload_json: String,
    /// Suggested reply JSON for SFT targets.
    pub recommended_reply_json: Option<String>,
}

/// Serialize `row` as a single JSONL line (no trailing newline).
pub fn jsonl_line<T: Serialize>(row: &T) -> anyhow::Result<String> {
    Ok(serde_json::to_string(row)?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegativeLexParseTypeckRecord {
    pub source: String,
    pub errors_json: String,
    pub origin: String,
    pub reward_signal: f64,
}

#[cfg(feature = "database")]
pub async fn auto_ingest_negative_vox(
    source: &str,
    errors_json: &str,
    db: &vox_db::VoxDb,
) -> anyhow::Result<()> {
    db.upsert_corpus_pair(source, errors_json, "agent", 0.0, "negative")
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Redact inline credentials, bearer tokens, and common API key prefixes from trajectory payloads (`CORPUS-04`).
///
/// Returns `(redacted_text, redaction_count)`.
#[must_use]
pub fn redact_trajectory_secrets(input: &str) -> (String, usize) {
    static SECRET_PATTERNS: std::sync::OnceLock<Vec<regex::Regex>> = std::sync::OnceLock::new();
    let patterns = SECRET_PATTERNS.get_or_init(|| {
        vec![
            regex::Regex::new(r"sk-[A-Za-z0-9_\-]{12,}").expect("sk regex"),
            regex::Regex::new(r"gh[pousr]_[A-Za-z0-9]{16,}").expect("gh regex"),
            regex::Regex::new(r"(?i)bearer\s+[A-Za-z0-9._\-]{12,}").expect("bearer regex"),
            regex::Regex::new(r"AIza[0-9A-Za-z_\-]{20,}").expect("gcp key regex"),
            regex::Regex::new(r"xox[baprs]-[0-9A-Za-z\-]{12,}").expect("slack key regex"),
        ]
    });

    let mut out = input.to_string();
    let mut count = 0usize;
    for re in patterns {
        let matches = re.find_iter(&out).count();
        if matches > 0 {
            count += matches;
            out = re.replace_all(&out, "[REDACTED]").to_string();
        }
    }
    (out, count)
}

/// Deterministic shell safety-floor classifier (`classify_floor` for `CORPUS-04`).
///
/// Rejects destructive filesystem deletion, force-history rewrites, privilege escalation,
/// and remote pipe-to-shell execution in `StructuredShell` trajectories.
pub fn classify_shell_safety_floor(command: &str) -> Result<(), String> {
    let lower = command.to_lowercase();
    const FORBIDDEN: &[(&str, &str)] = &[
        ("rm -rf /", "destructive root deletion"),
        ("rm -rf ~", "destructive home deletion"),
        ("git reset --hard", "destructive git history reset"),
        ("git push --force", "destructive force push"),
        ("mkfs", "filesystem format command"),
        ("dd if=", "raw block device write"),
        ("| sh", "unverified pipe to sh"),
        ("| bash", "unverified pipe to bash"),
        ("sudo ", "privilege escalation"),
    ];
    for &(pat, reason) in FORBIDDEN {
        if lower.contains(pat) {
            return Err(format!(
                "safety_floor violation ({reason}): matched `{pat}`"
            ));
        }
    }
    Ok(())
}

impl ToolTraceRecord {
    /// Convert a tool trace into a canonical [`crate::training::TrainingRecordV2`] for either
    /// `StructuredShell` (when `tool_name` is `term_run` / `vox_shell` / `nu`) or `McpToolChain` (`CORPUS-04`).
    #[must_use]
    pub fn to_training_record_v2(&self) -> Option<crate::training::TrainingRecordV2> {
        use crate::training::{OracleReceipt, RecordProvenance, SpokeKind, TrainingRecordV2};

        if !self.success {
            return None;
        }
        let is_shell = matches!(
            self.tool_name.as_str(),
            "term_run" | "vox_shell" | "nu" | "run_nu_structured"
        ) || self.tool_name.starts_with("vox ");

        let spoke = if is_shell {
            SpokeKind::StructuredShell
        } else {
            SpokeKind::McpToolChain
        };

        if is_shell {
            if classify_shell_safety_floor(&self.arguments_json).is_err()
                || classify_shell_safety_floor(&self.tool_name).is_err()
            {
                return None;
            }
        } else if !crate::corpus::eval_agentic_metrics::tool_name_exists(&self.tool_name) {
            return None;
        }

        let (clean_prompt, _) = redact_trajectory_secrets(&self.task_prompt);
        let (clean_args, _) = redact_trajectory_secrets(&self.arguments_json);
        let (clean_res, _) = redact_trajectory_secrets(&self.result_json);

        let args_val: serde_json::Value = serde_json::from_str(&clean_args).ok()?;
        let response_payload = serde_json::json!({
            "tool_name": self.tool_name,
            "arguments": args_val,
            "result": clean_res,
            "success": self.success,
        })
        .to_string();

        let receipt = OracleReceipt {
            spoke,
            passed: true,
            stage: if is_shell {
                "redact+safety_floor".to_string()
            } else {
                "redact+mcp_registry".to_string()
            },
            diagnostics: Vec::new(),
            exit_code: Some(0),
        };
        let prov = RecordProvenance {
            origin: "tool_trace".to_string(),
            source_id: self
                .session_id
                .clone()
                .unwrap_or_else(|| "tool_session".to_string()),
            task_family: self.tool_name.clone(),
            difficulty: crate::training::construct_difficulty(&self.tool_name, "tool_trace"),
        };
        Some(TrainingRecordV2::new(
            spoke,
            clean_prompt,
            "You are a Vox agent executing verified tool and shell workflows.",
            response_payload,
            receipt,
            prov,
        ))
    }
}

impl WorkflowTraceRecord {
    /// Convert an orchestrator workflow trace into a canonical `AgentHarness` [`crate::training::TrainingRecordV2`] (`CORPUS-04`).
    #[must_use]
    pub fn to_training_record_v2(&self) -> Option<crate::training::TrainingRecordV2> {
        use crate::training::{OracleReceipt, RecordProvenance, SpokeKind, TrainingRecordV2};

        if self.intent.trim().is_empty() || self.execution_log_excerpt.trim().is_empty() {
            return None;
        }
        if classify_shell_safety_floor(&self.execution_log_excerpt).is_err() {
            return None;
        }
        let (clean_intent, _) = redact_trajectory_secrets(&self.intent);
        let (clean_log, _) = redact_trajectory_secrets(&self.execution_log_excerpt);
        let body = if let Some(ref vox_src) = self.synthesized_vox {
            let (clean_vox, _) = redact_trajectory_secrets(vox_src);
            format!("{clean_log}\n```vox\n{clean_vox}\n```")
        } else {
            clean_log
        };

        let receipt = OracleReceipt {
            spoke: SpokeKind::AgentHarness,
            passed: true,
            stage: "redact+safety_floor+workflow".to_string(),
            diagnostics: Vec::new(),
            exit_code: Some(0),
        };
        let prov = RecordProvenance {
            origin: "workflow_trace".to_string(),
            source_id: self
                .workflow_name
                .clone()
                .unwrap_or_else(|| "workflow".to_string()),
            task_family: "agent_workflow".to_string(),
            difficulty: 8,
        };
        Some(TrainingRecordV2::new(
            SpokeKind::AgentHarness,
            clean_intent,
            "You are the Vox Orchestrator agent harness.",
            body,
            receipt,
            prov,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secrets_and_blocks_unsafe_shell_trajectories() {
        let (redacted, count) = redact_trajectory_secrets(
            "Authorization: Bearer secret_token_1234567890 and sk-abcdef1234567890",
        );
        assert_eq!(count, 2);
        assert!(!redacted.contains("secret_token_1234567890"));
        assert!(!redacted.contains("sk-abcdef1234567890"));
        assert!(redacted.contains("[REDACTED]"));

        assert!(classify_shell_safety_floor("vox ci status").is_ok());
        assert!(classify_shell_safety_floor("rm -rf /").is_err());
        assert!(classify_shell_safety_floor("git reset --hard HEAD~1").is_err());
    }

    #[test]
    fn converts_shell_mcp_and_harness_traces_to_v2() {
        let shell_trace = ToolTraceRecord {
            task_prompt: "List workspace crates via structured shell".to_string(),
            tool_name: "term_run".to_string(),
            arguments_json: r#"{"cmd":"cargo metadata --no-deps"}"#.to_string(),
            result_json: r#"{"exit_code":0}"#.to_string(),
            success: true,
            followup_text: None,
            session_id: Some("sess-shell".to_string()),
        };
        let shell_rec = shell_trace.to_training_record_v2().expect("shell record");
        assert_eq!(shell_rec.spoke, crate::training::SpokeKind::StructuredShell);
        assert!(shell_rec.verify_integrity());

        let unsafe_shell = ToolTraceRecord {
            arguments_json: r#"{"cmd":"rm -rf /"}"#.to_string(),
            ..shell_trace
        };
        assert!(unsafe_shell.to_training_record_v2().is_none());

        let mcp_trace = ToolTraceRecord {
            task_prompt: "List installed skills with token sk-1234567890abcdef".to_string(),
            tool_name: "vox_skill_list".to_string(),
            arguments_json: "{}".to_string(),
            result_json: "[]".to_string(),
            success: true,
            followup_text: None,
            session_id: Some("sess-mcp".to_string()),
        };
        let mcp_rec = mcp_trace.to_training_record_v2().expect("mcp record");
        assert_eq!(mcp_rec.spoke, crate::training::SpokeKind::McpToolChain);
        assert!(!mcp_rec.prompt.contains("sk-1234567890abcdef"));
        assert!(mcp_rec.verify_integrity());

        let wf_trace = WorkflowTraceRecord {
            intent: "Coordinate multi-step repair".to_string(),
            workflow_name: Some("repair_loop".to_string()),
            execution_log_excerpt: "step 1: check diagnostics -> step 2: apply patch".to_string(),
            synthesized_vox: Some("fn fix() to int { return 1 }".to_string()),
            routing_efficiency: Some(0.95),
        };
        let wf_rec = wf_trace.to_training_record_v2().expect("workflow record");
        assert_eq!(wf_rec.spoke, crate::training::SpokeKind::AgentHarness);
        assert!(wf_rec.verify_integrity());
    }
}
