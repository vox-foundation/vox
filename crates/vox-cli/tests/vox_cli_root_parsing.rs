//! Smoke: `VoxCliRoot` parses global flags + `completions` / Latin groupings.

use clap::CommandFactory;
use clap::Parser;
use vox_cli::commands::clavis::ClavisCmd;
use vox_cli::{Cli, VoxCliRoot};

#[test]
fn parse_completions_bash() {
    VoxCliRoot::try_parse_from(["vox", "completions", "bash"]).expect("completions bash");
}

#[test]
fn parse_global_color_and_build() {
    let r = VoxCliRoot::try_parse_from(["vox", "--color", "never", "build", "foo.vox"])
        .expect("build with global color");
    assert!(r.global.color.is_some());
}

#[test]
fn parse_fabrica_build() {
    VoxCliRoot::try_parse_from(["vox", "fabrica", "build", "foo.vox"]).expect("fabrica build");
}

#[test]
fn parse_fabrica_visible_alias_fab() {
    VoxCliRoot::try_parse_from(["vox", "fab", "build", "foo.vox"]).expect("fab build");
}

#[test]
fn parse_secrets_alias_routes_to_clavis_doctor() {
    let r = VoxCliRoot::try_parse_from(["vox", "secrets", "doctor"]).expect("secrets doctor");
    assert!(matches!(
        r.cmd,
        Cli::Clavis {
            cmd: ClavisCmd::Doctor { .. }
        }
    ));
}

#[test]
fn root_long_help_mentions_recommended_catalog() {
    let mut cmd = VoxCliRoot::command();
    let help = cmd.render_long_help().to_string();
    assert!(
        help.contains("commands") && help.contains("recommended"),
        "help should steer users to `vox commands --recommended`"
    );
}

#[test]
fn mens_long_help_canonical_train_is_mens_not_schola() {
    let mut root = VoxCliRoot::command();
    let mens = root
        .find_subcommand_mut("mens")
        .expect("vox mens subcommand should exist with default features");
    let help = mens.render_long_help().to_string();
    assert!(
        help.contains("vox mens train"),
        "mens quick-start should show registry-canonical `vox mens train`"
    );
    assert!(
        !help.contains("vox schola train"),
        "avoid stale `vox schola train` in user-facing mens help"
    );
}

#[test]
fn parse_db_explain_subcommand() {
    VoxCliRoot::try_parse_from(["vox", "db", "explain", "--file", "examples/golden/crud_api.vox"])
        .expect("db explain parse");
}

#[test]
fn parse_db_explain_jsonl_flag() {
    VoxCliRoot::try_parse_from([
        "vox",
        "db",
        "explain",
        "--file",
        "examples/golden/db_native_ir.vox",
        "--jsonl",
    ])
    .expect("db explain --jsonl parse");
}
