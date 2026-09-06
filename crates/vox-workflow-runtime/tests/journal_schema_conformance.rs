#![allow(missing_docs)]
//! Every event the runner ACTUALLY emits must validate against the v1 schema.

use jsonschema::validator_for;
use serde_json::Value;
use std::sync::Arc;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::VoxDbTracker;
use vox_workflow_runtime::workflow::interpret_workflow_durable;

const SRC: &str = r#"
activity charge_card(amount: int) to Result[str] {
    return Ok("tx")
}

workflow checkout(amount: int) to Result[str] {
    workflow.version("add-audit-v2", 1, 2)
    let tx = charge_card(amount)?
    return Ok(tx)
}
"#;

#[tokio::test]
async fn every_emitted_event_validates_against_v1_schema() {
    let module = parse(lex(SRC)).expect("parses");
    let hir = lower_module(&module);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    let mut tracker = VoxDbTracker::new(db, "schema-conformance-1");

    let journal =
        interpret_workflow_durable(&hir, "checkout", vec![serde_json::json!(500)], &mut tracker)
            .await
            .expect("workflow runs");

    let schema: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/workflow/workflow-journal.v1.schema.json"
    )))
    .expect("schema parses");
    let validator = validator_for(&schema).expect("schema compiles");

    for entry in &journal {
        if let Err(err) = validator.validate(entry) {
            panic!("runner emitted an event the v1 schema rejects: {err}\nentry={entry}");
        }
    }
    assert!(
        journal
            .iter()
            .any(|e| e["event"].as_str() == Some("WorkflowPatch")),
        "test must actually exercise the WorkflowPatch path; got {journal:#?}"
    );
}
