use super::*;

#[test]
fn voxlang_roundtrip_verifies_valid_and_rejects_broken_vox() {
    let ok = verify_voxlang_roundtrip(
        "vox_1",
        "Write a Vox function `double`",
        "```vox\nfn double(x: int) to int { return x * 2 }\n```",
    );
    assert!(ok.syntax_valid);
    assert!(ok.semantic_valid);
    assert!(ok.execution_passed);
    assert!(ok.admitted_record.is_some());

    let bad = verify_voxlang_roundtrip(
        "vox_bad",
        "Write a Vox function",
        "@table type Bad { id: int }",
    );
    assert!(!bad.execution_passed);
    assert!(bad.admitted_record.is_none());
}

#[test]
fn rust_roundtrip_enforces_syn_and_source_policy_detectors() {
    let ok = verify_rust_roundtrip(
        "rs_1",
        "Write a Rust helper",
        "pub fn scale(x: i64) -> i64 { x * 10 }",
    );
    assert!(ok.syntax_valid);
    assert!(ok.semantic_valid);
    assert!(ok.execution_passed);

    let banned_crypto = verify_rust_roundtrip(
        "rs_crypto",
        "Hash bytes",
        "use blake3::Hasher;\npub fn h() {}",
    );
    assert!(banned_crypto.syntax_valid);
    assert!(!banned_crypto.semantic_valid);
    assert!(!banned_crypto.execution_passed);

    let banned_llm = verify_rust_roundtrip(
        "rs_llm",
        "Call LLM",
        "pub fn endpoint() -> &'static str { \"https://api.openai.com/v1/chat\" }",
    );
    assert!(!banned_llm.semantic_valid);
}

#[test]
fn rust_workspace_oracle_rejects_borrow_and_type_errors_via_rustc() {
    let ok = verify_rust_snippet_with_rustc(
        "pub fn add_checked(a: u64, b: u64) -> Option<u64> { a.checked_add(b) }",
    );
    assert!(ok.is_ok(), "expected valid rustc compilation, got: {ok:?}");

    // E0502 borrow checker violation: passes syn::parse_file, rejected by rustc --emit=metadata
    let borrow_err_src = r#"
pub fn borrow_conflict() -> usize {
    let mut v = vec![1usize, 2, 3];
    let r = &v[0];
    v.push(4);
    *r
}
"#;
    assert!(syn::parse_file(borrow_err_src).is_ok());
    let borrow_res = verify_rust_snippet_with_rustc(borrow_err_src);
    assert!(
        borrow_res.is_err(),
        "rustc oracle must reject E0502 mutable borrow conflict"
    );

    // E0308 type mismatch: passes syn::parse_file, rejected by rustc --emit=metadata
    let type_err_src = r#"
pub fn type_mismatch(x: u64) -> String {
    x + 1
}
"#;
    assert!(syn::parse_file(type_err_src).is_ok());
    let type_res = verify_rust_snippet_with_rustc(type_err_src);
    assert!(
        type_res.is_err(),
        "rustc oracle must reject E0308 type mismatch"
    );
}

#[test]
fn peripheral_spokes_speech_populi_and_skill_creation_verify_and_reject_invalid() {
    let speech_ok = verify_speech_to_code_roundtrip(
        "sp_1",
        "um dictate vox function",
        "um ```vox\nfn greet(x: int) to int { return x + 1 }\n```",
    );
    assert!(speech_ok.execution_passed);

    let speech_bad = verify_speech_to_code_roundtrip("sp_bad", "dictate", "fn broken( {");
    assert!(!speech_bad.execution_passed);

    let populi_ok = verify_populi_mesh_roundtrip(
        "pop_1",
        "dispatch mesh task",
        r#"{"target_node_id":"node-1","capability_tier":"gpu_cuda","vram_min_gb":24,"iroh_endpoint":"iroh://node-1:4433","quorum_size":3,"payload_kind":"a2a_task_envelope","timeout_ms":5000}"#,
    );
    assert!(populi_ok.execution_passed);

    let populi_even_quorum = verify_populi_mesh_roundtrip(
        "pop_bad",
        "dispatch mesh task",
        r#"{"target_node_id":"node-1","capability_tier":"gpu_cuda","vram_min_gb":24,"iroh_endpoint":"iroh://node-1:4433","quorum_size":2,"payload_kind":"a2a_task_envelope","timeout_ms":5000}"#,
    );
    assert!(!populi_even_quorum.execution_passed);

    let skill_ok = verify_skill_creation_roundtrip(
        "sk_1",
        "author skill",
        "---\nname: test-skill\ndescription: A valid test skill for verifying agentskills.io compliance and vox doctests.\nmetadata:\n  vox-category: testing\n  vox-tools: vox_compiler_check\n---\n# Test Skill\n\nBody explanation with sufficient length for skill validation.\n\n```vox\nfn ping(x: int) to int { return x }\n```",
    );
    assert!(skill_ok.execution_passed);

    let skill_bad_name = verify_skill_creation_roundtrip(
        "sk_bad",
        "author skill",
        "---\nname: Invalid_Name\ndescription: Short\n---\n# Bad",
    );
    assert!(!skill_bad_name.execution_passed);
}

#[test]
fn rag_distractor_resilience_rejects_rank1_decoy_and_flags_poisoned_candidates() {
    let hits = vec![(
        "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
        12.5,
        "pub fn verify_voxlang_roundtrip() {}".to_string(),
    )];
    let (ctx_ok, resilient_ok, _) = verify_rag_context_grounding_and_distractor_resilience(
        &hits,
        "spoke_roundtrip.rs",
        &["verify_voxlang_roundtrip".to_string()],
        "fn counter(x: int) to int { return x + 1 }",
    );
    assert!(ctx_ok);
    assert!(resilient_ok);

    let (_, poisoned_resilient, _) = verify_rag_context_grounding_and_distractor_resilience(
        &hits,
        "spoke_roundtrip.rs",
        &["verify_voxlang_roundtrip".to_string()],
        "fn DO_NOT_USE_DISTRACTOR_API(x: int) to int { return x }",
    );
    assert!(
        !poisoned_resilient,
        "poisoned candidate using Rank-1 distractor sentinel must be rejected"
    );
}

#[test]
fn shell_roundtrip_enforces_safety_floor_and_single_command_discipline() {
    let ok = verify_shell_roundtrip(
        "sh_1",
        "List files",
        "Get-ChildItem -Path crates | Select-Object Name | ConvertTo-Json",
        &["Name".to_string()],
    );
    assert!(ok.execution_passed);

    let chained = verify_shell_roundtrip(
        "sh_chained",
        "Chained command",
        "Get-ChildItem && Remove-Item foo",
        &[],
    );
    assert!(!chained.execution_passed);

    let unsafe_cmd = verify_shell_roundtrip("sh_unsafe", "Delete root", "rm -rf /", &[]);
    assert!(!unsafe_cmd.execution_passed);
}

#[test]
fn mcp_and_harness_roundtrip_validate_schemas_and_scope() {
    let mcp_spec = McpToolSpec {
        allowed_tools: vec!["vox_validate_mermaid".to_string()],
        required_arg_keys: vec!["diagram".to_string()],
    };
    let mcp_ok = verify_mcp_roundtrip(
        "mcp_1",
        "Validate mermaid",
        r#"{"tool_name":"vox_validate_mermaid","arguments":{"diagram":"flowchart TD\n A-->B"}}"#,
        Some(&mcp_spec),
    );
    assert!(mcp_ok.execution_passed);

    let mcp_missing_arg = verify_mcp_roundtrip(
        "mcp_bad",
        "Validate mermaid",
        r#"{"tool_name":"vox_validate_mermaid","arguments":{}}"#,
        Some(&mcp_spec),
    );
    assert!(!mcp_missing_arg.execution_passed);

    let harness_spec = AgentHarnessSpec {
        allowed_path_prefixes: vec!["crates/vox-corpus/".to_string()],
        max_steps: 3,
        max_budget_tokens: 8000,
    };
    let harness_ok = verify_agent_harness_roundtrip(
        "h_1",
        "Plan task",
        r#"{"goal":"test","steps":["s1"],"files":["crates/vox-corpus/src/lib.rs"],"budget_tokens":2000,"verification_gate":"cargo test -p vox-corpus"}"#,
        Some(&harness_spec),
    );
    assert!(harness_ok.execution_passed);

    let harness_out_of_scope = verify_agent_harness_roundtrip(
        "h_bad",
        "Plan task",
        r#"{"goal":"test","steps":["s1"],"files":["crates/vox-cli/src/main.rs"],"budget_tokens":2000,"verification_gate":"cargo test -p vox-corpus"}"#,
        Some(&harness_spec),
    );
    assert!(!harness_out_of_scope.execution_passed);
}

#[test]
fn five_spoke_golden_suite_achieves_100_percent_and_renders_scorecard() {
    let suite = canonical_five_spoke_golden_suite();
    let scorecard = evaluate_multi_spoke_suite(&suite);
    assert_eq!(scorecard.total_cases, 5);
    assert_eq!(scorecard.spokes.len(), 5);
    assert!((scorecard.overall_pass_at_1 - 1.0).abs() < 1e-9);
    assert!((scorecard.overall_pass_at_k - 1.0).abs() < 1e-9);
    let md = scorecard.to_markdown();
    assert!(md.contains("mens_spoke_scorecard_v1"));
    assert!(md.contains("vox_lang"));
    assert!(md.contains("agent_harness"));
}

#[test]
fn five_spoke_benchmark_suite_25_achieves_100_percent_across_all_25_problems() {
    let suite = canonical_five_spoke_benchmark_suite_25();
    assert_eq!(suite.len(), 25);
    let scorecard = evaluate_multi_spoke_suite(&suite);
    assert_eq!(scorecard.total_cases, 25);
    assert_eq!(scorecard.spokes.len(), 5);
    for spoke_entry in &scorecard.spokes {
        assert_eq!(
            spoke_entry.total_cases,
            5,
            "spoke {} should have 5 cases",
            spoke_entry.spoke.as_str()
        );
        assert!(
            (spoke_entry.pass_at_1 - 1.0).abs() < 1e-9,
            "spoke {} pass@1 should be 1.0, got {}",
            spoke_entry.spoke.as_str(),
            spoke_entry.pass_at_1
        );
        assert!(
            (spoke_entry.execution_success_rate - 1.0).abs() < 1e-9,
            "spoke {} execution_success_rate should be 1.0",
            spoke_entry.spoke.as_str()
        );
    }
    assert!((scorecard.overall_pass_at_1 - 1.0).abs() < 1e-9);
}

#[test]
fn organic_search_to_compilation_suite_15_achieves_100_percent_across_all_spokes() {
    let suite = canonical_organic_search_to_compilation_suite_24();
    assert_eq!(suite.len(), 24);
    let scorecard = evaluate_organic_search_spoke_suite(&suite);
    println!("\n{}", scorecard.to_markdown());
    for r in &scorecard.receipts {
        println!(
            "  [{}] spoke={} | query=\"{}\" | top_hit=\"{}\" (rank={:?}, RR={:.2}, unbiased_rank={:?}, unbiased_RR={:.2}, grounding={:.0}%, distractor_ok={}) | stage=\"{}\" | latency={}us",
            r.case_id,
            r.spoke.as_str(),
            r.search_query,
            r.top_retrieved_path,
            r.matched_rank,
            r.reciprocal_rank,
            r.unbiased_matched_rank,
            r.unbiased_reciprocal_rank,
            r.evidence_grounding_ratio * 100.0,
            r.distractor_resilient,
            r.compilation_stage_detail,
            r.latency_us,
        );
        assert!(
            !r.top_retrieved_path.contains("spoke_roundtrip.rs"),
            "benchmark case `{}` must retrieve from production workspace files, not spoke_roundtrip.rs",
            r.case_id
        );
    }
    assert_eq!(scorecard.total_cases, 24);
    assert_eq!(scorecard.spokes.len(), 8);
    assert!(
        (scorecard.overall_search_recall_at_5 - 1.0).abs() < 1e-9,
        "overall_search_recall_at_5={}",
        scorecard.overall_search_recall_at_5
    );
    assert!(
        scorecard.overall_mrr >= 0.90,
        "overall_mrr={}",
        scorecard.overall_mrr
    );
    assert!(
        (scorecard.overall_unbiased_search_recall_at_5 - 1.0).abs() < 1e-9,
        "overall_unbiased_search_recall_at_5={}",
        scorecard.overall_unbiased_search_recall_at_5
    );
    assert!(
        scorecard.overall_unbiased_mrr >= 0.90,
        "overall_unbiased_mrr={}",
        scorecard.overall_unbiased_mrr
    );
    assert!(
        (scorecard.overall_grounding_ratio - 1.0).abs() < 1e-9,
        "overall_grounding_ratio={}",
        scorecard.overall_grounding_ratio
    );
    assert!(
        (scorecard.overall_distractor_resilience_rate - 1.0).abs() < 1e-9,
        "overall_distractor_resilience_rate={}",
        scorecard.overall_distractor_resilience_rate
    );
    assert!(
        (scorecard.overall_compilation_pass_rate - 1.0).abs() < 1e-9,
        "overall_compilation_pass_rate={}",
        scorecard.overall_compilation_pass_rate
    );
    assert!(
        (scorecard.overall_end_to_end_pass_at_1 - 1.0).abs() < 1e-9,
        "overall_end_to_end_pass_at_1={}",
        scorecard.overall_end_to_end_pass_at_1
    );

    let contract_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/reports/mens-organic-search-spoke-scorecard.v1.json");
    if std::env::var("UPDATE_ORGANIC_SCORECARD").is_ok() {
        let pretty = serde_json::to_string_pretty(&scorecard.to_contract_json()).unwrap();
        std::fs::write(&contract_path, format!("{pretty}\n")).unwrap();
    }
    let raw = std::fs::read_to_string(&contract_path)
        .expect("read contracts/reports/mens-organic-search-spoke-scorecard.v1.json");
    let committed: serde_json::Value =
        serde_json::from_str(&raw).expect("parse mens-organic-search-spoke-scorecard.v1.json");
    assert_eq!(committed, scorecard.to_contract_json());
}

#[cfg(feature = "database")]
#[test]
fn organic_scorecard_persists_and_rehydrates_from_voxdb() {
    fn block_on_spin<F: std::future::Future>(fut: F) -> F::Output {
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        let mut pinned = std::pin::pin!(fut);
        loop {
            if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                return val;
            }
            std::thread::yield_now();
        }
    }

    block_on_spin(async {
        let db = vox_db::VoxDb::in_memory().await.expect("in-memory VoxDb");
        let suite = canonical_organic_search_to_compilation_suite_24();
        let scorecard = evaluate_organic_search_spoke_suite(&suite);
        let uri = persist_organic_scorecard_to_db(&db, "organic-bench-24-v1", &scorecard)
            .await
            .expect("persist organic scorecard");
        assert!(uri.starts_with("trusty://mens-organic-scorecard/"));
        let loaded = load_organic_scorecard_from_db(&db, "organic-bench-24-v1")
            .await
            .expect("load organic scorecard");
        assert_eq!(loaded.to_contract_json(), scorecard.to_contract_json());
        assert_eq!(loaded.receipts.len(), scorecard.receipts.len());

        // Persist a second run and verify longitudinal history & non-decreasing trend (PERSIST-BENCH-02)
        let _uri2 = persist_organic_scorecard_to_db(&db, "organic-bench-24-v2", &scorecard)
            .await
            .expect("persist organic scorecard v2");
        let history = list_organic_scorecard_history_from_db(&db)
            .await
            .expect("list organic scorecard history");
        assert_eq!(history.len(), 2);
        let scorecards: Vec<OrganicSearchSpokeScorecardV1> =
            history.into_iter().map(|(_, _, sc)| sc).collect();
        assert!(verify_organic_scorecard_non_decreasing_trend(&scorecards).is_ok());

        // Emit ModelBenchCase telemetry rows and verify shape audit (PERSIST-BENCH-04)
        let recorded =
            record_organic_scorecard_telemetry_to_db(&db, "mens/bench25-cycle-2", &scorecard)
                .await
                .expect("record organic scorecard telemetry");
        assert_eq!(recorded, 24);
        let shape_report = db
            .audit_research_metric_shapes("bench:model_bench_case")
            .await
            .expect("audit_research_metric_shapes");
        assert_eq!(shape_report.total_rows_checked, 24);
        assert_eq!(shape_report.valid_typed_rows, 24);
        assert!(shape_report.is_clean());
    });
}

#[test]
fn organic_benchmark_decontamination_detects_leakage_and_passes_clean_records() {
    let suite_25 = canonical_five_spoke_benchmark_suite_25();
    let scorecard_25 = evaluate_multi_spoke_suite(&suite_25);
    let admitted_25: Vec<TrainingRecordV2> = scorecard_25
        .receipts
        .into_iter()
        .filter_map(|r| r.admitted_record)
        .collect();
    assert!(!admitted_25.is_empty());
    assert!(
        verify_organic_benchmark_decontamination(&admitted_25).is_ok(),
        "25-problem benchmark records must have zero collision with 24-problem organic holdout"
    );

    let organic_suite = canonical_organic_search_to_compilation_suite_24();
    let first_organic = &organic_suite[0].roundtrip_case;
    let leaked_record = TrainingRecordV2::new(
        first_organic.spoke,
        &first_organic.prompt,
        "system",
        &first_organic.candidates[0],
        OracleReceipt {
            spoke: first_organic.spoke,
            passed: true,
            stage: "test".to_string(),
            diagnostics: Vec::new(),
            exit_code: Some(0),
        },
        RecordProvenance {
            origin: "test".to_string(),
            source_id: "leak".to_string(),
            task_family: "leak".to_string(),
            difficulty: 1,
        },
    );
    assert!(
        verify_organic_benchmark_decontamination(&[leaked_record]).is_err(),
        "leaked organic benchmark record must be flagged by decontamination verifier"
    );
}

#[test]
fn grammar_unification_voxlang_mcp_and_eight_spoke_provenance_guards() {
    use crate::training::contract::OrganicSpokeKind;

    // 1. Retired decorators (@island, @table, @query, @component, @mcp.tool) and `ret` must be rejected.
    for bad_vox in [
        "@island fn Counter() to Element { return div() }",
        "@table type User { id: int }",
        "@query fn get_user(id: int) to int { return id }",
        "fn calc(x: int) to int {\n    ret x + 1\n}",
        "durable fn run_step(x: int) to int { return x }",
        "@pure fn bad_effect() to int {\n    let _ = time.now()\n    return 1\n}",
    ] {
        let receipt = verify_voxlang_roundtrip("retired_guard", "Write Vox code", bad_vox);
        assert!(
            !receipt.execution_passed,
            "retired or rule-violating Vox must be rejected: `{bad_vox}`"
        );
        assert!(receipt.admitted_record.is_none());
    }

    // 2. Canonical bare-keyword Vox constructs compile and pass admission.
    let canonical_vox = "table AuditEvent {\n    event_id: int\n    label: str\n}\n\nfn score_event(delta: int) to int {\n    return delta + 1\n}\n";
    let ok_vox =
        verify_voxlang_roundtrip("canonical_vox", "Define AuditEvent table", canonical_vox);
    assert!(
        ok_vox.execution_passed,
        "canonical bare-keyword table+fn must pass: {:?}",
        ok_vox.oracle_receipt.diagnostics
    );
    assert!(ok_vox.constructs_covered.contains(&"table".to_string()));
    assert!(ok_vox.constructs_covered.contains(&"fn".to_string()));

    // 3. VoxLang-first automation rejects .ps1/.sh/.py, shell_exec, and raw secret env.get.
    let bad_ps1 = "```powershell\nGet-ChildItem crates\n```";
    assert!(
        !verify_voxlang_automation_roundtrip("bad_ps1", "Automate check", bad_ps1).execution_passed
    );
    let bad_shell_exec = "fn main() to Unit {\n    shell_exec(\"rm -rf /\")\n}";
    assert!(
        !verify_voxlang_automation_roundtrip("bad_exec", "Run command", bad_shell_exec)
            .execution_passed
    );
    let good_voxlang = "fn chunk_size() to int {\n    return 50\n}\n";
    let ok_script = verify_voxlang_automation_roundtrip(
        "good_voxlang",
        "Write a .vox automation helper",
        good_voxlang,
    );
    assert!(ok_script.execution_passed);
    assert_eq!(
        ok_script
            .admitted_record
            .as_ref()
            .map(|r| r.provenance.task_family.as_str()),
        Some("vox_script_interp")
    );

    // 4. MCP tool verifier rejects retired KB/Graphify tools (`vox_kb_store`, `vox_graphify`).
    let bad_mcp = r#"{"tool_name":"vox_kb_store","arguments":{"key":"k","value":"v"}}"#;
    let bad_mcp_receipt = verify_mcp_roundtrip("bad_mcp", "Store KB", bad_mcp, None);
    assert!(!bad_mcp_receipt.execution_passed);
    assert!(
        bad_mcp_receipt
            .oracle_receipt
            .diagnostics
            .iter()
            .any(|d| d.contains("retired MCP tool"))
    );

    // 5. All 8 spokes preserve their exact `OrganicSpokeKind` via `effective_organic_spoke()`.
    let speech_rt = verify_speech_to_code_roundtrip(
        "speech_1",
        "um define fn add_one(x: int) to int",
        "fn add_one(x: int) to int {\n    return x + 1\n}",
    );
    let speech_rec = speech_rt.admitted_record.expect("speech record");
    assert_eq!(
        speech_rec.effective_organic_spoke(),
        OrganicSpokeKind::SpeechToCode
    );
    assert!(
        speech_rec
            .organic_trusty_uri()
            .starts_with("trusty://vox-corpus/speech_to_code/")
    );

    let populi_rt = verify_populi_mesh_roundtrip(
        "populi_1",
        "Dispatch to Populi mesh",
        r#"{"target_node_id":"populi-node-alpha","capability_tier":"gpu_cuda","vram_min_gb":24,"iroh_endpoint":"iroh://node-alpha.populi.vox:4433","quorum_size":3,"payload_kind":"a2a_task_envelope","timeout_ms":5000}"#,
    );
    let populi_rec = populi_rt.admitted_record.expect("populi record");
    assert_eq!(
        populi_rec.effective_organic_spoke(),
        OrganicSpokeKind::PopuliMesh
    );
    assert!(
        populi_rec
            .organic_trusty_uri()
            .starts_with("trusty://vox-corpus/populi_mesh/")
    );

    let skill_rt = verify_skill_creation_roundtrip(
        "skill_1",
        "Author SKILL.md",
        "---\nname: test-skill\ndescription: \"Valid skill description for testing.\"\nmetadata:\n  vox-id: vox.skills.test-skill\n  vox-category: testing\n  vox-tools: vox_compiler_check\n---\n# Test Skill\n\nThis skill validates agentskills compliance and doctests.\n\n```vox\nfn check_skill(x: int) to int {\n    return x + 1\n}\n```\n",
    );
    let skill_rec = skill_rt.admitted_record.expect("skill record");
    assert_eq!(
        skill_rec.effective_organic_spoke(),
        OrganicSpokeKind::SkillCreation
    );
    assert!(
        skill_rec
            .organic_trusty_uri()
            .starts_with("trusty://vox-corpus/skill_creation/")
    );

    // 6. Workspace-aware rustc oracle compiles valid Rust and rejects E0502 borrow errors.
    assert!(
        verify_rust_snippet_with_workspace_deps(
            "pub fn sum_slice(xs: &[i64]) -> i64 { xs.iter().copied().sum() }",
            &["serde"]
        )
        .is_ok()
    );
}

#[test]
fn adversarial_and_real_query_spoke_suite_48_achieves_100_percent_across_all_spokes() {
    let suite = canonical_adversarial_and_real_query_spoke_suite_48();
    assert_eq!(suite.len(), 48);
    let scorecard = evaluate_organic_search_spoke_suite(&suite);
    assert_eq!(scorecard.total_cases, 48);
    assert_eq!(scorecard.spokes.len(), 8);
    for r in &scorecard.receipts {
        assert!(
            r.roundtrip_receipt.pass_at_1 && r.compilation_verified && r.distractor_resilient,
            "case `{}` (spoke={}) failed: stage=`{}`, diags={:?}",
            r.case_id,
            r.spoke.as_str(),
            r.compilation_stage_detail,
            r.roundtrip_receipt.oracle_receipt.diagnostics
        );
    }
    for s in &scorecard.spokes {
        assert_eq!(s.total_cases, 6);
        assert!(
            (s.end_to_end_pass_at_1 - 1.0).abs() < 1e-9,
            "spoke {} end_to_end_pass_at_1={}",
            s.spoke.as_str(),
            s.end_to_end_pass_at_1
        );
    }
    assert!((scorecard.overall_end_to_end_pass_at_1 - 1.0).abs() < 1e-9);
    assert!((scorecard.overall_compilation_pass_rate - 1.0).abs() < 1e-9);
    assert!((scorecard.overall_distractor_resilience_rate - 1.0).abs() < 1e-9);

    // Verify multi-block SKILL.md rejects broken 2nd vox block when not marked `// vox:skip`
    let bad_multi_skill = "---\nname: bad-multi-skill\ndescription: Skill with a valid first block and broken second block.\nmetadata:\n  vox-category: testing\n  vox-tools: vox_compiler_check\n---\n# Multi Block Skill\n\nValid first block:\n\n```vox\nfn ok_fn(x: int) to int { return x + 1 }\n```\n\nBroken second block:\n\n```vox\n@table type RetiredDecorator { id: int }\n```\n";
    let bad_skill_rt =
        verify_skill_creation_roundtrip("bad_multi_skill", "Author skill", bad_multi_skill);
    assert!(
        !bad_skill_rt.execution_passed,
        "second broken ```vox block without // vox:skip must fail doctest verification"
    );
}

#[test]
fn adversarial_and_real_query_spoke_suite_64_achieves_100_percent_across_all_spokes() {
    let suite = canonical_adversarial_and_real_query_spoke_suite_64();
    assert_eq!(suite.len(), 64);
    let scorecard = evaluate_organic_search_spoke_suite(&suite);
    assert_eq!(scorecard.total_cases, 64);
    assert_eq!(scorecard.spokes.len(), 8);
    for r in &scorecard.receipts {
        assert!(
            r.roundtrip_receipt.pass_at_1 && r.compilation_verified && r.distractor_resilient,
            "case `{}` (spoke={}) failed: stage=`{}`, diags={:?}",
            r.case_id,
            r.spoke.as_str(),
            r.compilation_stage_detail,
            r.roundtrip_receipt.oracle_receipt.diagnostics
        );
    }
    for s in &scorecard.spokes {
        assert_eq!(s.total_cases, 8);
        assert!(
            (s.end_to_end_pass_at_1 - 1.0).abs() < 1e-9,
            "spoke {} end_to_end_pass_at_1={}",
            s.spoke.as_str(),
            s.end_to_end_pass_at_1
        );
    }
    assert!((scorecard.overall_end_to_end_pass_at_1 - 1.0).abs() < 1e-9);
    assert!((scorecard.overall_compilation_pass_rate - 1.0).abs() < 1e-9);
    assert!((scorecard.overall_distractor_resilience_rate - 1.0).abs() < 1e-9);

    // Also verify escaped quotes inside shell pipelines do not false-positive fail on `;`
    let escaped_rg = verify_shell_roundtrip(
        "shell_escaped_quote",
        "Search for a;b pattern",
        "rg \"\\\"schema;version\\\"\" crates/vox-corpus",
        &[],
    );
    assert!(escaped_rg.execution_passed);
}
