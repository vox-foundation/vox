#![allow(missing_docs)]

use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;

/// `activity` and `workflow` keywords are tombstoned (TASK-2.6).
/// Source using those keywords must produce a parse error, not HIR nodes.
#[test]
fn durable_workflow_recovery_keywords_are_tombstoned() {
    let source = r#"
activity send_email(recipient: str, body: str) to Result[bool] {
    return Ok(true)
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
    assert!(
        parse(tokens).is_err(),
        "tombstoned `activity` / `workflow` keywords must produce a parse error"
    );
}
