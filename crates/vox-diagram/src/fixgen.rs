//! Pure builders for machine-applicable fixes shared by the lexer, parser and resolver.
//! Everything here returns text that is valid by construction (escaped labels, validated
//! ASCII identifiers), so applying a fix can never introduce an injection.

use crate::diag::Fix;
use crate::lexer::is_forbidden;
use crate::limits::MAX_LABEL_CHARS;
use crate::resolve::RESERVED;
use crate::span::{Span, to_u32};

/// A fix replacing `span` with `replacement`.
pub(crate) fn replace(span: Span, replacement: impl Into<String>) -> Fix {
    Fix {
        span,
        replacement: replacement.into(),
    }
}

/// `text` as a double-quoted label: `\`, `"` and newline escaped, `\r` and forbidden characters
/// (which a label cannot hold) dropped.
pub(crate) fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            c if is_forbidden(c) => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A Mermaid label as `(decoded text, canonical quoted spelling)`: trimmed, surrounding
/// double quotes stripped, `<br>` and `<br/>` turned into line breaks (the `\n` escape in the
/// quoted form), then escaped like [`quote`].
pub(crate) fn mermaid_label(raw: &str) -> (String, String) {
    let t = raw.trim();
    let t = t
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .unwrap_or(t);
    let t = t
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n");
    let decoded: String = t
        .chars()
        .filter(|&c| c != '\r' && !is_forbidden(c))
        .collect();
    let lines: Vec<String> = decoded
        .split('\n')
        .map(|l| {
            let q = quote(l);
            q.get(1..q.len() - 1).unwrap_or_default().to_string()
        })
        .collect();
    let quoted = format!("\"{}\"", lines.join("\\n"));
    (decoded, quoted)
}

/// The leading blanks of the line that contains byte `at`.
pub(crate) fn indent_of(src: &str, at: u32) -> String {
    let before = src.get(..at as usize).unwrap_or_default();
    let line = before.rfind('\n').map_or(0, |i| i + 1);
    let text = src.get(line..).unwrap_or_default();
    text.chars()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .collect()
}

/// True when `name` can be declared as a node id without raising another diagnostic: an
/// ASCII identifier, not a keyword, short enough to be its own label.
pub(crate) fn declarable(name: &str) -> bool {
    name.len() <= MAX_LABEL_CHARS
        && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        && !RESERVED.contains(&name)
}

/// Rule R: the insert that declares `node <name>: "<name>"` inside the scope whose braces are
/// `open` and `close`. Multi-line scope: at the end of the `{` line, so the declaration is the
/// scope's first statement. Single-line scope: right after the `{`, with a trailing newline.
/// `None` when `name` is not safely declarable.
pub(crate) fn declare_node(src: &str, open: Span, close: Span, name: &str) -> Option<Fix> {
    if !declarable(name) {
        return None;
    }
    insert_decl(src, open, close, &format!("node {name}: \"{name}\""))
}

/// Rule R for an arbitrary declaration line `decl` (see [`declare_node`]).
pub(crate) fn insert_decl(src: &str, open: Span, close: Span, decl: &str) -> Option<Fix> {
    let (after_open, before_close) = (open.end as usize, close.start as usize);
    let indent = indent_of(src, open.start);
    let (at, replacement) = match src.get(after_open..before_close)?.find('\n') {
        None => (after_open, format!("\n{indent}    {decl}\n")),
        Some(nl) => {
            let mut eol = after_open + nl;
            if src.get(..eol)?.ends_with('\r') {
                eol -= 1;
            }
            (eol, format!("\n{indent}    {decl}"))
        }
    };
    let at = to_u32(at);
    Some(replace(Span { start: at, end: at }, replacement))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_escapes_and_drops_unrepresentable_chars() {
        assert_eq!(quote("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(quote("x\ry\u{202e}z\tw"), "\"xyz\tw\"");
        assert_eq!(quote(""), "\"\"");
        assert_eq!(quote("x\ny"), "\"x\\ny\"");
    }

    #[test]
    fn mermaid_label_decodes_and_escapes() {
        let q = |raw: &str| mermaid_label(raw);
        assert_eq!(q("  Plain  "), ("Plain".into(), "\"Plain\"".into()));
        assert_eq!(q("\"Quoted\""), ("Quoted".into(), "\"Quoted\"".into()));
        assert_eq!(
            q("a<br>b<br/>c"),
            ("a\nb\nc".into(), "\"a\\nb\\nc\"".into())
        );
        assert_eq!(
            q("say \"hi\" \\"),
            ("say \"hi\" \\".into(), "\"say \\\"hi\\\" \\\\\"".into())
        );
        assert_eq!(q(""), (String::new(), "\"\"".into()));
    }

    #[test]
    fn declare_node_refuses_unsafe_names() {
        let (o, c) = (Span { start: 6, end: 7 }, Span { start: 8, end: 9 });
        let src = "graph {\n}\n";
        assert!(declare_node(src, o, c, "ghost").is_some());
        for bad in [
            "9x",
            "caf\u{e9}",
            "node",
            "",
            "a.b",
            &"x".repeat(MAX_LABEL_CHARS + 1),
        ] {
            assert!(declare_node(src, o, c, bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn crlf_scope_inserts_before_the_carriage_return() {
        let src = "graph {\r\n    a -> g\r\n}\r\n";
        let (o, c) = (Span { start: 6, end: 7 }, Span { start: 24, end: 25 });
        let f = declare_node(src, o, c, "g").map(|f| (f.span.start, f.replacement));
        assert_eq!(f, Some((7, "\n    node g: \"g\"".to_string())));
    }
}
