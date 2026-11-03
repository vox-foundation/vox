//! Integration: every crate in `tiers.a_relational.allow_direct_access` of
//! `contracts/db/data-storage-policy.v1.yaml` must appear (as a `crates/<name>/`
//! prefix) in `docs/agents/turso-import-allowlist.txt`, OR be one of the
//! built-in prefixes hard-coded in `run_body_helpers/guards.rs`.

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
fn policy_allowlist_parity_passes_on_main() {
    let bin = env!("CARGO_BIN_EXE_vox");
    let out = Command::new(bin)
        .current_dir(workspace_root())
        .args(["ci", "policy-allowlist-parity"])
        .output()
        .expect("spawn vox ci policy-allowlist-parity");
    assert!(
        out.status.success(),
        "policy-allowlist-parity should exit 0; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}
