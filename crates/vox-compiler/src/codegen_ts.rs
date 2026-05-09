//! TypeScript code generation — stub module for tests.
//!
//! This module is a placeholder for TypeScript codegen functionality.
//! Tests referencing this module are marked as ignored pending implementation.

use crate::hir::{HirModule, HirExpr};
use std::collections::BTreeMap;

/// Output of TypeScript code generation.
#[derive(Debug, Clone)]
pub struct GeneratedFiles {
    /// Generated TS/TSX file contents keyed by filename.
    pub files: BTreeMap<String, String>,
}

/// Generate TypeScript/TSX code from HIR (stub implementation).
pub fn generate(_hir: &HirModule) -> Result<GeneratedFiles, String> {
    Err("codegen_ts is not yet implemented".to_string())
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
            Self { _state_names: state_names }
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

    /// Stub compat module.
    pub mod compat {
        /// Stub for compatibility.
        pub fn stub() {}
    }
}
