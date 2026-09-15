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

#[test]
fn test_escape_pipe_handles_pipes_newlines_and_crlf() {
    use vox_scientia::manuscript::scaffold::common::escape_pipe;

    assert_eq!(escape_pipe("a|b"), "a\\|b");
    assert_eq!(escape_pipe("line1\nline2"), "line1 line2");
    assert_eq!(escape_pipe("line1\r\nline2"), "line1 line2");
    assert_eq!(escape_pipe("line1\rline2"), "line1 line2");
    assert_eq!(escape_pipe("a|b\r\nc|d"), "a\\|b c\\|d");
}

#[test]
fn test_render_markdown_table_escapes_headers_and_rows() {
    use vox_scientia::manuscript::scaffold::common::render_markdown_table;

    let headers = vec!["Col|A", "Col\nB"];
    let rows = vec![
        vec!["Val|1".into(), "Val\r\n2".into()],
        vec!["Plain".into(), "Data".into()],
    ];

    let rendered = render_markdown_table(&headers, &rows);
    assert!(rendered.contains("| Col\\|A | Col B |"));
    assert!(rendered.contains("| :--- | :--- |"));
    assert!(rendered.contains("| Val\\|1 | Val 2 |"));
    assert!(rendered.contains("| Plain | Data |"));
}

#[test]
fn test_render_code_fence_handles_vox_skip_and_backtick_collisions() {
    use vox_scientia::manuscript::scaffold::common::render_code_fence;

    // Vox with skip_doctest
    let vox_code = render_code_fence("vox", "fn probe() -> bool { true }", true);
    assert!(vox_code.starts_with("```vox\n// vox:skip empirical sandbox probe\n"));
    assert!(vox_code.ends_with("```\n"));

    // Vox without skip_doctest
    let vox_no_skip = render_code_fence("vox", "fn probe() -> bool { true }", false);
    assert!(vox_no_skip.starts_with("```vox\nfn probe()"));
    assert!(!vox_no_skip.contains("vox:skip"));

    // Rust code
    let rust_code = render_code_fence("rust", "fn main() {}", true);
    assert!(rust_code.starts_with("```rust\n"));
    assert!(!rust_code.contains("vox:skip"));

    // Collision with triple backticks in code snippet
    let nested_code = "Here is a code block:\n```rust\nlet x = 1;\n```";
    let fenced_nested = render_code_fence("markdown", nested_code, false);
    assert!(fenced_nested.starts_with("````markdown\n"));
    assert!(fenced_nested.ends_with("````\n"));
}

#[test]
fn test_render_architecture_ssot_empty_collections_and_repaired_snippets() {
    let input = ArchitectureSsotInput {
        title: "Minimal Architecture (2026)".into(),
        description: "Minimal test description.\nWith newline.".into(),
        category: "Architecture SSOTs".into(),
        status: "draft".into(),
        training_eligible: false,
        training_rationale: None,
        sort_order: Some(99),
        session_id: 101,
        stability_score: 0.95,
        slug: "minimal-arch".into(),
        executive_summary: "Minimal exec summary.".into(),
        hypothesis: "Hypothesis.".into(),
        empirical_outcome_summary: "Outcome.".into(),
        codebase_refs: vec![],
        competitive_matrix: vec![],
        verified_claims: vec![],
        sandbox_probes: vec![SandboxExecutionRecord {
            title: "Repair probe".into(),
            language: "rust".into(),
            original_snippet: "broken code".into(),
            repaired_snippet: Some("repaired code".into()),
            compiler_output: "compiler error E0308".into(),
            success: false,
        }],
        gaps_and_recommendations: vec![],
        roadmap_phases: vec![],
    };

    let md = render_architecture_ssot(&input).expect("renders markdown");
    assert!(md.contains("title: \"Minimal Architecture (2026)\""));
    assert!(md.contains("description: \"Minimal test description. With newline.\""));
    assert!(md.contains("status: \"draft\""));
    assert!(!md.contains("training_eligible: true"));
    assert!(md.contains("sort_order: 99"));
    assert!(md.contains("Session #101, Stability $S = 0.95$"));
    assert!(md.contains("**Self-Corrected Repair:**"));
    assert!(md.contains("repaired code"));
    assert!(md.contains("compiler error E0308"));
}
