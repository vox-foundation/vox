//! Parity guard: every Library KB tool is wired through catalog, generated registry, dispatch,
//! input schemas and the HTTP read-role governance list (Phase 2, plan 02-06).

mod common;

const TOOLS: &[&str] = &["vox_kb_search"];
/// Tools that must be reachable by the HTTP read role.
const READ_ROLE: &[&str] = &["vox_kb_search"];
/// Tools that write and must never be read-role eligible.
const WRITE_TOOLS: &[&str] = &[];

fn read_role_tools() -> Vec<String> {
    let src = include_str!("../../../contracts/mcp/http-read-role-governance.yaml");
    src.lines()
        .filter_map(|l| l.trim().strip_prefix("- ").map(str::to_string))
        .collect()
}

#[test]
fn kb_tool_surface() {
    let dispatch = include_str!("../src/dispatch.rs");
    let schemas = include_str!("../src/input_schemas.rs");
    let registry = include_str!("../../../contracts/mcp/tool-registry.canonical.yaml");
    let catalog = include_str!("../../../contracts/operations/catalog.v1.yaml");
    for t in TOOLS {
        assert!(
            dispatch.contains(&format!("\"{t}\" =>")),
            "dispatch missing {t}"
        );
        assert!(
            schemas.contains(&format!("\"{t}\" =>")),
            "input_schemas missing {t}"
        );
        assert!(
            registry.contains(&format!("name: {t}\n")),
            "registry missing {t}"
        );
        assert!(
            catalog.contains(&format!("name: {t}\n")),
            "catalog missing {t}"
        );
    }
    let read_role = read_role_tools();
    for t in READ_ROLE {
        assert!(
            read_role.iter().any(|r| r == t),
            "read-role list missing {t}"
        );
    }
    for t in WRITE_TOOLS {
        assert!(
            !read_role.iter().any(|r| r == t),
            "{t} must not be read-role eligible"
        );
    }
}
