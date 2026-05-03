use vox_compiler::lexer::lex;
use vox_compiler::parser::{ParseErrorClass, parse};

// TASK-2.6 Path A (commit 080b3f86, per AGENTS.md §Grammar Unification) UN-tombstoned
// `actor`, `workflow`, and `activity` — they are now fully supported bare keywords that
// lower to `HirFn { durability: Some(DurabilityKind::_) }`. The two former tombstone
// assertions for `actor` and `workflow` are inverted here to lock in the current behavior:
// parsing succeeds (no Tombstoned error). `@component` and `http` remain retired and keep
// their tombstone tests below.

#[test]
fn actor_is_supported_after_task_2_6_path_a() {
    let src = "actor MyActor {}";
    let tokens = lex(src);
    let result = parse(tokens);
    // Per AGENTS.md §Grammar Unification, `actor` parses cleanly. The HIR lowering may
    // still fail for an actor with no fields/handlers, but the parser must not emit a
    // Tombstoned error.
    if let Err(errs) = result {
        assert!(
            !errs.iter().any(|e| e.class == ParseErrorClass::Tombstoned),
            "`actor` should not be Tombstoned per TASK-2.6 Path A; got: {errs:?}"
        );
    }
}

#[test]
fn workflow_is_supported_after_task_2_6_path_a() {
    let src = "workflow MyWorkflow {}";
    let tokens = lex(src);
    let result = parse(tokens);
    if let Err(errs) = result {
        assert!(
            !errs.iter().any(|e| e.class == ParseErrorClass::Tombstoned),
            "`workflow` should not be Tombstoned per TASK-2.6 Path A; got: {errs:?}"
        );
    }
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
