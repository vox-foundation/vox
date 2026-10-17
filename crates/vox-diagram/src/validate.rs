//! The single validation envelope shared by the `vox_validate_diagram` MCP tool and
//! `vox diagram --json` (D-01, D-04): check, canonical form and counters in one pure call.

use serde::Serialize;

use crate::diag::{Diagnostic, Stats};
use crate::{check, fmt};

/// Options of [`validate`]. Both fields arrive in Task 2; the type exists so callers are stable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ValidateOpts {}

/// The validation envelope; its JSON shape is the public contract of the MCP tool and the CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Validation {
    /// Envelope format version; always 1.
    pub schema_version: u32,
    /// True when no `Error` diagnostic exists.
    pub valid: bool,
    /// Every diagnostic, in source order, with machine-applicable fixes.
    pub diagnostics: Vec<Diagnostic>,
    /// Canonical `fmt` text once valid; always serialized, `null` while errors remain.
    pub normalized_source: Option<String>,
    /// Document counters.
    pub stats: Stats,
}

/// Validate `source`: pure, deterministic, never panics.
pub fn validate(source: &str, _opts: ValidateOpts) -> Validation {
    let r = check(source);
    Validation {
        schema_version: 1,
        valid: r.valid,
        normalized_source: if r.valid { fmt(source) } else { None },
        diagnostics: r.diagnostics,
        stats: r.stats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{check, fmt};

    const BROKEN: &str = "graph {\n    node a: \"A\"\n    a -> b\n    node c: C\n}\n";
    const OK: &str = "graph {\n  node a:\"A\"\n}";

    #[test]
    fn dsl_envelope_mirrors_check_and_fmt() {
        for src in [BROKEN, OK] {
            let r = check(src);
            let v = validate(src, ValidateOpts::default());
            assert_eq!(v.schema_version, 1);
            assert_eq!(v.valid, r.valid);
            assert_eq!(v.diagnostics, r.diagnostics);
            assert_eq!(v.stats, r.stats);
            assert_eq!(v.normalized_source, fmt(src));
        }
        assert!(validate(BROKEN, ValidateOpts::default()).diagnostics.len() >= 2);
    }

    #[test]
    fn envelope_always_serializes_normalized_source() {
        let j = serde_json::to_value(validate(BROKEN, ValidateOpts::default())).unwrap();
        assert!(j.as_object().unwrap().contains_key("normalized_source"));
        assert!(j["normalized_source"].is_null());
    }
}
