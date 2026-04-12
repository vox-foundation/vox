/// Return the Vox grammar prompt for LLM consumption.
///
/// Delegates to `vox-grammar-export` compact prompt — the single source of truth
/// derived from the authoritative EBNF grammar (57 productions).
///
/// **Research rationale (Grammar Constraints §K-complexity):** Reducing the token
/// count of the grammar prompt fed to the LLM reduces hallucination surface.
pub fn vox_grammar_prompt() -> String {
    vox_grammar_export::compact_prompt::emit_compact_llm_prompt()
}
