//! Single-pass lexer over `char_indices`. Offsets are original bytes (CRLF is one newline).
//! Lexer problems (bad escapes, forbidden characters, unterminated strings, bad identifiers)
//! are reported here and the token stream still covers the whole input, so the parser never
//! cascades a second error on the same mistake.

use std::iter::Peekable;
use std::str::CharIndices;

use crate::codes;
use crate::diag::{Diagnostic, Fix};
use crate::fixgen::replace;
use crate::limits::MAX_DIAGNOSTICS;
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
    /// A Mermaid link (`-->`, `==>`, `-.->`, `---`, `-- text -->`, `-->|text|`). `exact` means
    /// the head is `-{2,}>`; `label` is the raw link text.
    MArrow {
        exact: bool,
        label: Option<String>,
    },
    /// A Mermaid `id[Label]` bracket form directly after an identifier; `text` is the raw
    /// text between the brackets, `closed` false when the line ended first.
    Bracket {
        open: &'static str,
        text: String,
        closed: bool,
    },
    /// A whole Mermaid styling line (`style`, `classDef`, `click`, ...).
    Styling,
    /// A Mermaid `:::name` class suffix.
    StyleSuffix,
    Semi,
    Amp,
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
    /// End of the line on which a scan for a link label found no closer: nothing later on
    /// that line can close either, so the scan is not repeated (keeps a line O(n)).
    dash_fail_until: usize,
    pipe_fail_until: usize,
    /// Same for `>` openers: no `]` follows on the line past this `>`.
    gt_fail_until: usize,
    /// End offset of the last identifier and whether it is a keyword.
    prev_ident: (usize, bool),
    /// No token yet on the current line.
    fresh: bool,
    /// Cached end of the line last measured by `rest_of_line`.
    eol: usize,
}

/// A control character or bidirectional override that must never reach a label (A8).
pub(crate) fn is_forbidden(c: char) -> bool {
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
        dash_fail_until: 0,
        pipe_fail_until: 0,
        gt_fail_until: 0,
        prev_ident: (usize::MAX, false),
        fresh: true,
        eol: 0,
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
        self.error_fix(code, start, end, message, Vec::new());
    }

    fn error_fix(
        &mut self,
        code: &'static str,
        start: usize,
        end: usize,
        message: String,
        fixes: Vec<Fix>,
    ) {
        // The collector keeps at most MAX_DIAGNOSTICS anyway; stop paying for line/col lookups.
        if self.diags.len() >= MAX_DIAGNOSTICS {
            return;
        }
        let span = Span {
            start: to_u32(start),
            end: to_u32(end),
        };
        let loc = self.idx.location(self.src, span);
        let mut d = Diagnostic::error(code, message, loc);
        d.fixes = fixes;
        self.diags.push(d);
    }

    /// A zero- or non-zero-width byte range as a fix span.
    fn fix(start: usize, end: usize, replacement: &str) -> Fix {
        let span = Span {
            start: to_u32(start),
            end: to_u32(end),
        };
        replace(span, replacement)
    }

    fn peek_is(&mut self, want: char) -> bool {
        self.it.peek().is_some_and(|&(_, n)| n == want)
    }

    /// Lex the token that starts with `c` at byte `i`; `None` for skipped trivia.
    fn token(&mut self, i: usize, c: char) -> Option<Token> {
        let mut end = i + c.len_utf8();
        let kind = match self.mermaid_token(i, c, &mut end) {
            Some(kind) => kind,
            None => self.plain_token(i, c, &mut end)?,
        };
        self.fresh = matches!(kind, TokenKind::Newline);
        Some(Token {
            kind,
            span: Span {
                start: to_u32(i),
                end: to_u32(end),
            },
        })
    }

    /// The token for `c` outside the Mermaid forms; `None` for skipped trivia.
    fn plain_token(&mut self, i: usize, c: char, end: &mut usize) -> Option<TokenKind> {
        Some(match c {
            '\u{feff}' if i == 0 => return None,
            ' ' | '\t' => return None,
            '\r' if self.peek_is('\n') => {
                self.it.next();
                *end += 1;
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
                *end += 1;
                TokenKind::Arrow
            }
            '/' if self.peek_is('/') => self.comment(i, end),
            '"' => self.string(i, end),
            c if c.is_alphanumeric() || c == '_' => {
                let word = self.word(i, end);
                self.after_word(word, end)
            }
            other => TokenKind::Unknown(other),
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
                        .is_some_and(|r| r.starts_with(['>', '-']) || r.starts_with(".-")));
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
                    let fix = Self::fix(j, *end, "");
                    self.error_fix(codes::FORBIDDEN_CHAR, j, *end, msg, vec![fix]);
                    value.push(c);
                }
                c => value.push(c),
            }
        }
        if !terminated {
            let msg = "unterminated string; add a closing `\"` before the end of the line";
            // Close it after the text, not after trailing blanks.
            let kept = self
                .src
                .get(start..*end)
                .unwrap_or_default()
                .trim_end()
                .len();
            let fix = Self::fix(start + kept, start + kept, "\"");
            self.error_fix(
                codes::UNTERMINATED_STRING,
                start,
                *end,
                msg.into(),
                vec![fix],
            );
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
                // Doubling the backslash keeps the literal text.
                let fix = Self::fix(at, at + 1, "\\\\");
                self.error_fix(codes::INVALID_ESCAPE, at, *end, msg, vec![fix]);
                value.push_str(shown);
            }
        }
    }
}

mod mermaid;
#[cfg(test)]
mod tests;
