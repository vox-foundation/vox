//! Shared importer text rules: label and edge-text validation and decoding.
//! Every rejection except the length, emptiness and forbidden-character ones runs on the RAW
//! text, so the exporter's encoded forms (`#96;`, `#lt;`, `#amp;`, `#35;`) import back as
//! plain text.

use crate::lexer::is_forbidden;
use crate::limits::MAX_LABEL_CHARS;
use crate::mermaid::escape::has_direction_trap;

/// Characters Mermaid's parser rejects in unquoted node, title and pipe-edge text.
pub(super) const NODE_SPECIAL: &str = "()[]{}|\"@";
/// Only the quote is rejected in unquoted `-- text -->` edge text.
pub(super) const DASH_SPECIAL: &str = "\"";

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

/// A `#name;` or `#digits;` Mermaid entity right after the `#`. Mermaid rewrites every `#\w+;`,
/// so any other word run before a `;` (mixed, `x41`, `_a`) or an unknown name is `Err`: refuse
/// rather than guess. A `#` not followed by such a run and `;` is plain text (`None`).
fn entity(after_hash: &str) -> Result<Option<(char, usize)>, &'static str> {
    let run = after_hash
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
        .count();
    if run == 0 || !after_hash[run..].starts_with(';') {
        return Ok(None);
    }
    let word = &after_hash[..run];
    let c = if word.bytes().all(|b| b.is_ascii_digit()) {
        word.parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .ok_or("entity")?
    } else {
        match word {
            "quot" => '"',
            "lt" => '<',
            "gt" => '>',
            "amp" => '&',
            "apos" => '\'',
            "nbsp" => '\u{a0}',
            _ => return Err("entity"),
        }
    };
    Ok(Some((c, run + 1)))
}

/// Validate and decode one piece of Mermaid text. `quoted` text sat between double quotes;
/// unquoted text must avoid every character of `special`. The result is trimmed and may be
/// empty (the caller decides what an empty label means).
pub(super) fn decode(raw: &str, quoted: bool, special: &str) -> Result<String, &'static str> {
    decode_in(raw, quoted, special, true)
}

/// [`decode`]; `flow` is false for sequence text, which has no `direction` statement.
fn decode_in(raw: &str, quoted: bool, special: &str, flow: bool) -> Result<String, &'static str> {
    // Mermaid's lexer swallows the whole line (or flips the direction), so this is not text.
    if flow && has_direction_trap(raw) {
        return Err("direction-in-text");
    }
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

/// Free text to end of line (state transition text; sequence aliases, messages, notes). It has
/// no unquoted-character ban, but a raw `;` or `%%` ends the statement in Mermaid, with `wrap`
/// (sequence text) so does a `#` that opens no entity, and with `wrap` a leading `wrap:` / `nowrap:` (optionally `:`-prefixed) is stripped and sets a flag.
/// All rejection checks run on the RAW text, so `#59;`, `#119;rap:` and `#96;` import as text.
/// The result may be empty.
pub(super) fn free(raw: &str, wrap: bool) -> Result<String, &'static str> {
    if semicolon(raw)? {
        return Err("semicolon");
    }
    if raw.contains("%%") {
        return Err("comment");
    }
    if wrap && stray_hash(raw)? {
        return Err("hash");
    }
    if wrap {
        let t = raw.trim();
        let t = t.strip_prefix(':').unwrap_or(t);
        if t.starts_with("wrap:") || t.starts_with("nowrap:") {
            return Err("wrap");
        }
    }
    decode_in(raw, true, "", !wrap)
}

/// True when `raw` has a `;` that does not terminate a `#59;`-style entity.
pub(super) fn semicolon(raw: &str) -> Result<bool, &'static str> {
    let mut i = 0;
    while let Some(c) = raw[i..].chars().next() {
        i += c.len_utf8();
        match c {
            '#' => i += entity(&raw[i..])?.map_or(0, |(_, n)| n),
            ';' => return Ok(true),
            _ => {}
        }
    }
    Ok(false)
}

/// True when `raw` has a `#` that does not open a complete entity. Mermaid's sequence text rule
/// is `[^#\n;]*`, so such a `#` silently truncates the text there.
pub(super) fn stray_hash(raw: &str) -> Result<bool, &'static str> {
    let mut i = 0;
    while let Some(c) = raw[i..].chars().next() {
        i += c.len_utf8();
        if c == '#' {
            match entity(&raw[i..])? {
                Some((_, n)) => i += n,
                None => return Ok(true),
            }
        }
    }
    Ok(false)
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
    fn free_text_rules() {
        assert_eq!(
            free("lease expiry (swept)", false).as_deref(),
            Ok("lease expiry (swept)")
        );
        assert_eq!(free("", true).as_deref(), Ok(""));
        assert_eq!(free("a; b", false), Err("semicolon"));
        assert_eq!(free("a %% b", false), Err("comment"));
        for t in ["wrap: x", " nowrap:x", ":wrap: x", ":nowrap:"] {
            assert_eq!(free(t, true), Err("wrap"), "{t}");
            assert!(free(t, false).is_ok(), "{t}");
        }
        assert_eq!(free("#59; #119;rap:", true).as_deref(), Ok("; wrap:"));
        assert_eq!(free("x wrap: y", true).as_deref(), Ok("x wrap: y"));
    }
}
