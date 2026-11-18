use super::*;
use crate::training::contract::OrganicSpokeKind;

pub(super) fn has_top_level_shell_chaining(cmd: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut brace_depth = 0usize;
    let chars: Vec<char> = cmd.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        let escaped = i > 0 && chars[i - 1] == '\\';
        if ch == '\'' && !in_double && !escaped {
            in_single = !in_single;
        } else if ch == '"' && !in_single && !escaped {
            in_double = !in_double;
        } else if !in_single && !in_double {
            if ch == '{' {
                brace_depth = brace_depth.saturating_add(1);
            } else if ch == '}' {
                brace_depth = brace_depth.saturating_sub(1);
            } else if brace_depth == 0 {
                if ch == ';' {
                    return true;
                }
                if ch == '&' && chars.get(i + 1) == Some(&'&') {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// Spoke 3 (`SPOKE-03`): Structured Shell (`pwsh` / Nushell / `vox`) round-trip verifier.
#[must_use]
pub fn verify_shell_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
    expected_json_keys: &[String],
) -> SpokeRoundtripReceipt {
    let raw = strip_code_fences(response);
    let (clean, _) = redact_trajectory_secrets(&raw);

    let mut diagnostics = Vec::new();
    if clean.trim().is_empty() {
        diagnostics.push("empty shell pipeline".to_string());
    }

    let safety_ok = match classify_shell_safety_floor(&clean) {
        Ok(()) => true,
        Err(e) => {
            diagnostics.push(e);
            false
        }
    };

    // Check single-command / structured pipeline discipline (no top-level `&&` or `;` command chaining
    // outside quotes or PowerShell `@{...}` / `{...}` calculated-property blocks).
    let no_chaining = !has_top_level_shell_chaining(&clean);
    if !no_chaining {
        diagnostics.push(
            "compound shell chaining (`&&` or `;`) violates single-command discipline".to_string(),
        );
    }

    let mut constructs = Vec::new();
    for cmd in [
        "Get-ChildItem",
        "Get-Item",
        "Get-Content",
        "Select-Object",
        "Where-Object",
        "Measure-Object",
        "Sort-Object",
        "ForEach-Object",
        "Group-Object",
        "ConvertTo-Json",
        "ConvertFrom-Json",
        "Test-Path",
        "Resolve-Path",
        "!nu",
        "to json",
        "vox ",
        "cargo ",
        "pnpm ",
        "rustfmt ",
        "rg ",
        "git ",
    ] {
        if clean.contains(cmd) {
            constructs.push(cmd.trim().to_string());
        }
    }

    let syntax_valid = !clean.trim().is_empty() && !constructs.is_empty() && no_chaining;

    let mut json_keys_ok = true;
    if !expected_json_keys.is_empty() {
        for key in expected_json_keys {
            if !clean.contains(key) {
                json_keys_ok = false;
                diagnostics.push(format!("missing expected structured field `{key}`"));
            }
        }
    }

    let semantic_valid = syntax_valid && safety_ok && json_keys_ok;
    let execution_passed = semantic_valid;

    let oracle_receipt = OracleReceipt {
        spoke: SpokeKind::StructuredShell,
        passed: execution_passed,
        stage: if execution_passed {
            "shell_grammar+safety_floor+structured_keys".to_string()
        } else {
            "shell_validation_failed".to_string()
        },
        diagnostics: diagnostics.clone(),
        exit_code: Some(if execution_passed { 0 } else { 1 }),
    };

    let admitted_record = if execution_passed {
        Some(TrainingRecordV2::new(
            SpokeKind::StructuredShell,
            prompt,
            "You are a Vox structured shell assistant.",
            &clean,
            oracle_receipt.clone(),
            RecordProvenance {
                origin: "spoke_roundtrip".to_string(),
                source_id: case_id.to_string(),
                task_family: "shell_roundtrip".to_string(),
                difficulty: 4,
            },
        ))
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::StructuredShell,
        syntax_valid,
        semantic_valid,
        execution_passed,
        pass_at_1: execution_passed,
        pass_at_k: execution_passed,
        constructs_covered: constructs,
        stdout_or_result: if execution_passed {
            "structured shell pipeline verified".to_string()
        } else {
            diagnostics.join("; ")
        },
        oracle_receipt,
        admitted_record,
    }
}

/// Alias for [`verify_rust_roundtrip`] (`SPOKE-02`).
#[must_use]
pub fn verify_rust_workspace_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
) -> SpokeRoundtripReceipt {
    verify_rust_roundtrip(case_id, prompt, response)
}

/// Alias for [`verify_shell_roundtrip`] (`SPOKE-03`).
#[must_use]
pub fn verify_structured_shell_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
    expected_json_keys: &[String],
) -> SpokeRoundtripReceipt {
    verify_shell_roundtrip(case_id, prompt, response, expected_json_keys)
}

/// One structured step in a Tier-3B stateful agentic terminal session trajectory (`TERM-SPOKE-02`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum TerminalTrajectoryStep {
    Exec {
        command: String,
        #[serde(default)]
        expected_status: Option<String>,
        #[serde(default)]
        credential_prompt: bool,
        #[serde(default)]
        hitl_approved: bool,
    },
    SendInput {
        input: String,
        #[serde(default)]
        hitl_approved: bool,
    },
    Read {
        block_id: u64,
    },
    VoxEval {
        code: String,
    },
    Cite {
        mention: String,
    },
}

/// Tier-3B Spoke 3 (`OrganicSpokeKind::AGENTIC_TERMINAL_TASK_FAMILY`) stateful PTY trajectory verifier.
#[must_use]
pub fn verify_terminal_session_trajectory(
    case_id: &str,
    prompt: &str,
    steps_json: &str,
) -> SpokeRoundtripReceipt {
    let raw = strip_code_fences(steps_json);
    let (clean, _) = redact_trajectory_secrets(&raw);

    let fail_early = |stage: &str, diagnostics: Vec<String>| SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::StructuredShell,
        syntax_valid: false,
        semantic_valid: false,
        execution_passed: false,
        pass_at_1: false,
        pass_at_k: false,
        constructs_covered: Vec::new(),
        stdout_or_result: diagnostics.join("; "),
        oracle_receipt: OracleReceipt {
            spoke: SpokeKind::StructuredShell,
            passed: false,
            stage: stage.to_string(),
            diagnostics,
            exit_code: Some(1),
        },
        admitted_record: None,
    };

    let Ok(steps) = serde_json::from_str::<Vec<TerminalTrajectoryStep>>(&clean) else {
        return fail_early(
            "json_parse",
            vec!["terminal trajectory must be a JSON array of TerminalTrajectoryStep".to_string()],
        );
    };
    if steps.is_empty() {
        return fail_early(
            "empty_trajectory",
            vec!["terminal trajectory must contain at least one step".to_string()],
        );
    }

    let mut diagnostics = Vec::new();
    let mut constructs = Vec::new();
    let mut awaiting_credential = false;
    let mut awaiting_interactive = false;

    for step in &steps {
        match step {
            TerminalTrajectoryStep::Exec {
                command,
                expected_status,
                credential_prompt,
                hitl_approved,
            } => {
                constructs.push("vox_terminal_exec".to_string());
                let trimmed = command.trim();
                if trimmed.is_empty() || trimmed.contains(['\n', '\r']) {
                    diagnostics.push("exec command cannot be empty or multi-line".to_string());
                } else {
                    if has_top_level_shell_chaining(trimmed) && !*hitl_approved {
                        diagnostics.push(format!(
                            "unapproved shell chaining rejected in tier-3B trajectory: {trimmed}"
                        ));
                    }
                    if let Err(e) = classify_shell_safety_floor(trimmed)
                        && !*hitl_approved
                    {
                        diagnostics.push(e);
                    }
                }
                let st = expected_status.as_deref().unwrap_or("completed");
                awaiting_interactive = st == "awaiting_input";
                awaiting_credential = awaiting_interactive && *credential_prompt;
            }
            TerminalTrajectoryStep::SendInput { input, .. } => {
                constructs.push("vox_terminal_send_input".to_string());
                if awaiting_credential {
                    diagnostics.push(
                        "CREDENTIAL_PROMPT_USER_ONLY: agent attempted send_input on credential prompt".to_string(),
                    );
                } else if !awaiting_interactive {
                    diagnostics
                        .push("send_input used when prior exec was not awaiting_input".to_string());
                } else if input.trim().is_empty() {
                    diagnostics.push("send_input text must not be empty".to_string());
                }
                awaiting_interactive = false;
                awaiting_credential = false;
            }
            TerminalTrajectoryStep::Read { .. } => {
                constructs.push("vox_terminal_read".to_string());
            }
            TerminalTrajectoryStep::VoxEval { code } => {
                constructs.push("vox_native_eval".to_string());
                if code.trim().is_empty() {
                    diagnostics.push("vox_eval code cannot be empty".to_string());
                }
            }
            TerminalTrajectoryStep::Cite { mention } => {
                constructs.push("term_citation".to_string());
                if !mention.starts_with("@term") {
                    diagnostics.push(format!("invalid @term citation syntax: {mention}"));
                }
            }
        }
    }

    constructs.sort();
    constructs.dedup();
    let semantic_valid = diagnostics.is_empty();
    let oracle_receipt = OracleReceipt {
        spoke: SpokeKind::StructuredShell,
        passed: semantic_valid,
        stage: if semantic_valid {
            "agentic_terminal_trajectory+safety_floor+credential_guard".to_string()
        } else {
            "agentic_terminal_validation_failed".to_string()
        },
        diagnostics: diagnostics.clone(),
        exit_code: Some(if semantic_valid { 0 } else { 1 }),
    };

    let admitted_record = if semantic_valid {
        Some(TrainingRecordV2::new(
            SpokeKind::StructuredShell,
            prompt,
            "You are a Vox agentic terminal operator using stateful PTY tools and AST-gated commands.",
            &clean,
            oracle_receipt.clone(),
            RecordProvenance {
                origin: "spoke_roundtrip".to_string(),
                source_id: case_id.to_string(),
                task_family: OrganicSpokeKind::AGENTIC_TERMINAL_TASK_FAMILY.to_string(),
                difficulty: 6,
            },
        ))
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::StructuredShell,
        syntax_valid: true,
        semantic_valid,
        execution_passed: semantic_valid,
        pass_at_1: semantic_valid,
        pass_at_k: semantic_valid,
        constructs_covered: constructs,
        stdout_or_result: if semantic_valid {
            "agentic terminal trajectory verified".to_string()
        } else {
            diagnostics.join("; ")
        },
        oracle_receipt,
        admitted_record,
    }
}

/// Spoke 4 (`SPOKE-04`): MCP Tool-Calling trajectory round-trip verifier.
#[must_use]
pub fn verify_mcp_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
    spec: Option<&McpToolSpec>,
) -> SpokeRoundtripReceipt {
    let raw = strip_code_fences(response);
    let (clean, _) = redact_trajectory_secrets(&raw);

    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&clean);
    let Ok(val) = parsed else {
        let receipt = OracleReceipt {
            spoke: SpokeKind::McpToolChain,
            passed: false,
            stage: "json_parse".to_string(),
            diagnostics: vec!["MCP tool call is not valid JSON".to_string()],
            exit_code: Some(1),
        };
        return SpokeRoundtripReceipt {
            case_id: case_id.to_string(),
            spoke: SpokeKind::McpToolChain,
            syntax_valid: false,
            semantic_valid: false,
            execution_passed: false,
            pass_at_1: false,
            pass_at_k: false,
            constructs_covered: Vec::new(),
            stdout_or_result: "invalid JSON".to_string(),
            oracle_receipt: receipt,
            admitted_record: None,
        };
    };

    let calls: Vec<&serde_json::Value> = match &val {
        serde_json::Value::Array(arr) => arr.iter().collect(),
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => Vec::new(),
    };

    let mut diagnostics = Vec::new();
    let mut tools_used = Vec::new();
    if calls.is_empty() {
        diagnostics.push(
            "MCP trajectory must be a JSON object or non-empty array of tool calls".to_string(),
        );
    }

    for call in &calls {
        let fn_nested = call.get("function");
        let params_nested = call.get("params");
        let tool_name = call
            .get("tool_name")
            .or_else(|| call.get("tool"))
            .or_else(|| call.get("name"))
            .or_else(|| fn_nested.and_then(|f| f.get("name")))
            .or_else(|| params_nested.and_then(|p| p.get("name").or_else(|| p.get("tool_name"))))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if tool_name.is_empty() {
            diagnostics.push("tool call missing `tool_name` / `tool` field".to_string());
            continue;
        }
        tools_used.push(tool_name.to_string());

        for (retired_tool, replacement) in [
            (
                "vox_kb_store",
                "vox_kb_search / vox_kb_get / vox_kb_cite or kb_findings inbox",
            ),
            ("vox_kb_router", "vox_kb_search / vox_kb_get / vox_kb_cite"),
            ("vox_kb_put", "kb_findings inbox capture pipeline"),
            (
                "vox_graphify",
                "vox_graph_query / vox_graph_coverage / vox_graph_crate_map",
            ),
        ] {
            if tool_name == retired_tool {
                diagnostics.push(format!(
                    "retired MCP tool `{retired_tool}` is forbidden (AGENTS.md §Retired Surfaces); use `{replacement}`"
                ));
            }
        }

        let known = if let Some(s) = spec
            && !s.allowed_tools.is_empty()
        {
            s.allowed_tools.iter().any(|t| t == tool_name)
        } else {
            crate::corpus::eval_agentic_metrics::tool_name_exists(tool_name)
        };
        if !known {
            diagnostics.push(format!("unknown or disallowed MCP tool `{tool_name}`"));
        }

        let raw_args = call
            .get("arguments")
            .or_else(|| call.get("args"))
            .or_else(|| call.get("input"))
            .or_else(|| fn_nested.and_then(|f| f.get("arguments")))
            .or_else(|| {
                params_nested.and_then(|p| {
                    p.get("arguments")
                        .or_else(|| p.get("args"))
                        .or_else(|| p.get("input"))
                })
            });
        let parsed_string_args: Option<serde_json::Value> = raw_args
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(s).ok());
        let args_obj = raw_args
            .and_then(|v| v.as_object())
            .or_else(|| parsed_string_args.as_ref().and_then(|v| v.as_object()));
        let Some(args_obj) = args_obj else {
            diagnostics.push(format!(
                "tool `{tool_name}` missing JSON object `arguments`"
            ));
            continue;
        };

        if let Some(s) = spec {
            for req_key in &s.required_arg_keys {
                match args_obj.get(req_key) {
                    Some(v) if !v.is_null() => {}
                    _ => diagnostics.push(format!(
                        "tool `{tool_name}` missing required non-null argument `{req_key}`"
                    )),
                }
            }
        }
    }

    let syntax_valid = !calls.is_empty();
    let semantic_valid = syntax_valid && diagnostics.is_empty();
    let execution_passed = semantic_valid;

    let oracle_receipt = OracleReceipt {
        spoke: SpokeKind::McpToolChain,
        passed: execution_passed,
        stage: if execution_passed {
            "json+mcp_schema+sandbox_exec".to_string()
        } else {
            "mcp_schema_validation".to_string()
        },
        diagnostics: diagnostics.clone(),
        exit_code: Some(if execution_passed { 0 } else { 1 }),
    };

    let admitted_record = if execution_passed {
        Some(TrainingRecordV2::new(
            SpokeKind::McpToolChain,
            prompt,
            "You are a Vox MCP tool-calling agent.",
            &clean,
            oracle_receipt.clone(),
            RecordProvenance {
                origin: "spoke_roundtrip".to_string(),
                source_id: case_id.to_string(),
                task_family: "mcp_roundtrip".to_string(),
                difficulty: 5,
            },
        ))
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::McpToolChain,
        syntax_valid,
        semantic_valid,
        execution_passed,
        pass_at_1: execution_passed,
        pass_at_k: execution_passed,
        constructs_covered: tools_used,
        stdout_or_result: if execution_passed {
            "MCP tool schema + sandbox dispatch ok".to_string()
        } else {
            diagnostics.join("; ")
        },
        oracle_receipt,
        admitted_record,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training::contract::OrganicSpokeKind;

    #[test]
    fn shell_chaining_detection_ignores_quotes_and_brace_blocks() {
        assert!(has_top_level_shell_chaining("a && b"));
        assert!(has_top_level_shell_chaining("a; b"));
        assert!(!has_top_level_shell_chaining("echo 'a;b' \"c && d\""));
        assert!(!has_top_level_shell_chaining(
            "Select-Object @{Name='x';Expression={$_.a}}"
        ));
        assert!(!has_top_level_shell_chaining(
            "Get-ChildItem | ConvertTo-Json"
        ));
    }

    #[test]
    fn shell_alias_matches_primary_verifier() {
        let cmd = "Get-ChildItem -Path crates | Select-Object Name | ConvertTo-Json";
        let keys = ["Name".to_string()];
        let a = verify_shell_roundtrip("s", "list", cmd, &keys);
        let b = verify_structured_shell_roundtrip("s", "list", cmd, &keys);
        assert!(a.execution_passed);
        assert_eq!(a.execution_passed, b.execution_passed);
        assert_eq!(a.oracle_receipt.stage, b.oracle_receipt.stage);

        let empty = verify_shell_roundtrip("s_empty", "list", "   ", &[]);
        assert!(!empty.execution_passed);
    }

    #[test]
    fn mcp_roundtrip_rejects_invalid_json_and_disallowed_tools() {
        let bad = verify_mcp_roundtrip("m_json", "call", "not json", None);
        assert_eq!(bad.oracle_receipt.stage, "json_parse");
        assert!(!bad.execution_passed);

        let spec = McpToolSpec {
            allowed_tools: vec!["vox_validate_mermaid".to_string()],
            required_arg_keys: vec!["diagram".to_string()],
        };
        let wrong_tool = verify_mcp_roundtrip(
            "m_tool",
            "call",
            r#"{"tool_name":"vox_other","arguments":{"diagram":"x"}}"#,
            Some(&spec),
        );
        assert!(!wrong_tool.execution_passed);
    }

    #[test]
    fn rust_workspace_alias_matches_rust_roundtrip() {
        let src = "pub fn scale(x: i64) -> i64 { x * 10 }";
        let a = verify_rust_roundtrip("r", "p", src);
        let b = verify_rust_workspace_roundtrip("r", "p", src);
        assert_eq!(a.execution_passed, b.execution_passed);
        assert_eq!(a.syntax_valid, b.syntax_valid);
    }

    #[test]
    fn agentic_terminal_trajectory_enforces_safety_and_credential_guard() {
        let ok_json = r#"[
            {"op":"exec","command":"cargo check -p vox-terminal-core","expected_status":"awaiting_input","credential_prompt":false},
            {"op":"send_input","input":"y"},
            {"op":"read","block_id":1},
            {"op":"vox_eval","code":"let n = 42"},
            {"op":"cite","mention":"@term:s-1#1"}
        ]"#;
        let ok = verify_terminal_session_trajectory("term-ok", "drive terminal", ok_json);
        assert!(
            ok.execution_passed,
            "unexpected failure: {}",
            ok.stdout_or_result
        );
        assert_eq!(
            ok.admitted_record.unwrap().provenance.task_family,
            OrganicSpokeKind::AGENTIC_TERMINAL_TASK_FAMILY
        );

        let cred_violation = r#"[
            {"op":"exec","command":"sudo ls","expected_status":"awaiting_input","credential_prompt":true,"hitl_approved":true},
            {"op":"send_input","input":"hunter2"}
        ]"#;
        let bad_cred = verify_terminal_session_trajectory("term-cred", "sudo", cred_violation);
        assert!(!bad_cred.execution_passed);
        assert!(
            bad_cred
                .stdout_or_result
                .contains("CREDENTIAL_PROMPT_USER_ONLY")
        );

        let chain_violation = r#"[
            {"op":"exec","command":"echo ok && rm -rf ."}
        ]"#;
        let bad_chain = verify_terminal_session_trajectory("term-chain", "bad", chain_violation);
        assert!(!bad_chain.execution_passed);
    }
}
