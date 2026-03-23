//! Parse `examples/**/*.vox` recursively. A **golden** subset must parse cleanly; others are
//! informational until the grammar grows.

use std::fs;
use std::path::{Path, PathBuf};
use vox_parser::parse;

/// Example paths (relative to `examples/`, forward slashes) that must parse.
const MUST_PARSE: &[&str] = &[
    "chatbot.vox",
    "data_layer.vox",
    "durable_counter.vox",
    "full_stack_minimal.vox",
    "generics_option.vox",
    "hooks_demo.vox",
    "island_demo.vox",
    "mcp_tool_demo.vox",
    "multi_route_app.vox",
    "pattern_matching.vox",
    "server_fn.vox",
    "testing.vox",
    "v0_component.vox",
    "workflow.vox",
    "hello-vox/src/main.vox",
    "dashboard.vox",
    "chatbot_server_fn.vox",
    "simple_server_fn.vox",
    "durable_execution.vox",
    "agent.vox",
    "pytorch_inference.vox",
    "sharing.vox",
];

fn collect_vox_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_vox_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "vox") {
            out.push(p);
        }
    }
}

#[test]
fn test_parse_examples() {
    let mut examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    examples_dir.push("tests");
    examples_dir.push("golden");

    if !examples_dir.exists() {
        return;
    }

    let mut paths = Vec::new();
    collect_vox_files(&examples_dir, &mut paths);
    paths.sort();

    let mut golden_failed = 0usize;
    let mut optional_failed = 0usize;

    for path in paths {
        let rel = path.strip_prefix(&examples_dir).unwrap();
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let is_golden = MUST_PARSE.contains(&rel_str.as_str());

        let content = fs::read_to_string(&path).unwrap();
        let tokens = vox_lexer::lex(&content);
        let result = parse(tokens);
        match &result {
            Ok(_) => println!("Parsed {}", path.display()),
            Err(errs) => {
                println!("Failed to parse {}:", path.display());
                for e in errs {
                    println!("  {:?}", e);
                }
                if is_golden {
                    golden_failed += 1;
                } else {
                    optional_failed += 1;
                }
            }
        }
    }

    println!(
        "Golden parse failures: {} (required 0), other examples not yet in grammar: {}",
        golden_failed, optional_failed
    );
    assert_eq!(
        golden_failed, 0,
        "Parser regression: a golden example file failed to parse"
    );

    let strict = std::env::var("VOX_EXAMPLES_STRICT_PARSE")
        .ok()
        .as_deref()
        == Some("1");
    if strict {
        assert_eq!(
            optional_failed, 0,
            "VOX_EXAMPLES_STRICT_PARSE=1: every examples/**/*.vox must parse; see Failed to parse lines above. \
             Clear the env var for default CI (golden-only gate)."
        );
    }
}

