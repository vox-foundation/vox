//! XGrammar-2 emitter for `vox-grammar-export`.
//!
//! Produces a JSON-serialized PDA/Earley grammar spec for high-performance
//! constrained inference backends.

use crate::grammar_ir::Grammar;

/// Emit an XGrammar-2 compatible JSON specification of the Vox grammar.
pub fn emit_x_grammar_2() -> String {
    let ebnf = crate::ebnf::emit_ebnf();
    let grammar = Grammar::from_ebnf(&ebnf).expect("Authoritative EBNF must be valid for internal grammar parser");
    
    serde_json::to_string_pretty(&grammar).unwrap_or_else(|e| {
        format!("{{ \"error\": \"Failed to serialize grammar to XGrammar-2 JSON: {}\" }}", e)
    })
}
