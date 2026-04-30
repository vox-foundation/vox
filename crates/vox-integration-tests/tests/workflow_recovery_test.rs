#![allow(missing_docs)]

use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;

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

    use vox_compiler::hir::nodes::DurabilityKind;
    let activities: Vec<_> = hir.functions.iter().filter(|f| f.durability == Some(DurabilityKind::Activity)).collect();
    let workflows: Vec<_> = hir.functions.iter().filter(|f| f.durability == Some(DurabilityKind::Workflow)).collect();
    assert_eq!(activities.len(), 1);
    assert_eq!(workflows.len(), 1);

    let wf = workflows[0];
    assert_eq!(wf.name, "welcome_onboarding");
    assert_eq!(wf.params.len(), 1);
}
