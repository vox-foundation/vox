use vox_research_shim::research::claims::Claim;
use vox_research_shim::research::domain::codegen::CodeSelfCorrectionResult;
use vox_research_shim::research::orchestrator::wave::{
    ContradictionCategory, ContradictionRecord, ContradictionStatus, WaveExecutionPlan,
};
use vox_research_shim::research::verifier::{ClaimVerdict, Verdict};

#[test]
fn test_wave_plan_apply_sandbox_self_correction_resolves_contradictions() {
    let mut plan = WaveExecutionPlan::new("session-self-correct", 3);
    plan.resolved_verdicts = vec![ClaimVerdict {
        claim: Claim {
            claim_id: 42,
            text: "fn foo() { bar }".to_string(),
            is_numeric: false,
            is_recent: false,
            is_named_event: false,
        },
        verdict: Verdict::Unverified,
        confidence: 0.5,
        supporting_count: 0,
        contradicting_count: 0,
        evidence_spans: vec![],
        resample_stability: 0.5,
    }];

    plan.unresolved_contradictions = vec![ContradictionRecord {
        contradiction_id: 101,
        claim_id_a: 42,
        claim_text_a: "fn foo() { bar }".to_string(),
        source_url_a: "url-a".to_string(),
        claim_id_b: 99,
        claim_text_b: "fn foo() { let bar = 1; }".to_string(),
        source_url_b: "url-b".to_string(),
        category: ContradictionCategory::DirectFactualOpposition,
        severity: 0.9,
        status: ContradictionStatus::Unresolved,
        disambiguation_query: None,
    }];

    let correction = CodeSelfCorrectionResult {
        initial_passed: false,
        final_passed: true,
        iterations: 1,
        original_code: "fn foo() { bar }".to_string(),
        corrected_code: Some("fn foo() { let bar = 1; }".to_string()),
        initial_error: Some("cannot find value `bar`".to_string()),
        final_error: None,
        repair_explanation: Some("Declared variable `bar`.".to_string()),
    };

    plan.apply_sandbox_self_correction(42, &correction);

    assert_eq!(plan.resolved_verdicts[0].verdict, Verdict::Supported);
    assert_eq!(plan.resolved_verdicts[0].confidence, 1.0);
    // Contradiction must be resolved so early exit can succeed!
    assert!(matches!(
        plan.unresolved_contradictions[0].status,
        ContradictionStatus::ResolvedByEmpiricalSandbox { .. }
    ));
}
