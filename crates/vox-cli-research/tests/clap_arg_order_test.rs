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

#[test]
fn test_run_subcommand_parses_lane_waves_domain_mode() {
    let parsed = TestCli::try_parse_from([
        "test",
        "run",
        "tucson",
        "events",
        "--lane",
        "deep",
        "--waves",
        "3",
        "--domain-mode",
        "shopping",
    ])
    .expect("lane/waves/domain-mode flags parse");
    match parsed.cmd {
        ResearchCmd::Run {
            lane,
            waves,
            domain_mode,
            ..
        } => {
            assert_eq!(lane.as_deref(), Some("deep"));
            assert_eq!(waves, Some(3));
            assert_eq!(domain_mode.as_deref(), Some("shopping"));
        }
        _ => panic!("expected ResearchCmd::Run"),
    }
}

#[test]
fn test_flag_subcommand_parses() {
    let parsed = TestCli::try_parse_from([
        "test",
        "flag",
        "42",
        "--url",
        "https://example.com/a",
        "--defect",
        "hallucinated_api",
    ])
    .expect("flag parses");
    match parsed.cmd {
        ResearchCmd::Flag {
            session_id,
            url,
            defect,
            ..
        } => {
            assert_eq!(session_id, 42);
            assert_eq!(url, "https://example.com/a");
            assert_eq!(defect, "hallucinated_api");
        }
        _ => panic!("expected ResearchCmd::Flag"),
    }
}

#[test]
fn test_publish_subcommand_parses() {
    let parsed = TestCli::try_parse_from(["test", "publish", "7", "--slug", "tucson-events"])
        .expect("publish parses");
    match parsed.cmd {
        ResearchCmd::Publish { session_id, slug } => {
            assert_eq!(session_id, 7);
            assert_eq!(slug.as_deref(), Some("tucson-events"));
        }
        _ => panic!("expected ResearchCmd::Publish"),
    }
}

#[test]
fn test_probe_subcommand_parses() {
    let parsed = TestCli::try_parse_from([
        "test",
        "probe",
        "tucson",
        "tech",
        "events",
        "--provider",
        "wikipedia",
        "--json",
    ])
    .expect("probe parses");
    match parsed.cmd {
        ResearchCmd::Probe {
            query,
            provider,
            json,
        } => {
            assert_eq!(query, vec!["tucson", "tech", "events"]);
            assert_eq!(provider.as_deref(), Some("wikipedia"));
            assert!(json);
        }
        _ => panic!("expected ResearchCmd::Probe"),
    }
}
