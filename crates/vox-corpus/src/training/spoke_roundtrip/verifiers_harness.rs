use super::*;

/// Spoke 5 (`SPOKE-05`): Agent Harness (`vox-dei` `TaskPlan`) round-trip verifier.
#[must_use]
pub fn verify_agent_harness_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
    spec: Option<&AgentHarnessSpec>,
) -> SpokeRoundtripReceipt {
    let raw = strip_code_fences(response);
    let (clean, _) = redact_trajectory_secrets(&raw);

    let parsed: Result<TaskPlanEnvelope, _> = serde_json::from_str(&clean);
    let Ok(plan) = parsed else {
        let receipt = OracleReceipt {
            spoke: SpokeKind::AgentHarness,
            passed: false,
            stage: "task_plan_json_parse".to_string(),
            diagnostics: vec![
                "AgentHarness output did not parse as TaskPlanEnvelope JSON".to_string(),
            ],
            exit_code: Some(1),
        };
        return SpokeRoundtripReceipt {
            case_id: case_id.to_string(),
            spoke: SpokeKind::AgentHarness,
            syntax_valid: false,
            semantic_valid: false,
            execution_passed: false,
            pass_at_1: false,
            pass_at_k: false,
            constructs_covered: Vec::new(),
            stdout_or_result: "invalid TaskPlanEnvelope JSON".to_string(),
            oracle_receipt: receipt,
            admitted_record: None,
        };
    };

    let mut diagnostics = Vec::new();
    if plan.goal.trim().is_empty() {
        diagnostics.push("TaskPlan goal must not be empty".to_string());
    }
    if plan.steps.is_empty() {
        diagnostics.push("TaskPlan steps must not be empty".to_string());
    }
    if plan.files.is_empty() {
        diagnostics.push("TaskPlan files must not be empty".to_string());
    }
    if plan.verification_gate.trim().is_empty() {
        diagnostics.push("TaskPlan verification_gate must not be empty".to_string());
    }
    if let Err(e) = classify_shell_safety_floor(&plan.verification_gate) {
        diagnostics.push(format!("verification_gate failed safety floor: {e}"));
    }

    if let Some(s) = spec {
        if plan.steps.len() > s.max_steps {
            diagnostics.push(format!(
                "TaskPlan step count {} exceeds max_steps {}",
                plan.steps.len(),
                s.max_steps
            ));
        }
        if plan.budget_tokens > s.max_budget_tokens {
            diagnostics.push(format!(
                "TaskPlan budget_tokens {} exceeds max_budget_tokens {}",
                plan.budget_tokens, s.max_budget_tokens
            ));
        }
        if !s.allowed_path_prefixes.is_empty() {
            for file in &plan.files {
                let allowed = s
                    .allowed_path_prefixes
                    .iter()
                    .any(|prefix| file.starts_with(prefix));
                if !allowed || file.contains("..") {
                    diagnostics.push(format!("TaskPlan file `{file}` is outside allowed scope"));
                }
            }
        }
    }

    let syntax_valid = true;
    let semantic_valid = diagnostics.is_empty();
    let execution_passed = semantic_valid;

    let oracle_receipt = OracleReceipt {
        spoke: SpokeKind::AgentHarness,
        passed: execution_passed,
        stage: if execution_passed {
            "task_plan_schema+scope+budget+safety_floor".to_string()
        } else {
            "task_plan_validation_failed".to_string()
        },
        diagnostics: diagnostics.clone(),
        exit_code: Some(if execution_passed { 0 } else { 1 }),
    };

    let admitted_record = if execution_passed {
        Some(TrainingRecordV2::new(
            SpokeKind::AgentHarness,
            prompt,
            "You are the Vox Orchestrator agent harness (`vox-dei`).",
            &clean,
            oracle_receipt.clone(),
            RecordProvenance {
                origin: "spoke_roundtrip".to_string(),
                source_id: case_id.to_string(),
                task_family: "agent_harness_roundtrip".to_string(),
                difficulty: 7,
            },
        ))
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::AgentHarness,
        syntax_valid,
        semantic_valid,
        execution_passed,
        pass_at_1: execution_passed,
        pass_at_k: execution_passed,
        constructs_covered: plan.files,
        stdout_or_result: if execution_passed {
            format!(
                "TaskPlan verified ({} steps, gate: `{}`)",
                plan.steps.len(),
                plan.verification_gate
            )
        } else {
            diagnostics.join("; ")
        },
        oracle_receipt,
        admitted_record,
    }
}

/// Evaluate a single [`SpokeRoundtripCase`] across its `1..=k` candidate responses (`SPOKE-01..06`).
#[must_use]
pub fn evaluate_spoke_case(case: &SpokeRoundtripCase) -> SpokeRoundtripReceipt {
    let verify_one = |candidate: &str| match case.spoke {
        SpokeKind::VoxLang => verify_voxlang_roundtrip(&case.case_id, &case.prompt, candidate),
        SpokeKind::RustWorkspace => verify_rust_roundtrip(&case.case_id, &case.prompt, candidate),
        SpokeKind::StructuredShell => verify_shell_roundtrip(
            &case.case_id,
            &case.prompt,
            candidate,
            &case.expected_json_keys,
        ),
        SpokeKind::McpToolChain => verify_mcp_roundtrip(
            &case.case_id,
            &case.prompt,
            candidate,
            case.mcp_spec.as_ref(),
        ),
        SpokeKind::AgentHarness => verify_agent_harness_roundtrip(
            &case.case_id,
            &case.prompt,
            candidate,
            case.harness_spec.as_ref(),
        ),
    };

    let Some(first_candidate) = case.candidates.first() else {
        return SpokeRoundtripReceipt {
            case_id: case.case_id.clone(),
            spoke: case.spoke,
            syntax_valid: false,
            semantic_valid: false,
            execution_passed: false,
            pass_at_1: false,
            pass_at_k: false,
            constructs_covered: Vec::new(),
            stdout_or_result: "no candidates supplied".to_string(),
            oracle_receipt: OracleReceipt {
                spoke: case.spoke,
                passed: false,
                stage: "empty_candidates".to_string(),
                diagnostics: vec!["no candidates supplied".to_string()],
                exit_code: Some(1),
            },
            admitted_record: None,
        };
    };

    let mut first_receipt = verify_one(first_candidate);
    let pass_at_1 = first_receipt.execution_passed;
    let mut pass_at_k = pass_at_1;

    if !pass_at_1 {
        for cand in case.candidates.iter().skip(1) {
            let r = verify_one(cand);
            if r.execution_passed {
                pass_at_k = true;
                first_receipt.admitted_record = r.admitted_record;
                break;
            }
        }
    }

    first_receipt.pass_at_1 = pass_at_1;
    first_receipt.pass_at_k = pass_at_k;
    first_receipt
}

/// Evaluate a multi-spoke suite and aggregate [`MensSpokeScorecardV1`] (`SPOKE-06`).
#[must_use]
pub fn evaluate_multi_spoke_suite(cases: &[SpokeRoundtripCase]) -> MensSpokeScorecardV1 {
    let receipts: Vec<SpokeRoundtripReceipt> = cases.iter().map(evaluate_spoke_case).collect();

    let mut spokes = Vec::new();
    for spoke in SpokeKind::ALL {
        let spoke_receipts: Vec<&SpokeRoundtripReceipt> =
            receipts.iter().filter(|r| r.spoke == spoke).collect();
        let total = spoke_receipts.len();
        if total == 0 {
            spokes.push(SpokeScorecardEntry {
                spoke,
                total_cases: 0,
                pass_at_1: 0.0,
                pass_at_k: 0.0,
                syntax_validity_rate: 0.0,
                semantic_validity_rate: 0.0,
                execution_success_rate: 0.0,
            });
            continue;
        }
        let denom = total as f64;
        let pass_at_1 = spoke_receipts.iter().filter(|r| r.pass_at_1).count() as f64 / denom;
        let pass_at_k = spoke_receipts.iter().filter(|r| r.pass_at_k).count() as f64 / denom;
        let syntax_validity_rate =
            spoke_receipts.iter().filter(|r| r.syntax_valid).count() as f64 / denom;
        let semantic_validity_rate =
            spoke_receipts.iter().filter(|r| r.semantic_valid).count() as f64 / denom;
        let execution_success_rate =
            spoke_receipts.iter().filter(|r| r.execution_passed).count() as f64 / denom;

        spokes.push(SpokeScorecardEntry {
            spoke,
            total_cases: total,
            pass_at_1,
            pass_at_k,
            syntax_validity_rate,
            semantic_validity_rate,
            execution_success_rate,
        });
    }

    let total_cases = receipts.len();
    let denom = total_cases.max(1) as f64;
    let overall_pass_at_1 = receipts.iter().filter(|r| r.pass_at_1).count() as f64 / denom;
    let overall_pass_at_k = receipts.iter().filter(|r| r.pass_at_k).count() as f64 / denom;
    let overall_syntax_validity_rate =
        receipts.iter().filter(|r| r.syntax_valid).count() as f64 / denom;
    let overall_semantic_validity_rate =
        receipts.iter().filter(|r| r.semantic_valid).count() as f64 / denom;
    let overall_execution_success_rate =
        receipts.iter().filter(|r| r.execution_passed).count() as f64 / denom;

    MensSpokeScorecardV1 {
        schema_version: MensSpokeScorecardV1::SCHEMA_VERSION.to_string(),
        total_cases,
        overall_pass_at_1,
        overall_pass_at_k,
        overall_syntax_validity_rate,
        overall_semantic_validity_rate,
        overall_execution_success_rate,
        spokes,
        receipts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_VOX: &str = "fn double(x: int) to int { return x * 2 }";
    const BAD_VOX: &str = "fn broken( {";

    fn vox_case(id: &str, candidates: &[&str]) -> SpokeRoundtripCase {
        SpokeRoundtripCase {
            case_id: id.to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Write a Vox function".to_string(),
            candidates: candidates.iter().map(|s| (*s).to_string()).collect(),
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        }
    }

    #[test]
    fn agent_harness_rejects_unparsable_and_budget_violating_plans() {
        let bad_json = verify_agent_harness_roundtrip("h_json", "plan", "not json", None);
        assert!(!bad_json.syntax_valid);
        assert_eq!(bad_json.oracle_receipt.stage, "task_plan_json_parse");

        let spec = AgentHarnessSpec {
            allowed_path_prefixes: vec!["crates/vox-corpus/".to_string()],
            max_steps: 3,
            max_budget_tokens: 1000,
        };
        let over_budget = verify_agent_harness_roundtrip(
            "h_budget",
            "plan",
            r#"{"goal":"g","steps":["s1"],"files":["crates/vox-corpus/src/lib.rs"],"budget_tokens":2000,"verification_gate":"cargo test -p vox-corpus"}"#,
            Some(&spec),
        );
        assert!(over_budget.syntax_valid && !over_budget.execution_passed);
        assert_eq!(
            over_budget.oracle_receipt.stage,
            "task_plan_validation_failed"
        );
        assert!(
            over_budget
                .stdout_or_result
                .contains("exceeds max_budget_tokens 1000")
        );
    }

    #[test]
    fn evaluate_spoke_case_tracks_pass_at_1_vs_pass_at_k() {
        let none = evaluate_spoke_case(&vox_case("empty", &[]));
        assert_eq!(none.oracle_receipt.stage, "empty_candidates");
        assert!(!none.pass_at_k);

        let first_ok = evaluate_spoke_case(&vox_case("first", &[GOOD_VOX]));
        assert!(first_ok.pass_at_1 && first_ok.pass_at_k);

        let second_ok = evaluate_spoke_case(&vox_case("second", &[BAD_VOX, GOOD_VOX]));
        assert!(!second_ok.pass_at_1);
        assert!(second_ok.pass_at_k);
        assert!(second_ok.admitted_record.is_some());
    }

    #[test]
    fn multi_spoke_suite_aggregates_rates_per_spoke() {
        let sc =
            evaluate_multi_spoke_suite(&[vox_case("a", &[GOOD_VOX]), vox_case("b", &[BAD_VOX])]);
        assert_eq!(sc.schema_version, MensSpokeScorecardV1::SCHEMA_VERSION);
        assert_eq!(sc.total_cases, 2);
        assert_eq!(sc.overall_pass_at_1, 0.5);
        assert_eq!(sc.overall_pass_at_k, 0.5);
        assert_eq!(sc.spokes.len(), SpokeKind::ALL.len());
        let vox = sc
            .spokes
            .iter()
            .find(|s| s.spoke == SpokeKind::VoxLang)
            .unwrap();
        assert_eq!(vox.total_cases, 2);
        assert_eq!(vox.pass_at_1, 0.5);

        let empty = evaluate_multi_spoke_suite(&[]);
        assert_eq!(empty.total_cases, 0);
        assert_eq!(empty.overall_pass_at_1, 0.0);
    }
}
