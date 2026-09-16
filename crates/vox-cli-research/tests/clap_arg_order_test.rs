use clap::Parser;
use vox_cli_research::ResearchCmd;

#[derive(Parser)]
struct TestCli {
    #[command(subcommand)]
    cmd: ResearchCmd,
}

#[test]
fn test_run_subcommand_parses_flags_after_query() {
    let args = vec![
        "test",
        "run",
        "what",
        "is",
        "accessibility",
        "--scope",
        "web",
        "--verify-claims",
    ];
    let parsed = TestCli::try_parse_from(args).expect("should parse flags after query");
    match parsed.cmd {
        ResearchCmd::Run {
            query,
            scope,
            verify_claims,
            ..
        } => {
            assert_eq!(query, vec!["what", "is", "accessibility"]);
            assert_eq!(scope.as_deref(), Some("web"));
            assert!(verify_claims);
        }
        _ => panic!("expected ResearchCmd::Run"),
    }
}

#[test]
fn test_preview_subcommand_parses_flags_after_query() {
    let args = vec!["test", "preview", "what", "is", "accessibility", "--json"];
    let parsed = TestCli::try_parse_from(args).expect("should parse flags after query in preview");
    match parsed.cmd {
        ResearchCmd::Preview { query, json } => {
            assert_eq!(query, vec!["what", "is", "accessibility"]);
            assert!(json);
        }
        _ => panic!("expected ResearchCmd::Preview"),
    }
}

#[test]
fn test_search_subcommand_parses_flags_after_query() {
    let args = vec![
        "test",
        "search",
        "what",
        "is",
        "accessibility",
        "--limit",
        "5",
        "--json",
    ];
    let parsed = TestCli::try_parse_from(args).expect("should parse flags after query in search");
    match parsed.cmd {
        ResearchCmd::Search { query, limit, json } => {
            assert_eq!(query, vec!["what", "is", "accessibility"]);
            assert_eq!(limit, 5);
            assert!(json);
        }
        _ => panic!("expected ResearchCmd::Search"),
    }
}
