//! The only Mermaid text encoder in the crate, plus the pure id-keep predicate.

use std::fmt::Write as _;

/// Ids Mermaid rejects, or silently mishandles, wherever they appear (lowercase compare;
/// a deliberate superset because `End` and `END` are accepted by Mermaid).
const KEYWORDS: [&str; 14] = [
    "end",
    "graph",
    "flowchart",
    "subgraph",
    "style",
    "class",
    "classdef",
    "linkstyle",
    "click",
    "default",
    "interpolate",
    "call",
    "href",
    "direction",
];

/// Encode `s` for a quoted Mermaid label (node, edge or subgraph title).
pub(crate) fn label(s: &str) -> String {
    let colon = s.contains("style") || s.contains("classDef");
    let percent = s.contains("%%");
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("#amp;"),
            '"' => out.push_str("#quot;"),
            '<' => out.push_str("#lt;"),
            '>' => out.push_str("#gt;"),
            '#' => out.push_str("#35;"),
            '`' => out.push_str("#96;"),
            '\n' => out.push_str("<br>"),
            '%' if percent => out.push_str("#37;"),
            ':' if colon => out.push_str("#58;"),
            '\u{0}'..='\u{1f}' | '\u{7f}' | '\u{2028}' | '\u{2029}' | '\u{fb02}' | '\u{b6}' => {
                let _ = write!(out, "#{};", u32::from(c));
            }
            c => out.push(c),
        }
    }
    if out.trim().is_empty() {
        out = " ".to_owned();
    }
    out
}

/// True when `id` can be written into Mermaid text unchanged (see the 84-01 id policy).
pub(crate) fn keeps_id(id: &str) -> bool {
    let mut chars = id.chars();
    let shaped = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !shaped {
        return false;
    }
    let lower = id.to_ascii_lowercase();
    let generated = lower == id
        && id
            .strip_prefix('n')
            .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()));
    !(KEYWORDS.contains(&lower.as_str())
        || lower.contains("style")
        || lower.contains("classdef")
        || id.starts_with(['o', 'x'])
        || generated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_encoding_row() {
        assert_eq!(label("a&b"), "a#amp;b");
        assert_eq!(label("say \"hi\""), "say #quot;hi#quot;");
        assert_eq!(label("<b>"), "#lt;b#gt;");
        assert_eq!(label("#quot;"), "#35;quot;");
        assert_eq!(label("`x`"), "#96;x#96;");
        assert_eq!(label("a\nb"), "a<br>b");
    }

    #[test]
    fn percent_only_with_double_percent() {
        assert_eq!(label("100%"), "100%");
        assert_eq!(label("%%{init}"), "#37;#37;{init}");
    }

    #[test]
    fn colon_only_with_style_or_classdef() {
        assert_eq!(label("a: b"), "a: b");
        assert_eq!(label("style: x"), "style#58; x");
        assert_eq!(label("classDef: x"), "classDef#58; x");
    }

    #[test]
    fn controls_and_private_placeholders_are_numeric() {
        assert_eq!(label("a\tb"), "a#9;b");
        assert_eq!(label("a\u{1}b"), "a#1;b");
        assert_eq!(label("a\u{7f}b"), "a#127;b");
        assert_eq!(label("a\u{2028}b"), "a#8232;b");
        assert_eq!(label("\u{fb02}"), "#64258;");
        assert_eq!(label("a\u{b6}b"), "a#182;b");
    }

    #[test]
    fn empty_and_blank_become_one_space() {
        assert_eq!(label(""), " ");
        assert_eq!(label("   "), " ");
    }

    #[test]
    fn keeps_id_accepts_plain_ids() {
        for id in ["a", "Api_2", "start", "_x", "Order"] {
            assert!(keeps_id(id), "{id}");
        }
    }

    #[test]
    fn keeps_id_rejects_hazards() {
        for id in [
            "End", "end", "click", "classDef", "o1", "x", "n12", "a-b", "my_style", "ü", "", "1a",
            "Default", "graph",
        ] {
            assert!(!keeps_id(id), "{id}");
        }
    }
}
