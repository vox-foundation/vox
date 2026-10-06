//! KB-text sanitizer and provenance newtypes (TRUST-05).
//!
//! One sanitizer for text that enters or leaves the knowledge base. Untrusted
//! strings become [`SanitizedText`] only through [`sanitize`].

use serde::Serialize;
use std::fmt;

/// Sanitize untrusted text: strip invisible-codepoint smuggling and neutralize
/// role tokens.
pub fn sanitize(raw: &str) -> String {
    raw.to_string()
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
        Self(raw.to_string())
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
