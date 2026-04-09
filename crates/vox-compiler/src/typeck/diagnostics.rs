use crate::ast::span::Span;

/// Function / call arity mismatch (SSOT message for Checker + check).
#[must_use]
pub fn msg_arg_count_mismatch(expected: usize, found: usize) -> String {
    format!("Argument count mismatch: expected {expected} arguments, found {found}")
}

/// Tuple arity mismatch (SSOT for Checker + unification).
#[must_use]
pub fn msg_tuple_size_mismatch(expected: usize, found: usize) -> String {
    format!("Tuple size mismatch: expected {expected}, found {found}")
}

/// Function type arity mismatch during unification.
#[must_use]
pub fn msg_function_arity_mismatch(expected: usize, found: usize) -> String {
    format!("Function arity mismatch: expected {expected}, found {found}")
}

/// Record field-count mismatch during unification.
#[must_use]
pub fn msg_record_size_mismatch(expected: usize, found: usize) -> String {
    format!("Record size mismatch: expected {expected}, found {found}")
}

/// Type checking diagnostic severity (distinct from lint / TOESTUB severities).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum TypeckSeverity {
    Error,
    Warning,
}

/// Machine-applicable edit (LSP / MCP repair loops).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticFix {
    pub label: String,
    pub span: Span,
    pub replacement: String,
}

/// Which compiler / pipeline stage produced a diagnostic (taxonomy for tooling and docs).
///
/// See `docs/src/reference/diagnostic-taxonomy.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCategory {
    /// Surface parse failures (typically surfaced before HIR).
    Parse,
    /// AST → HIR lowering or IR-shape issues not covered by type rules.
    Lowering,
    /// Principal type checker / inference (default for historical diagnostics).
    #[default]
    Typecheck,
    /// Structural HIR invariants ([`crate::hir::validate::validate_module`]).
    HirInvariant,
    /// Host / runtime contracts (embed checks, deploy guards).
    RuntimeContract,
    /// Optional lints and style rules.
    Lint,
}

/// Line/column enrichment added on demand by machine consumers (LSP, healing loop).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LineCol {
    pub line_start: usize,
    pub col_start: usize,
    pub line_end: usize,
    pub col_end: usize,
}

/// A structured diagnostic emitted by the type checker and related frontend passes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Diagnostic {
    pub severity: TypeckSeverity,
    pub message: String,
    pub span: Span,
    pub expected_type: Option<String>,
    pub found_type: Option<String>,
    /// Optional source snippet for autofix / IDE.
    pub context: Option<String>,
    pub suggestions: Vec<String>,
    /// Origin category for filtering, metrics, and LSP `code` mapping.
    #[serde(default)]
    pub category: DiagnosticCategory,
    /// Stable code for stall detection and speech-to-code traces (`typecheck.reactive.state`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Optional structured fixes (additive; consumers ignore if unsupported).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixes: Vec<DiagnosticFix>,
    /// Line/column info enriched from source (optional, computed on demand).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_col: Option<LineCol>,
}

impl Diagnostic {
    /// Enrich this diagnostic with line/column data computed from `source`.
    ///
    /// Call on the way out of the compiler pipeline when a machine consumer
    /// (healing loop, LSP, `vox check --json`) needs precise cursor locations.
    #[must_use]
    pub fn with_line_col(mut self, source: &str) -> Self {
        let compute = |byte_offset: usize| -> (usize, usize) {
            let mut line = 1usize;
            let mut col = 1usize;
            for (i, ch) in source.char_indices() {
                if i == byte_offset {
                    break;
                }
                if ch == '\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
            }
            (line, col)
        };
        let (line_start, col_start) = compute(self.span.start);
        let (line_end, col_end) = compute(self.span.end.min(source.len().saturating_sub(1)));
        self.line_col = Some(LineCol {
            line_start,
            col_start,
            line_end,
            col_end,
        });
        self
    }
    /// Build a simple error diagnostic (no type diff).
    #[must_use]
    pub fn error(message: String, span: Span, source: &str) -> Self {
        Self {
            severity: TypeckSeverity::Error,
            message,
            span,
            expected_type: None,
            found_type: None,
            context: Some(Self::capture_context(source, span)),
            suggestions: vec![],
            category: DiagnosticCategory::Typecheck,
            code: None,
            fixes: vec![],
            line_col: None,
        }
    }

    /// Build a simple warning diagnostic (no type diff).
    #[must_use]
    pub fn warning(message: String, span: Span, source: &str) -> Self {
        Self {
            severity: TypeckSeverity::Warning,
            message,
            span,
            expected_type: None,
            found_type: None,
            context: Some(Self::capture_context(source, span)),
            suggestions: vec![],
            category: DiagnosticCategory::Typecheck,
            code: None,
            fixes: vec![],
            line_col: None,
        }
    }

    /// HIR structural invariant violation (after lowering).
    #[must_use]
    pub fn hir_invariant(
        message: String,
        span: Span,
        source: &str,
        correction_hint: Option<String>,
    ) -> Self {
        Self {
            severity: TypeckSeverity::Error,
            message,
            span,
            expected_type: None,
            found_type: None,
            context: Some(Self::capture_context(source, span)),
            suggestions: correction_hint.into_iter().collect(),
            category: DiagnosticCategory::HirInvariant,
            code: None,
            fixes: vec![],
            line_col: None,
        }
    }

    /// AST -> HIR lowering diagnostic surfaced through structured diagnostics.
    #[must_use]
    pub fn lowering(message: String, span: Span, source: &str) -> Self {
        Self {
            severity: TypeckSeverity::Error,
            message,
            span,
            expected_type: None,
            found_type: None,
            context: Some(Self::capture_context(source, span)),
            suggestions: vec![],
            category: DiagnosticCategory::Lowering,
            code: None,
            fixes: vec![],
            line_col: None,
        }
    }

    /// Runtime/embedding contract diagnostic surfaced through structured diagnostics.
    #[must_use]
    pub fn runtime_contract(message: String, span: Span, source: &str) -> Self {
        Self {
            severity: TypeckSeverity::Error,
            message,
            span,
            expected_type: None,
            found_type: None,
            context: Some(Self::capture_context(source, span)),
            suggestions: vec![],
            category: DiagnosticCategory::RuntimeContract,
            code: None,
            fixes: vec![],
            line_col: None,
        }
    }

    /// Extract a few lines around `span` for display.
    #[must_use]
    pub fn capture_context(source: &str, span: Span) -> String {
        let lines: Vec<&str> = source.lines().collect();
        if lines.is_empty() {
            return String::new();
        }
        let mut offset = 0usize;
        let mut start_line = 0usize;
        for (i, line) in lines.iter().enumerate() {
            let next = offset + line.len() + 1;
            if span.start >= offset && span.start < next {
                start_line = i;
                break;
            }
            offset = next;
        }
        let from = start_line.saturating_sub(1);
        let to = (start_line + 2).min(lines.len());
        lines[from..to].join("\n")
    }
}
