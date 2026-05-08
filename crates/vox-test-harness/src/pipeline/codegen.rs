//! Codegen pipeline helpers: shortcuts from source string to output.
//!
//! These eliminate the lex→parse→lower→generate boilerplate that every codegen
//! test file previously re-declared as a local helper function.

use vox_compiler_emit::codegen_ts::emitter::CodegenOutput;
use vox_compiler_emit::codegen_ts::generate;
use vox_compiler::hir::{HirModule, lower_module};

use crate::pipeline::parser::parse_str_unwrap;

/// Lex, parse, and lower `src` to HIR.  Panics on parse error.
///
/// Use when you need the `HirModule` for further inspection before codegen.
#[track_caller]
pub fn lower_str(src: &str) -> HirModule {
    let module = parse_str_unwrap(src);
    lower_module(&module)
}

/// Full pipeline: lex → parse → lower → TypeScript codegen.
///
/// Returns the full [`CodegenOutput`] so callers can inspect individual files.
/// Panics on parse error or codegen failure.
#[track_caller]
pub fn codegen_ts_str(src: &str) -> CodegenOutput {
    let hir = lower_str(src);
    generate(&hir).unwrap_or_else(|e| panic!("codegen_ts_str: codegen failed: {e}"))
}

/// Full pipeline returning a single named file's content.
///
/// Panics if the file is absent in the output.
#[track_caller]
pub fn codegen_ts_file(src: &str, filename: &str) -> String {
    let out = codegen_ts_str(src);
    out.files
        .iter()
        .find(|(n, _)| n == filename)
        .unwrap_or_else(|| {
            let names: Vec<_> = out.files.iter().map(|(n, _)| n.as_str()).collect();
            panic!(
                "codegen_ts_file: file {:?} not found in output; available: {:?}",
                filename, names
            );
        })
        .1
        .clone()
}
