//! Importer leniency catalogue: each documented leniency has one named test that asserts the
//! exact canonical DSL. Constructs the importer refuses live in `mermaid_unsupported.rs`.

use vox_diagram::ir::Ir;
use vox_diagram::mermaid::{ImportOpts, ImportReport, Unsupported, import};
use vox_diagram::{check, fmt};

fn run(block: &str) -> ImportReport {
    import(block, ImportOpts::default())
}

/// The imported DSL; panics (with the unsupported list) when the block did not import.
fn dsl(block: &str) -> String {
    let r = run(block);
    assert!(r.unsupported.is_empty() && r.dropped.is_empty(), "{r:?}");
    let Some(src) = r.source else {
        panic!("not imported: {:?}", r.unsupported);
    };
    assert_eq!(
        fmt(&src).as_deref(),
        Some(src.as_str()),
        "not a fixed point"
    );
    assert!(check(&src).valid, "imported source does not check:\n{src}");
    src
}

fn graph(src: &str) -> vox_diagram::ir::GraphIr {
    let Some(Ir::Graph(g)) = check(src).ir else {
        panic!("no graph IR for:\n{src}");
    };
    g
}

#[test]
fn import_cross_scope_edge_uses_dotted_path() {
    let block = "flowchart LR\n    client[Desktop Client]\n    subgraph backend [Orchestrator]\n        gateway[Gateway] --> engine[Agent Runtime]\n    end\n    client -->|TLS| gateway\n";
    let want = "graph {\n    direction: right\n    node client: \"Desktop Client\"\n    container backend: \"Orchestrator\" {\n        node gateway: \"Gateway\"\n        node engine: \"Agent Runtime\"\n        gateway -> engine\n    }\n    client -> backend.gateway: \"TLS\"\n}\n";
    let src = dsl(block);
    assert_eq!(src, want);
    let g = graph(&src);
    let edge = g
        .edges
        .iter()
        .find(|e| e.from == "client")
        .map(|e| (e.from.as_str(), e.to.as_str(), e.label.as_deref()));
    assert_eq!(edge, Some(("client", "backend.gateway", Some("TLS"))));
}

#[test]
fn import_node_first_used_outside_then_inside() {
    let block =
        "flowchart TD\n    a --> w1\n    subgraph eng [Engine]\n        w1 --> w2\n    end\n";
    let want = "graph {\n    node a: \"a\"\n    container eng: \"Engine\" {\n        node w1: \"w1\"\n        node w2: \"w2\"\n        w1 -> w2\n    }\n    a -> eng.w1\n}\n";
    assert_eq!(dsl(block), want);
}

#[test]
fn import_reports_have_no_stray_entries() {
    let r = run("flowchart TD\n    a --> b\n");
    assert_eq!(r.unsupported, Vec::<Unsupported>::new());
    assert_eq!(r.dropped, Vec::<Unsupported>::new());
}
