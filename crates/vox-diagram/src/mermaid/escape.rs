//! The only Mermaid text encoder in the crate, plus the pure id-keep predicate.

use std::fmt::Write as _;

/// Ids Mermaid rejects, or silently mishandles, wherever they appear (lowercase compare;
/// a deliberate superset because `End` and `END` are accepted by Mermaid). `_self`, `_blank`,
/// `_parent` and `_top` are the link targets of `click`, and fail as ids.
const KEYWORDS: [&str; 18] = [
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
    "_self",
    "_blank",
    "_parent",
    "_top",
];

/// Mermaid's direction words, lowercase.
const DIRECTION_WORDS: [&str; 5] = ["tb", "td", "lr", "rl", "bt"];

fn is_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

/// Byte offsets of the `d` of every `direction` that Mermaid's flowchart/state lexer reads as a
/// direction statement: `direction`, whitespace, then `TB`, `TD`, `LR`, `RL` or `BT`. The lexer
/// swallows the whole line, quoted label or not. Compared case-insensitively (a deliberate
/// superset). Shared by the exporter (defuse) and the importer (refuse).
pub(crate) fn direction_traps(s: &str) -> Vec<usize> {
    let lower = s.to_ascii_lowercase();
    lower
        .match_indices("direction")
        .filter(|(i, w)| {
            let tail = &lower[i + w.len()..];
            let rest = tail.trim_start_matches(is_space);
            rest.len() < tail.len() && DIRECTION_WORDS.iter().any(|t| rest.starts_with(t))
        })
        .map(|(i, _)| i)
        .collect()
}

/// True when `s` holds a [`direction_traps`] hit.
pub(crate) fn has_direction_trap(s: &str) -> bool {
    !direction_traps(s).is_empty()
}

/// True when `line` ends in `direction` and `next` (the next non-blank line) starts with a
/// direction word: the lexer's whitespace runs over the newline, so it is one statement.
pub(crate) fn direction_spans_lines(line: &str, next: &str) -> bool {
    let end = line.trim_end_matches(is_space).to_ascii_lowercase();
    let start = next.trim_start_matches(is_space).to_ascii_lowercase();
    end.ends_with("direction") && DIRECTION_WORDS.iter().any(|t| start.starts_with(t))
}

/// Shared encoder. `seq` adds the sequence-diagram rules: `;` ends a statement there, and a text
/// starting with `wrap:`, `nowrap:`, `:wrap:` or `:nowrap:` is stripped and changes wrapping.
/// Flowchart text (`!seq`) also numerically encodes the `d` of a [`direction_traps`] hit.
fn encode(s: &str, seq: bool) -> String {
    let colon = s.contains("style") || s.contains("classDef");
    let percent = s.contains("%%");
    let trimmed = s.trim_start();
    let wrap_at = (seq
        && ["wrap:", "nowrap:", ":wrap:", ":nowrap:"]
            .iter()
            .any(|p| trimmed.starts_with(p)))
    .then(|| s.len() - trimmed.len());
    let traps = if seq { Vec::new() } else { direction_traps(s) };
    let mut out = String::with_capacity(s.len());
    for (at, c) in s.char_indices() {
        if wrap_at == Some(at) || traps.contains(&at) {
            let _ = write!(out, "#{};", u32::from(c));
            continue;
        }
        match c {
            '&' => out.push_str("#amp;"),
            '"' => out.push_str("#quot;"),
            '<' => out.push_str("#lt;"),
            '>' => out.push_str("#gt;"),
            '#' => out.push_str("#35;"),
            '`' => out.push_str("#96;"),
            '\n' => out.push_str("<br>"),
            ';' if seq => out.push_str("#59;"),
            '%' if percent => out.push_str("#37;"),
            ':' if colon => out.push_str("#58;"),
            '\u{0}'..='\u{1f}' | '\u{7f}' | '\u{2028}' | '\u{2029}' | '\u{fb02}' | '\u{b6}' => {
                let _ = write!(out, "#{};", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out
}

/// Encode `s` for a quoted Mermaid label (node, edge or subgraph title). Blank becomes one space.
pub(crate) fn label(s: &str) -> String {
    let out = encode(s, false);
    if out.trim().is_empty() {
        " ".to_owned()
    } else {
        out
    }
}

/// Encode `s` as unquoted sequence-diagram text (actor alias, message or note). Blank text stays
/// blank; the exporter decides how to print it.
pub(crate) fn seq_text(s: &str) -> String {
    encode(s, true)
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
    // A line that ends in `direction` and a next line that starts `LR` (or `TB`, ...) are one
    // statement to the flowchart lexer; an id is the only unquoted text that can end a line.
    !(KEYWORDS.contains(&lower.as_str())
        || lower.ends_with("direction")
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
    fn direction_statement_text_is_defused_in_flowchart_labels_only() {
        assert_eq!(label("p direction LR q"), "p #100;irection LR q");
        assert_eq!(label("go Direction td"), "go #68;irection td");
        assert_eq!(label("xdirection BT"), "x#100;irection BT");
        assert_eq!(label("direction LRX"), "#100;irection LRX");
        assert_eq!(label("direction"), "direction");
        assert_eq!(label("direction XY"), "direction XY");
        // Sequence text has no `direction` statement.
        assert_eq!(seq_text("direction LR"), "direction LR");
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

    #[test]
    fn keeps_id_rejects_link_targets_and_direction_endings() {
        for id in [
            "_self",
            "_blank",
            "_parent",
            "_top",
            "_Self",
            "adirection",
            "my_Direction",
        ] {
            assert!(!keeps_id(id), "{id}");
        }
        for id in ["_selfie", "top", "direction_x", "dir"] {
            assert!(keeps_id(id), "{id}");
        }
    }

    #[test]
    fn seq_text_encodes_semicolons_and_hashes() {
        assert_eq!(seq_text("a; b"), "a#59; b");
        assert_eq!(seq_text("a # b"), "a #35; b");
        assert_eq!(
            seq_text("say \"hi\" & <b>"),
            "say #quot;hi#quot; #amp; #lt;b#gt;"
        );
        assert_eq!(seq_text("a\nb"), "a<br>b");
        assert_eq!(seq_text(""), "");
    }

    #[test]
    fn seq_text_defuses_wrap_prefixes() {
        assert_eq!(seq_text("wrap: x"), "#119;rap: x");
        assert_eq!(seq_text("nowrap: x"), "#110;owrap: x");
        assert_eq!(seq_text(":wrap: x"), "#58;wrap: x");
        assert_eq!(seq_text(":nowrap:x"), "#58;nowrap:x");
        assert_eq!(seq_text("  wrap: x"), "  #119;rap: x");
        // Only a prefix, only at the start, only exact case.
        assert_eq!(seq_text("a wrap: x"), "a wrap: x");
        assert_eq!(seq_text("Wrap: x"), "Wrap: x");
        assert_eq!(seq_text("wrapper"), "wrapper");
    }
}
