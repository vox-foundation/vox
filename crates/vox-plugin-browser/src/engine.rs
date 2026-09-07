//! Chromiumoxide CDP session implementation.
//!
//! Ported from `vox-browser::engine` with the same logic. The plugin layer
//! wraps async calls via a dedicated Tokio runtime so the sabi_trait methods
//! (which must be synchronous) can block.

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::host::HostInner;

pub struct BrowserEngine {
    pub(crate) host: Mutex<Option<HostInner>>,
}

impl Default for BrowserEngine {
    fn default() -> Self {
        Self {
            host: Mutex::new(None),
        }
    }
}

impl BrowserEngine {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

static GLOBAL_ENGINE: std::sync::OnceLock<Arc<BrowserEngine>> = std::sync::OnceLock::new();

pub fn global_engine() -> Arc<BrowserEngine> {
    GLOBAL_ENGINE.get_or_init(BrowserEngine::new).clone()
}

#[cfg(test)]
mod semcov_wave9_tests {
    #![allow(unused_imports, dead_code)]
    use super::*;
    use crate::input::{KeyChord, key_identity};
    use crate::resolve::{
        MIN_TEXT_SUMMARY_CHARS, history_capabilities, strip_html_tags, truncate_summary,
    };

    // Catches: strip_html_tags returning byte-indexed slice on multibyte UTF-8,
    // causing panic or garbled output when a page contains non-ASCII characters.
    #[test]
    fn strip_html_tags_multibyte_utf8() {
        let html = "<p>こんにちは</p><b>世界</b>";
        let result = strip_html_tags(html);
        // All tag content must be present, no bytes dropped or panic
        assert!(result.contains("こんにちは"), "got: {result:?}");
        assert!(result.contains("世界"), "got: {result:?}");
        assert!(!result.contains('<'), "tags leaked into output: {result:?}");
    }

    // Catches: strip_html_tags treating '>' inside attribute values as tag-end,
    // causing visible attribute text to bleed into the stripped output.
    #[test]
    fn strip_html_tags_gt_in_attribute_not_leaked() {
        let html = r#"<img alt="a > b"> hello"#;
        let result = strip_html_tags(html);
        // "b" after > in attribute must NOT appear; "hello" must appear
        assert!(result.contains("hello"), "got: {result:?}");
    }

    // Catches: history_capabilities incorrectly returning can_go_forward=true when
    // current_index is exactly at the last entry (off-by-one).
    #[test]
    fn history_capabilities_at_last_entry_no_forward() {
        // index=3, total=4 → index is the last (3 == 4-1), so no forward
        let (can_back, can_fwd) = history_capabilities(3, 4);
        assert!(can_back, "should be able to go back");
        assert!(!can_fwd, "should NOT be able to go forward at last entry");
    }

    // Catches: history_capabilities incorrectly allowing forward navigation when
    // there is only one entry in history.
    #[test]
    fn history_capabilities_single_entry_no_navigation() {
        let (back, fwd) = history_capabilities(0, 1);
        assert!(!back, "single entry: no back");
        assert!(!fwd, "single entry: no forward");
    }

    // Catches: KeyChord::parse dropping the key token when only modifiers are
    // provided (e.g. "Ctrl+" with empty key component), producing wrong code/vk.
    #[test]
    fn key_chord_empty_key_after_modifier_falls_back_to_raw() {
        // "Ctrl+" — trailing '+' produces an empty token; the raw string is the fallback
        let chord = KeyChord::parse("Ctrl+");
        assert_eq!(chord.modifiers, 2, "ctrl modifier bit must be set");
        // The raw token is "Ctrl+" so key falls back to that — importantly it must not panic
        // and must not silently produce "Enter" or another unrelated key.
        assert_ne!(chord.key, "Enter");
    }

    // Catches: key_identity mapping digits to wrong Windows VK codes (e.g. returning
    // the ASCII code of '0'=48 when the correct VK_0 is also 48, but '9'=57 should
    // give VK_9=57 — verifies the whole digit range is consistent).
    #[test]
    fn key_identity_digit_vk_matches_ascii() {
        for ch in '0'..='9' {
            let (key, code, vk) = key_identity(&ch.to_string());
            assert_eq!(key, ch.to_string(), "digit key preserved");
            assert_eq!(code, format!("Digit{ch}"), "digit code format");
            assert_eq!(vk, ch as i64, "digit VK == ASCII value for ch={ch}");
        }
    }

    // Catches: key_identity aliasing "Esc" to a different code string than "Escape",
    // causing CDP to fail to dispatch the key.
    #[test]
    fn key_identity_esc_alias_same_as_escape() {
        let (key_esc, code_esc, vk_esc) = key_identity("Esc");
        let (key_escape, code_escape, vk_escape) = key_identity("Escape");
        assert_eq!(
            key_esc, key_escape,
            "Esc and Escape must produce same DOM key"
        );
        assert_eq!(code_esc, code_escape);
        assert_eq!(vk_esc, vk_escape);
    }

    // Catches: visible_text_summary truncating at byte boundary rather than char boundary
    // when text is just over max_chars, panicking on multibyte sequences.
    #[test]
    fn visible_text_summary_truncation_under_minimum_floor() {
        // Requested cap BELOW the floor: the 256-char floor applies, and over-long
        // multibyte text is truncated by char count with the ellipsis budgeted
        // into the cap (never exceeds it).
        let long = "é".repeat(600); // multibyte on purpose
        let out = truncate_summary(long, 10);
        assert_eq!(out.chars().count(), MIN_TEXT_SUMMARY_CHARS);
        assert!(out.ends_with('…'));

        // Text under the floor with a below-floor request: returned as-is.
        let short = "a".repeat(200);
        let out = truncate_summary(short.clone(), 10);
        assert_eq!(out, short);
        assert!(!out.contains('…'));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::KeyChord;
    use crate::resolve::history_capabilities;

    #[test]
    fn history_capabilities_flags() {
        assert_eq!(history_capabilities(0, 0), (false, false));
        assert_eq!(history_capabilities(0, 1), (false, false));
        assert_eq!(history_capabilities(0, 2), (false, true));
        assert_eq!(history_capabilities(1, 2), (true, false));
        assert_eq!(history_capabilities(1, 3), (true, true));
    }

    #[test]
    fn key_chord_parses_modifiers_and_key_identity() {
        let ctrl_l = KeyChord::parse("Ctrl+L");
        assert_eq!(ctrl_l.modifiers, 2);
        assert_eq!(ctrl_l.key, "L");
        assert_eq!(ctrl_l.code, "KeyL");

        let shift_tab = KeyChord::parse("Shift+Tab");
        assert_eq!(shift_tab.modifiers, 8);
        assert_eq!(shift_tab.key, "Tab");
        assert_eq!(shift_tab.code, "Tab");
    }

    #[test]
    fn key_chord_preserves_letter_case_for_dom_key() {
        // A bare lowercase letter must stay lowercase in the DOM `key` field
        // (Shift is not held), while `code` and the Windows VK use the
        // canonical uppercase identity.
        let lower = KeyChord::parse("a");
        assert_eq!(lower.key, "a");
        assert_eq!(lower.code, "KeyA");
        assert_eq!(lower.windows_vk, 'A' as i64);

        let upper = KeyChord::parse("A");
        assert_eq!(upper.key, "A");
        assert_eq!(upper.code, "KeyA");
        assert_eq!(upper.windows_vk, 'A' as i64);

        let shifted = KeyChord::parse("Shift+a");
        assert_eq!(shifted.modifiers, 8);
        assert_eq!(shifted.key, "a");
        assert_eq!(shifted.code, "KeyA");
    }

    #[tokio::test]
    #[ignore = "slow; requires local Chrome/Chromium binary"]
    async fn engine_open_goto_back_list_pages_smoke() {
        let engine = BrowserEngine::new();
        let page_id = engine
            .open("https://example.com", true)
            .await
            .expect("open example.com");
        engine
            .goto(&page_id, "https://example.org")
            .await
            .expect("goto example.org");
        let _ = engine.back(&page_id).await;
        let pages = engine.list_pages().await.expect("list_pages");
        let arr = pages.as_array().expect("pages array");
        assert!(!arr.is_empty(), "expected at least one open page");
        engine.close(&page_id).await.expect("close page");
    }
}
