//! Smoke: `vox migrate web` walks `.vox` and reports without panicking.

use std::fs;

use tempfile::tempdir;

#[test]
fn migrate_web_scans_temp_vox_file() {
    let dir = tempdir().expect("tempdir");
    let vox = dir.path().join("sample.vox");
    fs::write(&vox, "component X() {\n  view: <div>\"hi\"</div>\n}\n").expect("write");

    vox_cli::commands::migrate::run(vox_cli::commands::migrate::MigrateCmd::Web(
        vox_cli::commands::migrate::WebMigrateArgs {
            path: dir.path().to_path_buf(),
            json: false,
            write: false,
            check: false,
        },
    ))
    .expect("migrate web");
}

#[test]
fn migrate_web_json_reports_parse_error() {
    let dir = tempdir().expect("tempdir");
    let vox = dir.path().join("bad.vox");
    fs::write(&vox, "this is not valid vox top level").expect("write");

    vox_cli::commands::migrate::run(vox_cli::commands::migrate::MigrateCmd::Web(
        vox_cli::commands::migrate::WebMigrateArgs {
            path: dir.path().to_path_buf(),
            json: true,
            write: false,
            check: false,
        },
    ))
    .expect("migrate web json");
}

#[test]
fn migrate_web_write_replaces_at_component_fn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vox = dir.path().join("legacy.vox");
    fs::write(
        &vox,
        "@component fn X() to Element {\n  view: <div>\"hi\"</div>\n}\n",
    )
    .expect("write");

    vox_cli::commands::migrate::run(vox_cli::commands::migrate::MigrateCmd::Web(
        vox_cli::commands::migrate::WebMigrateArgs {
            path: dir.path().to_path_buf(),
            json: false,
            write: true,
            check: false,
        },
    ))
    .expect("migrate write");

    let out = fs::read_to_string(&vox).expect("read");
    assert!(
        out.contains("component X()"),
        "expected keyword patch, got: {out:?}"
    );
    assert!(!out.contains("@component fn"));
}

#[test]
fn migrate_web_check_fails_on_findings() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vox = dir.path().join("ctx.vox");
    fs::write(&vox, "context C { }\n").expect("write");

    let err = vox_cli::commands::migrate::run(vox_cli::commands::migrate::MigrateCmd::Web(
        vox_cli::commands::migrate::WebMigrateArgs {
            path: dir.path().to_path_buf(),
            json: true,
            write: false,
            check: true,
        },
    ))
    .expect_err("check should fail");

    assert!(err.to_string().contains("migrate web --check"), "{err:?}");
}
