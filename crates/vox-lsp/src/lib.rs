//! Language Server Protocol integration for Vox.
//!
//! Shared validation helpers used by the LSP binary and MCP / orchestrator quality gates.

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};
use vox_compiler::ast::decl::Decl;
use vox_compiler::ast::expr::Expr;
use vox_compiler::ast::stmt::Stmt;
use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::Diagnostic as TypeckDiagnostic;
use vox_compiler::typeck::diagnostics::TypeckSeverity;
use vox_compiler::typeck::typecheck_ast_module;

pub mod bounded_fs;
pub mod completions;
pub mod grammar;
pub mod symbols;

/// Convert UTF-8 byte index to LSP line / column (character count per line, not UTF-16 code units).
pub fn byte_index_to_line_col(text: &str, index: usize) -> (u32, u32) {
    vox_compiler::ast::span::byte_offset_to_line_col_zero_based(text, index)
}

fn typeck_diagnostic_to_lsp(text: &str, err: TypeckDiagnostic) -> Diagnostic {
    let (sl, sc) = byte_index_to_line_col(text, err.span.start);
    let (el, ec) = byte_index_to_line_col(text, err.span.end);
    let start = Position {
        line: sl,
        character: sc,
    };
    let end = Position {
        line: el,
        character: ec,
    };
    Diagnostic {
        range: Range { start, end },
        severity: Some(match err.severity {
            TypeckSeverity::Error => DiagnosticSeverity::ERROR,
            TypeckSeverity::Warning => DiagnosticSeverity::WARNING,
        }),
        code: err.code.clone().map(NumberOrString::String),
        code_description: None,
        source: Some("vox-lsp".to_string()),
        message: err.message,
        related_information: None,
        tags: None,
        data: None,
    }
}

fn validate_document_impl(text: &str, include_hir: bool) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tokens = lex(text);
    match parse(tokens) {
        Ok(module) => {
            diagnostics.extend(mesh_workflow_env_warnings(text, &module));
            let mut type_errors = typecheck_ast_module(text, &module);
            if include_hir {
                let hir = vox_compiler::hir::lower_module(&module);
                for e in vox_compiler::hir::validate_module(&hir) {
                    type_errors.push(TypeckDiagnostic::hir_invariant(e.message, e.span, text));
                }
            }
            diagnostics.extend(
                type_errors
                    .into_iter()
                    .map(|err| typeck_diagnostic_to_lsp(text, err)),
            );
        }
        Err(parse_errors) => {
            for err in parse_errors {
                let (sl, sc) = byte_index_to_line_col(text, err.span.start);
                let (el, ec) = byte_index_to_line_col(text, err.span.end);
                let start = Position {
                    line: sl,
                    character: sc,
                };
                let end = Position {
                    line: el,
                    character: ec,
                };
                diagnostics.push(Diagnostic {
                    range: Range { start, end },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    message: err.to_string(),
                    source: Some("vox-lsp".to_string()),
                    ..Default::default()
                });
            }
        }
    }
    diagnostics
}

/// Run lexer → parser → typecheck and return LSP diagnostics (no side effects).
///
/// Note: This does **not** run HIR structural validation. For parity with the CLI
/// frontend (`run_frontend_str`), use [`validate_document_with_hir`].
pub fn validate_document(text: &str) -> Vec<Diagnostic> {
    validate_document_impl(text, false)
}

/// Full frontend validation: lexer → parser → typecheck → HIR lower → HIR validate.
///
/// Matches the diagnostic shape produced by `vox-cli` `run_frontend_str` for type/HIR issues
/// (parse errors are always surfaced as errors).
#[must_use]
pub fn validate_document_with_hir(text: &str) -> Vec<Diagnostic> {
    validate_document_impl(text, true)
}

fn vox_populi_enabled_from_env() -> bool {
    std::env::var("VOX_MESH_ENABLED")
        .map(|v| {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true")
        })
        .unwrap_or(false)
}

/// When `VOX_MESH_ENABLED` is unset/false, warn on `mesh_*` activity calls inside `workflow` bodies.
fn mesh_workflow_env_warnings(
    text: &str,
    module: &vox_compiler::ast::decl::Module,
) -> Vec<Diagnostic> {
    if vox_populi_enabled_from_env() {
        return Vec::new();
    }
    let mut spans = Vec::new();
    for d in &module.declarations {
        if let Decl::Workflow(w) = d {
            collect_mesh_activity_spans_from_stmts(&w.body, &mut spans);
        }
    }
    spans
        .into_iter()
        .map(|span| {
            let (sl, sc) = byte_index_to_line_col(text, span.start);
            let (el, ec) = byte_index_to_line_col(text, span.end);
            Diagnostic {
                range: Range {
                    start: Position {
                        line: sl,
                        character: sc,
                    },
                    end: Position {
                        line: el,
                        character: ec,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                code: None,
                code_description: None,
                source: Some("vox-lsp".to_string()),
                message: "Mens activity call: enable VOX_MESH_ENABLED=1 (and mens control-plane env) at runtime for mens hooks.".to_string(),
                related_information: None,
                tags: None,
                data: None,
            }
        })
        .collect()
}

fn collect_mesh_activity_spans_from_stmts(stmts: &[Stmt], out: &mut Vec<vox_compiler::ast::Span>) {
    for s in stmts {
        match s {
            Stmt::Let { value, .. }
            | Stmt::Assign { value, .. }
            | Stmt::Expr { expr: value, .. } => {
                collect_mesh_activity_spans_from_expr(value, out);
            }
            Stmt::Return {
                value: Some(value), ..
            } => collect_mesh_activity_spans_from_expr(value, out),
            Stmt::Return { value: None, .. } => {}
        }
    }
}

fn collect_mesh_activity_spans_from_expr(expr: &Expr, out: &mut Vec<vox_compiler::ast::Span>) {
    match expr {
        Expr::With { operand, .. } => collect_mesh_activity_spans_from_expr(operand, out),
        Expr::Call { callee, .. } => {
            if let Expr::Ident { name, span } = callee.as_ref() {
                if name.starts_with("mesh_") {
                    out.push(*span);
                }
            } else {
                collect_mesh_activity_spans_from_expr(callee, out);
            }
        }
        Expr::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            collect_mesh_activity_spans_from_expr(condition, out);
            collect_mesh_activity_spans_from_stmts(then_body, out);
            if let Some(e) = else_body {
                collect_mesh_activity_spans_from_stmts(e, out);
            }
        }
        Expr::Block { stmts, .. } => collect_mesh_activity_spans_from_stmts(stmts, out),
        Expr::Binary { left, right, .. } => {
            collect_mesh_activity_spans_from_expr(left, out);
            collect_mesh_activity_spans_from_expr(right, out);
        }
        Expr::Unary { operand, .. } => collect_mesh_activity_spans_from_expr(operand, out),
        Expr::Match { subject, arms, .. } => {
            collect_mesh_activity_spans_from_expr(subject, out);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    collect_mesh_activity_spans_from_expr(g, out);
                }
                collect_mesh_activity_spans_from_expr(arm.body.as_ref(), out);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            collect_mesh_activity_spans_from_expr(object, out);
            for a in args {
                collect_mesh_activity_spans_from_expr(&a.value, out);
            }
        }
        Expr::FieldAccess { object, .. } => collect_mesh_activity_spans_from_expr(object, out),
        Expr::Pipe { left, right, .. } => {
            collect_mesh_activity_spans_from_expr(left, out);
            collect_mesh_activity_spans_from_expr(right, out);
        }
        Expr::Lambda { body, .. } => collect_mesh_activity_spans_from_expr(body, out),
        Expr::For { iterable, body, .. } => {
            collect_mesh_activity_spans_from_expr(iterable, out);
            collect_mesh_activity_spans_from_expr(body, out);
        }
        Expr::Spawn { target, .. } => collect_mesh_activity_spans_from_expr(target, out),
        Expr::ListLit { elements, .. } => {
            for el in elements {
                collect_mesh_activity_spans_from_expr(el, out);
            }
        }
        Expr::ObjectLit { fields, .. } => {
            for (_, v) in fields {
                collect_mesh_activity_spans_from_expr(v, out);
            }
        }
        Expr::TupleLit { elements, .. } => {
            for el in elements {
                collect_mesh_activity_spans_from_expr(el, out);
            }
        }
        Expr::StringInterp { parts, .. } => {
            for p in parts {
                if let vox_compiler::ast::expr::StringPart::Interpolation(inner) = p {
                    collect_mesh_activity_spans_from_expr(inner, out);
                }
            }
        }
        _ => {}
    }
}

/// Identifier-like character (ASCII alnum + underscore).
#[inline]
fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Word at LSP `(line, character)` using **0-based** lines and **character = column in Unicode scalars** (same convention as [`byte_index_to_line_col`]).
#[must_use]
pub fn word_at_position(text: &str, line: u32, character: u32) -> Option<String> {
    let line_str = text.lines().nth(line as usize)?;
    let col = character as usize;
    let chars: Vec<char> = line_str.chars().collect();
    if col >= chars.len() {
        return None;
    }
    if !is_ident_char(chars[col]) {
        return None;
    }
    let mut start = col;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col + 1;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }
    Some(chars[start..end].iter().collect())
}

/// True if `line` contains a `Speech.transcribe` / `Speech . transcribe` call (word boundaries).
#[must_use]
pub fn line_has_speech_transcribe(line: &str) -> bool {
    for (idx, _) in line.match_indices("Speech") {
        if idx > 0
            && let Some(prev) = line[..idx].chars().next_back()
            && (prev.is_ascii_alphanumeric() || prev == '_')
        {
            continue;
        }
        let after_speech = &line[idx + "Speech".len()..];
        let after_speech = after_speech.trim_start();
        let Some(after_dot) = after_speech.strip_prefix('.') else {
            continue;
        };
        let after_dot = after_dot.trim_start();
        let Some(rest) = after_dot.strip_prefix("transcribe") else {
            continue;
        };
        let boundary_ok = match rest.chars().next() {
            None => true,
            Some('(') => true,
            Some(c) if c.is_whitespace() => true,
            Some(c) => !is_ident_char(c),
        };
        if boundary_ok {
            return true;
        }
    }
    false
}

/// Markdown hover using the **current line** so `transcribe` only hovers in `Speech.transcribe` context.
#[must_use]
pub fn builtin_hover_markdown_in_line(line: &str, word: &str) -> Option<String> {
    match word {
        "transcribe" => {
            if line_has_speech_transcribe(line) {
                builtin_hover_markdown("transcribe")
            } else {
                None
            }
        }
        _ => builtin_hover_markdown(word),
    }
}

/// Short markdown hovers for selected builtins (Oratio, HTTP, stdlib).
#[must_use]
pub fn builtin_hover_markdown(word: &str) -> Option<String> {
    match word {
        "Speech" => Some(
            "**Speech** — Oratio (Candle Whisper, pure Rust). \
             `Speech.transcribe(path: str) → Result[str]` returns **refined** transcript text. \
             Rust codegen uses `vox-oratio`; TS/browser cannot call it directly."
                .to_string(),
        ),
        "transcribe" => Some(
            "**transcribe** — On `Speech`, runs STT via **Vox Oratio** (HF Candle). \
             Server/Rust only; in TS use `@server` or `POST /api/audio/transcribe` — see repo **`examples/oratio/codexAudioTranscribe.ts`**."
                .to_string(),
        ),
        "HTTP" => Some(
            "**HTTP** — builtin client module: `get` / `post` / `put` / `delete` (paths return `Result[Response]`)."
                .to_string(),
        ),
        "print" => Some("**print** — `print(value: str) → Unit`.".to_string()),
        "len" => Some("**len** — length of a collection.".to_string()),
        _ => None,
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn word_at_position_finds_ident() {
        let t = "fn demo():\n    Speech.transcribe(p)\n";
        assert_eq!(word_at_position(t, 1, 4).as_deref(), Some("Speech"));
    }

    #[test]
    fn line_has_speech_transcribe_detects_call() {
        assert!(line_has_speech_transcribe("    Speech.transcribe(path)"));
        assert!(line_has_speech_transcribe("Speech . transcribe (x)"));
        assert!(!line_has_speech_transcribe("FooSpeech.transcribe(x)"));
        assert!(!line_has_speech_transcribe("let transcribe = 1"));
    }

    #[test]
    fn hover_transcribe_only_with_speech_receiver() {
        let line = "    Speech.transcribe(p)";
        assert!(builtin_hover_markdown_in_line(line, "transcribe").is_some());
        assert!(builtin_hover_markdown_in_line("other.transcribe(p)", "transcribe").is_none());
    }

    // Mutex guard for environment mutations in tests that set VOX_MESH_ENABLED.
    static MESH_WARNING_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn mesh_activity_warning_test() {
        let doc = "workflow w() { mesh_snapshot() }";
        let _lock = MESH_WARNING_ENV_LOCK
            .lock()
            .expect("mens env mutex poisoned");

        // When mens is DISABLED, we expect a WARNING diagnostic.
        // SAFETY: single-threaded under the mutex guard above.
        unsafe {
            std::env::set_var("VOX_MESH_ENABLED", "0");
        }
        let diags = validate_document(doc);
        assert!(
            diags
                .iter()
                .any(|d| d.severity == Some(DiagnosticSeverity::WARNING)
                    && d.message.contains("Mens activity call")),
            "Expected mens call warning when VOX_MESH_ENABLED=0, got: {:?}",
            diags
        );

        // When mens is ENABLED, no mens WARNING should fire.
        // SAFETY: single-threaded under the mutex guard above.
        unsafe {
            std::env::set_var("VOX_MESH_ENABLED", "1");
        }
        let diags = validate_document(doc);
        assert!(
            !diags
                .iter()
                .any(|d| d.severity == Some(DiagnosticSeverity::WARNING)
                    && d.message.contains("Mens activity call")),
            "Expected no mens call warning when VOX_MESH_ENABLED=1, got: {:?}",
            diags
        );

        // SAFETY: restore to neutral state.
        unsafe {
            std::env::remove_var("VOX_MESH_ENABLED");
        }
    }
}
