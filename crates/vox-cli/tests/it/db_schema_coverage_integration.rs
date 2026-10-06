//! Integration: every CREATE TABLE in the workspace must live in a crate
//! listed under `tiers.a_relational.{owners, temporary_exceptions}` of
//! `contracts/db/data-storage-policy.v1.yaml`. Anything else fails CI.

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
fn db_schema_coverage_passes() {
    let bin = env!("CARGO_BIN_EXE_vox");
    let out = Command::new(bin)
        .current_dir(workspace_root())
        .args(["ci", "db-schema-coverage"])
        .output()
        .expect("spawn vox ci db-schema-coverage");
    assert!(
        out.status.success(),
        "db-schema-coverage should exit 0;\nstdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}
