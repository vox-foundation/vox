#![allow(missing_docs)]

use vox_hir::lower_module;
use vox_lexer::cursor::lex;
use vox_parser::parser::parse;

#[test]
fn durable_workflow_recovery_logic() {
    let source = r#"
activity send_email(recipient: str, body: str) to Result[bool] {
    ret Ok(true)
}

workflow welcome_onboarding(user_id: str) to Unit {
    let email_sent = send_email("user@example.com", "Welcome!")
    match email_sent {
        Ok(s) -> print("Onboarding started for " + user_id)
        Error(e) -> print("Error")
    }
}
"#;

    let tokens = lex(source);
    let module = parse(tokens).expect("Parse failed");
    let hir = lower_module(&module);

    assert_eq!(hir.activities.len(), 1);
    assert_eq!(hir.workflows.len(), 1);

    let wf = &hir.workflows[0];
    assert_eq!(wf.name, "welcome_onboarding");
    assert_eq!(wf.params.len(), 1);
}
