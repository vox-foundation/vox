//! Canonical `.vox` serialization helpers.
//!
//! This module defines a strict, parse-validated compact representation used by
//! orchestration and generation pipelines that require deterministic, single-line
//! `.vox` output.

use crate::lexer::{compact, lex};
use crate::parser::parse;

/// Error returned when canonicalization fails.
#[derive(Debug, thiserror::Error)]
pub enum CanonicalizeError {
    /// The compacted output does not parse as valid Vox syntax.
    #[error("compacted output failed parse validation")]
    ParseValidationFailed,
}

/// Produce canonical compact `.vox` source.
///
/// Guarantees:
/// - output is produced by the lexer-based compact serializer,
/// - compacted output parses successfully,
/// - canonicalization is idempotent (`canon(canon(x)) == canon(x)`).
pub fn canonicalize_vox(source: &str) -> Result<String, CanonicalizeError> {
    let compacted = compact(source);
    if parse(lex(&compacted)).is_err() {
        return Err(CanonicalizeError::ParseValidationFailed);
    }
    Ok(compacted)
}

#[cfg(test)]
mod tests {
    use super::canonicalize_vox;

    #[test]
    fn canonicalize_vox_is_idempotent() {
        let source = r#"
fn greet(name: str) to str {
    if name is "" {
        ret "Hello, stranger"
    }
    ret "Hello, " + name
}
"#;
        let once = canonicalize_vox(source).expect("first canonicalization");
        let twice = canonicalize_vox(&once).expect("second canonicalization");
        assert_eq!(once, twice, "canonicalization must be idempotent");
    }

    #[test]
    fn canonicalize_vox_golden_serialization() {
        let source = r#"
fn main() {
    let x = 10
    ret x
}
"#;
        let out = canonicalize_vox(source).expect("canonicalized output");
        assert_eq!(out, "fn main(){let x=10 ret x}");
    }

    #[test]
    fn canonicalize_vox_rejects_invalid_output() {
        let invalid = "fn main( { ret 1 }";
        let err = canonicalize_vox(invalid).expect_err("invalid source must fail");
        assert!(
            err.to_string().contains("parse validation"),
            "unexpected error: {err}"
        );
    }
}
