use crate::diag::CheckReport;
use crate::ir::{Direction, Ir};
use crate::{check, codes};

fn graph(r: &CheckReport) -> &crate::ir::GraphIr {
    let Some(Ir::Graph(g)) = &r.ir else {
        panic!("expected graph ir, got {:?}", r.diagnostics)
    };
    g
}

#[test]
fn canonical_shape_dfs_order_and_labels() {
    let r = check(include_str!("../../tests/fixtures/valid/canonical.vdg"));
    let g = graph(&r);
    let ids: Vec<_> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        ["client", "backend.gateway", "backend.engine", "backend.db"]
    );
    assert_eq!(g.nodes[3].container.as_deref(), Some("backend"));
    assert_eq!(g.containers[0].depth, 1);
    assert_eq!(g.containers[0].parent, None);
    assert_eq!(g.containers[0].label, "Orchestrator Host");
    let e: Vec<_> = g
        .edges
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_str()))
        .collect();
    assert_eq!(
        e,
        [
            ("backend.gateway", "backend.engine"),
            ("backend.engine", "backend.db"),
            ("client", "backend.gateway")
        ]
    );
}

#[test]
fn innermost_scope_wins_then_outward() {
    let src = "graph {\n node x: \"X\"\n node y: \"Y\"\n container be: \"B\" {\n node x: \"BX\"\n x -> y\n }\n}\n";
    let r = check(src);
    let g = graph(&r);
    assert_eq!(
        (g.edges[0].from.as_str(), g.edges[0].to.as_str()),
        ("be.x", "y")
    );
}

#[test]
fn forward_reference_and_direction_statement() {
    let src = "graph {\n a -> b\n direction: right\n node a: \"A\"\n node b: \"B\"\n}\n";
    let r = check(src);
    assert_eq!(graph(&r).direction, Direction::Right);
    assert_eq!(graph(&r).edges.len(), 1);
}

#[test]
fn dotted_path_through_nested_containers() {
    let src = "graph {\n container o: \"O\" {\n container i: \"I\" {\n node x: \"X\"\n }\n }\n o.i.x -> o.i.x\n}\n";
    let r = check(src);
    let g = graph(&r);
    assert_eq!(g.nodes[0].id, "o.i.x");
    assert_eq!(g.containers[1].id, "o.i");
    assert_eq!(g.containers[1].depth, 2);
    assert_eq!(g.containers[1].parent.as_deref(), Some("o"));
    assert_eq!(r.stats.max_depth, 2);
}

#[test]
fn path_through_a_node_is_not_a_container() {
    let src = "graph {\n container o: \"O\" {\n node leaf: \"L\"\n }\n node a: \"A\"\n a -> o.leaf.x\n}\n";
    let r = check(src);
    assert_eq!(r.diagnostics.len(), 1);
    let d = &r.diagnostics[0];
    assert_eq!(d.code, codes::NOT_A_CONTAINER);
    assert_eq!((d.span.line, d.span.col, d.span.end_col), (6, 14, 15));
}

#[test]
fn container_as_endpoint_is_rejected_with_help() {
    let src = "graph {\n node client: \"C\"\n container backend: \"B\" {\n node gateway: \"G\"\n }\n client -> backend\n}\n";
    let r = check(src);
    assert_eq!(r.diagnostics.len(), 1);
    let d = &r.diagnostics[0];
    assert_eq!(d.code, codes::CONTAINER_ENDPOINT);
    assert!(d.fixes.is_empty());
    assert!(
        d.help
            .as_deref()
            .is_some_and(|h| h.contains("backend.gateway")),
        "{:?}",
        d.help
    );
    assert_eq!((d.span.line, d.span.col), (6, 12));
}

fn fix_edits(r: &CheckReport) -> Vec<Vec<(u32, u32, String)>> {
    let ds = r.diagnostics.iter();
    ds.map(|d| {
        let f = d.fixes.iter();
        f.map(|f| (f.span.start, f.span.end, f.replacement.clone()))
            .collect()
    })
    .collect()
}

#[test]
fn undeclared_at_root_declares_at_the_open_brace_line_once_per_id() {
    let src = "graph {\n    node a: \"A\"\n    a -> ghost\n    ghost -> a\n}\n";
    let edits = fix_edits(&check(src));
    // Both mention `ghost`: only the first owns the fix, inserted at the end of line 1.
    assert_eq!(edits.len(), 2);
    assert_eq!(
        edits[0],
        [(7, 7, "\n    node ghost: \"ghost\"".to_string())]
    );
    assert!(edits[1].is_empty());
}

#[test]
fn undeclared_in_a_container_uses_its_indent_and_scope() {
    let src = "graph {\n    node a: \"A\"\n    container be: \"B\" {\n        node x: \"X\"\n        x -> phantom\n    }\n    a -> be.ghost\n}\n";
    let at = src.find("{\n        node x").map_or(0, |i| i as u32 + 1);
    let edits = fix_edits(&check(src));
    // Unknown first segment inside `be` and unknown later segment `be.ghost` both declare
    // inside `be`, at 8 spaces, in diagnostic order.
    assert_eq!(
        edits,
        [
            [(at, at, "\n        node phantom: \"phantom\"".to_string())],
            [(at, at, "\n        node ghost: \"ghost\"".to_string())]
        ]
    );
}

#[test]
fn single_line_scope_inserts_after_the_brace_with_a_trailing_newline() {
    let src = "graph {\n    node a: \"A\"\n    container be: \"B\" { }\n    a -> be.ghost\n}\n";
    let at = src.find("{ }").map_or(0, |i| i as u32 + 1);
    let edits = fix_edits(&check(src));
    assert_eq!(
        edits,
        [[(at, at, "\n        node ghost: \"ghost\"\n".to_string())]]
    );
}

#[test]
fn no_fix_for_dotted_unknown_prefix_unclosed_scope_or_bad_ids() {
    for src in [
        "graph {\n    node a: \"A\"\n    a -> nope.x\n}\n",
        "graph {\n    node a: \"A\"\n    a -> ghost\n",
        "graph {\n    node a: \"A\"\n    a -> 9x\n}\n",
        "graph {\n    node a: \"A\"\n    a -> node\n}\n",
    ] {
        let r = check(src);
        let undeclared = r
            .diagnostics
            .iter()
            .find(|d| d.code == codes::UNDECLARED_NODE);
        assert!(
            undeclared.is_some_and(|d| d.fixes.is_empty()),
            "{src:?}: {r:?}"
        );
    }
}
