//! TypeScript code generation — stub module.
//!
//! Full TypeScript emit is handled by `vox-codegen` (the `codegen_ts` crate path).
//! This in-compiler shim exists so compiler test utilities can reference a
//! `generate()` entry point without pulling in the full codegen crate.
//!
//! Status: **Wave-3 migration in progress** — real emit lives in
//! `crates/vox-codegen/src/codegen_ts/`.  This function returns
//! `Err(E0404)` so callers get a structured, actionable diagnostic rather
//! than a raw string.

use crate::hir::HirModule;
use std::collections::BTreeMap;

/// Output of TypeScript code generation.
#[derive(Debug, Clone)]
pub struct GeneratedFiles {
    /// Generated TS/TSX file contents keyed by filename.
    pub files: BTreeMap<String, String>,
}

/// Generate TypeScript/TSX code from HIR.
///
/// # Errors
///
/// Returns `Err` with diagnostic code `E0404` when the TS codegen backend is
/// not available in this build.  Callers in production use `vox-codegen`
/// directly; this shim is for compiler-internal test utilities only.
pub fn generate(_hir: &HirModule) -> Result<GeneratedFiles, String> {
    Err(
        "[E0404] TypeScript codegen is not available from the compiler crate. \
         Use `vox_codegen::codegen_ts::generate()` (crates/vox-codegen) instead. \
         See docs/src/architecture/where-things-live.md §codegen."
            .to_string(),
    )
}

/// Stub module for HIR to TS emission (tests only).
pub mod hir_emit {
    use crate::hir::HirExpr;
    use std::collections::HashSet;

    /// Emission context stub.
    #[derive(Debug)]
    pub struct EmitCtx<'a> {
        _state_names: &'a HashSet<String>,
    }

    impl<'a> EmitCtx<'a> {
        /// Create stub context.
        pub fn new(state_names: &'a HashSet<String>) -> Self {
            Self {
                _state_names: state_names,
            }
        }
    }

    /// Stub emit function.
    pub fn emit_hir_expr(_expr: HirExpr, _ctx: &EmitCtx<'_>) -> String {
        "/* stub */".to_string()
    }

    /// Stub for JSX attribute mapping.
    pub fn map_jsx_attr_name(name: &str) -> String {
        name.to_string()
    }

    /// Compatibility shim module.
    pub mod compat {
        /// Marker for backwards compatibility.
        pub fn noop() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn generate_reports_e0404_pointing_at_vox_codegen() {
        let err = generate(&HirModule::default()).unwrap_err();
        assert!(err.starts_with("[E0404]"), "{err}");
        assert!(err.contains("vox_codegen::codegen_ts::generate"), "{err}");
    }

    #[test]
    fn hir_emit_stubs_are_inert() {
        let names = HashSet::new();
        let ctx = hir_emit::EmitCtx::new(&names);
        let expr = crate::hir::HirExpr::IntLit(1, crate::ast::span::Span::new(0, 0));
        assert_eq!(hir_emit::emit_hir_expr(expr, &ctx), "/* stub */");
        assert_eq!(hir_emit::map_jsx_attr_name("className"), "className");
    }
}
