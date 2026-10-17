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

/// DSL with the given statements as the whole body (4-space indent already applied).
fn body(stmts: &str) -> String {
    format!("graph {{\n{stmts}}}\n")
}

#[test]
fn import_unquoted_labels() {
    let block =
        "flowchart TD\n    A[API: gw / x.y]\n    B(Some text, more)\n    C{  Hit or miss?  }\n";
    let want = body(
        "    node A: \"API: gw / x.y\"\n    node B: \"Some text, more\" { shape: rounded }\n    node C: \"Hit or miss?\" { shape: diamond }\n",
    );
    assert_eq!(dsl(block), want);
}

#[test]
fn import_quoted_labels_decode_entities() {
    let block = "flowchart TD\n    A[\"say #quot;hi#quot; #35;1 #amp; #lt;b#gt; #9829;\"]\n    B[\"#35;quot;\"]\n    C[\"C# code\"]\n    D[\"lone # here\"]\n";
    let want = body(
        "    node A: \"say \\\"hi\\\" #1 & <b> \u{2665}\"\n    node B: \"#quot;\"\n    node C: \"C# code\"\n    node D: \"lone # here\"\n",
    );
    assert_eq!(dsl(block), want);
}

#[test]
fn import_br_variants_become_newlines() {
    let block = "flowchart TD\n    A[\"a<br>b<br/>c<br />d<BR>e\"]\n    B[x <br/> y]\n";
    let want = body("    node A: \"a\\nb\\nc\\nd\\ne\"\n    node B: \"x \\n y\"\n");
    assert_eq!(dsl(block), want);
}

#[test]
fn import_all_six_shapes() {
    let block =
        "flowchart TD\n    A[r]\n    B(o)\n    C{d}\n    D[(c)]\n    E((s))\n    F(((e)))\n";
    let want = body(
        "    node A: \"r\"\n    node B: \"o\" { shape: rounded }\n    node C: \"d\" { shape: diamond }\n    node D: \"c\" { shape: cylinder }\n    node E: \"s\" { shape: start }\n    node F: \"e\" { shape: end }\n",
    );
    assert_eq!(dsl(block), want);
}

#[test]
fn import_fan_out_and_chains() {
    let nodes = |ids: &str| -> String {
        ids.chars()
            .map(|c| format!("    node {c}: \"{c}\"\n"))
            .collect()
    };
    let fan = dsl("flowchart TD\n    A & B --> C & D\n");
    assert_eq!(
        fan,
        body(&format!(
            "{}    A -> C\n    A -> D\n    B -> C\n    B -> D\n",
            nodes("ABCD")
        ))
    );
    let chain = dsl("flowchart TD\n    A --> B --> C\n");
    assert_eq!(
        chain,
        body(&format!("{}    A -> B\n    B -> C\n", nodes("ABC")))
    );
    let labels = dsl("flowchart TD\n    A -->|x| B -- y --> C\n");
    assert_eq!(
        labels,
        body(&format!(
            "{}    A -> B: \"x\"\n    B -> C: \"y\"\n",
            nodes("ABC")
        ))
    );
}

#[test]
fn import_edge_text_forms() {
    let block = "flowchart TD\n    A -->|t1| B\n    B -->|\"t2\"| C\n    C -- t3 --> D\n    D -- \"t4\" --> E\n    E-->F\n    F-->|t5|G\n    G -->|\"a|b\"| H\n    H -->|\" \"| I\n";
    let nodes: String = "ABCDEFGHI"
        .chars()
        .map(|c| format!("    node {c}: \"{c}\"\n"))
        .collect();
    let edges = "    A -> B: \"t1\"\n    B -> C: \"t2\"\n    C -> D: \"t3\"\n    D -> E: \"t4\"\n    E -> F\n    F -> G: \"t5\"\n    G -> H: \"a|b\"\n    H -> I\n";
    assert_eq!(dsl(block), body(&format!("{nodes}{edges}")));
    let paren = dsl("flowchart TD\n    A -- Yes (n < 3) --> B\n");
    assert!(paren.contains("A -> B: \"Yes (n < 3)\""), "{paren}");
}

#[test]
fn import_text_inside_quotes_is_not_a_construct() {
    let block = "flowchart TD\n    A[\"a; b\"]\n    B[\"%% not a comment\"]\n    C[\"x:::y\"]\n    D[\"a --> b\"]\n    E[\"end\"]\n    F[\"subgraph x\"]\n    A -->|\"x; y\"| B\n";
    let src = dsl(block);
    for want in [
        "node A: \"a; b\"",
        "node B: \"%% not a comment\"",
        "node C: \"x:::y\"",
        "node D: \"a --> b\"",
        "node E: \"end\"",
        "node F: \"subgraph x\"",
        "A -> B: \"x; y\"",
    ] {
        assert!(src.contains(want), "{want} missing from\n{src}");
    }
}

#[test]
fn import_implicit_and_repeated_nodes() {
    let block =
        "flowchart TD\n    A --> B\n    A --> B\n    B --> A\n    A[Alpha]\n    A[Alpha]\n    B\n";
    let want =
        body("    node A: \"Alpha\"\n    node B: \"B\"\n    A -> B\n    A -> B\n    B -> A\n");
    assert_eq!(dsl(block), want);
}

#[test]
fn import_header_directions() {
    for (header, line) in [
        ("flowchart TD", ""),
        ("flowchart TB", ""),
        ("graph", ""),
        ("graph LR", "    direction: right\n"),
        ("flowchart RL", "    direction: left\n"),
        ("graph BT", "    direction: up\n"),
    ] {
        let want = body(&format!("{line}    node a: \"a\"\n"));
        assert_eq!(dsl(&format!("{header}\n    a\n")), want, "{header}");
    }
}

#[test]
fn import_subgraph_forms() {
    let block = "flowchart TD\n    subgraph s1\n        a1\n    end\n    subgraph s2[Two]\n        a2\n    end\n    subgraph s3 [Three]\n        a3\n    end\n    subgraph s4[\"Four; 4\"]\n        a4\n    end\n    subgraph s5 [\"Five\"]\n        a5\n    end\n";
    let src = dsl(block);
    for (id, title) in [
        ("s1", "s1"),
        ("s2", "Two"),
        ("s3", "Three"),
        ("s4", "Four; 4"),
        ("s5", "Five"),
    ] {
        assert!(
            src.contains(&format!("container {id}: \"{title}\" {{")),
            "{id}\n{src}"
        );
    }
    let nested = "flowchart TD\n    subgraph outer [Outer]\n        o1\n        subgraph inner [Inner]\n            i1\n        end\n        o1 --> i1\n    end\n    i1 --> o1\n    x --> i1\n";
    let want = body(
        "    node x: \"x\"\n    container outer: \"Outer\" {\n        node o1: \"o1\"\n        container inner: \"Inner\" {\n            node i1: \"i1\"\n        }\n        o1 -> inner.i1\n        inner.i1 -> o1\n    }\n    x -> outer.inner.i1\n",
    );
    assert_eq!(dsl(nested), want);
}

#[test]
fn import_membership_first_closed_wins() {
    let block = "flowchart TD\n    late\n    subgraph s1\n        shared\n    end\n    subgraph s2\n        shared\n        late\n    end\n    subgraph p\n        pn\n        subgraph c\n            pn\n        end\n    end\n    r\n";
    let want = body(
        "    node r: \"r\"\n    container s1: \"s1\" {\n        node shared: \"shared\"\n    }\n    container s2: \"s2\" {\n        node late: \"late\"\n    }\n    container p: \"p\" {\n        container c: \"c\" {\n            node pn: \"pn\"\n        }\n    }\n",
    );
    assert_eq!(dsl(block), want);
}

#[test]
fn import_subgraph_direction_equal_is_dropped() {
    for (head, dir) in [
        ("flowchart TD", "TB"),
        ("flowchart LR", "LR"),
        ("graph", "TD"),
    ] {
        let r = run(&format!(
            "{head}\n    subgraph s\n        direction {dir}\n        a\n    end\n"
        ));
        assert!(r.source.is_some(), "{head}: {:?}", r.unsupported);
        assert!(r.unsupported.is_empty());
        assert_eq!(
            r.dropped,
            vec![Unsupported {
                line: 3,
                construct: "subgraph-direction".to_owned()
            }]
        );
    }
}

#[test]
fn import_styling_head_words_can_still_be_ids() {
    // Mermaid's flowchart keywords are case-sensitive; bare `class`/`style` ids are refused
    // (`84-REVIEW` WR-04, mermaid_import_review.rs).
    for line in ["Class --> b", "Style --> b", "click --> b", "click"] {
        let r = run(&format!("flowchart TD\n    {line}\n"));
        assert!(r.source.is_some(), "{line}: {:?}", r.unsupported);
    }
}
