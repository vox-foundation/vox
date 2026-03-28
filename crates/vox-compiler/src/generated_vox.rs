//! Shared normalization and validation contract for generated `.vox` outputs.
//!
//! This module is intentionally dependency-light so CLI, MCP, and scorecard
//! paths can use the same behavior without divergent wrappers.

use crate::ast::span::Span;
use crate::hir::{lower_module, validate_module};
use crate::lexer::lex;
use crate::parser::parse;
use crate::typeck::typecheck_ast_module;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputSurfaceMode {
    RawCodeOnly,
    FencedTransport,
}

#[derive(Debug, Clone)]
pub struct NormalizedGeneratedVox {
    pub raw: String,
    pub normalized: String,
    pub had_fence: bool,
    pub had_prose_markers: bool,
    pub surface_contract_ok: bool,
}

#[derive(Debug, Clone)]
pub struct GeneratedVoxError {
    pub category: &'static str,
    pub code: Option<String>,
    pub message: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct GeneratedVoxValidation {
    pub parse_ok: bool,
    pub typecheck_ok: bool,
    pub hir_ok: bool,
    pub canonical_ok: bool,
    pub canonicalized: Option<String>,
    pub errors: Vec<GeneratedVoxError>,
}

impl GeneratedVoxValidation {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

#[must_use]
pub fn normalize_generated_vox(raw: &str, mode: OutputSurfaceMode) -> NormalizedGeneratedVox {
    let trimmed = raw.trim();
    let had_fence = trimmed.contains("```");
    let lowered = trimmed.to_ascii_lowercase();
    let had_prose_markers = ["here is", "explanation", "```text", "```md"]
        .iter()
        .any(|needle| lowered.contains(needle));

    let normalized = strip_vox_codeblock_fence(trimmed);
    let surface_contract_ok = match mode {
        OutputSurfaceMode::RawCodeOnly => !had_fence && !had_prose_markers,
        OutputSurfaceMode::FencedTransport => !had_prose_markers,
    };

    NormalizedGeneratedVox {
        raw: raw.to_string(),
        normalized,
        had_fence,
        had_prose_markers,
        surface_contract_ok,
    }
}

#[must_use]
pub fn strip_vox_codeblock_fence(text: &str) -> String {
    let block = text.trim();
    if block.starts_with("```vox") {
        return block
            .strip_prefix("```vox")
            .unwrap_or(block)
            .strip_suffix("```")
            .unwrap_or(block)
            .trim()
            .to_string();
    }
    if block.starts_with("```") {
        return block
            .strip_prefix("```")
            .unwrap_or(block)
            .strip_suffix("```")
            .unwrap_or(block)
            .trim()
            .to_string();
    }
    block.to_string()
}

#[must_use]
pub fn validate_generated_vox(source: &str, canonicalize: bool) -> GeneratedVoxValidation {
    let mut errors = Vec::new();
    let tokens = lex(source);
    let module = match parse(tokens) {
        Ok(module) => module,
        Err(parse_errors) => {
            for err in parse_errors {
                errors.push(GeneratedVoxError {
                    category: "parse",
                    code: None,
                    message: err.to_string(),
                    span: Some(err.span),
                });
            }
            return GeneratedVoxValidation {
                parse_ok: false,
                typecheck_ok: false,
                hir_ok: false,
                canonical_ok: false,
                canonicalized: None,
                errors,
            };
        }
    };

    let type_errors = typecheck_ast_module(source, &module);
    for err in type_errors {
        errors.push(GeneratedVoxError {
            category: "typeck",
            code: err.code,
            message: err.message,
            span: Some(err.span),
        });
    }

    let hir = lower_module(&module);
    for err in validate_module(&hir) {
        errors.push(GeneratedVoxError {
            category: "hir",
            code: None,
            message: err.message,
            span: Some(err.span),
        });
    }

    let parse_ok = true;
    let typecheck_ok = !errors.iter().any(|e| e.category == "typeck");
    let hir_ok = !errors.iter().any(|e| e.category == "hir");
    let mut canonical_ok = false;
    let mut canonicalized = None;

    if errors.is_empty() && canonicalize {
        if let Ok(canon) = crate::canonicalize_vox(source) {
            canonical_ok = validate_generated_vox(&canon, false).is_valid();
            canonicalized = Some(canon);
        }
    } else if errors.is_empty() {
        canonical_ok = true;
    }

    GeneratedVoxValidation {
        parse_ok,
        typecheck_ok,
        hir_ok,
        canonical_ok,
        canonicalized,
        errors,
    }
}
