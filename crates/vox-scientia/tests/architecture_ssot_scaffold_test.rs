use vox_scientia::manuscript::scaffold::architecture_ssot::{
    ArchitectureSsotInput, CodebaseReference, CompetitiveComparisonRow, EmpiricalClaimRow,
    GapRecommendation, RoadmapPhase, SandboxExecutionRecord, SeverityTier,
    render_architecture_ssot,
};

#[test]
fn test_render_architecture_ssot_generates_all_seven_sections() {
    let input = ArchitectureSsotInput {
        title: "Test System Architecture (2026)".into(),
        description: "Empirical evaluation of test system.".into(),
        category: "Architecture SSOTs".into(),
        status: "current".into(),
        training_eligible: true,
        training_rationale: Some("Empirically verified test system.".into()),
        sort_order: None,
        session_id: 42,
        stability_score: 0.88,
        slug: "test-system".into(),
        executive_summary: "Executive summary text.".into(),
        hypothesis: "Hypothesis text.".into(),
        empirical_outcome_summary: "Outcome text.".into(),
        codebase_refs: vec![CodebaseReference {
            crate_name: "vox-test".into(),
            file_path: "crates/vox-test/src/lib.rs".into(),
            line_start: Some(10),
            line_end: Some(25),
            observation: "Defines core test harness.".into(),
        }],
        competitive_matrix: vec![CompetitiveComparisonRow {
            dimension: "Latency".into(),
            vox_feature: "12ms".into(),
            alternative_a_name: "Gemini".into(),
            alternative_a_val: "450ms".into(),
            alternative_b_name: "Claude".into(),
            alternative_b_val: "380ms".into(),
        }],
        verified_claims: vec![EmpiricalClaimRow {
            subject: "Candle Metal".into(),
            predicate: "forward pass latency".into(),
            object: "14.2ms".into(),
            verdict: "Supported".into(),
            confidence: 0.94,
            primary_evidence_source: "local benchmark".into(),
            trusty_uri: Some("urn:nanopub:test".into()),
        }],
        sandbox_probes: vec![SandboxExecutionRecord {
            title: "Metal probe".into(),
            language: "vox".into(),
            original_snippet: "fn test() -> bool { true }".into(),
            repaired_snippet: None,
            compiler_output: "Finished release [optimized]".into(),
            success: true,
        }],
        gaps_and_recommendations: vec![GapRecommendation {
            severity: SeverityTier::P0Critical,
            title: "Buffer allocation overhead".into(),
            description: "High reallocation cost.".into(),
            remedy: "Use arena allocator".into(),
        }],
        roadmap_phases: vec![RoadmapPhase {
            phase_number: 1,
            title: "Arena Allocation".into(),
            description: "Replace standard vector with bumpalo.".into(),
            verification_gate: "cargo test -p vox-test".into(),
        }],
    };

    let markdown = render_architecture_ssot(&input).expect("renders markdown");

    assert!(markdown.contains("title: \"Test System Architecture (2026)\""));
    assert!(markdown.contains("category: \"Architecture SSOTs\""));
    assert!(markdown.contains("status: \"current\""));
    assert!(markdown.contains("## 1. Executive Summary & Codebase Reality"));
    assert!(markdown.contains("## 2. SOTA Competitive Analysis & Technical Benchmark Matrix"));
    assert!(markdown.contains("## 3. Verified Empirical Claims Table"));
    assert!(markdown.contains("## 4. Code Verification & Sandbox Reproducibility Evidence"));
    assert!(markdown.contains("## 5. Gap Analysis & Concrete Architectural Recommendations"));
    assert!(markdown.contains("## 6. Phased Implementation Roadmap & Verification Gates"));
    assert!(markdown.contains("crates/vox-test/src/lib.rs"));
    assert!(markdown.contains("// vox:skip empirical sandbox probe"));
}
