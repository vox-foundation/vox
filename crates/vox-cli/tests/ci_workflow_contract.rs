//! Regression: GitHub Actions CI must stay on `vox ci` / `cargo run -p vox-cli` guards
//! (hybrid migration — do not reintroduce Python doc-inventory or raw bash matrices).

#[test]
fn github_ci_doc_inventory_is_rust() {
    let yml = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.github/workflows/ci.yml"
    ));
    assert!(
        yml.contains("ci command-compliance"),
        "ci.yml should run `vox ci command-compliance` for registry/docs/MCP parity"
    );
    assert!(
        yml.contains("ci doc-inventory verify"),
        "ci.yml should verify inventory via `vox ci doc-inventory verify`"
    );
    assert!(
        !yml.contains("verify_doc_inventory_fresh.py"),
        "retired Python doc-inventory verifier must not return to ci.yml"
    );
}

#[test]
fn github_ci_populi_gate_is_unified() {
    let yml = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.github/workflows/ci.yml"
    ));
    assert!(
        yml.contains("ci mens-gate --profile ci_full"),
        "ci.yml should run a single Mens gate profile (ci_full)"
    );
    assert!(
        !yml.contains("populi_release_gate.sh"),
        "do not call populi_release_gate.sh from CI; use `vox ci mens-gate`"
    );
}

#[test]
fn ml_workflow_grammar_drift_and_eval_stay_native() {
    let yml = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.github/workflows/ml_data_extraction.yml"
    ));
    assert!(
        yml.contains("ci grammar-drift") && yml.contains("--emit github"),
        "ml_data_extraction.yml should detect grammar drift via `vox ci grammar-drift`"
    );
    assert!(
        yml.contains("corpus eval") && yml.contains("--print-summary"),
        "ml_data_extraction.yml should summarize eval via `vox corpus eval --print-summary`"
    );
    assert!(
        !yml.contains("python3 -c"),
        "do not use inline Python in ml_data_extraction.yml; use Vox/Rust CLI output"
    );
}
