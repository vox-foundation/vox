#![allow(missing_docs)]

use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;

/// `activity` and `workflow` keywords are tombstoned (TASK-2.6).
/// Parsing source that uses them produces a parse error rather than HIR nodes.
#[test]
fn tombstoned_activity_and_workflow_keywords_produce_parse_errors() {
    let source_activity = r#"
activity generate_banner(prompt: str) to Result[str] {
    return Ok("ok")
}
"#;
    let source_workflow = r#"
workflow handle_branding(description: str) to Unit {
    return ()
}
"#;

    let result_activity = parse(lex(source_activity));
    assert!(
        result_activity.is_err(),
        "tombstoned `activity` keyword should produce a parse error"
    );

    let result_workflow = parse(lex(source_workflow));
    assert!(
        result_workflow.is_err(),
        "tombstoned `workflow` keyword should produce a parse error"
    );
}
