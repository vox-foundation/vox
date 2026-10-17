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
