//! LLVM / Inkwell backend — **optional** and often **excluded** from local `cargo clippy` when LLVM is not installed (see `.agents/cargo-safety.md`).
//!
//! Defines the [`CodegenBackend`] trait that codegen backends implement. Full LLVM lowering is deferred; prefer `vox-codegen-rust` / WASM paths for active work.

/// Trait for code generation backends.
pub trait CodegenBackend {
    /// The output type produced by this backend.
    type Output;
    /// The error type for code generation failures.
    type Error: std::error::Error;

    /// Generate output from a HIR module.
    fn generate(&self, module: &vox_compiler::hir::hir::HirModule) -> Result<Self::Output, Self::Error>;
}
