use std::process::Command;

#[test]
fn parity_passes_on_current_tree() {
    let status = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["ci", "plugin-skill-parity"])
        .status()
        .expect("vox should run");
    assert!(status.success(), "skill-parity should pass");
}
