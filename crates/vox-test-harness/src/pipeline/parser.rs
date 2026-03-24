//! Parser pipeline helpers.

use vox_compiler::ast::decl::Module;
use vox_compiler::parser::parse;
use crate::pipeline::lexer::lex_str;

/// Lex and parse `src`, panicking with a clear message if parsing fails.
///
/// Use in tests that verify downstream behaviour (HIR lowering, codegen)
/// and don't need to exercise the parser itself.
#[track_caller]
pub fn parse_str_unwrap(src: &str) -> Module {
    let tokens = lex_str(src);
    parse(tokens).unwrap_or_else(|errs| {
        panic!(
            "parse_str_unwrap: expected clean parse, got {} error(s):\n{:#?}",
            errs.len(),
            errs
        )
    })
}
