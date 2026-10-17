//! stateDiagram-v2 import: terminals, transitions and the fail-closed catalogue. State blocks
//! fold into the `graph` DSL (`source`); `sequence` stays `None`.

use vox_diagram::mermaid::{ImportOpts, import};
use vox_diagram::{check, fmt};

#[test]
fn state_terminals_and_self_loop() {
    let block = "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Leased: lease granted\n    Leased --> Leased: renew\n    Leased --> Idle : lease expiry (swept)\n    Leased --> [*]\n";
    let want = "graph {\n    node start: \"Start\" { shape: start }\n    node Idle: \"Idle\"\n    node Leased: \"Leased\"\n    node end: \"End\" { shape: end }\n    start -> Idle\n    Idle -> Leased: \"lease granted\"\n    Leased -> Leased: \"renew\"\n    Leased -> Idle: \"lease expiry (swept)\"\n    Leased -> end\n}\n";
    let r = import(block, ImportOpts::default());
    assert!(r.unsupported.is_empty() && r.dropped.is_empty(), "{r:?}");
    assert_eq!(r.sequence, None);
    let Some(src) = r.source else {
        panic!("not imported");
    };
    assert_eq!(src, want);
    assert_eq!(fmt(&src).as_deref(), Some(src.as_str()));
    assert!(check(&src).valid);
}

#[test]
fn state_start_end_ids_clash() {
    for id in ["start", "end"] {
        let block = format!("stateDiagram-v2\n    A --> B\n    {id} --> A\n");
        let r = import(&block, ImportOpts::default());
        assert_eq!(r.source, None, "{id}");
        let got: Vec<(u32, &str)> = r
            .unsupported
            .iter()
            .map(|u| (u.line, u.construct.as_str()))
            .collect();
        assert_eq!(got, vec![(3, "id-clash")], "{id}");
    }
}

fn refused(block: &str) -> Vec<(u32, String)> {
    let r = import(block, ImportOpts::default());
    assert_eq!((&r.source, &r.sequence), (&None, &None), "{block}");
    r.unsupported
        .into_iter()
        .map(|u| (u.line, u.construct))
        .collect()
}

/// Every body line (from line 2) is refused with the matching construct, one per line.
fn each<L: AsRef<str>, C: AsRef<str>>(lines: &[L], constructs: &[C]) {
    assert_eq!(lines.len(), constructs.len());
    let body: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
    let want: Vec<(u32, String)> = (2u32..)
        .zip(constructs.iter().map(|c| c.as_ref().to_owned()))
        .collect();
    assert_eq!(
        refused(&format!("stateDiagram-v2\n{}\n", body.join("\n"))),
        want
    );
}

#[test]
fn state_unsupported_catalogue() {
    each(
        &[
            "state \"Long name\" as L",
            "A : description",
            "state Composite {",
            "}",
            "state F <<fork>>",
            "state J <<join>>",
            "state C <<choice>>",
            "note right of A : n",
            "end note",
            "--",
            "direction LR",
            "classDef hot fill:#f00",
            "class A hot",
            "A --> B:::hot",
            "%% comment",
            "%%{init: {}}%%",
            "A --> B;",
            "state Lone",
            "A --- B",
            "A --> B --> C",
            "1a --> B",
            "node --> B",
            "A --> B: x; y",
            "A --> B: <b>x",
        ],
        &[
            "state:alias",
            "state:description",
            "state:composite",
            "state:composite",
            "state:pseudo",
            "state:pseudo",
            "state:pseudo",
            "state:note",
            "state:note",
            "state:concurrency",
            "direction",
            "styling:classDef",
            "styling:class",
            "styling:shorthand",
            "comment",
            "directive",
            "semicolon",
            "statement",
            "statement",
            "invalid-id",
            "invalid-id",
            "reserved-id",
            "semicolon",
            "html-label",
        ],
    );
}

#[test]
fn state_header_forms_and_text() {
    assert_eq!(
        refused("stateDiagram\n    A --> B\n"),
        vec![(1, "header:stateDiagram".to_owned())]
    );
    assert_eq!(
        refused("stateDiagram-v2;\n"),
        vec![(1, "semicolon".to_owned())]
    );
    // Empty transition text is no label; `A: x` text keeps its characters.
    let r = import(
        "stateDiagram-v2\n    A --> B:\n    B --> A:a: b (c) + d\n",
        ImportOpts::default(),
    );
    let src = r.source.unwrap_or_default();
    assert!(src.contains("A -> B\n"), "{src}");
    assert!(src.contains("B -> A: \"a: b (c) + d\""), "{src}");
}

#[test]
fn state_both_terminals_and_caps() {
    let r = import("stateDiagram-v2\n    [*] --> [*]\n", ImportOpts::default());
    let src = r.source.unwrap_or_default();
    assert!(src.contains("start -> end"), "{src}");
    let many: String = (0..=500).map(|i| format!("    S{i} --> S{i}\n")).collect();
    assert_eq!(
        refused(&format!("stateDiagram-v2\n{many}")),
        vec![(502, "limit-nodes".to_owned())]
    );
    let loops = "    A --> B\n".repeat(1_001);
    assert_eq!(
        refused(&format!("stateDiagram-v2\n{loops}")),
        vec![(1_002, "limit-edges".to_owned())]
    );
}

#[test]
fn drop_styling_in_state() {
    let opts = ImportOpts { drop_styling: true };
    let r = import(
        "stateDiagram-v2\n    classDef hot fill:#f00\n    A --> B\n    class A hot\n",
        opts,
    );
    assert!(r.unsupported.is_empty(), "{r:?}");
    assert!(r.source.is_some());
    let d: Vec<(u32, &str)> = r
        .dropped
        .iter()
        .map(|u| (u.line, u.construct.as_str()))
        .collect();
    assert_eq!(d, vec![(2, "styling:classDef"), (4, "styling:class")]);
}
