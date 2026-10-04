//! `vox emit client` (== `vox build --target=client`) on any file with a
//! `component` writes `<out_dir>/components/<Name>.tsx` in Library mode
//! (see `crates/vox-codegen-ts/src/emitter.rs`), but the client-target write
//! loop in `commands::build::run_inner` did not `create_dir_all` the file's
//! parent before `fs::write`, so it failed with `os error 2` on a fresh
//! `out_dir` unless something else had already created `components/`.
//! Regression test for that bug (2026-09-21).

use std::path::PathBuf;

#[tokio::test(flavor = "multi_thread")]
async fn emit_client_creates_nested_component_dir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let vox_file = tmp.path().join("main.vox");
    std::fs::write(
        &vox_file,
        "component Dashboard() {\n    view: text() { \"hi\" }\n}\n",
    )
    .expect("write vox fixture");
    let out_dir: PathBuf = tmp.path().join("dist");

    vox_cli::commands::build::run(
        &vox_file,
        &out_dir,
        None,
        Some(vox_config::BuildTarget::Client),
        false,
        false,
        vox_cli::cli_args::BuildMode::Library,
        vox_cli::RustAppShell::default(),
        None,
        None,
    )
    .await
    .expect("emit client should succeed and create dist/components/");

    assert!(
        out_dir.join("components/Dashboard.tsx").is_file(),
        "expected dist/components/Dashboard.tsx to be written"
    );
}
