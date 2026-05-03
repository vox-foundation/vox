use std::process::Command;

#[test]
fn parity_passes_when_no_plugin_tomls_exist() {
    let status = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["ci", "plugin-catalog-parity"])
        .status()
        .expect("vox should run");
    assert!(status.success(), "parity should pass with empty plugin tree");
}
