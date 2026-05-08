/// Verify that an indexed for-loop (`for v, i in arr`) binds the index
/// variable correctly in the body.  The for expression returns a list, so
/// `for v, i in [10, 20, 30] { v + i }` should produce [10+0, 20+1, 30+2]
/// = [10, 21, 32].
#[test]
fn for_loop_with_index_binds_index_in_body() {
    let source = "
    fn main() -> List {
        return for v, i in [10, 20, 30] { v + i }
    }
    ";

    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::descent::parse(tokens).expect("Failed to parse");
    let lowered = vox_compiler::hir::lower::lower_module(&module);

    let mut interpreter = vox_compiler::eval::Interpreter::new(100_000);
    interpreter
        .run_module(&lowered)
        .expect("Failed to run module");

    let res = interpreter
        .call("main", vec![])
        .expect("Failed to call main");
    assert_eq!(
        res,
        vox_compiler::eval::value::VoxValue::List(vec![
            vox_compiler::eval::value::VoxValue::Int(10),
            vox_compiler::eval::value::VoxValue::Int(21),
            vox_compiler::eval::value::VoxValue::Int(32),
        ]),
        "for v, i in [10,20,30] {{ v+i }} should yield [10, 21, 32]"
    );
}

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
