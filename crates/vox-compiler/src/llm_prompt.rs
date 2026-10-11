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

/// Interim training-syntax fingerprint: `(sha256 of the emitted EBNF, "ebnf-sha256")`.
///
/// D-06: stands in until the Phase 13 behavioral fingerprint lands; the kind string lets
/// readers tell the two apart.
pub fn grammar_fingerprint() -> (String, &'static str) {
    (
        vox_grammar_export::versioning::compute_ebnf_hash(),
        "ebnf-sha256",
    )
}

#[cfg(test)]
mod tests {
    use super::grammar_fingerprint;

    #[test]
    fn grammar_fingerprint_is_stable_hex() {
        let (fp, kind) = grammar_fingerprint();
        assert_eq!(kind, "ebnf-sha256");
        assert_eq!(fp.len(), 64);
        assert!(fp.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
        assert_eq!(fp, grammar_fingerprint().0);
        assert_eq!(fp, vox_grammar_export::versioning::compute_ebnf_hash());
    }
}
