//! Unified compiler pipeline orchestrator.
//!
//! Provides a single entry point (`run_frontend`) that runs the full
//! lex → parse → typecheck → HIR validation pass and returns structured
//! results.

use crate::ast::decl::Module;
use crate::hir::HirModule;
use crate::hir::lower::LowerConfig;
use crate::typeck::Diagnostic;
use crate::typeck::diagnostics::{TypeckSeverity, VoxCompilerDiagnosticPayload};
use anyhow::Result;

/// Options for the unified compiler pipeline.
#[derive(Debug, Clone, Default)]
pub struct PipelineOptions {
    pub lower_config: LowerConfig,
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

    // 2. Parse
    let module = crate::parser::parse(tokens)
        .map_err(|errors| anyhow::anyhow!("Parsing failed with {} error(s)", errors.len()))?;

    // 3. Lower to HIR + structural validation
    let mut hir = crate::hir::lower::lower_module_with_config(&module, &options.lower_config);

    // 4. Type-check HIR (populates inferred types)
    let mut diagnostics = crate::typeck::typecheck_hir_module(source, &mut hir);

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
    match crate::parser::parse(tokens) {
        Ok(module) => {
            let mut hir = crate::hir::lower_module(&module);
            let mut diagnostics = crate::typeck::typecheck_hir_module(source, &mut hir);
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
