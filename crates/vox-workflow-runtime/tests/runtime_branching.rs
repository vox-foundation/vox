#![allow(missing_docs)]

use serde_json::json;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_workflow_runtime::workflow::{DefaultTracker, interpret_workflow_durable};

fn hir(src: &str) -> vox_compiler::hir::HirModule {
    lower_module(&parse(lex(src)).expect("parses"))
}

#[tokio::test]
async fn runtime_if_on_activity_result() {
    const SRC: &str = r#"
activity charge(amount: int) to Result[str] {
    if amount > 0 { return Ok("tx") }
    return Error("bad")
}
workflow checkout(amount: int) to str {
    let result = charge(amount)
    let succeeded = match result {
        Ok(_) => true
        Error(_) => false
    }
    if succeeded {
        return "paid"
    } else {
        return "declined"
    }
}
"#;
    let mut tracker = DefaultTracker;
    let paid = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(10)], &mut tracker)
        .await
        .expect("ok branch");
    let paid_ret = paid
        .iter()
        .rev()
        .find(|e| e["event"] == "WorkflowCompleted")
        .unwrap();
    assert_eq!(paid_ret["return_value"], json!("paid"));

    let mut tracker = DefaultTracker;
    let declined = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(0)], &mut tracker)
        .await
        .expect("err branch");
    let declined_ret = declined
        .iter()
        .rev()
        .find(|e| e["event"] == "WorkflowCompleted")
        .unwrap();
    assert_eq!(declined_ret["return_value"], json!("declined"));
}
