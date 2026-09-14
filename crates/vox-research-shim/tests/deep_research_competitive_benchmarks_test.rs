use vox_research_shim::research::claims::Claim;
use vox_research_shim::research::domain::codegen::{
    attempt_code_self_correction, wrap_code_snippet_if_needed,
};
use vox_research_shim::research::orchestrator::wave::{
    ContradictionCategory, ContradictionRecord, ContradictionStatus, WaveExecutionPlan,
};
use vox_research_shim::research::verifier::{ClaimVerdict, Verdict};

#[tokio::test]
async fn benchmark_rust_api_signature_self_correction() {
    // Simulates an outdated API call with mismatched argument type (std::thread::sleep(10) instead of Duration)
    let outdated_code = "pub fn wait_a_bit() {\n    std::thread::sleep(10);\n}";
    let wrapped = wrap_code_snippet_if_needed(outdated_code);

    let res = attempt_code_self_correction(&wrapped, &[], 2, |_code, stderr| async move {
        assert!(
            stderr.contains("E0308") || stderr.contains("mismatched types"),
            "Expected E0308 diagnostic from rustc, got: {stderr}"
        );
        let fixed = "pub fn wait_a_bit() {\n    // Updated from raw integer to Duration for modern Rust\n    std::thread::sleep(std::time::Duration::from_millis(10));\n}";
        Ok((
            wrap_code_snippet_if_needed(fixed),
            "Replaced raw integer with std::time::Duration::from_millis(10)".to_string(),
        ))
    })
    .await
    .unwrap();

    assert!(
        res.final_passed,
        "Self-correction should successfully produce working code"
    );
    assert_eq!(res.iterations, 1);
    assert!(res.corrected_code.is_some());
}

#[test]
fn benchmark_wave_stopping_metric_convergence() {
    let mut plan = WaveExecutionPlan::new("bm-convergence", 3);
    assert_eq!(plan.compute_stability(), 0.0);
    assert!(!plan.should_early_terminate());

    // Populate with 4 claims: 3 supported, 1 unverified (75% supported, 25% unverified)
    plan.resolved_verdicts = vec![
        ClaimVerdict {
            claim: Claim {
                claim_id: 1,
                text: "c1".into(),
                is_numeric: false,
                is_recent: false,
                is_named_event: false,
            },
            verdict: Verdict::Supported,
            confidence: 0.95,
            supporting_count: 2,
            contradicting_count: 0,
            evidence_spans: vec![],
            resample_stability: 1.0,
        },
        ClaimVerdict {
            claim: Claim {
                claim_id: 2,
                text: "c2".into(),
                is_numeric: false,
                is_recent: false,
                is_named_event: false,
            },
            verdict: Verdict::Supported,
            confidence: 0.95,
            supporting_count: 2,
            contradicting_count: 0,
            evidence_spans: vec![],
            resample_stability: 1.0,
        },
        ClaimVerdict {
            claim: Claim {
                claim_id: 3,
                text: "c3".into(),
                is_numeric: false,
                is_recent: false,
                is_named_event: false,
            },
            verdict: Verdict::Supported,
            confidence: 0.95,
            supporting_count: 2,
            contradicting_count: 0,
            evidence_spans: vec![],
            resample_stability: 1.0,
        },
        ClaimVerdict {
            claim: Claim {
                claim_id: 4,
                text: "c4".into(),
                is_numeric: false,
                is_recent: false,
                is_named_event: false,
            },
            verdict: Verdict::Unverified,
            confidence: 0.50,
            supporting_count: 0,
            contradicting_count: 0,
            evidence_spans: vec![],
            resample_stability: 0.5,
        },
    ];
    // 25% unverified > 15% max unverified threshold -> must NOT early exit
    assert!(!plan.should_early_terminate());

    // Resolve the 4th claim to Supported
    plan.resolved_verdicts[3].verdict = Verdict::Supported;
    plan.resolved_verdicts[3].confidence = 0.95;
    plan.resolved_verdicts[3].resample_stability = 1.0;

    // Now 100% supported, 0% unverified, 0 unresolved contradictions -> early exit triggers
    assert!(plan.compute_stability() >= 0.85);
    assert!(plan.should_early_terminate());
}

#[tokio::test]
async fn benchmark_empirical_sandbox_overrules_hallucinated_claim() {
    let mut plan = WaveExecutionPlan::new("bm-overrule", 3);
    plan.resolved_verdicts.push(ClaimVerdict {
        claim: Claim {
            claim_id: 42,
            text: "Calling non_existent_function() compiles in Rust".into(),
            is_numeric: false,
            is_recent: false,
            is_named_event: false,
        },
        verdict: Verdict::Supported, // Erroneously supported by an LLM in Wave 1
        confidence: 0.85,
        supporting_count: 1,
        contradicting_count: 0,
        evidence_spans: vec![],
        resample_stability: 0.9,
    });

    plan.unresolved_contradictions.push(ContradictionRecord {
        contradiction_id: 1,
        claim_id_a: 42,
        claim_text_a: "non_existent_function() exists".into(),
        source_url_a: "https://example.com/a".into(),
        claim_id_b: 43,
        claim_text_b: "non_existent_function() does not exist".into(),
        source_url_b: "https://example.com/b".into(),
        category: ContradictionCategory::DirectFactualOpposition,
        severity: 0.9,
        status: ContradictionStatus::Unresolved,
        disambiguation_query: Some("check if non_existent_function exists".into()),
    });

    let broken_code = "pub fn probe() { non_existent_function_xyz(); }";
    let correction = attempt_code_self_correction(broken_code, &[], 1, |code, _err| async move {
        // Repair fails to fix nonexistent function
        Ok((code, "Cannot repair nonexistent function".to_string()))
    })
    .await
    .unwrap();

    assert!(!correction.final_passed);

    // Apply empirical sandbox result to WaveExecutionPlan
    plan.apply_sandbox_self_correction(42, &correction);

    let v = &plan.resolved_verdicts[0];
    assert_eq!(
        v.verdict,
        Verdict::Contradicted,
        "Sandbox MUST overrule LLM verdict"
    );
    assert_eq!(
        v.confidence, 1.0,
        "Empirical failure provides 100% confidence"
    );
    assert!(v.contradicting_count >= 1);

    // Verify contradiction was resolved by empirical sandbox
    assert!(matches!(
        plan.unresolved_contradictions[0].status,
        ContradictionStatus::ResolvedByEmpiricalSandbox { .. }
    ));
}
