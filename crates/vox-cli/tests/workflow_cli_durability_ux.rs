use clap::{CommandFactory, Parser};
use vox_cli::VoxCliRoot;

#[test]
fn workflow_run_help_mentions_run_id_resume() {
    let mut root = VoxCliRoot::command();
    let mens = root
        .find_subcommand_mut("mens")
        .expect("vox mens should exist when mens-dei is enabled");
    let workflow = mens
        .find_subcommand_mut("workflow")
        .expect("vox mens workflow should exist");
    let run = workflow
        .find_subcommand_mut("run")
        .expect("vox mens workflow run should exist");
    let help = run.render_long_help().to_string();
    assert!(
        help.contains("--run-id"),
        "workflow run help should include --run-id option"
    );
    assert!(
        help.contains("Resume a specific interpreted workflow run"),
        "workflow run help should describe durable resume semantics"
    );
}

#[test]
fn workflow_run_parse_accepts_run_id_argument() {
    VoxCliRoot::try_parse_from([
        "vox",
        "mens",
        "workflow",
        "run",
        "examples/mens/workflow_mesh_demo.vox",
        "wf_mesh_demo",
        "--run-id",
        "resume-run-123",
    ])
    .expect("workflow run should parse with --run-id");
}
