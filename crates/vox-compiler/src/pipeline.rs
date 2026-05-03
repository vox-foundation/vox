//! Unified compiler pipeline orchestrator.
//!
//! Provides a single entry point (`run_frontend`) that runs the full
//! lex → parse → typecheck → HIR validation pass and returns structured
//! results.

use crate::ast::decl::Module;
use crate::hir::HirModule;
use crate::hir::lower::LowerConfig;
use crate::typeck::Diagnostic;
use crate::typeck::diagnostics::{DiagnosticCategory, TypeckSeverity, VoxCompilerDiagnosticPayload};
use anyhow::Result;

/// ADR-028: emit an error diagnostic for each reserved durability keyword found in `source`.
///
/// `@scheduled`, `@durable`, `workflow`, and `activity` have been removed from the public
/// grammar.  They are detected at the source-text level (before full parsing) so that the
/// error is reported even when the token stream is otherwise broken.
fn check_adr028_reserved_keywords(source: &str) -> Vec<Diagnostic> {
    // (pattern, keyword_label, error_code, identifier_boundary)
    // `identifier_boundary` is true when the pattern is a bare keyword that could appear inside
    // a longer identifier (e.g. `workflow_handle`); for those we additionally require the byte
    // immediately after the match to NOT continue the identifier (alpha/digit/underscore).
    // Decorator forms like `@scheduled` use `@` as a leading sentinel and don't need it.
    const RESERVED: &[(&str, &str, &str, bool)] = &[
        ("@scheduled", "@scheduled", "E028", false),
        ("@durable",   "@durable",   "E028", false),
        ("workflow",   "workflow",   "E028", true),
        ("activity",   "activity",   "E028", true),
    ];

    let mut diags = Vec::new();
    for (pattern, label, code, ident_boundary) in RESERVED {
        let Some(offset) = find_keyword_outside_comments_and_strings(source, pattern, *ident_boundary) else {
            continue;
        };
        diags.push(Diagnostic {
                severity: TypeckSeverity::Error,
                message: format!(
                    "{} is not yet implemented and has been reserved for a future release (ADR-028). \
                     Remove this declaration or replace it with a plain `fn`.",
                    label
                ),
                span: crate::ast::span::Span::new(offset, offset + pattern.len()),
                expected_type: None,
                found_type: None,
                context: None,
                suggestions: vec![
                    format!("Replace `{}` with a plain `fn` declaration.", label),
                ],
                category: DiagnosticCategory::Parse,
                code: Some(code.to_string()),
                fixes: vec![],
                line_col: None,
                missing_cases: vec![],
                ast_node_kind: None,
            });
    }
    diags
}

/// Find the first occurrence of `pattern` in `source` that is NOT inside a `//` line comment,
/// `/* */` block comment, or a `"…"` string literal. Returns the byte offset of the match.
///
/// Needed because ADR-028's reserved-keyword scan runs at the source-text level (before parsing)
/// and would otherwise flag the word "workflow" appearing in a doc comment as a real declaration.
fn find_outside_comments_and_strings(source: &str, pattern: &str) -> Option<usize> {
    find_keyword_outside_comments_and_strings(source, pattern, false)
}

/// Same as `find_outside_comments_and_strings` but with optional identifier-boundary enforcement
/// for bare keyword matches. When `ident_boundary` is true, the byte immediately following the
/// match must NOT be an identifier-continuing character (alpha/digit/underscore), so substrings
/// inside longer identifiers (e.g. `workflow_handle`) don't trigger.
fn find_keyword_outside_comments_and_strings(source: &str, pattern: &str, ident_boundary: bool) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = 0usize;
    let plen = pattern.len();
    while i + plen <= bytes.len() {
        // Skip over `// …\n` line comments.
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // Skip over `/* … */` block comments.
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        // Skip over `"…"` string literals (handle simple backslash escapes).
        if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            i = (i + 1).min(bytes.len());
            continue;
        }
        if &bytes[i..i + plen] == pattern.as_bytes() {
            if ident_boundary {
                let next = bytes.get(i + plen).copied().unwrap_or(0);
                let continues_ident = next.is_ascii_alphanumeric() || next == b'_';
                if continues_ident {
                    i += 1;
                    continue;
                }
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod adr028_comment_skip_tests {
    use super::find_outside_comments_and_strings;

    #[test]
    fn finds_keyword_outside_comment() {
        assert_eq!(
            find_outside_comments_and_strings("workflow Foo {}", "workflow "),
            Some(0)
        );
    }

    #[test]
    fn skips_keyword_in_line_comment() {
        // The bare word "workflow" inside a `//` comment must NOT be matched.
        assert_eq!(
            find_outside_comments_and_strings("// workflow time-travel scrubber\nfn foo() {}", "workflow "),
            None
        );
    }

    #[test]
    fn skips_keyword_in_block_comment() {
        assert_eq!(
            find_outside_comments_and_strings("/* see workflow doc */\nfn foo() {}", "workflow "),
            None
        );
    }

    #[test]
    fn skips_keyword_in_string_literal() {
        assert_eq!(
            find_outside_comments_and_strings(r#"let s = "workflow demo";"#, "workflow "),
            None
        );
    }

    #[test]
    fn finds_after_passing_comment() {
        let src = "// pre-amble mentioning workflow\nworkflow Real {}";
        let offset = find_outside_comments_and_strings(src, "workflow ").unwrap();
        // Must be the second occurrence (start of the `workflow Real` line), not the comment.
        assert!(offset > 30, "expected match past the comment, got offset {offset}");
    }
}

/// Options for the unified compiler pipeline.
#[derive(Debug, Clone, Default)]
pub struct PipelineOptions {
    pub lower_config: LowerConfig,
    pub script_mode: bool,
}

/// The result of running the frontend pipeline.
pub struct FrontendResult {
    pub module: Module,
    pub hir: HirModule,
    pub diagnostics: Vec<Diagnostic>,
    pub source: String,
}

impl FrontendResult {
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == TypeckSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == TypeckSeverity::Warning)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }
}

/// Run the frontend pipeline on a source string.
pub fn run_frontend_str(source: &str, _file_path: &str) -> Result<FrontendResult> {
    run_frontend_str_with_options(source, _file_path, &PipelineOptions::default())
}

pub fn run_frontend_str_with_options(
    source: &str,
    _file_path: &str,
    options: &PipelineOptions,
) -> Result<FrontendResult> {
    // 1. Lex
    let tokens = crate::lexer::lex(source);

    // 1.5. Prevent Syntactic Configurability (K-Complexity Guard)
    for spanned in &tokens {
        if let crate::lexer::token::Token::Ident(ref name) = spanned.token {
            if name == "macro_rules" || name == "macro" {
                let diag = Diagnostic {
                    severity: TypeckSeverity::Error,
                    message: "SyntacticConfigurabilityNotAllowed: Vox is strictly constrained. Do not use macros or custom syntactic configurability. Use vox-skills for extended actions.".to_string(),
                    span: crate::ast::span::Span::new(spanned.span.start, spanned.span.end),
                    expected_type: None,
                    found_type: None,
                    context: None,
                    suggestions: vec!["Rewrite using standard syntax and route out-of-band logic through MCP skills.".to_string()],
                    category: crate::typeck::diagnostics::DiagnosticCategory::Parse,
                    code: Some("E091".to_string()),
                    fixes: vec![],
                    line_col: None,
                    missing_cases: vec![],
                    ast_node_kind: None,
                };
                return Ok(FrontendResult {
                    module: crate::ast::decl::Module {
                        declarations: vec![],
                        span: crate::ast::span::Span::new(0, 0),
                    },
                    hir: crate::hir::HirModule::default(),
                    diagnostics: vec![diag],
                    source: source.to_owned(),
                });
            }
        }
    }

    // 1.6. ADR-028: reject reserved durability grammar keywords early.
    {
        let reserved_diags = check_adr028_reserved_keywords(source);
        if !reserved_diags.is_empty() {
            return Ok(FrontendResult {
                module: crate::ast::decl::Module {
                    declarations: vec![],
                    span: crate::ast::span::Span::new(0, 0),
                },
                hir: crate::hir::HirModule::default(),
                diagnostics: reserved_diags,
                source: source.to_owned(),
            });
        }
    }

    // 2. Parse
    let module_res = if options.script_mode {
        crate::parser::parse_script(tokens.clone())
    } else {
        crate::parser::parse(tokens.clone())
    };
    let module = module_res
        .map_err(|errors| anyhow::anyhow!("Parsing failed with {} error(s)", errors.len()))?;

    // 3. Lower to HIR + structural validation
    let mut hir = crate::hir::lower::lower_module_with_config(&module, &options.lower_config);

    // 4. Type-check HIR (populates inferred types)
    let mut diagnostics = crate::typeck::typecheck_hir_module(source, &mut hir);

    // 5. Deprecated Usage Detector (Item 16, @deprecated)
    for line in source.lines() {
        let line_start_byte = (line.as_ptr() as usize).saturating_sub(source.as_ptr() as usize);
        if line.trim_start().starts_with("@deprecated") {
            let start = line_start_byte + line.find("@deprecated").unwrap_or(0);
            diagnostics.push(Diagnostic {
                severity: TypeckSeverity::Warning,
                message: "Found @deprecated annotation. Consider removing this obsolete code."
                    .to_string(),
                span: crate::ast::span::Span::new(start, start + 11),
                expected_type: None,
                found_type: None,
                context: None,
                suggestions: vec![
                    "Refactor dependents and remove this deprecated item.".to_string(),
                ],
                category: crate::typeck::diagnostics::DiagnosticCategory::Parse,
                code: Some("W092".to_string()),
                fixes: vec![],
                line_col: None,
                missing_cases: vec![],
                ast_node_kind: None,
            });
        }

        let jsx_leaks = ["className=", "onClick=", "onChange=", "onSubmit="];
        for leak in jsx_leaks {
            if let Some(idx) = line.find(leak) {
                let start = line_start_byte + idx;
                let attr = leak.trim_end_matches('=');
                let mut vox_attr = attr.to_lowercase();
                if vox_attr.starts_with("on") {
                    vox_attr = format!("on:{}", &vox_attr[2..]);
                }
                if vox_attr == "classname" {
                    vox_attr = "class".to_string();
                }
                diagnostics.push(Diagnostic {
                    severity: TypeckSeverity::Warning,
                    message: format!("Raw JSX '{}' leaks into Vox source (Item 16).", attr),
                    span: crate::ast::span::Span::new(start, start + leak.len()),
                    expected_type: None,
                    found_type: None,
                    context: None,
                    suggestions: vec![format!(
                        "Use Vox-native syntax: '{}=' instead of '{}='.",
                        vox_attr, attr
                    )],
                    category: crate::typeck::diagnostics::DiagnosticCategory::Parse,
                    code: Some("W093".to_string()),
                    fixes: vec![],
                    line_col: None,
                    missing_cases: vec![],
                    ast_node_kind: None,
                });
            }
        }
    }

    for e in crate::hir::validate_module(&hir) {
        diagnostics.push(Diagnostic::hir_invariant(
            e.message,
            e.span,
            source,
            e.correction_hint,
        ));
    }

    Ok(FrontendResult {
        module,
        hir,
        diagnostics,
        source: source.to_owned(),
    })
}

pub fn format_diagnostics_json(result: &FrontendResult, file_path: &str) -> String {
    let output: Vec<VoxCompilerDiagnosticPayload> = result
        .diagnostics
        .iter()
        .map(|d| VoxCompilerDiagnosticPayload::from_diagnostic(d, file_path, &result.source))
        .collect();
    serde_json::to_string_pretty(&output).unwrap_or_default()
}

/// Run the full check pipeline and return machine-readable diagnostics even on parse failure.
pub fn check_file(source: &str, file_path: &str) -> Vec<VoxCompilerDiagnosticPayload> {
    let tokens = crate::lexer::lex(source);

    // 1.5. Prevent Syntactic Configurability (K-Complexity Guard)
    for spanned in &tokens {
        if let crate::lexer::token::Token::Ident(ref name) = spanned.token {
            if name == "macro_rules" || name == "macro" {
                let diag = Diagnostic {
                    severity: TypeckSeverity::Error,
                    message: "SyntacticConfigurabilityNotAllowed: Vox is strictly constrained. Do not use macros or custom syntactic configurability. Use vox-skills for extended actions.".to_string(),
                    span: crate::ast::span::Span::new(spanned.span.start, spanned.span.end),
                    expected_type: None,
                    found_type: None,
                    context: None,
                    suggestions: vec!["Rewrite using standard syntax and route out-of-band logic through MCP skills.".to_string()],
                    category: crate::typeck::diagnostics::DiagnosticCategory::Parse,
                    code: Some("E091".to_string()),
                    fixes: vec![],
                    line_col: None,
                    missing_cases: vec![],
                    ast_node_kind: None,
                };
                return vec![VoxCompilerDiagnosticPayload::from_diagnostic(
                    &diag, file_path, source,
                )];
            }
        }
    }

    // 1.6. ADR-028: reject reserved durability grammar keywords early.
    {
        let reserved_diags = check_adr028_reserved_keywords(source);
        if !reserved_diags.is_empty() {
            return reserved_diags
                .iter()
                .map(|d| VoxCompilerDiagnosticPayload::from_diagnostic(d, file_path, source))
                .collect();
        }
    }

    match crate::parser::parse(tokens) {
        Ok(module) => {
            let mut hir = crate::hir::lower_module(&module);
            let mut diagnostics = crate::typeck::typecheck_hir_module(source, &mut hir);

            // 5. Deprecated Usage Detector (Item 16, @deprecated)
            for line in source.lines() {
                let line_start_byte =
                    (line.as_ptr() as usize).saturating_sub(source.as_ptr() as usize);
                if line.trim_start().starts_with("@deprecated") {
                    let start = line_start_byte + line.find("@deprecated").unwrap_or(0);
                    diagnostics.push(Diagnostic {
                        severity: TypeckSeverity::Warning,
                        message:
                            "Found @deprecated annotation. Consider removing this obsolete code."
                                .to_string(),
                        span: crate::ast::span::Span::new(start, start + 11),
                        expected_type: None,
                        found_type: None,
                        context: None,
                        suggestions: vec![
                            "Refactor dependents and remove this deprecated item.".to_string(),
                        ],
                        category: crate::typeck::diagnostics::DiagnosticCategory::Parse,
                        code: Some("W092".to_string()),
                        fixes: vec![],
                        line_col: None,
                        missing_cases: vec![],
                        ast_node_kind: None,
                    });
                }

                let jsx_leaks = ["className=", "onClick=", "onChange=", "onSubmit="];
                for leak in jsx_leaks {
                    if let Some(idx) = line.find(leak) {
                        let start = line_start_byte + idx;
                        let attr = leak.trim_end_matches('=');
                        let mut vox_attr = attr.to_lowercase();
                        if vox_attr.starts_with("on") {
                            vox_attr = format!("on:{}", &vox_attr[2..]);
                        }
                        if vox_attr == "classname" {
                            vox_attr = "class".to_string();
                        }
                        diagnostics.push(Diagnostic {
                            severity: TypeckSeverity::Warning,
                            message: format!("Raw JSX '{}' leaks into Vox source (Item 16).", attr),
                            span: crate::ast::span::Span::new(start, start + leak.len()),
                            expected_type: None,
                            found_type: None,
                            context: None,
                            suggestions: vec![format!(
                                "Use Vox-native syntax: '{}=' instead of '{}='.",
                                vox_attr, attr
                            )],
                            category: crate::typeck::diagnostics::DiagnosticCategory::Parse,
                            code: Some("W093".to_string()),
                            fixes: vec![],
                            line_col: None,
                            missing_cases: vec![],
                            ast_node_kind: None,
                        });
                    }
                }
            }

            for e in crate::hir::validate_module(&hir) {
                diagnostics.push(Diagnostic::hir_invariant(
                    e.message,
                    e.span,
                    source,
                    e.correction_hint,
                ));
            }
            diagnostics
                .into_iter()
                .map(|d| VoxCompilerDiagnosticPayload::from_diagnostic(&d, file_path, source))
                .collect()
        }
        Err(errors) => errors
            .into_iter()
            .map(|e| {
                let diag = Diagnostic {
                    severity: TypeckSeverity::Error,
                    message: e.message,
                    span: e.span,
                    expected_type: None,
                    found_type: None,
                    context: None,
                    suggestions: vec![],
                    category: crate::typeck::diagnostics::DiagnosticCategory::Parse,
                    code: Some("E0001".to_string()),
                    fixes: vec![],
                    line_col: None,
                    missing_cases: vec![],
                    ast_node_kind: None,
                };
                VoxCompilerDiagnosticPayload::from_diagnostic(&diag, file_path, source)
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reject_macros_e091() {
        let source = "macro_rules! my_macro { () => {} }";
        let diagnostics = check_file(source, "test.vox");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].error_code, "E091".to_string());
        assert!(
            diagnostics[0]
                .message
                .contains("SyntacticConfigurabilityNotAllowed")
        );

        // Also test run_frontend_str
        let frontend_res = run_frontend_str(source, "test.vox").unwrap();
        assert_eq!(frontend_res.diagnostics.len(), 1);
        assert_eq!(frontend_res.diagnostics[0].code, Some("E091".to_string()));
    }

    // ADR-028: @scheduled, @durable, workflow, activity are reserved/removed from public grammar.
    // actor is retained and must compile cleanly.

    #[test]
    fn test_reject_scheduled_adr028() {
        // @scheduled parses successfully but must be rejected with a diagnostic.
        let source = r#"@scheduled("1h") fn tick() {}"#;
        let diagnostics = check_file(source, "test.vox");
        assert!(
            !diagnostics.is_empty(),
            "@scheduled should produce a compile error (ADR-028)"
        );
        assert!(
            diagnostics.iter().any(|d| d.message.contains("@scheduled")),
            "diagnostic message should mention @scheduled; got: {:?}",
            diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert!(
            diagnostics.iter().any(|d| d.severity == crate::typeck::diagnostics::TypeckSeverity::Error),
            "severity should be error"
        );
    }

    #[test]
    fn test_reject_durable_adr028() {
        // @durable is not a recognised token — currently a parse error.
        // ADR-028 requires a clear diagnostic mentioning @durable.
        let source = r#"@durable fn process() {}"#;
        let diagnostics = check_file(source, "test.vox");
        assert!(
            !diagnostics.is_empty(),
            "@durable should produce a compile error (ADR-028)"
        );
        assert!(
            diagnostics.iter().any(|d| d.message.contains("@durable")),
            "diagnostic message should mention @durable; got: {:?}",
            diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_reject_workflow_adr028() {
        // workflow parses successfully but must be rejected with a diagnostic.
        let source = r#"workflow order() {}"#;
        let diagnostics = check_file(source, "test.vox");
        assert!(
            !diagnostics.is_empty(),
            "workflow keyword should produce a compile error (ADR-028)"
        );
        assert!(
            diagnostics.iter().any(|d| d.message.contains("workflow")),
            "diagnostic message should mention workflow; got: {:?}",
            diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert!(
            diagnostics.iter().any(|d| d.severity == crate::typeck::diagnostics::TypeckSeverity::Error),
            "severity should be error"
        );
    }

    #[test]
    fn test_reject_activity_adr028() {
        // activity parses successfully but must be rejected with a diagnostic.
        let source = r#"activity charge() {}"#;
        let diagnostics = check_file(source, "test.vox");
        assert!(
            !diagnostics.is_empty(),
            "activity keyword should produce a compile error (ADR-028)"
        );
        assert!(
            diagnostics.iter().any(|d| d.message.contains("activity")),
            "diagnostic message should mention activity; got: {:?}",
            diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert!(
            diagnostics.iter().any(|d| d.severity == crate::typeck::diagnostics::TypeckSeverity::Error),
            "severity should be error"
        );
    }

    #[test]
    fn test_actor_still_compiles_adr028() {
        // actor is retained per ADR-028 — must produce zero errors.
        let source = r#"actor Counter { on increment(n: int) to int { return n } }"#;
        let diagnostics = check_file(source, "test.vox");
        assert!(
            diagnostics.iter().all(|d| d.severity != crate::typeck::diagnostics::TypeckSeverity::Error),
            "actor should still compile successfully (ADR-028 retains actor); errors: {:?}",
            diagnostics.iter().filter(|d| d.severity == crate::typeck::diagnostics::TypeckSeverity::Error).collect::<Vec<_>>()
        );
    }
}
