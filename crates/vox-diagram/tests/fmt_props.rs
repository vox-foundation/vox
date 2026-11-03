//! Properties of the formatter and parser over generated documents (P1, P2, P3, P5).

mod support;

use proptest::prelude::*;
use serde_json::{Value, to_value};
use support::{model_graph, model_ir_json, noise, noisy_source, strip_spans};
use vox_diagram::{check, fmt, parse};

fn ir(src: &str) -> Value {
    strip_spans(to_value(check(src).ir).unwrap_or(Value::Null))
}

fn ast(src: &str) -> Value {
    strip_spans(to_value(parse(src).ast).unwrap_or(Value::Null))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// P1: the parser agrees with an oracle built from the model alone.
    #[test]
    fn p1_check_matches_the_model_oracle(m in model_graph(), n in noise()) {
        let src = noisy_source(&m, &n);
        let report = check(&src);
        prop_assert!(report.diagnostics.is_empty(), "{:?}\n{src}", report.diagnostics);
        prop_assert!(report.valid);
        prop_assert_eq!(ir(&src), model_ir_json(&m), "{}", src);
    }

    /// P2: formatting is idempotent and canonical in shape.
    #[test]
    fn p2_fmt_is_idempotent(m in model_graph(), n in noise()) {
        let src = noisy_source(&m, &n);
        let once = fmt(&src);
        prop_assert!(once.is_some(), "valid source must format:\n{src}");
        let once = once.unwrap_or_default();
        let twice = fmt(&once);
        prop_assert_eq!(twice.as_deref(), Some(once.as_str()));
        prop_assert!(!once.contains('\r') && once.ends_with('\n') && !once.ends_with("\n\n"));
        prop_assert!(once.lines().all(|l| l == l.trim_end()), "trailing whitespace:\n{once}");
    }

    /// P3: formatting never changes the AST (comments and blank lines included) or the IR.
    #[test]
    fn p3_fmt_preserves_ast_and_ir(m in model_graph(), n in noise()) {
        let src = noisy_source(&m, &n);
        let out = fmt(&src).unwrap_or_default();
        prop_assert_eq!(ast(&out), ast(&src), "{}\n---\n{}", src, out);
        prop_assert_eq!(ir(&out), ir(&src));
    }

    /// P5: the IR serialises byte-identically across runs.
    #[test]
    fn p5_ir_json_is_stable(m in model_graph(), n in noise()) {
        let src = noisy_source(&m, &n);
        let a = serde_json::to_string(&check(&src).ir);
        let b = serde_json::to_string(&check(&src).ir);
        prop_assert!(a.is_ok());
        prop_assert_eq!(a.ok(), b.ok());
    }
}
