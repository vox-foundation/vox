#![allow(missing_docs)]
//! Smoke tests for vox-bootstrap engine evaluation logic.

use vox_bootstrap::engine::{BootstrapOptions, evaluate};

#[test]
fn evaluate_default_options_checks_rustc_and_cargo() {
    let opts = BootstrapOptions {
        dev: false,
        install_clang: false,
        apply: false,
        install: false,
        source_only: false,
        version: None,
    };
    let report = evaluate(opts);

    // rustc and cargo are always checked
    assert!(
        report.items.iter().any(|i| i.id == "rustc"),
        "rustc probe missing"
    );
    assert!(
        report.items.iter().any(|i| i.id == "cargo"),
        "cargo probe missing"
    );

    // Since we run this test in a Rust workspace, these should pass.
    let rustc_item = report.items.iter().find(|i| i.id == "rustc").unwrap();
    assert!(rustc_item.ok, "rustc should be installed in CI/dev");
    let cargo_item = report.items.iter().find(|i| i.id == "cargo").unwrap();
    assert!(cargo_item.ok, "cargo should be installed in CI/dev");
}

#[test]
fn evaluate_dev_options_checks_rustfmt_and_clippy() {
    let opts = BootstrapOptions {
        dev: true,
        install_clang: false,
        apply: false,
        install: false,
        source_only: false,
        version: None,
    };
    let report = evaluate(opts);

    assert!(
        report.items.iter().any(|i| i.id == "rustfmt"),
        "rustfmt probe missing with dev=true"
    );
    assert!(
        report.items.iter().any(|i| i.id == "clippy"),
        "clippy probe missing with dev=true"
    );
}

#[test]
fn platform_is_populated() {
    let opts = BootstrapOptions {
        dev: false,
        install_clang: false,
        apply: false,
        install: false,
        source_only: false,
        version: None,
    };
    let report = evaluate(opts);
    assert!(
        !report.platform.is_empty(),
        "platform string should not be empty"
    );
}
