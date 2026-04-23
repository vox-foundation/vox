//! Integration: `vox ci query-all-guard --all` passes on the workspace tree.

use std::path::Path;
use std::process::Command;

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
}

#[test]
fn query_all_guard_all_ok() {
    let bin = env!("CARGO_BIN_EXE_vox");
    let st = Command::new(bin)
        .current_dir(workspace_root())
        .args(["ci", "query-all-guard", "--all"])
        .status()
        .expect("spawn vox ci query-all-guard");
    assert!(st.success(), "query-all-guard --all should exit 0");
}
