use crate::extractor::LanguageExtractor;
use crate::features::{ExtractedFeatures, LiteralContext, LiteralLoc, Loc, NumericLoc};
use anyhow::Result;
use std::path::Path;
use vox_code_audit::rules::Language;

pub struct VoxExtractor;

struct VoxSpanLit {
    offset: usize,
    kind: VoxLitKind,
}

enum VoxLitKind {
    String(String),
    Number(f64),
}

fn scan_vox_literals(src: &str) -> Vec<VoxSpanLit> {
    let mut out = Vec::new();
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        // Line comment: // ...
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // Block comment: /* ... */
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            } else {
                i = len;
            }
            continue;
        }
        // Double-quoted or single-quoted string: "..." or '...'
        if b == b'"' || b == b'\'' {
            let quote = b as char;
            let start = i;
            i += 1;
            let mut s = String::new();
            let mut chars = src[i..].char_indices();
            while let Some((rel_idx, ch)) = chars.next() {
                if ch == quote {
                    i += rel_idx + ch.len_utf8();
                    break;
                }
                if ch == '\\' {
                    if let Some((next_idx, esc)) = chars.next() {
                        match esc {
                            'n' => s.push('\n'),
                            'r' => s.push('\r'),
                            't' => s.push('\t'),
                            '\\' => s.push('\\'),
                            '"' => s.push('"'),
                            '\'' => s.push('\''),
                            '0' => s.push('\0'),
                            other => {
                                s.push('\\');
                                s.push(other);
                            }
                        }
                        if i + next_idx + esc.len_utf8() >= len {
                            i = len;
                            break;
                        }
                    } else {
                        s.push('\\');
                        i = len;
                        break;
                    }
                } else {
                    s.push(ch);
                    if i + rel_idx + ch.len_utf8() >= len {
                        i = len;
                        break;
                    }
                }
            }
            out.push(VoxSpanLit {
                offset: start,
                kind: VoxLitKind::String(s),
            });
            continue;
        }
        // Numeric literal: digits [0-9]
        let prev_is_ident = i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
        if b.is_ascii_digit() && !prev_is_ident {
            let start = i;
            while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                i += 1;
            }
            if i < len && bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit() {
                i += 1; // skip '.'
                while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                    i += 1;
                }
            }
            let raw = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
            let clean: String = raw.chars().filter(|&c| c != '_').collect();
            if let Ok(v) = clean.parse::<f64>() {
                out.push(VoxSpanLit {
                    offset: start,
                    kind: VoxLitKind::Number(v),
                });
            }
            continue;
        }
        i += 1;
    }
    out
}

impl LanguageExtractor for VoxExtractor {
    fn extract(&self, path: &Path, content: &str) -> Result<ExtractedFeatures> {
        let mut features = ExtractedFeatures::new(path.to_path_buf(), Language::Vox);
        features.crate_name = crate::extractor::crate_name_from_path(path);
        features.allowed_lines = crate::extractor::parse_drift_allow_comments(content);

        let tokens = scan_vox_literals(content);
        for spanned in &tokens {
            match &spanned.kind {
                VoxLitKind::String(s) => {
                    let line = byte_offset_to_line(content, spanned.offset);
                    features.string_literals.push(LiteralLoc {
                        value: s.clone(),
                        loc: Loc { line, col: 0 },
                        ctx: LiteralContext::Code,
                    });
                }
                VoxLitKind::Number(v) => {
                    let line = byte_offset_to_line(content, spanned.offset);
                    features.numeric_literals.push(NumericLoc {
                        value: *v,
                        unit: None,
                        loc: Loc { line, col: 0 },
                        in_const: false,
                    });
                }
            }
        }

        Ok(features)
    }
}

fn byte_offset_to_line(src: &str, offset: usize) -> usize {
    src[..offset.min(src.len())]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(src: &str) -> ExtractedFeatures {
        VoxExtractor.extract(Path::new("test.vox"), src).unwrap()
    }

    #[test]
    fn extracts_string_literal_vox() {
        let f = extract(r#"let x = "hello ⌘K → world\n""#);
        assert!(
            f.string_literals
                .iter()
                .any(|l| l.value == "hello ⌘K → world\n")
        );
    }
}
