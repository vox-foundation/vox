//! Regression test: every method the typecheck builtin registry knows about
//! must also dispatch under the eval-side `call_builtin_method` (or its
//! result type must match what eval returns).
//!
//! Surfaced after the 2026-05-23 audit found two false-positive passes
//! (scripts passing `vox check` but failing `vox run`) caused by
//! eval↔typeck drift on `Object.get` and `process.run`. The fix in both
//! cases was to align eval's return shape with the typecheck signature.
//! This test locks the alignment in.
//!
//! The test runs probe `.vox` snippets through the full
//! lex → parse → lower → eval pipeline and asserts they don't fail with
//! `Method ... not found`. Any future eval/typeck drift on the audited
//! methods will surface here as a hard failure.

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

// Every probe below was crafted from a real `vox check`-passes-but-`vox run`-
// fails case that the 2026-05-23 audit found. Each one exercises one
// eval/typeck parity point.

/// `record.get(key).unwrap()` — typeck says `Option[T]`; eval used to return
/// the bare value, breaking `.unwrap()`. Fixed by eval/builtins.rs change.
#[test]
fn object_get_returns_option_compatible_with_unwrap() {
    let source = r#"
    fn main() to bool {
        let m = { a: "x", b: "y" }
        let v = m.get("a").unwrap()
        return v == "x"
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

/// Missing key returns Option(None) — `is_some()` is false. Same impl path.
#[test]
fn object_get_missing_key_returns_none() {
    let source = r#"
    fn main() to bool {
        let m = { a: "x" }
        return m.get("missing").is_none()
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

/// `process.run(...).unwrap()` — typeck says `Option[Record]`; eval used to
/// return the bare Object, breaking `.unwrap()`. Fixed by eval/builtins.rs
/// change. We pick a command that exists on every supported platform.
#[test]
fn process_run_returns_option_compatible_with_unwrap() {
    // Use `cargo --version` because cargo is on every supported platform
    // (the project itself requires it). On Windows the shim is `cargo.exe`
    // but std::process::Command resolves it.
    let source = r#"
    fn main() to bool {
        let p = process.run("cargo", ["--version"])
        if p isnt null {
            let r = p.unwrap()
            return r.code == 0
        }
        return false
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(
        res,
        VoxValue::Bool(true),
        "process.run must wrap its Record return in Option(Some(...))"
    );
}

/// `regex.is_match` / `regex.replace` / `regex.captures` — added 2026-05-23.
#[test]
fn regex_namespace_methods_dispatch() {
    let source = r#"
    fn main() to bool {
        let a = regex.is_match("hello 42 world", "[0-9]+")
        let b = regex.replace("hello world", "world", "vox")
        return a and (b == "hello vox")
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

/// `regex.find` — returns Option[str] with first match, or None on no match.
/// Added 2026-05-27 (CR-L fix: "find" was missing from RegexModule methods map
/// in typeck/builtins.rs; it was only in builtin_registry.rs for the std.regex path).
#[test]
fn regex_find_dispatch() {
    let source = r#"
    fn main() to bool {
        let hit = regex.find("hello 42 world", "[0-9]+")
        let miss = regex.find("no digits here", "[0-9]+")
        return hit.is_some() and miss.is_none()
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

/// `regex.captures` — returns Option[list[str]] with all capture groups (incl. full match).
#[test]
fn regex_captures_dispatch() {
    let source = r#"
    fn main() to bool {
        let caps = regex.captures("2026-05-27", r"(\d{4})-(\d{2})-(\d{2})")
        let miss = regex.captures("not a date", r"(\d{4})-(\d{2})-(\d{2})")
        return caps.is_some() and miss.is_none()
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

/// `fs.cwd` / `fs.exists` — both should dispatch.
#[test]
fn fs_cwd_and_exists_dispatch() {
    let source = r#"
    fn main() to bool {
        let cwd_res = fs.cwd()
        let cwd_ok = cwd_res.is_ok()
        let here = fs.exists(".")
        return cwd_ok and here
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

/// `path.extension` / `path.parent` / `path.file_name` / `path.stem` —
/// added 2026-05-23.
#[test]
fn path_helpers_dispatch() {
    let source = r#"
    fn main() to bool {
        let ext = path.extension("foo/bar.txt")
        // `path.stem` returns Option[str] (typeck/interp parity); unwrap it.
        let stem = match path.stem("foo/bar.txt") {
            Some(s) => s
            None => ""
        }
        return ext == "txt" and stem == "bar"
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

/// `process.which` returns Option[str]. `cargo` should resolve.
#[test]
fn process_which_returns_option() {
    let source = r#"
    fn main() to bool {
        let cargo = process.which("cargo")
        let absent = process.which("definitely-not-a-real-command-xyz")
        return cargo.is_some() and absent.is_none()
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

/// `Result.is_ok` / `Result.unwrap` / `Result.is_err` — used by every
/// fs.* call site.
#[test]
fn result_methods_dispatch() {
    let source = r#"
    fn main() to bool {
        // fs.exists returns bool not Result, so use fs.read which does.
        // The read may succeed or fail; either way the method dispatch
        // must not error.
        let r = fs.read("Cargo.toml")
        let ok = r.is_ok()
        return ok or r.is_err()
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

/// `Option.unwrap_or` — added 2026-05-23 to typeck; eval must have it too.
#[test]
fn option_unwrap_or_dispatch() {
    let source = r#"
    fn main() to bool {
        let m = { a: 1 }
        let present = m.get("a").unwrap_or(99)
        let missing = m.get("nope").unwrap_or(99)
        return present == 1 and missing == 99
    }
    "#;
    let res = run_probe(source).expect("should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

/// `Option.unwrap()` on `None` MUST raise an EvalError — not silently
/// return `Null`. Health regression: the prior behavior was a silent
/// foot-gun (same class as the `!`-operator bug fixed earlier this
/// session). Locked in via `_Panic` sentinel + caught in eval/expr.rs.
///
/// We get a `None` from `Object.get("missing")` since `None` isn't an
/// expression-position binding in interp scope.
#[test]
fn option_unwrap_on_none_panics_with_useful_message() {
    let source = r#"
    fn main() to int {
        let m = { a: 1 }
        return m.get("missing").unwrap()
    }
    "#;
    let err = run_probe(source).expect_err("unwrap on None must error");
    assert!(
        err.contains("unwrap") && err.contains("None"),
        "error must name unwrap and None; got: {err}"
    );
}

/// `Option.expect(msg)` on `None` MUST surface the supplied message in
/// the panic, not swallow it.
#[test]
fn option_expect_carries_the_supplied_message() {
    let source = r#"
    fn main() to int {
        let m = { a: 1 }
        return m.get("missing").expect("config value must be present")
    }
    "#;
    let err = run_probe(source).expect_err("expect on None must error");
    assert!(
        err.contains("config value must be present"),
        "expect's message must surface; got: {err}"
    );
}

/// `Result.unwrap_err()` on `Ok(_)` MUST panic. Prior impl returned an
/// empty string, silently masking the misuse.
///
/// We construct a Result by trying to read a file we know exists; the
/// success case gives Ok, and `unwrap_err` should panic on it.
#[test]
fn result_unwrap_err_on_ok_panics() {
    let source = r#"
    fn main() to str {
        return fs.read("Cargo.toml").unwrap_err()
    }
    "#;
    let err = run_probe(source).expect_err("unwrap_err on Ok must error");
    assert!(
        err.contains("unwrap_err") && err.contains("Ok"),
        "error must name unwrap_err and Ok; got: {err}"
    );
}

/// `Result.unwrap()` on `Err(e)` MUST panic with the error message,
/// not silently return Null. Try to read a path that doesn't exist.
#[test]
fn result_unwrap_on_err_carries_message() {
    let source = r#"
    fn main() to str {
        return fs.read("/definitely/not/a/real/path/xyzzy.txt").unwrap()
    }
    "#;
    let err = run_probe(source).expect_err("unwrap on Err must error");
    assert!(
        err.contains("Err") && err.contains("Result.unwrap"),
        "error must name unwrap and Err; got: {err}"
    );
}

/// Closures: `fn(params) { body }` literal must parse, lower, and eval.
/// Per closures-rfc-2026-05-23.md §11: `fn(...)` is the canonical
/// anonymous-function syntax (NOT `|x|`).
#[test]
fn closure_literal_parses_and_evaluates() {
    let source = r#"
    fn main() to int {
        let dbl = fn(x: int) to int { x * 2 }
        return dbl(21)
    }
    "#;
    let res = run_probe(source).expect("closure should evaluate");
    assert_eq!(res, vox_compiler::eval::value::VoxValue::Int(42));
}

/// Closures capture their enclosing scope by clone (RFC §3.3).
#[test]
fn closure_captures_lexical_scope() {
    let source = r#"
    fn main() to int {
        let factor = 10
        let scale = fn(x: int) to int { x * factor }
        return scale(7)
    }
    "#;
    let res = run_probe(source).expect("closure with capture should evaluate");
    assert_eq!(res, vox_compiler::eval::value::VoxValue::Int(70));
}

/// `List.map(closure)` applies the closure per element (RFC §9.5).
#[test]
fn list_map_with_closure() {
    let source = r#"
    fn main() to bool {
        let xs = [1, 2, 3]
        let doubled = xs.map(fn(n: int) to int { n * 2 })
        return doubled.len() == 3
    }
    "#;
    let res = run_probe(source).expect("List.map with closure should evaluate");
    assert_eq!(res, vox_compiler::eval::value::VoxValue::Bool(true));
}

/// `List.filter(closure)` keeps elements where the closure returns true.
#[test]
fn list_filter_with_closure() {
    let source = r#"
    fn main() to int {
        return [1, 2, 3, 4, 5].filter(fn(n: int) to bool { n % 2 == 0 }).len()
    }
    "#;
    let res = run_probe(source).expect("List.filter with closure should evaluate");
    assert_eq!(res, vox_compiler::eval::value::VoxValue::Int(2));
}

/// `List.fold(init, closure)` threads an accumulator.
#[test]
fn list_fold_with_closure() {
    let source = r#"
    fn main() to int {
        return [1, 2, 3, 4, 5].fold(0, fn(acc: int, n: int) to int { acc + n })
    }
    "#;
    let res = run_probe(source).expect("List.fold with closure should evaluate");
    assert_eq!(res, vox_compiler::eval::value::VoxValue::Int(15));
}

/// `Option.map(closure)` transforms Some, passes through None.
#[test]
fn option_map_with_closure() {
    let source = r#"
    fn main() to bool {
        let m = { a: 10 }
        let some = m.get("a").map(fn(v: int) to int { v + 5 })
        let none = m.get("missing").map(fn(v: int) to int { v + 5 })
        return some.is_some() and none.is_none()
    }
    "#;
    let res = run_probe(source).expect("Option.map should evaluate");
    assert_eq!(res, vox_compiler::eval::value::VoxValue::Bool(true));
}

/// `Result.map_err(closure)` transforms Err, passes through Ok.
#[test]
fn result_map_err_with_closure() {
    let source = r#"
    fn main() to bool {
        let r = fs.read("/definitely/missing/xyz.txt").map_err(fn(e: str) to str { "wrap: " + e })
        return r.is_err()
    }
    "#;
    let res = run_probe(source).expect("Result.map_err should evaluate");
    assert_eq!(res, vox_compiler::eval::value::VoxValue::Bool(true));
}

/// Diagnostic-quality gate (closures RFC §11 Q4): when typecheck can't
/// resolve a method-call receiver, the error MUST avoid leaking
/// internal TypeVar IDs and MUST suggest the canonical annotation form
/// (`fn(x: <Type>) { ... }`). Locks in the `<unknown>` renderer + hint.
#[test]
fn typeck_method_not_found_on_unknown_suggests_closure_annotation() {
    let source = r#"
    fn main() {
        let xs = []
        for x in xs {
            x.frobnicate()
        }
    }
    "#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::descent::parse(tokens).expect("parse should succeed");
    let mut hir = vox_compiler::hir::lower::lower_module(&module);
    let diags = vox_compiler::typeck::typecheck_hir_module(source, &mut hir);
    let combined = diags
        .iter()
        .map(|d| d.message.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        !combined.contains("TypeVar"),
        "diagnostic must not leak internal TypeVar IDs; got: {combined}"
    );
    assert!(
        combined.contains("<unknown>") || combined.contains("explicit type annotation"),
        "diagnostic must use the user-facing `<unknown>` placeholder or \
         suggest an annotation; got: {combined}"
    );
}

/// `abs(int)` / `abs(float)` must each typecheck cleanly with no diagnostics
/// — locks in the Important-4 fix in `typeck/checker/expr.rs`'s `is_abs_call`
/// arm, which used to fall through into a second `check_arguments` pass over
/// the same argument node after already peeking its type once. That
/// double-check produced a spurious "Cannot unify Int with Float" diagnostic
/// for `abs(-2.0)` on some resolution orders. The arm now returns
/// unconditionally (matching the sibling `is_range_one_arg` special case),
/// checking `args[0].value` exactly once.
#[test]
fn abs_typechecks_for_both_int_and_float_without_double_check() {
    for src in [
        "fn main() to int { return abs(-2) }",
        "fn main() to float { return abs(-2.0) }",
    ] {
        let tokens = vox_compiler::lexer::lex(src);
        let module = vox_compiler::parser::descent::parse(tokens).expect("parse should succeed");
        let mut hir = vox_compiler::hir::lower::lower_module(&module);
        let diags = vox_compiler::typeck::typecheck_hir_module(src, &mut hir);
        assert!(
            diags.is_empty(),
            "abs() should typecheck cleanly for {src:?}; got: {:#?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

/// Integer arithmetic overflow MUST produce a clean EvalError, not a
/// Rust panic that takes down the interpreter process. Locks in the
/// `checked_*` arithmetic semantics added in this session.
#[test]
fn int_overflow_produces_clean_error_not_panic() {
    let source = r#"
    fn main() to int {
        let huge = 9223372036854775806
        return huge + 5
    }
    "#;
    let err = run_probe(source).expect_err("overflow must error");
    assert!(
        err.contains("overflow"),
        "error must name overflow; got: {err}"
    );
}

/// Integer division by zero MUST produce a clean EvalError.
#[test]
fn int_div_by_zero_produces_clean_error() {
    let source = r#"
    fn main() to int {
        let n = 10
        let z = 0
        return n / z
    }
    "#;
    let err = run_probe(source).expect_err("div-by-zero must error");
    assert!(
        err.contains("division by zero"),
        "error must name div-by-zero; got: {err}"
    );
}

// ── `?` operator (try / error-propagation) ──────────────────────────────────

/// `?` on `Result` — unwraps `Ok` inline, or early-returns with the `Err`.
/// Landed 2026-05-27: `HirExpr::Try` eval arm was missing; the
/// `_ => VoxValue::Null` catch-all silently returned Null instead of
/// propagating the error. Now properly implemented.
#[test]
fn try_operator_on_result_propagates_err_and_unwraps_ok() {
    // Helper that returns `Ok("success")` on truthy input, `Err("fail")` otherwise.
    let source = r#"
    fn maybe_ok(flag: bool) to Result[str] {
        if flag { return Ok("success") }
        return Err("fail")
    }
    fn main() to bool {
        // ? on Ok: unwraps to the inner string.
        let val = maybe_ok(true)?
        let ok_case = val == "success"
        return ok_case
    }
    "#;
    let res = run_probe(source).expect("? on Ok should evaluate cleanly");
    assert_eq!(res, VoxValue::Bool(true));
}

/// `?` on `Result(Err)` early-returns the enclosing function with Err.
#[test]
fn try_operator_on_err_causes_early_return() {
    let source = r#"
    fn failing() to Result[str] {
        return Err("oops")
    }
    fn caller() to Result[str] {
        let _ = failing()?
        return Ok("unreachable")
    }
    fn main() to bool {
        let r = caller()
        return r.is_err()
    }
    "#;
    let res = run_probe(source).expect("? on Err should propagate to caller");
    assert_eq!(res, VoxValue::Bool(true));
}

/// `?` on `Option(None)` early-returns the enclosing function with None.
#[test]
fn try_operator_on_none_causes_early_return() {
    let source = r#"
    fn returns_none() to Option[str] {
        return None
    }
    fn caller() to Option[str] {
        let _ = returns_none()?
        return Some("unreachable")
    }
    fn main() to bool {
        let r = caller()
        return r.is_none()
    }
    "#;
    let res = run_probe(source).expect("? on None should propagate to caller");
    assert_eq!(res, VoxValue::Bool(true));
}

// ── Tuple literals ───────────────────────────────────────────────────────────

/// Tuple literal `(a, b, c)` must parse, lower, and evaluate to `VoxValue::Tuple`.
/// Added 2026-05-27: `HirExpr::TupleLit` was hitting the `_ => VoxValue::Null`
/// catch-all in the evaluator.
#[test]
fn tuple_literal_evaluates_to_vox_tuple() {
    let source = r#"
    fn main() to int {
        let t = (1, 2, 3)
        match t {
            (a, b, c) => return a + b + c
        }
    }
    "#;
    let res = run_probe(source).expect("tuple eval should succeed");
    assert_eq!(res, VoxValue::Int(6));
}

/// Negation must use `not`, not `!`. Lock the phonetic-only choice
/// (audit doc §8).
#[test]
fn bang_operator_still_errors_with_phonetic_hint() {
    let source = "fn main() to bool { return !true }";
    let tokens = lexer::lex(source);
    let err = parser::descent::parse(tokens).expect_err("`!` must error");
    let s = format!("{err:?}");
    assert!(
        s.contains("not a valid operator") && s.contains("not"),
        "error should name `not` as the canonical form; got: {s}"
    );
}

// ── Task 2 (interpreter-first execution, PR 2): close the measured drift ────
// See .superpowers/sdd/task-2-brief.md. These probes were crafted from the
// eight EXPECT-bearing goldens landed in Task 1b (examples/golden/*.vox) that
// the nightly differential gate (crates/vox-integration-tests/tests/
// golden_differential_gate.rs) exercises interp-vs-native.

/// Asserts both that codegen emits *something* for each eval-only method
/// AND that the emitted snippet is the exact native shape the interp/typeck
/// alignment (above) assumes — not just `is_some()`, which would pass even
/// if the emit regressed to a stale or wrong-shaped call. Each expected
/// string is copied verbatim from `std_namespace_runtime_call`'s match arms
/// in `builtin_registry.rs`; if either side edits its shape without the
/// other, this test fails instead of silently drifting.
#[test]
fn registry_emits_every_eval_only_method() {
    use vox_compiler::builtin_registry::std_namespace_runtime_call;
    for (ns, m, args, expected) in [
        (
            "time",
            "now",
            vec![],
            "vox_actor_runtime::builtins::vox_now_ms()".to_string(),
        ),
        (
            "json",
            "encode",
            vec!["x".to_string()],
            "vox_actor_runtime::builtins::vox_json_render(&(x)).unwrap_or_default()".to_string(),
        ),
        (
            "json",
            "stringify",
            vec!["x".to_string()],
            "vox_actor_runtime::builtins::vox_json_render(&(x)).unwrap_or_default()".to_string(),
        ),
        (
            "process",
            "cwd",
            vec![],
            "vox_actor_runtime::builtins::vox_process_cwd()".to_string(),
        ),
        (
            "secrets",
            "resolve",
            vec!["\"K\"".to_string()],
            "vox_actor_runtime::builtins::vox_secrets_resolve((\"K\").as_str())".to_string(),
        ),
        (
            "crypto",
            "hash_fast",
            vec!["\"abc\"".to_string()],
            "vox_crypto::hash_fast_hex((\"abc\").as_bytes())".to_string(),
        ),
    ] {
        let emitted = std_namespace_runtime_call(ns, m, &args);
        assert_eq!(
            emitted.as_deref(),
            Some(expected.as_str()),
            "codegen emit for {ns}.{m} drifted from expected native shape"
        );
    }
}

#[test]
fn display_of_composites_matches_the_surface_form() {
    let v = run_probe(
        r#"pub fn main() { return str([1, 2]) + "|" + str({a: 1}) + "|" + str(Some(3)) + "|" + str(Ok(1)) }"#,
    )
    .unwrap();
    assert!(
        matches!(v, VoxValue::Str(ref s) if s.as_ref() == "[1, 2]|{a: 1}|Some(3)|Ok(1)"),
        "{v:?}"
    );
}

#[test]
fn glob_is_sorted_and_propagates_errors() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("b.txt"), "b").unwrap();
    std::fs::write(d.path().join("a.txt"), "a").unwrap();
    let pat = format!("{}/*", d.path().display());
    let src = format!(
        r#"pub fn main() {{ return match fs.glob("{pat}") {{ Ok(xs) => xs is xs.sorted() and len(xs) is 2, Error(e) => false }} }}"#
    );
    let v = run_probe(&src).unwrap();
    assert!(matches!(v, VoxValue::Bool(true)), "{v:?}");
}

#[test]
fn list_push_is_amortised_constant_not_quadratic() {
    // 5_000 pushes must finish well inside the 10 M step budget and well under a second.
    let t0 = std::time::Instant::now();
    let v = run_probe(
        r#"pub fn main() { let mut xs = []; let mut i = 0; while i < 5000 { xs = xs.push(i); i = i + 1 }; return len(xs) }"#,
    )
    .unwrap();
    assert!(matches!(v, VoxValue::Int(5000)), "{v:?}");
    assert!(
        t0.elapsed() < std::time::Duration::from_millis(500),
        "list.push is still cloning the receiver: {:?}",
        t0.elapsed()
    );
}

/// `env.args()` needs `Interpreter.source_path`/`script_args` (Task 2 Step 9)
/// and is dispatched inline in `eval/expr.rs`, not the generic
/// `call_builtin_method` that `builtin_registry.rs`'s two parity probes
/// (`interpreter_dispatches_every_interp_builtin`,
/// `interpreter_return_shape_matches_typecheck`) exercise — both carry an
/// explicit `env.args` skip for that reason. This test restores coverage via
/// the one path that *can* observe it: the full interpreter with
/// `source_path`/`script_args` set, asserting the exact
/// `[source_path] ++ script_args` shape that mirrors native codegen's argv
/// emission (`backend/native.rs`) and matches `typeck::builtins`' `env.args`
/// signature (`List[str]`).
#[test]
fn env_args_matches_source_path_plus_script_args() {
    let source = r#"
    fn main() to list[str] {
        return env.args()
    }
    "#;
    let tokens = lexer::lex(source);
    let module = parser::descent::parse(tokens).expect("parse");
    let lowered = hir::lower::lower_module(&module);
    let mut interp = eval::Interpreter::new(1_000_000);
    interp.set_source_path("probe_script.vox");
    interp.script_args = vec!["--flag".to_string(), "value".to_string()];
    interp.run_module(&lowered).expect("run_module");
    let res = interp.call("main", vec![]).expect("call main");
    assert_eq!(
        res,
        VoxValue::list(vec![
            VoxValue::Str("probe_script.vox".to_string().into()),
            VoxValue::Str("--flag".to_string().into()),
            VoxValue::Str("value".to_string().into()),
        ]),
        "env.args() must yield [source_path] ++ script_args"
    );
}

#[test]
fn hash_fast_matches_vox_crypto() {
    let v = run_probe(r#"pub fn main() { return crypto.hash_fast("abc") }"#).unwrap();
    let expected = vox_crypto::hash_fast_hex(b"abc");
    assert!(
        matches!(v, VoxValue::Str(ref s) if s.as_ref() == expected.as_str()),
        "{v:?} != {expected}"
    );
}
