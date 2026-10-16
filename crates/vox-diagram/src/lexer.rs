//! Single-pass lexer over `char_indices`. Offsets are original bytes (CRLF is one newline).

use crate::span::{Span, to_u32};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Ident(String),
    Str(String),
    Arrow,
    Colon,
    Dot,
    LBrace,
    RBrace,
    Comma,
    Newline,
    Unknown(char),
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: Span,
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Tokenise `src`; the last token is always `Eof`. Never fails: bad input becomes `Unknown`.
pub(crate) fn lex(src: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut it = src.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        let mut end = i + c.len_utf8();
        let kind = match c {
            ' ' | '\t' => continue,
            '\r' if it.peek().is_some_and(|&(_, n)| n == '\n') => {
                it.next();
                end += 1;
                TokenKind::Newline
            }
            '\r' => continue,
            '\n' => TokenKind::Newline,
            ':' => TokenKind::Colon,
            '.' => TokenKind::Dot,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            ',' => TokenKind::Comma,
            '-' if it.peek().is_some_and(|&(_, n)| n == '>') => {
                it.next();
                end += 1;
                TokenKind::Arrow
            }
            '"' => {
                // Single-line string; escapes and unterminated recovery arrive with plan 02.
                let mut text = String::new();
                let mut closed = false;
                while let Some(&(j, n)) = it.peek() {
                    if n == '\n' || n == '\r' {
                        break;
                    }
                    it.next();
                    end = j + n.len_utf8();
                    if n == '"' {
                        closed = true;
                        break;
                    }
                    text.push(n);
                }
                if closed {
                    TokenKind::Str(text)
                } else {
                    TokenKind::Unknown('"')
                }
            }
            c if is_ident_start(c) => {
                let mut name = String::from(c);
                while let Some(&(j, n)) = it.peek() {
                    if !is_ident_continue(n) {
                        break;
                    }
                    it.next();
                    end = j + n.len_utf8();
                    name.push(n);
                }
                TokenKind::Ident(name)
            }
            other => TokenKind::Unknown(other),
        };
        out.push(Token {
            kind,
            span: Span {
                start: to_u32(i),
                end: to_u32(end),
            },
        });
    }
    let eof = to_u32(src.len());
    out.push(Token {
        kind: TokenKind::Eof,
        span: Span {
            start: eof,
            end: eof,
        },
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(src).into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn lexes_edge_and_string_with_byte_spans() {
        let toks = lex("a -> \"é\"\r\n");
        let k: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            k,
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::Arrow,
                TokenKind::Str("é".into()),
                TokenKind::Newline,
                TokenKind::Eof
            ]
        );
        assert_eq!(toks[2].span, Span { start: 5, end: 9 });
        assert_eq!(toks[3].span, Span { start: 9, end: 11 });
    }

    #[test]
    fn unterminated_string_and_stray_chars_become_unknown() {
        assert_eq!(
            kinds("\"abc\n?"),
            vec![
                TokenKind::Unknown('"'),
                TokenKind::Newline,
                TokenKind::Unknown('?'),
                TokenKind::Eof
            ]
        );
    }
}
