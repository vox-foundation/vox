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

/// `text` as a double-quoted label: `\` and `"` escaped, `\r` and forbidden characters
/// (which a label cannot hold) dropped.
pub(crate) fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\r' => {}
            c if is_forbidden(c) => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// True when `name` can be declared as a node id without raising another diagnostic: an
/// ASCII identifier, not a keyword, short enough to be its own label.
fn declarable(name: &str) -> bool {
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
    let (after_open, before_close) = (open.end as usize, close.start as usize);
    let line_start = src
        .get(..open.start as usize)?
        .rfind('\n')
        .map_or(0, |i| i + 1);
    let indent: String = src
        .get(line_start..)?
        .chars()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .collect();
    let decl = format!("node {name}: \"{name}\"");
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
