use clap::Parser;
use vox_cli::VoxCliRoot;

#[test]
fn graph_history_subcommands_parse() {
    for argv in [
        vec!["vox", "graph", "history", "brief"],
        vec![
            "vox",
            "graph",
            "history",
            "log",
            "crates/vox-cli/src/main.rs",
            "--limit",
            "5",
        ],
        vec![
            "vox",
            "graph",
            "history",
            "focus",
            "--since-days",
            "7",
            "--by",
            "dir",
            "--json",
        ],
        vec![
            "vox",
            "graph",
            "history",
            "forgotten",
            "--min-age-days",
            "90",
        ],
        vec!["vox", "graph", "history", "search", "hakari"],
        vec!["vox", "graph", "history", "timeline", "--bucket-days", "14"],
    ] {
        VoxCliRoot::try_parse_from(&argv).unwrap_or_else(|e| panic!("{argv:?}: {e}"));
    }
}
