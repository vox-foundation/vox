//! Compile-error coverage for `std.array` dtype pairs and required names.
//!
//! Mixed array arithmetic must name `astype`. `sort` accepts only
//! `nulls="first"` or `nulls="last"`. Host scalar `1 + 1.0` stays the
//! existing promotion and is not asserted here.

use vox_compiler::hir::{HirModule, HirType, lower_module};
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::diagnostics::TypeckSeverity;
use vox_compiler::typeck::typecheck_hir_module;

fn check(src: &str) -> (Vec<String>, HirModule) {
    let module = parse(lex(src)).unwrap_or_else(|errs| panic!("parse failed: {errs:?}\n{src}"));
    let mut hir = lower_module(&module);
    let errors = typecheck_hir_module(src, &mut hir)
        .into_iter()
        .filter(|diag| diag.severity == TypeckSeverity::Error)
        .map(|diag| diag.message)
        .collect();
    (errors, hir)
}

fn type_at<'a>(src: &str, hir: &'a HirModule, snippet: &str) -> &'a HirType {
    hir.inferred_types
        .iter()
        .find_map(|(span, ty)| {
            let text = src.get(span.start..span.end)?;
            (text.trim() == snippet).then_some(ty)
        })
        .unwrap_or_else(|| {
            let seen: Vec<_> = hir
                .inferred_types
                .keys()
                .filter_map(|span| src.get(span.start..span.end))
                .collect();
            panic!("no inferred type for {snippet:?}; spans: {seen:?}");
        })
}

fn array_of(dtype: &str) -> HirType {
    HirType::Generic("Array".into(), vec![HirType::Named(dtype.into())])
}

#[test]
fn mixed_array_arithmetic_names_astype() {
    let src = "\
fn mismatch(a: Array[i64], b: Array[f64]) {
    let _ = a + b
    let _ = a - b
    let _ = a * b
    let _ = a / b
}
fn widths(a: Array[i32], b: Array[i64]) {
    let _ = a + b
}
";
    let (errors, _) = check(src);
    assert!(
        errors.len() >= 5,
        "add, sub, mul, div, and i32/i64 must each diagnose, got {errors:?}"
    );
    assert!(
        errors.iter().all(|message| message.contains("astype")),
        "every mixed-dtype diagnostic must name astype, got {errors:?}"
    );

    let same = "\
import std.array
fn same(a: Array[f64], b: Array[f64], xs: list[float]) {
    let s = a + b
    let p = a * b
    let broadcast = a + a.mean()
    let _ = array.from_list(xs)
}
";
    let (same_errors, hir) = check(same);
    assert!(
        same_errors.is_empty(),
        "same-dtype add, mul, and 0-d mean must typecheck: {same_errors:?}"
    );
    assert_eq!(type_at(same, &hir, "a + b"), &array_of("f64"));
    assert_eq!(type_at(same, &hir, "a * b"), &array_of("f64"));
    assert_eq!(type_at(same, &hir, "a + a.mean()"), &array_of("f64"));
}

#[test]
fn std_requires_named_correction() {
    let missing = "\
import std.array
fn missing(xs: list[float]) {
    let a = array.from_list(xs)
    let _ = a.std()
}
";
    let (missing_errors, _) = check(missing);
    assert!(
        !missing_errors.is_empty(),
        "std() must be a diagnostic, got none"
    );

    let positional = "\
import std.array
fn positional(xs: list[float]) {
    let a = array.from_list(xs)
    let _ = a.std(0)
}
";
    let (positional_errors, _) = check(positional);
    assert!(
        !positional_errors.is_empty(),
        "positional std(0) must be a diagnostic, got none"
    );

    let named = "\
import std.array
fn named(a: Array[f64]) {
    let _ = a.std(correction=0)
}
";
    let (named_errors, _) = check(named);
    assert!(
        named_errors.is_empty(),
        "std(correction=0) on Array of f64 typechecks: {named_errors:?}"
    );
}

#[test]
fn sort_requires_nulls_first_or_last() {
    let missing = "\
fn missing(a: Array[f64]) {
    let _ = a.sort()
}
";
    let (missing_errors, _) = check(missing);
    assert!(
        !missing_errors.is_empty(),
        "sort() must be a diagnostic, got none"
    );

    let positional = "\
fn positional(a: Array[f64]) {
    let _ = a.sort(\"first\")
}
";
    let (positional_errors, _) = check(positional);
    assert!(
        !positional_errors.is_empty(),
        "positional sort(\"first\") must be a diagnostic, got none"
    );
    assert!(
        positional_errors
            .iter()
            .any(|message| message.contains("nulls")),
        "positional sort must name nulls, got {positional_errors:?}"
    );

    let other = "\
fn other(a: Array[f64]) {
    let _ = a.sort(nulls=\"middle\")
}
";
    let (other_errors, _) = check(other);
    assert!(
        !other_errors.is_empty(),
        "nulls=\"middle\" must be a diagnostic, got none"
    );
    assert!(
        other_errors.iter().any(|message| message.contains("nulls")),
        "other nulls string must name nulls, got {other_errors:?}"
    );

    for literal in ["first", "last"] {
        let src =
            format!("fn legal(a: Array[f64]) {{\n    let _ = a.sort(nulls=\"{literal}\")\n}}\n");
        let (errors, _) = check(&src);
        assert!(
            errors.is_empty(),
            "nulls={literal:?} on Array of f64 typechecks: {errors:?}"
        );
    }
}
