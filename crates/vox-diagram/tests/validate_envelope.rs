//! Golden JSON shape of the validation envelope shared by the MCP tool and `vox diagram --json`.

use serde_json::json;
use vox_diagram::{Emit, ValidateOpts, validate};

const BROKEN: &str = "graph {\n    node a: \"A\"\n    a -> b\n    node c: C\n}\n";
const SEQ: &str = "sequence {\n    actor web: \"Web\"\n    actor api: \"API\"\n    actor db: \"Database\"\n    web -> api: \"POST /orders\"\n    reply db -> api: \"id\"\n    note over api: \"idempotent\"\n}\n";
const FLOW: &str = "flowchart TD\n    a[Start] --> b{Ok?}\n    b --> c\n";
const CLASSDEF: &str = "flowchart TD\n    a --> b\n    classDef x fill:#f00\n";

fn mermaid(emit: Option<Emit>) -> ValidateOpts {
    ValidateOpts {
        from_mermaid: true,
        emit,
    }
}

#[test]
fn broken_graph() {
    insta::assert_json_snapshot!(validate(BROKEN, ValidateOpts::default()));
}

#[test]
fn sequence_emit_mermaid() {
    let o = ValidateOpts {
        from_mermaid: false,
        emit: Some(Emit::Mermaid),
    };
    insta::assert_json_snapshot!(validate(SEQ, o));
}

#[test]
fn from_mermaid_ok() {
    insta::assert_json_snapshot!(validate(FLOW, mermaid(None)));
}

#[test]
fn from_mermaid_unsupported() {
    insta::assert_json_snapshot!(validate(CLASSDEF, mermaid(None)));
}

#[test]
fn opts_deserialize_from_tool_arguments() {
    let o: ValidateOpts =
        serde_json::from_value(json!({"from_mermaid": true, "emit": "svg"})).unwrap();
    assert_eq!(o, mermaid(Some(Emit::Svg)));
    assert!(serde_json::from_value::<ValidateOpts>(json!({"emit": "png"})).is_err());
}
