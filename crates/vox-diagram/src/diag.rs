//! Diagnostics, machine-applicable fixes and the `check` report.

use serde::Serialize;

use crate::codes;
use crate::ir::Ir;
use crate::lexer::is_forbidden;
use crate::limits::MAX_DIAGNOSTICS;
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

impl Diagnostic {
    pub(crate) fn warning(code: &'static str, message: String, span: Location) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(code, message, span)
        }
    }
}

/// First `MAX_CLIP` chars of user text echoed in a message (T-83-11).
const MAX_CLIP: usize = 40;

/// At most [`MAX_CLIP`] chars of `s`, with `...` appended when cut. Control characters, bidi
/// marks and line breaks are shown as `\u{..}` escapes, so echoed user text cannot drive a
/// terminal or forge a log line.
pub(crate) fn clip(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().take(MAX_CLIP) {
        if is_forbidden(c) || matches!(c, '\n' | '\r' | '\t') {
            out.extend(c.escape_unicode());
        } else {
            out.push(c);
        }
    }
    if s.chars().nth(MAX_CLIP).is_some() {
        out.push_str("...");
    }
    out
}

/// The single funnel every lexer, parser and resolver diagnostic passes through. `finish`
/// sorts by `(span.start, code)`, drops exact duplicates and caps the list at
/// `MAX_DIAGNOSTICS`: 99 real diagnostics plus a `limit-diagnostics` warning.
#[derive(Default)]
pub(crate) struct Collector {
    items: Vec<Diagnostic>,
    overflow: bool,
    /// Parser fuel ran out: the AST is truncated, so the report can never be valid.
    halted: bool,
    has_error: bool,
}

impl Collector {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// True once no further diagnostic can be stored.
    pub(crate) fn is_full(&self) -> bool {
        self.items.len() >= MAX_DIAGNOSTICS - 1
    }

    /// Store `d`; returns whether the cap is now reached. A dropped `Error` still counts
    /// toward [`Self::has_error`], so a flood of warnings can never make a report look valid.
    // ponytail: first 99 arrivals win (lexer, then parser, then resolver), not first 99 by
    // position; sort afterwards. Upgrade to a bounded heap if position-fairness matters.
    pub(crate) fn push(&mut self, d: Diagnostic) -> bool {
        self.has_error |= d.severity == Severity::Error;
        if self.is_full() {
            self.overflow = true;
        } else {
            self.items.push(d);
        }
        self.is_full()
    }

    pub(crate) fn extend(&mut self, ds: impl IntoIterator<Item = Diagnostic>) {
        for d in ds {
            self.push(d);
        }
    }

    /// Record that the parser halted (fuel ran out) without storing a diagnostic. The AST is
    /// truncated, so this counts as an error: a partial document must never look valid.
    pub(crate) fn trip(&mut self) {
        self.overflow = true;
        self.halted = true;
        self.has_error = true;
    }

    pub(crate) fn has_error(&self) -> bool {
        self.has_error
    }

    /// Sorted, deduplicated list; `eof` locates the `limit-diagnostics` sentinel.
    pub(crate) fn finish(self, eof: Location) -> Vec<Diagnostic> {
        let mut v = self.items;
        v.sort_by_key(|d| (d.span.start, d.code, d.span.end));
        v.dedup_by(|a, b| a.code == b.code && a.span == b.span);
        if self.overflow {
            let msg = if self.halted {
                "input too complex to check in full; simplify it and check again".to_string()
            } else {
                format!(
                    "more than {} diagnostics; fix the ones above and check again",
                    MAX_DIAGNOSTICS - 1
                )
            };
            v.push(Diagnostic::warning(codes::LIMIT_DIAGNOSTICS, msg, eof));
        }
        v
    }
}

/// Size counters of a checked document. For a `sequence`, `nodes` counts actors, `edges` counts
/// events (messages plus notes) and `containers` and `max_depth` are 0.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    /// Declared nodes (actors in a sequence).
    pub nodes: u32,
    /// Declared containers (always 0 in a sequence).
    pub containers: u32,
    /// Edge statements (messages plus notes in a sequence).
    pub edges: u32,
    /// Deepest container nesting (always 0 in a sequence).
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

    fn at(start: u32) -> Location {
        Location {
            start,
            end: start + 1,
            line: 1,
            col: start + 1,
            end_line: 1,
            end_col: start + 2,
        }
    }

    fn err(code: &'static str, start: u32) -> Diagnostic {
        Diagnostic::error(code, "m".into(), at(start))
    }

    #[test]
    fn finish_sorts_by_start_then_code() {
        let mut c = Collector::new();
        c.push(err("diagram/b", 9));
        c.push(err("diagram/z", 2));
        c.push(err("diagram/a", 2));
        let codes: Vec<_> = c.finish(at(99)).iter().map(|d| d.code).collect();
        assert_eq!(codes, ["diagram/a", "diagram/z", "diagram/b"]);
    }

    #[test]
    fn finish_removes_exact_duplicates_only() {
        let mut c = Collector::new();
        c.push(err("diagram/a", 2));
        c.push(err("diagram/a", 2));
        c.push(err("diagram/a", 3));
        c.push(err("diagram/b", 2));
        assert_eq!(c.finish(at(99)).len(), 3);
    }

    #[test]
    fn cap_keeps_99_then_one_limit_warning() {
        let mut c = Collector::new();
        for i in 0..150 {
            c.push(err("diagram/e", i));
        }
        let out = c.finish(at(500));
        assert_eq!(out.len(), MAX_DIAGNOSTICS);
        let last = &out[out.len() - 1];
        assert_eq!(last.code, codes::LIMIT_DIAGNOSTICS);
        assert_eq!(last.severity, Severity::Warning);
        assert!(out[..99].iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn exactly_99_has_no_sentinel_and_dropped_errors_still_count() {
        let mut c = Collector::new();
        for i in 0..99 {
            c.push(Diagnostic::warning("diagram/w", "m".into(), at(i)));
        }
        assert!(c.is_full() && !c.has_error());
        c.push(err("diagram/e", 200));
        assert!(
            c.has_error(),
            "a dropped Error must still invalidate the report"
        );
        assert_eq!(c.finish(at(500)).len(), MAX_DIAGNOSTICS);
    }

    #[test]
    fn halted_parser_invalidates_the_report() {
        let mut c = Collector::new();
        c.trip();
        assert!(c.has_error(), "a truncated AST must never look valid");
        let out = c.finish(at(5));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].code, codes::LIMIT_DIAGNOSTICS);
        assert!(out[0].message.contains("too complex"), "{}", out[0].message);
    }

    #[test]
    fn clip_cuts_echoed_text_at_40_chars() {
        assert_eq!(clip("short"), "short");
        assert_eq!(clip(&"x".repeat(40)).chars().count(), 40);
        assert_eq!(clip(&"x".repeat(41)), format!("{}...", "x".repeat(40)));
    }
}
