use vox_hir::lower_module;
use vox_lexer::lex;
use vox_parser::parser::parse;
use vox_typeck::{builtins::BuiltinTypes, env::TypeEnv, typecheck_hir};

fn check_hir(source: &str) -> Vec<vox_typeck::Diagnostic> {
    let tokens = lex(source);
    let module = parse(tokens).expect("Should parse");
    let hir = lower_module(&module);
    let mut env = TypeEnv::new();
    let builtins = BuiltinTypes::register_all(&mut env);
    typecheck_hir(&hir, &mut env, &builtins, source)
}

#[test]
fn test_hir_checker_basic_func() {
    let diags = check_hir("fn foo() to int { ret 5 }");
    assert!(diags.is_empty(), "Should have no diags: {diags:?}");
}

#[test]
fn test_hir_checker_return_mismatch() {
    let diags = check_hir("fn foo() to int { ret \"hello\" }");
    assert!(
        !diags.is_empty(),
        "Expected error diagnostic for str vs int, got none"
    );
}

#[test]
fn test_hir_checker_actor_handler() {
    let src = "actor MyActor { on Update(x: int) to Unit { let y: int = x\n let _ = y\n } }";
    let diags = check_hir(src);
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn test_hir_checker_simple_list() {
    let src = "fn test() to Unit { let list = [1, 2, 3]\n let _ = list\n }";
    let diags = check_hir(src);
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn test_hir_checker_pattern_match() {
    let src = r#"
type Opt = | Some(v: int) | None

fn opt_unwrap(x: Opt) to int {
    match x {
        Some(v) -> v
        None -> 0
    }
}
"#;
    let diags = check_hir(src);
    assert!(diags.is_empty(), "{diags:?}");
}
