//! .vox Source code validation and machine-parsable diagnostics (repair loop support).
//!
//! Provides CLI parity with `vox check --output-format json` and LSP-style
//! diagnostics for tool-driven self-repair.

use crate::params::{
    DiagnosticInfo, ToolResult, ValidateFileParams, ValidateResponse, VoxCheckParams,
    VoxCheckResponse,
};
use crate::server::ServerState;
use std::path::Path;

/// Validate a .vox file using the full compiler pipeline (lexer → parser → typeck → HIR).
pub async fn validate_file(state: &ServerState, params: ValidateFileParams) -> String {
    let path = match super::workspace_path::resolve_existing_path_in_repository(state, &params.path)
    {
        Ok(p) => p,
        Err(e) => {
            return ToolResult::<ValidateResponse>::err_with_remediation(
                e.message(),
                e.remediation(),
            )
            .to_json();
        }
    };

    let text = match tokio::fs::read_to_string(&path).await {
        Ok(t) => t,
        Err(e) => {
            return ToolResult::<ValidateResponse>::err_with_remediation(
                format!("failed to read file: {e}"),
                super::workspace_path::REM_VALIDATE_IO,
            )
            .to_json();
        }
    };

    let correlation_id = vox_oratio::trace::new_correlation_id();
    tracing::debug!(
        target: "vox_mcp_speech",
        correlation_id = %correlation_id,
        path = %params.path,
        bytes = text.len(),
        "validate_file: running HIR validation"
    );

    let diagnostics = vox_lsp::validate_document_with_hir(&text);
    let infos: Vec<DiagnosticInfo> = diagnostics
        .iter()
        .map(|d| DiagnosticInfo {
            severity: match d.severity {
                Some(s) if s == tower_lsp::lsp_types::DiagnosticSeverity::ERROR => {
                    "error".to_string()
                }
                _ => "warning".to_string(),
            },
            message: d.message.clone(),
            source: d.source.clone().unwrap_or_default(),
            start_line: d.range.start.line,
            start_col: d.range.start.character,
            end_line: d.range.end.line,
            end_col: d.range.end.character,
        })
        .collect();

    ToolResult::ok(ValidateResponse {
        count: infos.len(),
        diagnostics: infos,
        hir_validation_included: true,
        correlation_id: Some(correlation_id),
    })
    .to_json()
}

/// Check a .vox file and return machine-readable diagnostics (parity with CLI `check --output-format json`).
pub async fn vox_check(state: &ServerState, params: VoxCheckParams) -> String {
    let path = match super::workspace_path::resolve_existing_path_in_repository(state, &params.path)
    {
        Ok(p) => p,
        Err(e) => {
            return ToolResult::<VoxCheckResponse>::err_with_remediation(
                e.message(),
                e.remediation(),
            )
            .to_json();
        }
    };

    let source = match tokio::fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(e) => {
            return ToolResult::<VoxCheckResponse>::err_with_remediation(
                format!("failed to read file: {e}"),
                super::workspace_path::REM_VALIDATE_IO,
            )
            .to_json();
        }
    };

    let diagnostics = vox_compiler::pipeline::check_file(&source, &params.path);
    let has_errors = diagnostics
        .iter()
        .any(|d| d.severity == vox_compiler::typeck::diagnostics::TypeckSeverity::Error);

    ToolResult::ok(VoxCheckResponse {
        count: diagnostics.len(),
        has_errors,
        diagnostics,
    })
    .to_json()
}
