use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

use super::docs_sync::{ref_cli_vox_ci_section, ref_cli_vox_codex_section};
use super::mcp_wiring::{check_mcp_tool_wiring, extract_mcp_handler_tools};
use super::registry::{extract_mcp_registry_tool_names, parse_mcp_registry_yaml};
use super::validators::kebab_to_pascal;

#[test]
fn kebab_pascal() {
    assert_eq!(kebab_to_pascal("stub-check"), "StubCheck");
    assert_eq!(kebab_to_pascal("fmt.check"), "FmtCheck");
}

#[test]
fn ref_cli_vox_ci_finds_manifest_backtick() {
    let md = "\n### `vox ci …`\n\n| `manifest` | x |\n\n### `vox dev`\n";
    let sec = ref_cli_vox_ci_section(md).expect("section");
    assert!(sec.contains("`manifest"));
}

#[test]
fn mcp_registry_yaml_tolerates_bracket_in_description() {
    let yaml = r#"
version: 1
tools:
  - name: "vox_bracket_test"
    description: "Description with ] bracket inside string"
"#;
    let tools = parse_mcp_registry_yaml(yaml).expect("parse");
    assert_eq!(tools, vec!["vox_bracket_test".to_string()]);
}

#[test]
fn mcp_handler_extract_includes_alternation_arms() {
    let src = r#"
pub async fn handle_tool_call() {
    match name {
        "vox_config_get" | "vox_get_config" => { ok }
        "vox_other" => { ok }
        _ => { err }
    }
}
"#;
    let h = extract_mcp_handler_tools(src).expect("parse");
    assert!(h.contains("vox_config_get"));
    assert!(h.contains("vox_get_config"));
    assert!(h.contains("vox_other"));
}

#[test]
fn mcp_handler_default_arm_tolerates_indent() {
    let src = r#"pub async fn handle_tool_call() {
    match name {
			"vox_indented_only" => { ok }
			_ => { err }
    }
}"#;
    let h = extract_mcp_handler_tools(src).expect("parse");
    assert!(h.contains("vox_indented_only"));
}

#[test]
fn ref_cli_vox_codex_section_excludes_other_headings() {
    let md = "\n### `vox codex`\n\n| `import` | x |\n\n### `vox dev`\nverify unrelated\n";
    let sec = ref_cli_vox_codex_section(md).expect("codex section");
    assert!(sec.contains("`import"));
    assert!(!sec.contains("verify"));
}

#[test]
fn ref_cli_vox_ci_section_until_eof_when_last_heading() {
    let md = "### `vox ci …`\n\n| `manifest` | x |\n";
    let sec = ref_cli_vox_ci_section(md).expect("ci section");
    assert!(sec.contains("`manifest"));
}

/// Guard against drift in `vox-mcp` layout: full wiring must stay parseable by the compliance gate.
#[test]
fn mcp_extract_matches_workspace_vox_mcp_mod_rs() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("vox-cli lives at crates/vox-cli");
    let base = repo_root.join("crates/vox-mcp/src/tools");
    let mcp_mod = read_utf8_path_capped(&base.join("mod.rs")).expect("read vox-mcp tools/mod.rs");
    let dispatch =
        read_utf8_path_capped(&base.join("dispatch.rs")).expect("read vox-mcp tools/dispatch.rs");
    let aliases = read_utf8_path_capped(&base.join("tool_aliases.rs"))
        .expect("read vox-mcp tools/tool_aliases.rs");
    let reg = extract_mcp_registry_tool_names(repo_root).expect("registry tools");
    let han = extract_mcp_handler_tools(&dispatch).expect("handler tools");
    let missing: Vec<&String> = reg.iter().filter(|t| !han.contains(*t)).collect();
    assert!(
        missing.is_empty(),
        "registry tools missing from handle_tool_call parse: {:?}",
        missing
    );
    check_mcp_tool_wiring(repo_root, &mcp_mod, &dispatch, &aliases).expect("mcp wiring + aliases");
}
