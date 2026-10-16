//! Diagnostics, machine-applicable fixes and the `check` report.

use serde::Serialize;

use crate::ir::Ir;
use crate::span::{Location, Span};

/// How serious a diagnostic is. `valid` means "no `Error`".
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The document is invalid.
    Error,
    /// Advisory; never blocks validity and never carries fixes.
    Warning,
}

/// One text edit; all fixes of a diagnostic form a single atomic remedy.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    /// Byte range to replace.
    pub span: Span,
    /// Replacement text.
    pub replacement: String,
}

/// A secondary location that explains a diagnostic.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Related {
    /// What the location is.
    pub message: String,
    /// Where it is.
    pub span: Location,
}

/// A stable, span-precise problem report.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable `diagram/*` code (see [`crate::codes`]).
    pub code: &'static str,
    /// Error or warning.
    pub severity: Severity,
    /// Human-readable description.
    pub message: String,
    /// Where the problem is.
    pub span: Location,
    /// Optional remediation hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Machine-applicable edits (empty when none is safe).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fixes: Vec<Fix>,
    /// Secondary locations.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<Related>,
}

impl Diagnostic {
    pub(crate) fn error(code: &'static str, message: String, span: Location) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message,
            span,
            help: None,
            fixes: Vec::new(),
            related: Vec::new(),
        }
    }
}

/// Size counters of a checked document.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    /// Declared nodes.
    pub nodes: u32,
    /// Declared containers.
    pub containers: u32,
    /// Edge statements.
    pub edges: u32,
    /// Deepest container nesting.
    pub max_depth: u32,
}

/// Result of [`crate::check`].
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    /// Report format version; always 1.
    pub schema_version: u32,
    /// True when no `Error` diagnostic exists.
    pub valid: bool,
    /// All diagnostics, in source order.
    pub diagnostics: Vec<Diagnostic>,
    /// Document counters.
    pub stats: Stats,
    /// Resolved IR; present only when `valid`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ir: Option<Ir>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_optional_fields_are_not_serialized() {
        let d = Diagnostic::error("diagram/x", "m".into(), Location::default());
        let json = serde_json::to_string(&d).unwrap_or_default();
        assert!(!json.contains("help") && !json.contains("fixes") && !json.contains("related"));
        assert!(json.contains("\"severity\":\"error\""));
    }
}
