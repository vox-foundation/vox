//! Byte-span classification for Rust sources: comments vs string literals vs code.
//!
//! Used to cut false positives (skip matches in comments while still matching credentials in strings).

/// Kind of non-code span (comments vs strings) for targeted detector filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonCodeKind {
    /// `//`, `/* */`, doc comments.
    Comment,
    /// `"`, `b"`, raw `r#"..."#`.
    String,
}

#[derive(Debug, Clone, Default)]
pub struct TokenMap {
    spans: Vec<(usize, usize, NonCodeKind)>,
}

impl TokenMap {
    /// Build a token map for Rust source text.
    pub fn from_rust_source(src: &str) -> Self {
        let bytes = src.as_bytes();
        let mut spans: Vec<(usize, usize, NonCodeKind)> = Vec::new();
        let mut i = 0usize;
        while i < bytes.len() {
            // Line comment
            if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
                let start = i;
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                spans.push((start, i, NonCodeKind::Comment));
                continue;
            }
            // Block comment (nested)
            if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                let start = i;
                i += 2;
                let mut depth = 1u32;
                while i + 1 < bytes.len() && depth > 0 {
                    if bytes[i] == b'/' && bytes[i + 1] == b'*' {
                        depth += 1;
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                        depth -= 1;
                        i += 2;
                        continue;
                    }
                    i += 1;
                }
                spans.push((start, i.min(bytes.len()), NonCodeKind::Comment));
                continue;
            }
            // Raw string
            if let Some((end, span)) = try_scan_raw_string(bytes, i) {
                spans.push((span.0, span.1, NonCodeKind::String));
                i = end;
                continue;
            }
            // Byte string b"..."
            if bytes[i] == b'b'
                && bytes.get(i + 1) == Some(&b'"')
                && bytes.get(i + 2) != Some(&b'"')
                && let Some(end) = scan_normal_string(bytes, i + 1)
            {
                spans.push((i, end, NonCodeKind::String));
                i = end;
                continue;
            }
            // Normal " string
            if bytes[i] == b'"'
                && let Some(end) = scan_normal_string(bytes, i)
            {
                spans.push((i, end, NonCodeKind::String));
                i = end;
                continue;
            }
            i += 1;
        }
        merge_spans_kind(&mut spans);
        Self { spans }
    }

    /// True if `byte_idx` lies inside a comment or string literal span.
    #[inline]
    pub fn is_non_code_byte(&self, byte_idx: usize) -> bool {
        self.spans
            .iter()
            .any(|&(a, b, _)| a <= byte_idx && byte_idx < b)
    }

    /// True if `byte_idx` is inside a **comment** (line or block), not a string.
    #[inline]
    pub fn is_comment_byte(&self, byte_idx: usize) -> bool {
        self.spans
            .iter()
            .any(|&(a, b, k)| k == NonCodeKind::Comment && a <= byte_idx && byte_idx < b)
    }

    /// True if `byte_idx` is inside a string literal.
    #[inline]
    pub fn is_string_byte(&self, byte_idx: usize) -> bool {
        self.spans
            .iter()
            .any(|&(a, b, k)| k == NonCodeKind::String && a <= byte_idx && byte_idx < b)
    }

    /// True if `byte_idx` is considered executable / non-string / non-comment ("code").
    #[inline]
    pub fn is_code_byte(&self, byte_idx: usize) -> bool {
        !self.is_non_code_byte(byte_idx)
    }
}

fn merge_spans_kind(spans: &mut Vec<(usize, usize, NonCodeKind)>) {
    if spans.is_empty() {
        return;
    }
    spans.sort_by_key(|&(a, _, _)| a);
    let sorted = std::mem::take(spans);
    let mut out: Vec<(usize, usize, NonCodeKind)> = Vec::with_capacity(sorted.len());
    let mut cur = sorted[0];
    for &(a, b, k) in sorted.iter().skip(1) {
        if k == cur.2 && a <= cur.1 {
            cur.1 = cur.1.max(b);
        } else {
            out.push(cur);
            cur = (a, b, k);
        }
    }
    out.push(cur);
    *spans = out;
}

/// `open_quote` points at the opening Rust `"` (normal or byte string payload).
fn scan_normal_string(bytes: &[u8], open_quote: usize) -> Option<usize> {
    let mut i = open_quote + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i = i.saturating_add(2);
                if i > bytes.len() {
                    return Some(bytes.len());
                }
            }
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

/// Returns `(next_index_after_raw_string, (start, end))` if `i` starts a raw string.
fn try_scan_raw_string(bytes: &[u8], i: usize) -> Option<(usize, (usize, usize))> {
    let mut j = i;
    if bytes.get(j) == Some(&b'b') && bytes.get(j + 1) == Some(&b'r') {
        j += 2;
    } else if bytes.get(j) == Some(&b'r') {
        j += 1;
    } else {
        return None;
    }
    let hash_start = j;
    while bytes.get(j) == Some(&b'#') {
        j += 1;
    }
    let n_hashes = j - hash_start;
    if bytes.get(j) != Some(&b'"') {
        return None;
    }
    let start = i;
    j += 1;
    while j < bytes.len() {
        if bytes[j] == b'"' {
            let mut k = j + 1;
            let mut ok = true;
            for _ in 0..n_hashes {
                if bytes.get(k) != Some(&b'#') {
                    ok = false;
                    break;
                }
                k += 1;
            }
            if ok {
                return Some((k, (start, k)));
            }
        }
        j += 1;
    }
    Some((bytes.len(), (start, bytes.len())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_comment_not_code() {
        let s = r#"let x = 1; // foo "bar"
let y = 2;"#;
        let m = TokenMap::from_rust_source(s);
        let comment_start = s.find("//").unwrap();
        assert!(m.is_non_code_byte(comment_start));
        assert!(m.is_comment_byte(comment_start));
        assert!(!m.is_string_byte(comment_start));
        assert!(m.is_code_byte(s.find("let x").unwrap()));
    }

    #[test]
    fn string_hides_slash_slash() {
        let s = r##"let _ = "// not a comment";"##;
        let m = TokenMap::from_rust_source(s);
        let idx = s.find("//").unwrap();
        assert!(m.is_non_code_byte(idx), "inside string");
        assert!(m.is_string_byte(idx));
        assert!(!m.is_comment_byte(idx));
    }

    #[test]
    fn nested_block_comment() {
        let s = "code1 /* outer /* inner */ end */ code2";
        let m = TokenMap::from_rust_source(s);
        let inner = s.find("inner").unwrap();
        assert!(m.is_non_code_byte(inner));
        assert!(m.is_comment_byte(inner));
        let c2 = s.rfind("code2").unwrap();
        assert!(m.is_code_byte(c2));
    }

    #[test]
    fn raw_string_basic() {
        let s = r##"let _ = r#"hello"#;"##;
        let m = TokenMap::from_rust_source(s);
        let hel = s.find("hello").unwrap();
        assert!(m.is_string_byte(hel));
        assert!(!m.is_comment_byte(hel));
    }

    #[test]
    fn trailing_comment_distinct_from_code() {
        let s = "let a = 1; // secret";
        let m = TokenMap::from_rust_source(s);
        let sec = s.find("secret").unwrap();
        assert!(m.is_comment_byte(sec));
        let one = s.find('1').unwrap();
        assert!(m.is_code_byte(one));
    }
}
