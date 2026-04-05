#[test]
fn test_interpreter_basic() {
    let source = "
    fn add(a: int, b: int) -> int {
        return a + b
    }

    fn main() -> int {
        let x = 10
        let mut y = 20
        while y < 30 {
            y = y + 2
        }
        return add(x, y)
    }
    ";

    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::descent::parse(tokens).expect("Failed to parse");
    // We need to lower it to HIR
    let lowered = vox_compiler::hir::lower::lower_module(&module);

    let mut interpreter = vox_compiler::eval::Interpreter::new(100_000);
    interpreter
        .run_module(&lowered)
        .expect("Failed to run module");

    let res = interpreter
        .call("main", vec![])
        .expect("Failed to call main");
    assert_eq!(res, vox_compiler::eval::value::VoxValue::Int(40));
}
