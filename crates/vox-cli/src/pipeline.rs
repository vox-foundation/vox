//! Shared compiler pipeline for the Vox CLI.
//!
//! Provides a single entry point (`run_frontend`) that runs the full
//! lex → parse → typecheck → HIR validation pass and returns structured
//! results. All CLI commands (`build`, `check`) and the LSP use this so
//! that error formatting stays consistent and pipeline changes need to be
//! made in exactly one place.

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::path::Path;
use vox_compiler::ast::decl::Module;
use vox_compiler::hir::HirModule;
use vox_compiler::pipeline::PipelineOptions;
use vox_compiler::typeck::Diagnostic;
use vox_compiler::typeck::diagnostics::TypeckSeverity;

use vox_bounded_fs::read_utf8_path_capped;

fn line_col_for_byte_offset(source: &str, byte_idx: usize) -> (usize, usize) {
    let (l0, c0) = vox_compiler::ast::span::byte_offset_to_line_col_zero_based(source, byte_idx);
    (l0 as usize + 1, c0 as usize + 1)
}

fn source_line_at(source: &str, line_1based: usize) -> Option<&str> {
    source.lines().nth(line_1based.saturating_sub(1))
}

/// The result of running the frontend pipeline (lex → parse → typecheck → HIR).
pub struct FrontendResult {
    /// Parsed AST module root.
    pub module: Module,
    /// Lowered and validated HIR module.
    pub hir: HirModule,
    /// Diagnostics emitted during typecheck and HIR validation.
    pub diagnostics: Vec<Diagnostic>,
    /// Full source text (for span rendering and line snippets).
    pub source: String,
}

impl FrontendResult {
    /// Count of error-severity diagnostics.
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == TypeckSeverity::Error)
            .count()
    }

    /// Count of warning-severity diagnostics.
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == TypeckSeverity::Warning)
            .count()
    }

    /// Returns `true` if any error-severity diagnostic was produced.
    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }
}

/// Run the frontend pipeline on a source file.
///
/// Steps:
/// 1. Lex
/// 2. Parse (returns `Err` on parse failure with pretty-printed errors)
/// 3. Type-check
/// 4. Lower to HIR + run HIR validation
///
/// Parse errors are printed to stderr in rustc style and returned as `Err`.
/// Type/HIR diagnostics are stored in [`FrontendResult::diagnostics`]; it is
/// the caller's responsibility to decide whether to treat them as fatal.
pub async fn run_frontend(file: &Path, json: bool) -> Result<FrontendResult> {
    run_frontend_with_options(file, json, &PipelineOptions::default()).await
}

pub async fn run_frontend_with_options(
    file: &Path,
    json: bool,
    options: &PipelineOptions,
) -> Result<FrontendResult> {
    let source = read_utf8_path_capped(file)
        .with_context(|| format!("Failed to read source file: {}", file.display()))?;

    run_frontend_str_with_options(&source, file, json, options)
}

/// Same as [`run_frontend`] but takes an already-loaded source string.
pub fn run_frontend_str(source: &str, file: &Path, json: bool) -> Result<FrontendResult> {
    run_frontend_str_with_options(source, file, json, &PipelineOptions::default())
}

pub fn run_frontend_str_with_options(
    source: &str,
    file: &Path,
    json: bool,
    options: &PipelineOptions,
) -> Result<FrontendResult> {
    let file_path = file.to_string_lossy();
    match vox_compiler::pipeline::run_frontend_str_with_options(source, &file_path, options) {
        Ok(res) => Ok(FrontendResult {
            module: res.module,
            hir: res.hir,
            diagnostics: res.diagnostics,
            source: res.source,
        }),
        Err(e) => {
            if json {
                let diagnostics = vox_compiler::pipeline::check_file(source, &file_path);
                if let Ok(s) = serde_json::to_string_pretty(&diagnostics) {
                    println!("{}", s);
                }
            } else {
                // We need the parse errors to print them pretty.
                // For now, we'll re-lex/parse if we need pretty printing,
                // but usually, run_frontend_str failure means parse failure.
                let tokens = vox_compiler::lexer::lex(source);
                if let Err(errors) = vox_compiler::parser::parse(tokens) {
                    print_parse_errors(&errors, source, file);
                }
            }
            Err(e)
        }
    }
}

#[must_use]
pub fn format_diagnostics_json_pretty(result: &FrontendResult, file: &Path) -> String {
    use vox_compiler::typeck::diagnostics::VoxCompilerDiagnosticPayload;
    let file_path = file.to_string_lossy();
    let output: Vec<VoxCompilerDiagnosticPayload> = result
        .diagnostics
        .iter()
        .map(|d| VoxCompilerDiagnosticPayload::from_diagnostic(d, &file_path, &result.source))
        .collect();
    serde_json::to_string_pretty(&output).unwrap_or_default()
}

/// Print diagnostics in rustc-style to stderr, or JSON to stdout if `json` is true.
pub fn print_diagnostics(result: &FrontendResult, file: &Path, json: bool) {
    if json {
        println!("{}", format_diagnostics_json_pretty(result, file));
    } else {
        for (i, d) in result.diagnostics.iter().enumerate() {
            let code = format!("E{:04}", i + 1);
            let (line, col) = line_col_for_byte_offset(&result.source, d.span.start);
            let sev = match d.severity {
                TypeckSeverity::Error => "error",
                TypeckSeverity::Warning => "warning",
            };
            eprintln!(
                "{sev}[{code}]: {} at {}:{}:{}",
                d.message,
                file.display(),
                line,
                col
            );
        }
    }
}

/// Print parse errors to stderr in rustc style.
pub fn print_parse_errors_to_stderr(
    errors: &[vox_compiler::parser::ParseError],
    source: &str,
    file: &Path,
) {
    print_parse_errors(errors, source, file);
}

fn print_parse_errors(errors: &[vox_compiler::parser::ParseError], source: &str, file: &Path) {
    for e in errors {
        let (line, col) = line_col_for_byte_offset(source, e.span.start);
        let context_line = source_line_at(source, line).unwrap_or("");
        eprintln!("{} {}", "error[parse]".red().bold(), e.message.bold());
        eprintln!(
            "  {} {}:{}:{}",
            "-->".blue().bold(),
            file.display(),
            line,
            col
        );
        eprintln!("   {}", "|".blue().bold());
        eprintln!("   {} {}", format!("{line} |").blue().bold(), context_line);
        let arrow = " ".repeat(col.saturating_sub(1)) + "^";
        eprintln!("   {} {}", "|".blue().bold(), arrow.red().bold());
        eprintln!();
    }
    eprintln!(
        "{} aborting due to {} previous {}",
        "error".red().bold(),
        errors.len(),
        if errors.len() == 1 { "error" } else { "errors" }
    );
}
