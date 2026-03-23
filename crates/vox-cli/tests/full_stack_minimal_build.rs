//! Golden `vox build` for `examples/full_stack_minimal.vox` (no Node).
#![allow(missing_docs)]

use std::path::PathBuf;

use vox_cli::commands::build;

#[tokio::test]
async fn full_stack_minimal_build_writes_app_tsx_and_api() {
    let mut repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    repo.pop();
    repo.pop();
    let vox_file = repo.join("crates/vox-parser/tests/golden/full_stack_minimal.vox");

    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out");
    build::run(&vox_file, &out).await.expect("build");

    assert!(out.join("App.tsx").is_file());
    assert!(out.join("Home.tsx").is_file());
    assert!(out.join("api.ts").is_file());
}
