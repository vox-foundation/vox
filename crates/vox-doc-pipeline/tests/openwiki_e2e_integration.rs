//! End-to-End Integration Suite for Milestone `v3.6` (`Phases 188–192`, `OPENWIKI-*`).

use std::collections::BTreeMap;
use std::path::Path;

use vox_doc_pipeline::claims::consolidation::{
    DuplicateCluster, plan_duplicate_cluster_consolidation, render_redirects_block,
};
use vox_doc_pipeline::claims::gate::{BaselineFile, evaluate_drift_gate, is_reader_facing_page};
use vox_doc_pipeline::claims::llm_verifier::{
    DocVerifyBudget, evaluate_golden_drift_set, is_valid_line_range_citation_on_disk,
};
use vox_doc_pipeline::claims::self_heal_bot::{
    BotPatchRejection, CandidateDocPatch, run_advisory_claim_verification, run_self_heal_bot_check,
    validate_bot_patch_scope_and_gates,
};
use vox_doc_pipeline::claims::suppressions::SuppressionsFile;
use vox_doc_pipeline::pipeline::openwiki::{
    compute_reverse_dependents, generate_subsystem_hubs, render_crate_dependency_diagram,
};
use vox_doc_pipeline::pipeline::openwiki_symbols::{
    extract_crate_deep_symbols_from_dir, render_deep_crate_api_sections,
};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

#[test]
fn phase_188_deep_crate_symbols_reverse_deps_and_diagram() {
    let root = repo_root();
    let crate_dir = root.join("crates/vox-doc-pipeline");
    let entry_file = crate_dir.join("src/lib.rs");

    let syms = extract_crate_deep_symbols_from_dir(&crate_dir, &entry_file, root, 45);
    assert!(
        !syms.is_empty(),
        "expected deep symbols from vox-doc-pipeline submodules"
    );
    assert!(
        syms.iter()
            .any(|s| s.name.starts_with("claims::") || s.name.starts_with("pipeline::")),
        "expected submodule-prefixed symbols, got: {:?}",
        syms.iter().map(|s| &s.name).collect::<Vec<_>>()
    );

    let mut fwd = BTreeMap::new();
    fwd.insert(
        "vox-cli-ci".to_string(),
        vec!["vox-doc-pipeline".to_string()],
    );
    fwd.insert("vox-cli".to_string(), vec!["vox-doc-pipeline".to_string()]);
    let rev = compute_reverse_dependents(&fwd);
    let used_by = rev.get("vox-doc-pipeline").expect("used_by entry");
    assert_eq!(
        used_by,
        &vec!["vox-cli".to_string(), "vox-cli-ci".to_string()]
    );

    let chart =
        render_crate_dependency_diagram("vox-doc-pipeline", &["vox-compiler".to_string()], used_by)
            .expect("valid diagram");
    assert!(chart.starts_with("graph {"), "{chart}");
    assert!(vox_diagram::check(&chart).valid, "{chart}");

    let rendered = render_deep_crate_api_sections(
        "vox-doc-pipeline",
        &["vox-compiler".to_string()],
        used_by,
        &syms,
    )
    .expect("render sections");
    assert!(rendered.contains("## Dependency Topology\n\n```vox-diagram\n"));
    assert!(rendered.contains("## Reverse Dependents (Used By)"));
    assert!(rendered.contains("### Deep Public Symbols (Source-Anchored)"));
}

#[test]
fn phase_189_subsystem_hubs_materialized_and_lint_clean() {
    let root = repo_root();
    if std::env::var_os("VOX_WIKI_SYNC_WRITE").is_some() {
        let _ = generate_subsystem_hubs(root, true).expect("materialize subsystem hubs");
    }
    let paths = generate_subsystem_hubs(root, false).expect("verify subsystem hubs on disk");
    assert_eq!(paths.len(), 7);
    assert!(
        paths
            .iter()
            .any(|p| p == "docs/src/architecture/subsystems/index.generated.md")
    );

    let args = vec![
        "vox-doc-pipeline".to_string(),
        "--lint-only".to_string(),
        "--paths".to_string(),
        "architecture/subsystems,architecture/openwiki-self-healing-docs-ssot-2026.md".to_string(),
    ];
    let code = vox_doc_pipeline::pipeline::lint_in(root, &args);
    assert_eq!(
        code, 0,
        "vox-doc-pipeline --lint-only failed on subsystem hubs or SSOT doc"
    );
}

#[test]
fn phase_190_zero_deficit_baseline_and_duplicate_consolidation() {
    let root = repo_root();
    assert!(!is_reader_facing_page("docs/src/reference/changelog.md"));
    assert!(!is_reader_facing_page(
        "docs/src/reference/env-vars.generated.md"
    ));
    assert!(is_reader_facing_page("docs/src/reference/cli.md"));

    let baseline_path = root.join("contracts/reports/docs-freshness/baseline.v1.json");
    let baseline = BaselineFile::load_from_file(&baseline_path).expect("load baseline");
    assert_eq!(
        baseline.deficits.len(),
        0,
        "baseline.v1.json must have 0 grandfathered deficits"
    );

    let suppressions_path = root.join("contracts/reports/docs-freshness/suppressions.v1.json");
    let suppressions =
        SuppressionsFile::load_from_file(&suppressions_path).expect("load suppressions");
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
    let gate = evaluate_drift_gate(&[], &baseline, &suppressions, today);
    assert!(gate.passed);

    let clusters = vec![DuplicateCluster {
        cluster_id: "vox-codegen-ts".to_string(),
        paths: vec![
            "docs/src/api/vox-codegen-ts.md".to_string(),
            "docs/src/api/vox-compiler.generated.md".to_string(),
        ],
    }];
    let plans = plan_duplicate_cluster_consolidation(&clusters);
    assert_eq!(
        plans[0].canonical_path,
        "docs/src/api/vox-compiler.generated.md"
    );
    let redirects = render_redirects_block(&plans);
    assert!(redirects.contains("/api/vox-codegen-ts/ /api/vox-compiler/ 301"));
}

#[test]
fn phase_191_llm_claim_verifier_and_golden_drift_benchmark() {
    let root = repo_root();
    assert!(is_valid_line_range_citation_on_disk(
        root,
        "[crates/vox-doc-pipeline/src/lib.rs:L1-L4]"
    ));
    assert!(!is_valid_line_range_citation_on_disk(
        root,
        "[crates/vox-doc-pipeline/src/nonexistent_file.rs:L1-L4]"
    ));

    let eval_path = root.join("contracts/reports/docs-freshness/golden-drift-eval-set.v1.json");
    let metrics = evaluate_golden_drift_set(&eval_path).expect("evaluate golden drift set");
    assert!(metrics.evaluated_pairs >= 50);
    assert!(metrics.precision >= 0.80);
    assert!(metrics.false_positive_rate <= 0.10);
    assert!(metrics.eligible_for_bot_promotion);

    let cache_path = root.join("contracts/reports/docs-freshness/llm-verify-cache.v1.json");
    let sample_doc =
        vec![root.join("docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md")];
    let cache = run_advisory_claim_verification(
        root,
        &cache_path,
        DocVerifyBudget::default(),
        Some(&sample_doc),
    )
    .expect("run advisory claim verification");
    assert!(!cache.entries.is_empty());
}

#[test]
fn phase_192_self_heal_bot_gates_and_metrics_ledger() {
    let root = repo_root();
    let ledger = run_self_heal_bot_check(root, false).expect("self heal bot check");
    assert!(!ledger.is_auto_paused());
    assert!(ledger.acceptance_precision() >= 0.80);

    let bad_code_patch = CandidateDocPatch {
        subsystem: "compiler".to_string(),
        target_doc_path: "crates/vox-compiler/src/lib.rs".to_string(),
        updated_markdown: "---\ntitle: \"x\"\n---\n".to_string(),
        evidence_citations: vec!["[crates/vox-compiler/src/lib.rs:L1-L5]".to_string()],
        summary: "bad target".to_string(),
    };
    assert_eq!(
        validate_bot_patch_scope_and_gates(&bad_code_patch),
        Err(BotPatchRejection::OutOfScopePath(
            "crates/vox-compiler/src/lib.rs".to_string()
        ))
    );
}
