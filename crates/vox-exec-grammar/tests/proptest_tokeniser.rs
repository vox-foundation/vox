//! Property-based tests for the `vox-exec-grammar` tokeniser and pipeline splitter.
//!
//! Goals:
//! 1. **Total parsing**: any input either parses successfully or returns a typed
//!    [`ParseError`] — never panics, never hangs.
//! 2. **Round-trip stability**: parsing the same input twice yields the same AST.
//! 3. **Pipeline invariants**: every parsed segment has a non-empty `command`.
//! 4. **Quote balancing**: balanced quotes never produce `UnmatchedQuote`;
//!    unbalanced quotes always do (when the unbalance is outside the other quote type).

use proptest::prelude::*;
use vox_exec_grammar::{ParseError, parse, parse_pipeline};

/// Strategy: arbitrary printable ASCII tokens (no quotes, no separators).
fn safe_token() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_./=-]{1,12}"
}

/// Strategy: a "well-formed" command line — random tokens joined by spaces.
fn well_formed_command() -> impl Strategy<Value = String> {
    proptest::collection::vec(safe_token(), 1..6).prop_map(|toks| toks.join(" "))
}

/// Strategy: an arbitrary pipeline of well-formed commands.
fn well_formed_pipeline() -> impl Strategy<Value = String> {
    let separator = prop_oneof![
        Just(" | "),
        Just(" || "),
        Just(" && "),
        Just("; "),
        Just(" & "),
    ];
    proptest::collection::vec(well_formed_command(), 1..5).prop_flat_map(move |segs| {
        let n = segs.len();
        proptest::collection::vec(separator.clone(), n.saturating_sub(1)).prop_map(
            move |seps| {
                let mut out = String::new();
                for (i, seg) in segs.iter().enumerate() {
                    out.push_str(seg);
                    if i + 1 < segs.len() {
                        out.push_str(seps[i]);
                    }
                }
                out
            },
        )
    })
}

proptest! {
    /// Any UTF-8 string either parses or returns a typed error — never panics.
    #[test]
    fn parse_never_panics(s in ".*") {
        let _ = parse(&s);
    }

    /// Same total-parsing guarantee for the pipeline parser.
    #[test]
    fn parse_pipeline_never_panics(s in ".*") {
        let _ = parse_pipeline(&s);
    }

    /// Round-trip: parsing the same input twice yields equal ASTs.
    #[test]
    fn parse_is_deterministic(s in well_formed_command()) {
        let a = parse(&s);
        let b = parse(&s);
        match (a, b) {
            (Ok(a), Ok(b)) => prop_assert_eq!(a, b),
            (Err(_), Err(_)) => {} // both errored — also deterministic
            _ => prop_assert!(false, "non-deterministic parse for {s:?}"),
        }
    }

    /// Every successfully-parsed pipeline segment has a non-empty command.
    #[test]
    fn pipeline_segments_have_commands(s in well_formed_pipeline()) {
        if let Ok(asts) = parse_pipeline(&s) {
            for ast in asts {
                prop_assert!(
                    !ast.command.is_empty(),
                    "empty command from {s:?}: {ast:?}"
                );
            }
        }
    }

    /// Well-formed pipelines (random tokens joined by separators) always parse.
    #[test]
    fn well_formed_pipelines_always_parse(s in well_formed_pipeline()) {
        prop_assert!(parse_pipeline(&s).is_ok(), "failed to parse {s:?}");
    }

    /// Strings with a single unmatched double-quote always produce UnmatchedQuote.
    /// (We add the `"` at the start so it can't be inside a single-quoted region
    /// from the random text.)
    #[test]
    fn unmatched_double_quote_is_detected(suffix in "[a-zA-Z ]{0,20}") {
        let payload = format!("\"{suffix}");
        match parse(&payload) {
            Err(ParseError::UnmatchedQuote(_)) => {}
            Err(ParseError::Empty) => {} // an entirely-whitespace suffix is fine
            other => prop_assert!(
                false,
                "expected UnmatchedQuote for {payload:?}, got {other:?}"
            ),
        }
    }
}
