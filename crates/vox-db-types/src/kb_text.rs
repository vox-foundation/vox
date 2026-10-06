//! KB-text sanitizer and provenance newtypes (TRUST-05).
//!
//! One sanitizer for text that enters or leaves the knowledge base. Untrusted
//! strings become [`SanitizedText`] only through [`sanitize`].

use serde::Serialize;
use std::fmt;

/// True for codepoints used to smuggle hidden instructions: Unicode tag block,
/// zero-width, invisible operators, bidi controls and variation selectors (D-06).
fn is_banned(c: char) -> bool {
    matches!(c,
        '\u{E0000}'..='\u{E007F}'
        | '\u{200B}'..='\u{200F}'
        | '\u{2060}'..='\u{2064}'
        | '\u{FEFF}'
        | '\u{202A}'..='\u{202E}'
        | '\u{2066}'..='\u{2069}'
        | '\u{FE00}'..='\u{FE0F}'
        | '\u{E0100}'..='\u{E01EF}')
}

/// Role-token replacements, applied in order (moved verbatim from the research shim).
const ROLE_TOKENS: &[(&str, &str)] = &[
    ("<|im_start|>", "[im_start]"),
    ("<|im_end|>", "[im_end]"),
    ("[INST]", "[inst_neutralized]"),
    ("[/INST]", "[/inst_neutralized]"),
    ("<<SYS>>", "[sys_neutralized]"),
    ("<</SYS>>", "[/sys_neutralized]"),
    ("<antThinking>", "[ant_thinking]"),
    ("</antThinking>", "[/ant_thinking]"),
    ("<start_of_turn>", "[start_of_turn]"),
    ("<end_of_turn>", "[end_of_turn]"),
    ("<\u{FF5C}User\u{FF5C}>", "[user_neutralized]"),
    ("<\u{FF5C}Assistant\u{FF5C}>", "[assistant_neutralized]"),
];

/// Sanitize untrusted text: strip invisible-codepoint smuggling first (so
/// interleaved evasion characters cannot split a role token), then neutralize
/// role tokens.
pub fn sanitize(raw: &str) -> String {
    let mut out: String = raw.chars().filter(|c| !is_banned(*c)).collect();
    for (from, to) in ROLE_TOKENS {
        out = out.replace(from, to);
    }
    out
}

/// Text that has passed through [`sanitize`]. The field is private and there is
/// no `Default`, `From<String>` or `Deserialize`, so the only way to build one
/// is [`SanitizedText::new`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SanitizedText(String);

impl SanitizedText {
    /// Sanitize `raw` and wrap the result.
    pub fn new(raw: &str) -> Self {
        Self(sanitize(raw))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for SanitizedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tag_block_zero_width_invisible_bidi_and_variation_selectors() {
        let raw = "a\u{E0041}b\u{200B}c\u{2062}d\u{202E}e\u{FE0F}f\u{E0100}g";
        assert_eq!(sanitize(raw), "abcdefg");
    }

    #[test]
    fn neutralizes_role_tokens_with_shim_replacements() {
        let s = sanitize("<|im_start|>x<|im_end|> [INST]y[/INST] <<SYS>>z<</SYS>>");
        assert_eq!(
            s,
            "[im_start]x[im_end] [inst_neutralized]y[/inst_neutralized] [sys_neutralized]z[/sys_neutralized]"
        );
    }

    #[test]
    fn zero_width_split_role_token_is_still_neutralized() {
        assert_eq!(sanitize("<|\u{200B}im_start|>"), "[im_start]");
    }

    #[test]
    fn sanitized_text_runs_the_sanitizer() {
        let t = SanitizedText::new("x\u{E0041}y");
        assert_eq!(t.as_str(), "xy");
        assert_eq!(t.to_string(), "xy");
        assert_eq!(t.into_string(), "xy");
    }
}
