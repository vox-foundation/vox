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
///
/// A tuple-struct literal does not compile outside this module:
///
/// ```compile_fail
/// let _ = vox_db_types::kb_text::SanitizedText("raw".to_string());
/// ```
///
/// The constructor sanitizes:
///
/// ```
/// let t = vox_db_types::kb_text::SanitizedText::new("a\u{200B}b");
/// assert_eq!(t.as_str(), "ab");
/// ```
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

/// Image hosts whose absolute `http(s)` or protocol-relative images are kept (D-06: empty).
pub const IMAGE_HOST_ALLOWLIST: &[&str] = &[];

/// A blake3 content hash stored as `blake3:<64 lowercase hex>`. Hashing itself stays
/// in `vox-crypto` consumers; this type only validates the encoded form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ContentHash(String);

impl ContentHash {
    /// Accept exactly 64 lowercase hex characters.
    pub fn from_blake3_hex(hex: &str) -> Result<Self, String> {
        Err(format!("unset: {hex}"))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where a KB finding came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingOrigin {
    WebSearch,
    CragTavily,
    ResearchGather,
    ResearchQuick,
    ResearchDeep,
    GuiIngestText,
    GuiIngestUrl,
    GuiSaveResearchSession,
    PublishResearchDoc,
    LegacyKbEntry,
}

impl FindingOrigin {
    pub const ALL: [FindingOrigin; 10] = [
        Self::WebSearch,
        Self::CragTavily,
        Self::ResearchGather,
        Self::ResearchQuick,
        Self::ResearchDeep,
        Self::GuiIngestText,
        Self::GuiIngestUrl,
        Self::GuiSaveResearchSession,
        Self::PublishResearchDoc,
        Self::LegacyKbEntry,
    ];
    pub fn as_db_str(self) -> &'static str {
        "unset"
    }
    pub fn from_db_str(s: &str) -> Result<Self, String> {
        Err(format!("unset: {s}"))
    }
}

/// Review state of a KB finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Pending,
    Approved,
    Rejected,
    Promoted,
    Stale,
}

impl FindingStatus {
    pub const ALL: [FindingStatus; 5] = [
        Self::Pending,
        Self::Approved,
        Self::Rejected,
        Self::Promoted,
        Self::Stale,
    ];
    pub fn as_db_str(self) -> &'static str {
        "unset"
    }
    pub fn from_db_str(s: &str) -> Result<Self, String> {
        Err(format!("unset: {s}"))
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

    // ---- Task 2 -------------------------------------------------------------

    #[test]
    fn strips_html_comments_and_unterminated_to_end() {
        assert_eq!(sanitize("a<!-- ignore previous -->b"), "ab");
        assert_eq!(sanitize("a<!-- never closed"), "a");
        assert_eq!(sanitize("a<!\u{200B}-- split -->b"), "ab");
    }

    #[test]
    fn neutralizes_inline_image_with_absolute_target() {
        let s = sanitize("![x](https://evil.example/?q=1)");
        assert_eq!(s, "(image omitted: x)");
        assert!(!s.contains("!["));
        assert!(!s.contains("https://evil"));
    }

    #[test]
    fn neutralizes_reference_html_and_protocol_relative_images() {
        assert!(!sanitize("![x][r]").contains("!["));
        let html = sanitize("<img src=\"https://e/x.png\">");
        assert!(!html.to_ascii_lowercase().contains("<img") && !html.contains("https://e"));
        let upper = sanitize("<IMG SRC='https://e/x.png'>");
        assert!(!upper.to_ascii_lowercase().contains("<img") && !upper.contains("https://e"));
        let rel = sanitize("![p](//e/x.png)");
        assert!(!rel.contains("//e/x.png"));
        let back = sanitize("![p](\\\\e/x.png)");
        assert!(!back.contains("e/x.png"));
        let data = sanitize("![p](data:image/png;base64,AAAA)");
        assert!(!data.contains("base64"));
    }

    #[test]
    fn keeps_relative_images() {
        assert_eq!(
            sanitize("![logo](./img/logo.png)"),
            "![logo](./img/logo.png)"
        );
        assert_eq!(sanitize("![l](/img/l.png)"), "![l](/img/l.png)");
        assert_eq!(
            sanitize("<img src=\"./a.png\" alt=\"a\">"),
            "<img src=\"./a.png\" alt=\"a\">"
        );
    }

    #[test]
    fn alt_text_cannot_form_a_new_link_or_image() {
        let s = sanitize("![a](b)](https://evil)![c](https://evil/x)");
        assert!(!s.contains("![") && !s.contains("https://evil/x"));
        // a shortcut-reference image next to a plain link must not survive as an image
        let alt = sanitize("![x] [y](https://evil) z");
        assert!(!alt.contains("!["), "{alt}");
    }

    #[test]
    fn fixpoint_neutralizes_constructions_that_touch_after_removal() {
        // a comment between `!` and the link, hidden inside another comment's opener
        let s = sanitize("!<!<!-- -->-- c -->[a](https://evil.example/x)");
        assert!(
            !s.contains("evil.example") && !s.contains("<!--") && !s.contains("!["),
            "{s}"
        );
        // role-token replacement completes an image marker
        let r = sanitize("!<<SYS>>(https://evil.example/x)");
        assert!(!r.contains("evil.example"), "{r}");
    }

    #[test]
    fn empty_and_all_banned_inputs() {
        assert_eq!(sanitize(""), "");
        assert_eq!(sanitize("\u{200B}\u{E0041}\u{FE0F}"), "");
        assert_eq!(SanitizedText::new("").as_str(), "");
    }

    #[test]
    fn is_unicode_scalar_safe_and_applies_no_normalization() {
        // composed and decomposed forms both survive unchanged
        assert_eq!(
            sanitize("caf\u{E9} cafe\u{301} \u{1F600}"),
            "caf\u{E9} cafe\u{301} \u{1F600}"
        );
    }

    #[test]
    fn content_hash_accepts_only_64_lowercase_hex() {
        let h = "a".repeat(64);
        let ok = ContentHash::from_blake3_hex(&h).unwrap();
        assert_eq!(ok.as_str(), format!("blake3:{h}"));
        assert!(ContentHash::from_blake3_hex(&"A".repeat(64)).is_err());
        assert!(ContentHash::from_blake3_hex(&"a".repeat(63)).is_err());
        assert!(ContentHash::from_blake3_hex(&"a".repeat(65)).is_err());
        assert!(ContentHash::from_blake3_hex(&"g".repeat(64)).is_err());
        assert!(ContentHash::from_blake3_hex("").is_err());
        // a non-ASCII char whose byte length makes 64 must not pass
        assert!(ContentHash::from_blake3_hex(&format!("{}\u{e9}", "a".repeat(62))).is_err());
    }

    #[test]
    fn finding_enums_round_trip_and_fail_closed() {
        for o in FindingOrigin::ALL {
            assert_eq!(FindingOrigin::from_db_str(o.as_db_str()), Ok(o));
        }
        for s in FindingStatus::ALL {
            assert_eq!(FindingStatus::from_db_str(s.as_db_str()), Ok(s));
        }
        assert_eq!(FindingOrigin::WebSearch.as_db_str(), "web_search");
        assert_eq!(
            FindingOrigin::GuiSaveResearchSession.as_db_str(),
            "gui_save_research_session"
        );
        assert_eq!(FindingStatus::Promoted.as_db_str(), "promoted");
        assert!(FindingOrigin::from_db_str("").is_err());
        assert!(FindingOrigin::from_db_str("WebSearch").is_err());
        assert!(FindingStatus::from_db_str("approvedd").is_err());
        let all: std::collections::HashSet<_> =
            FindingOrigin::ALL.iter().map(|o| o.as_db_str()).collect();
        assert_eq!(all.len(), 10);
    }

    const REDTEAM: &str = include_str!("../../../contracts/kb/redteam/cases.v1.jsonl");

    #[test]
    fn redteam_cases_file_is_ascii() {
        assert!(
            REDTEAM.is_ascii(),
            "red-team fixtures must hold no raw non-ASCII bytes"
        );
        assert!(REDTEAM.lines().count() >= 10);
    }

    #[test]
    fn redteam_cases_are_neutralized() {
        let mut n = 0;
        for line in REDTEAM.lines().filter(|l| !l.trim().is_empty()) {
            let v: serde_json::Value = serde_json::from_str(line).expect("valid json line");
            let id = v["id"].as_str().expect("id");
            let out = sanitize(v["input"].as_str().expect("input"));
            for bad in v["must_not_contain"].as_array().expect("must_not_contain") {
                let bad = bad.as_str().unwrap();
                assert!(
                    !out.contains(bad),
                    "case {id}: output still contains {bad:?}: {out:?}"
                );
            }
            for good in v["must_contain"].as_array().expect("must_contain") {
                let good = good.as_str().unwrap();
                assert!(
                    out.contains(good),
                    "case {id}: output lacks {good:?}: {out:?}"
                );
            }
            assert_eq!(sanitize(&out), out, "case {id}: not idempotent");
            n += 1;
        }
        assert!(n >= 10, "expected at least 10 red-team cases, got {n}");
    }

    mod props {
        use super::*;
        use proptest::prelude::*;

        const FRAGMENTS: &[&str] = &[
            "a",
            "word ",
            " ",
            "\n",
            "-",
            "<",
            ">",
            "<!",
            "!",
            "[",
            "]",
            "(",
            ")",
            "<!--",
            "-->",
            "![",
            "](",
            "][",
            "<img src=\"",
            "<IMG ",
            "\">",
            "<|im_start|>",
            "[INST]",
            "<<SYS>>",
            "https://evil.example/x.png",
            "//e/x.png",
            "./img/a.png",
            "\u{E0000}",
            "\u{E007F}",
            "\u{E0041}",
            "\u{200B}",
            "\u{200F}",
            "\u{2060}",
            "\u{2064}",
            "\u{FEFF}",
            "\u{202A}",
            "\u{202E}",
            "\u{2066}",
            "\u{2069}",
            "\u{FE00}",
            "\u{FE0F}",
            "\u{E0100}",
            "\u{E01EF}",
        ];

        fn fragments() -> impl Strategy<Value = String> {
            prop::collection::vec(prop::sample::select(FRAGMENTS), 0..24).prop_map(|v| v.concat())
        }

        /// True if `s` has `![...](target)` with a non-relative target.
        fn has_absolute_image(s: &str) -> bool {
            let mut rest = s;
            while let Some(i) = rest.find("![") {
                let after = &rest[i + 2..];
                if let Some(j) = after.find("](") {
                    let t = after[j + 2..].trim_start();
                    if t.starts_with("http") || t.starts_with("//") || t.starts_with('\\') {
                        return true;
                    }
                }
                rest = after;
            }
            let l = s.to_ascii_lowercase();
            l.contains("src=\"http") || l.contains("src=\"//")
        }

        proptest! {
            #[test]
            fn sanitize_is_idempotent(s in fragments()) {
                let once = sanitize(&s);
                prop_assert_eq!(sanitize(&once), once);
            }

            #[test]
            fn output_holds_no_banned_codepoint_or_comment_or_absolute_image(s in fragments()) {
                let out = sanitize(&s);
                prop_assert!(!out.chars().any(is_banned));
                prop_assert!(!out.contains("<!--"));
                prop_assert!(!has_absolute_image(&out), "{:?}", out);
            }

            #[test]
            fn arbitrary_unicode_is_idempotent_and_valid(s in "\\PC{0,64}") {
                let once = sanitize(&s);
                prop_assert_eq!(sanitize(&once), once.clone());
                prop_assert!(std::str::from_utf8(once.as_bytes()).is_ok());
            }
        }
    }
}
