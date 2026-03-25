//! Objective evidence for hybrid migration claims (narrative vs checks).

#[test]
fn run_benchmark_avoids_invalid_vox_and_fake_clean() {
    let src = include_str!("run_benchmark.rs");
    assert!(
        src.contains("print(str(") && src.contains("hello from vox run benchmark"),
        "benchmark Vox fixture must use print(), not println"
    );
    assert!(
        !src.contains("vox clean"),
        "benchmark must not assume a `vox clean` subcommand"
    );
}

#[test]
fn script_execution_integration_test_is_feature_gated() {
    let cargo_toml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    assert!(
        cargo_toml.contains("name = \"run_mode_dispatch\"")
            && cargo_toml.contains("path = \"tests/run_mode_dispatch.rs\"")
            && cargo_toml.contains("required-features = [\"script-execution\"]"),
        "run_mode_dispatch integration test must stay behind `required-features = [\"script-execution\"]`"
    );
}

#[test]
fn populi_pipeline_ps1_is_thin_delegate() {
    let ps1 = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/run_mens_pipeline.ps1"
    ));
    assert!(
        ps1.contains("mens") && ps1.contains("pipeline"),
        "PS1 should call `vox mens pipeline`"
    );
    assert!(
        !ps1.contains("corpus extract"),
        "orchestration belongs in Rust (`vox mens pipeline`), not PS1"
    );
}
