use vox_cli::commands::audit_research_parity::{
    ResearchParityArgs, extract_empirical_test_references, run_research_parity_audit_sync,
};

#[test]
fn test_extract_empirical_test_references() {
    let sample = r#"
Here is `crates/vox-populi/tests/metal_optimization_probe_test.rs` and another
([`crates/vox-plugin-browser/tests/ax_snapshot_probe_test.rs`]).
Also `tests/sample_test.rs`. And some other word without test.
"#;
    let refs = extract_empirical_test_references(sample);
    assert_eq!(
        refs,
        vec![
            "crates/vox-populi/tests/metal_optimization_probe_test.rs",
            "crates/vox-plugin-browser/tests/ax_snapshot_probe_test.rs",
            "tests/sample_test.rs",
        ]
    );
}

#[test]
fn test_research_parity_audit_detects_probes() {
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let docs_dir = repo_root.join("docs/src/architecture");
    let args = ResearchParityArgs {
        docs_dir: docs_dir.clone(),
        json: false,
    };
    let count = run_research_parity_audit_sync(&args).expect("run audit");
    assert!(count >= 2); // metal_optimization_probe_test.rs and ax_snapshot_probe_test.rs

    let json_args = ResearchParityArgs {
        docs_dir,
        json: true,
    };
    let json_count = run_research_parity_audit_sync(&json_args).expect("run json audit");
    assert_eq!(count, json_count);
}
