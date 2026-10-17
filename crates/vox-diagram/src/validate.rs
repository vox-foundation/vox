//! The single validation envelope shared by the `vox_validate_diagram` MCP tool and
//! `vox diagram --json` (D-01, D-04): check, canonical form, optional Mermaid import and
//! SVG/Mermaid emit in one pure, deterministic call.

use serde::{Deserialize, Serialize};

use crate::diag::{Diagnostic, Stats};
use crate::mermaid::{self, ImportOpts, Unsupported};
use crate::svg::{self, SvgOpts};
use crate::{check, fmt};

/// What [`validate`] should additionally render from a valid diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Emit {
    /// Deterministic SVG via [`svg::render`].
    Svg,
    /// Mermaid text via [`mermaid::export`].
    Mermaid,
}

/// Options of [`validate`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct ValidateOpts {
    /// Treat `source` as one Mermaid block and validate its conversion.
    #[serde(default)]
    pub from_mermaid: bool,
    /// Also render the valid diagram.
    #[serde(default)]
    pub emit: Option<Emit>,
}

/// The validation envelope; its JSON shape is the public contract of the MCP tool and the CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Validation {
    /// Envelope format version; always 1.
    pub schema_version: u32,
    /// True when no `Error` diagnostic exists (and, for Mermaid input, the import was complete).
    pub valid: bool,
    /// Every diagnostic, in source order, with machine-applicable fixes.
    pub diagnostics: Vec<Diagnostic>,
    /// Canonical `fmt` text once valid; always serialized, `null` while errors remain.
    pub normalized_source: Option<String>,
    /// Document counters.
    pub stats: Stats,
    /// The requested rendering of a valid diagram.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emitted: Option<String>,
    /// Why a requested SVG could not be rendered (never silently dropped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emit_error: Option<String>,
    /// Mermaid lines the importer refused; `Some` exactly when `from_mermaid`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsupported: Option<Vec<Unsupported>>,
    /// Mermaid lines deliberately dropped by the importer.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dropped: Vec<Unsupported>,
}

/// Validate `source`: pure, deterministic, never panics.
pub fn validate(source: &str, opts: ValidateOpts) -> Validation {
    let (dsl, unsupported, dropped) = if opts.from_mermaid {
        let rep = mermaid::import(source, ImportOpts::default());
        (rep.source, Some(rep.unsupported), rep.dropped)
    } else {
        (Some(source.to_owned()), None, Vec::new())
    };
    let mut v = Validation {
        schema_version: 1,
        valid: false,
        diagnostics: Vec::new(),
        normalized_source: None,
        stats: Stats::default(),
        emitted: None,
        emit_error: None,
        unsupported,
        dropped,
    };
    let Some(dsl) = dsl else { return v };
    let r = check(&dsl);
    v.valid = r.valid;
    v.diagnostics = r.diagnostics;
    v.stats = r.stats;
    if let (true, Some(ir)) = (r.valid, r.ir) {
        v.normalized_source = fmt(&dsl);
        match opts.emit {
            Some(Emit::Svg) => match svg::render(&ir, &SvgOpts::default()) {
                Ok(s) => v.emitted = Some(s),
                Err(e) => v.emit_error = Some(e.to_string()),
            },
            Some(Emit::Mermaid) => v.emitted = Some(mermaid::export(&ir)),
            None => {}
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    const BROKEN: &str = "graph {\n    node a: \"A\"\n    a -> b\n    node c: C\n}\n";
    const OK: &str = "graph {\n  node a:\"A\"\n}";
    const SEQ: &str = "sequence {\n    actor a: \"A\"\n    actor b: \"B\"\n    a -> b: \"hi\"\n}\n";

    const FLOW: &str = "flowchart TD\n    a --> b\n";

    fn opts(from_mermaid: bool, emit: Option<Emit>) -> ValidateOpts {
        ValidateOpts { from_mermaid, emit }
    }

    #[test]
    fn from_mermaid_success_is_the_dsl_path_over_the_conversion() {
        let rep = mermaid::import(FLOW, ImportOpts::default());
        let dsl = rep.source.expect("flowchart imports");
        let v = validate(FLOW, opts(true, None));
        assert_eq!(v.unsupported, Some(vec![]));
        assert_eq!(v.normalized_source.as_deref(), Some(dsl.as_str()));
        assert!(v.valid);
        assert_eq!(v.stats, check(&dsl).stats);
    }

    #[test]
    fn from_mermaid_failure_lists_lines() {
        let v = validate(
            "flowchart TD\n    a --> b\n    classDef x fill:#f00\n",
            opts(true, None),
        );
        assert!(!v.valid && v.normalized_source.is_none() && v.diagnostics.is_empty());
        assert_eq!(v.stats, Stats::default());
        let u = v.unsupported.expect("from_mermaid reports unsupported");
        assert!(u.iter().any(|x| x.line == 3), "{u:?}");
        let g = validate("gantt\n    title X\n", opts(true, None));
        assert!(!g.valid && g.unsupported.is_some_and(|u| !u.is_empty()));
    }

    #[test]
    fn emit_matches_the_renderers() {
        for src in [OK, SEQ] {
            let ir = check(src).ir.expect("valid");
            let svg = validate(src, opts(false, Some(Emit::Svg)));
            assert_eq!(svg.emitted, svg::render(&ir, &SvgOpts::default()).ok());
            let m = validate(src, opts(false, Some(Emit::Mermaid)));
            assert_eq!(m.emitted, Some(mermaid::export(&ir)));
            assert!(svg.emit_error.is_none() && m.emit_error.is_none());
        }
    }

    #[test]
    fn no_emit_on_invalid_input() {
        let v = validate(BROKEN, opts(false, Some(Emit::Svg)));
        assert!(v.emitted.is_none() && v.emit_error.is_none());
    }

    #[test]
    fn render_failure_is_emit_error() {
        let mut src = String::from("graph {\n");
        for i in 0..=crate::limits::MAX_RENDER_NODES {
            src.push_str(&format!("    node n{i}: \"N\"\n"));
        }
        src.push_str("}\n");
        let v = validate(&src, opts(false, Some(Emit::Svg)));
        assert!(v.valid && v.emitted.is_none());
        assert!(v.emit_error.is_some_and(|e| !e.is_empty()));
    }

    #[test]
    fn deterministic() {
        let o = opts(false, Some(Emit::Svg));
        assert_eq!(validate(OK, o), validate(OK, o));
    }

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
