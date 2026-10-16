//! Single-pass lexer over `char_indices`. Offsets are original bytes (CRLF is one newline).
//! Lexer problems (bad escapes, forbidden characters, unterminated strings, bad identifiers)
//! are reported here and the token stream still covers the whole input, so the parser never
//! cascades a second error on the same mistake.

use std::iter::Peekable;
use std::str::CharIndices;

use crate::codes;
use crate::diag::Diagnostic;
use crate::span::{LineIndex, Span, to_u32};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Ident(String),
    /// A quoted label; `terminated` is false when the line ended first.
    Str {
        value: String,
        terminated: bool,
    },
    /// Text after `//`, trailing whitespace trimmed.
    Comment(String),
    /// A word that is not a valid identifier (digit start or non-ASCII letters).
    BadIdent(String),
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

type Chars<'a> = Peekable<CharIndices<'a>>;

struct Lexer<'a> {
    src: &'a str,
    idx: &'a LineIndex,
    it: Chars<'a>,
    diags: Vec<Diagnostic>,
}

/// A control character or bidirectional override that must never reach a label (A8).
fn is_forbidden(c: char) -> bool {
    matches!(c,
        '\u{0}'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{7f}'
        | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

fn clip(s: &str) -> String {
    s.chars().take(20).collect()
}

/// Tokenise `src`; the last token is always `Eof`. Never fails: bad input becomes a diagnostic
/// plus a best-effort token.
pub(crate) fn lex(src: &str, idx: &LineIndex) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut lx = Lexer {
        src,
        idx,
        it: src.char_indices().peekable(),
        diags: Vec::new(),
    };
    let mut out = Vec::new();
    while let Some((i, c)) = lx.it.next() {
        if let Some(tok) = lx.token(i, c) {
            out.push(tok);
        }
    }
    let eof = to_u32(src.len());
    out.push(Token {
        kind: TokenKind::Eof,
        span: Span {
            start: eof,
            end: eof,
        },
    });
    (out, lx.diags)
}

impl Lexer<'_> {
    fn error(&mut self, code: &'static str, start: usize, end: usize, message: String) {
        let span = Span {
            start: to_u32(start),
            end: to_u32(end),
        };
        let loc = self.idx.location(self.src, span);
        self.diags.push(Diagnostic::error(code, message, loc));
    }

    fn peek_is(&mut self, want: char) -> bool {
        self.it.peek().is_some_and(|&(_, n)| n == want)
    }

    /// Lex the token that starts with `c` at byte `i`; `None` for skipped trivia.
    fn token(&mut self, i: usize, c: char) -> Option<Token> {
        let mut end = i + c.len_utf8();
        let kind = match c {
            '\u{feff}' if i == 0 => return None,
            ' ' | '\t' => return None,
            '\r' if self.peek_is('\n') => {
                self.it.next();
                end += 1;
                TokenKind::Newline
            }
            '\r' => return None,
            '\n' => TokenKind::Newline,
            ':' => TokenKind::Colon,
            '.' => TokenKind::Dot,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            ',' => TokenKind::Comma,
            '-' if self.peek_is('>') => {
                self.it.next();
                end += 1;
                TokenKind::Arrow
            }
            '/' if self.peek_is('/') => self.comment(i, &mut end),
            '"' => self.string(i, &mut end),
            c if c.is_alphanumeric() || c == '_' => self.word(i, &mut end),
            other => TokenKind::Unknown(other),
        };
        Some(Token {
            kind,
            span: Span {
                start: to_u32(i),
                end: to_u32(end),
            },
        })
    }

    /// `//` to end of line; the token span ends at the last non-blank character.
    fn comment(&mut self, start: usize, end: &mut usize) -> TokenKind {
        while let Some(&(j, n)) = self.it.peek() {
            if n == '\n'
                || (n == '\r' && self.src.get(j + 1..).is_some_and(|r| r.starts_with('\n')))
            {
                break;
            }
            self.it.next();
            *end = j + n.len_utf8();
        }
        let text = self.src.get(start + 2..*end).unwrap_or_default().trim_end();
        *end = start + 2 + text.len();
        TokenKind::Comment(text.to_string())
    }

    /// An identifier or a bad word. A `-` joins the word unless it starts `->` or `--` (A1).
    fn word(&mut self, start: usize, end: &mut usize) -> TokenKind {
        while let Some(&(j, n)) = self.it.peek() {
            let joins = n.is_alphanumeric()
                || n == '_'
                || (n == '-'
                    && !self
                        .src
                        .get(j + 1..)
                        .is_some_and(|r| r.starts_with('>') || r.starts_with('-')));
            if !joins {
                break;
            }
            self.it.next();
            *end = j + n.len_utf8();
        }
        let word = self.src.get(start..*end).unwrap_or_default();
        let valid =
            word.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') && word.is_ascii();
        if valid {
            return TokenKind::Ident(word.to_string());
        }
        let msg = format!(
            "invalid identifier `{}`; ids start with an ASCII letter or `_` and use only ASCII letters, digits, `_` and `-`",
            clip(word)
        );
        self.error(codes::INVALID_IDENTIFIER, start, *end, msg);
        TokenKind::BadIdent(word.to_string())
    }

    /// A single-line string with `\"`, `\\` and `\n` escapes.
    fn string(&mut self, start: usize, end: &mut usize) -> TokenKind {
        let mut value = String::new();
        let mut terminated = false;
        while let Some(&(j, n)) = self.it.peek() {
            if n == '\n' || n == '\r' {
                break;
            }
            self.it.next();
            *end = j + n.len_utf8();
            match n {
                '"' => {
                    terminated = true;
                    break;
                }
                '\\' => self.escape(j, end, &mut value),
                c if is_forbidden(c) => {
                    let msg = format!(
                        "control or bidirectional-override character U+{:04X} is not allowed in a label",
                        c as u32
                    );
                    self.error(codes::FORBIDDEN_CHAR, j, *end, msg);
                    value.push(c);
                }
                c => value.push(c),
            }
        }
        if !terminated {
            let msg = "unterminated string; add a closing `\"` before the end of the line";
            self.error(codes::UNTERMINATED_STRING, start, *end, msg.into());
        }
        TokenKind::Str { value, terminated }
    }

    /// Decode the escape after the backslash at byte `at`, or report it and keep it literal.
    fn escape(&mut self, at: usize, end: &mut usize, value: &mut String) {
        let next = match self.it.peek() {
            Some(&(_, c)) if c != '\n' && c != '\r' => Some(c),
            _ => None,
        };
        let decoded = match next {
            Some('"') => Some('"'),
            Some('\\') => Some('\\'),
            Some('n') => Some('\n'),
            _ => None,
        };
        if next.is_some() {
            self.it.next();
            *end = at + 1 + next.map_or(0, char::len_utf8);
        }
        match decoded {
            Some(c) => value.push(c),
            None => {
                let shown = self.src.get(at..*end).unwrap_or("\\");
                let msg = format!(
                    "invalid escape `{shown}`; the supported escapes are `\\\"`, `\\\\` and `\\n`"
                );
                self.error(codes::INVALID_ESCAPE, at, *end, msg);
                value.push_str(shown);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes;

    fn run(src: &str) -> (Vec<Token>, Vec<Diagnostic>) {
        lex(src, &LineIndex::new(src))
    }

    fn kinds(src: &str) -> Vec<TokenKind> {
        run(src).0.into_iter().map(|t| t.kind).collect()
    }

    fn id(s: &str) -> TokenKind {
        TokenKind::Ident(s.into())
    }

    fn st(s: &str) -> TokenKind {
        TokenKind::Str {
            value: s.into(),
            terminated: true,
        }
    }

    #[test]
    fn lexes_edge_and_string_with_byte_spans() {
        let (toks, diags) = run("a -> \"é\"\r\n");
        let k: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            k,
            vec![
                id("a"),
                TokenKind::Arrow,
                st("é"),
                TokenKind::Newline,
                TokenKind::Eof
            ]
        );
        assert!(diags.is_empty());
        assert_eq!(toks[2].span, Span { start: 5, end: 9 });
        assert_eq!(toks[3].span, Span { start: 9, end: 11 });
    }

    #[test]
    fn kebab_idents_stop_before_arrows() {
        assert_eq!(
            kinds("api-gw -> db"),
            vec![id("api-gw"), TokenKind::Arrow, id("db"), TokenKind::Eof]
        );
        assert_eq!(
            kinds("a->b"),
            vec![id("a"), TokenKind::Arrow, id("b"), TokenKind::Eof]
        );
        // `a-->b`: the dash run is left for the parser.
        assert_eq!(kinds("a-->b")[0], id("a"));
    }

    #[test]
    fn string_escapes_decode() {
        let (toks, diags) = run(r#""say \"hi\" \\ x\nnext""#);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(toks[0].kind, st("say \"hi\" \\ x\nnext"));
    }

    #[test]
    fn invalid_escape_is_one_error_and_keeps_literal_text() {
        let (toks, diags) = run(r#""bad \t""#);
        assert_eq!(toks[0].kind, st("bad \\t"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, codes::INVALID_ESCAPE);
        assert_eq!((diags[0].span.start, diags[0].span.end), (5, 7));
    }

    #[test]
    fn forbidden_char_is_reported_per_char_with_exact_span() {
        let (_, diags) = run("\"a\u{202e}b\u{7}\"");
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.code == codes::FORBIDDEN_CHAR));
        assert_eq!((diags[0].span.start, diags[0].span.end), (2, 5));
        assert_eq!((diags[1].span.start, diags[1].span.end), (6, 7));
        let (_, ok) = run("\"a\tb\"");
        assert!(ok.is_empty(), "a raw tab is allowed: {ok:?}");
    }

    #[test]
    fn unterminated_string_does_not_cascade() {
        let (toks, diags) = run("\"open\nnext");
        assert_eq!(
            toks.into_iter().map(|t| t.kind).collect::<Vec<_>>(),
            vec![
                TokenKind::Str {
                    value: "open".into(),
                    terminated: false
                },
                TokenKind::Newline,
                id("next"),
                TokenKind::Eof
            ]
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, codes::UNTERMINATED_STRING);
    }

    #[test]
    fn bad_words_are_one_invalid_identifier_each() {
        for word in ["9lives", "école", "caf\u{e9}"] {
            let (toks, diags) = run(word);
            assert_eq!(toks[0].kind, TokenKind::BadIdent(word.into()), "{word}");
            assert_eq!(diags.len(), 1, "{word}");
            assert_eq!(diags[0].code, codes::INVALID_IDENTIFIER);
            assert_eq!(diags[0].span.end as usize, word.len());
        }
    }

    #[test]
    fn comments_crlf_and_bom() {
        let (toks, diags) = run("\u{feff}// note  \r\nx");
        assert!(diags.is_empty());
        let k: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            k,
            vec![
                TokenKind::Comment(" note".into()),
                TokenKind::Newline,
                id("x"),
                TokenKind::Eof
            ]
        );
        assert_eq!(toks[0].span, Span { start: 3, end: 10 });
    }

    #[test]
    fn stray_chars_become_unknown() {
        assert_eq!(kinds("?"), vec![TokenKind::Unknown('?'), TokenKind::Eof]);
    }
}
