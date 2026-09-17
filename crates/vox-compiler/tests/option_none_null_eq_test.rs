//! Regression test: `Option::None` must compare equal to the `null` literal
//! under `==`/`is` and `!=`/`isnt`/`is not`, matching `is_none()`/`is_some()`.
//!
//! Root cause: `VoxValue`'s custom `PartialEq` impl (crates/vox-compiler/src/
//! eval/value.rs) had `(Null, Null) => true` and `(Option(a), Option(b)) => a
//! == b` but no cross-variant arm, so `VoxValue::Option(None) ==
//! VoxValue::Null` fell through to the catch-all `_ => false`. Every
//! `x is null` / `x == null` guard on an `Option[T]`-typed value (the
//! idiomatic pattern after `env.get()` / `process.run()`, both of which
//! return `Option`) was therefore dead code, and `x isnt null` was always
//! true — masking a genuine `None` and later panicking on `.unwrap()`.

use vox_compiler::eval::value::VoxValue;
use vox_compiler::{eval, hir, lexer, parser};

fn run_probe(source: &str) -> Result<VoxValue, String> {
    let tokens = lexer::lex(source);
    let module = parser::descent::parse(tokens).map_err(|e| format!("parse: {e:?}"))?;
    let lowered = hir::lower::lower_module(&module);
    let mut interp = eval::Interpreter::new(1_000_000);
    interp
        .run_module(&lowered)
        .map_err(|e| format!("run: {e:?}"))?;
    interp
        .call("main", vec![])
        .map_err(|e| format!("call: {e:?}"))
}

#[test]
fn none_option_equals_null_literal() {
    let source = r#"
    fn main() to bool {
        let x: Option[str] = None
        return x == null
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

#[test]
fn none_option_is_null_via_is_keyword() {
    let source = r#"
    fn main() to bool {
        let x: Option[str] = None
        return x is null
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

#[test]
fn none_option_isnt_null_is_false() {
    let source = r#"
    fn main() to bool {
        let x: Option[str] = None
        return x isnt null
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(false));
}

#[test]
fn some_option_isnt_null() {
    let source = r#"
    fn main() to bool {
        let x: Option[str] = Some("a")
        return x isnt null
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

#[test]
fn some_option_is_not_equal_to_null() {
    let source = r#"
    fn main() to bool {
        let x: Option[str] = Some("a")
        return x == null
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(false));
}

/// Direct unit-level proof at the root-cause type, independent of the
/// language pipeline above.
#[test]
fn vox_value_option_none_partial_eq_null() {
    assert_eq!(VoxValue::Option(None), VoxValue::Null);
    assert_eq!(VoxValue::Null, VoxValue::Option(None));
    assert_ne!(
        VoxValue::Option(Some(Box::new(VoxValue::Int(1)))),
        VoxValue::Null
    );
}
