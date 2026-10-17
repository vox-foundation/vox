//! Shared importer text rules: the DSL id rule and label/edge-text validation and decoding.
//! Every rejection except the length, emptiness and forbidden-character ones runs on the RAW
//! text, so the exporter's encoded forms (`#96;`, `#lt;`, `#amp;`, `#35;`) import back as
//! plain text.

use crate::lexer::is_forbidden;
use crate::limits::MAX_LABEL_CHARS;
use crate::resolve::RESERVED;

/// Characters Mermaid's parser rejects in unquoted node, title and pipe-edge text.
pub(super) const NODE_SPECIAL: &str = "()[]{}|\"@";

/// `Ok` when `s` is a legal DSL id; otherwise the construct name.
pub(super) fn valid_id(s: &str) -> Result<(), &'static str> {
    let mut chars = s.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    if !first_ok
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        || s.ends_with('-')
    {
        return Err("invalid-id");
    }
    if RESERVED.contains(&s) {
        return Err("reserved-id");
    }
    Ok(())
}

/// Length in bytes of a `<br>`, `<br/>` or `<br />` (any case) at the start of `s`.
fn br_len(s: &str) -> Option<usize> {
    ["<br>", "<br/>", "<br />"].into_iter().find_map(|p| {
        s.as_bytes()
            .get(..p.len())
            .filter(|b| b.eq_ignore_ascii_case(p.as_bytes()))
            .map(|_| p.len())
    })
}

/// True for `&name;`, `&#digits;` and `&#xhex;` at the start of `s` (after the `&`).
fn html_entity(s: &str) -> bool {
    let (body, numeric) = match s.strip_prefix('#') {
        Some(h) => (h.strip_prefix(['x', 'X']).unwrap_or(h), true),
        None => (s, false),
    };
    let n = body.bytes().take_while(u8::is_ascii_alphanumeric).count();
    let lead = body.bytes().next().is_some_and(|b| {
        if numeric {
            b.is_ascii_hexdigit()
        } else {
            b.is_ascii_alphabetic()
        }
    });
    n > 0 && lead && body[n..].starts_with(';')
}

/// A `#name;` or `#digits;` Mermaid entity right after the `#`; `Err` for an unknown one.
fn entity(after_hash: &str) -> Result<Option<(char, usize)>, &'static str> {
    let letters = after_hash
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .count();
    if letters > 0 && after_hash[letters..].starts_with(';') {
        let c = match &after_hash[..letters] {
            "quot" => '"',
            "lt" => '<',
            "gt" => '>',
            "amp" => '&',
            "apos" => '\'',
            "nbsp" => '\u{a0}',
            _ => return Err("entity"),
        };
        return Ok(Some((c, letters + 1)));
    }
    let digits = after_hash.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && after_hash[digits..].starts_with(';') {
        let c = after_hash[..digits]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .ok_or("entity")?;
        return Ok(Some((c, digits + 1)));
    }
    Ok(None)
}

/// Validate and decode one piece of Mermaid text. `quoted` text sat between double quotes;
/// unquoted text must avoid every character of `special`. The result is trimmed and may be
/// empty (the caller decides what an empty label means).
pub(super) fn decode(raw: &str, quoted: bool, special: &str) -> Result<String, &'static str> {
    if quoted && raw.starts_with('`') {
        return Err("markdown-string");
    }
    if !quoted && raw.chars().any(|c| special.contains(c)) {
        return Err("label-syntax");
    }
    for (i, c) in raw.char_indices() {
        let rest = &raw[i + c.len_utf8()..];
        if c == '<' && br_len(&raw[i..]).is_none() {
            let tag = rest
                .chars()
                .next()
                .is_some_and(|n| n.is_ascii_alphabetic() || matches!(n, '/' | '!' | '?'));
            if tag {
                return Err("html-label");
            }
        } else if c == '&' && html_entity(rest) {
            return Err("html-entity");
        }
    }
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while let Some(c) = raw[i..].chars().next() {
        if c == '<'
            && let Some(n) = br_len(&raw[i..])
        {
            out.push('\n');
            i += n;
        } else if c == '#'
            && let Some((d, n)) = entity(&raw[i + 1..])?
        {
            out.push(d);
            i += 1 + n;
        } else {
            out.push(c);
            i += c.len_utf8();
        }
    }
    let out = out.trim().to_owned();
    if out.chars().count() > MAX_LABEL_CHARS {
        return Err("label-too-long");
    }
    if out.chars().any(|c| is_forbidden(c) || c == '\r') {
        return Err("forbidden-char");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: &str = NODE_SPECIAL;

    #[test]
    fn entities_decode_once() {
        assert_eq!(decode("#35;quot;", true, ""), Ok("#quot;".to_owned()));
        assert_eq!(
            decode("a #amp; #lt;b#gt; C# x", false, N).as_deref(),
            Ok("a & <b> C# x")
        );
        assert_eq!(decode("#9829;", true, "").as_deref(), Ok("\u{2665}"));
        assert_eq!(decode("#bogus;", true, ""), Err("entity"));
        assert_eq!(decode("#55296;", true, ""), Err("entity"));
    }

    #[test]
    fn br_forms_become_newlines_and_other_tags_are_refused() {
        assert_eq!(
            decode("a<BR>b <br/> c<br />d", false, N).as_deref(),
            Ok("a\nb \n c\nd")
        );
        assert_eq!(decode("a<b>x", false, N), Err("html-label"));
        assert_eq!(decode("x < y <3", false, N).as_deref(), Ok("x < y <3"));
        assert_eq!(decode("R&amp;D", false, N), Err("html-entity"));
        assert_eq!(decode("a & b", false, N).as_deref(), Ok("a & b"));
    }

    #[test]
    fn syntax_markdown_and_size_rules() {
        assert_eq!(decode("a(b)", false, N), Err("label-syntax"));
        assert_eq!(decode("a(b)", true, ""), Ok("a(b)".to_owned()));
        assert_eq!(decode("`x`", true, ""), Err("markdown-string"));
        assert_eq!(decode("x `y`", true, "").as_deref(), Ok("x `y`"));
        assert_eq!(decode(&"x".repeat(201), true, ""), Err("label-too-long"));
        assert_eq!(decode("a#13;b", true, ""), Err("forbidden-char"));
    }

    #[test]
    fn id_rule() {
        assert_eq!(valid_id("a-b_1"), Ok(()));
        assert_eq!(valid_id("1a"), Err("invalid-id"));
        assert_eq!(valid_id("node"), Err("reserved-id"));
    }
}
