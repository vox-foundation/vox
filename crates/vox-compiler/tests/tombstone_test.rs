use vox_compiler::lexer::lex;
use vox_compiler::parser::{ParseErrorClass, parse};

#[test]
fn actor_is_tombstoned() {
    let src = "actor MyActor {}";
    let tokens = lex(src);
    let errs = parse(tokens).expect_err("expected parse failure for actor");
    assert!(errs.iter().any(|e| e.class == ParseErrorClass::Tombstoned));
    assert!(errs[0].message.contains("actor"));
    assert!(errs[0].message.contains("tombstoned"));
}

#[test]
fn workflow_is_tombstoned() {
    let src = "workflow MyWorkflow {}";
    let tokens = lex(src);
    let errs = parse(tokens).expect_err("expected parse failure for workflow");
    assert!(errs.iter().any(|e| e.class == ParseErrorClass::Tombstoned));
}

#[test]
fn at_component_is_tombstoned() {
    let src = "@component fn Legacy() {}";
    let tokens = lex(src);
    let errs = parse(tokens).expect_err("expected parse failure for @component");
    assert!(errs.iter().any(|e| e.class == ParseErrorClass::Tombstoned));
}

#[test]
fn http_is_tombstoned() {
    let src = "http get \"/\"";
    let tokens = lex(src);
    let errs = parse(tokens).expect_err("expected parse failure for http");
    assert!(errs.iter().any(|e| e.class == ParseErrorClass::Tombstoned));
}
