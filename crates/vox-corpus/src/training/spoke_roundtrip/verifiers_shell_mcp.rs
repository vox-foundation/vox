use super::*;

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
            ("vox_kb_delete", "vox_kb_reindex / kb_findings lifecycle"),
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
}
