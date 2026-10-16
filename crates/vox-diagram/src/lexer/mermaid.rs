//! Mermaid tokens. Links (dash, equals and dotted runs with the three label spellings),
//! `id[Label]` bracket forms, styling lines and `%%` comments are each one token, so their
//! free text never reaches the identifier rules and cannot raise lexical noise.

use super::{Lexer, TokenKind};
use crate::codes;
use crate::resolve::RESERVED;

/// Statement heads that start a styling line.
const HEADS: [&str; 8] = [
    "style",
    "classDef",
    "class",
    "linkStyle",
    "click",
    "callback",
    "accTitle",
    "accDescr",
];

/// Bracket forms, longest opener first: `(open, close)`.
const FORMS: [(&str, &str); 9] = [
    ("[(", ")]"),
    ("((", "))"),
    ("([", "])"),
    ("[[", "]]"),
    ("{{", "}}"),
    ("[", "]"),
    ("(", ")"),
    ("{", "}"),
    (">", "]"),
];

fn is_link(s: &str) -> bool {
    s.starts_with("--") || s.starts_with("==") || s.starts_with("-.-")
}

/// Byte offset of the matching `close` in `body`; nested pairs of a single-char opener count.
fn find_close(body: &str, open: &str, close: &str) -> Option<usize> {
    if open.len() > 1 || open == ">" {
        return body.find(close);
    }
    let (o, c) = (open.chars().next()?, close.chars().next()?);
    let mut depth = 0usize;
    for (j, ch) in body.char_indices() {
        if ch == o {
            depth += 1;
        } else if ch == c {
            if depth == 0 {
                return Some(j);
            }
            depth -= 1;
        }
    }
    None
}

impl<'a> Lexer<'a> {
    /// The Mermaid token starting with `c` at byte `i`, if any.
    pub(super) fn mermaid_token(
        &mut self,
        i: usize,
        c: char,
        end: &mut usize,
    ) -> Option<TokenKind> {
        Some(match c {
            '-' | '=' if self.starts_link(i) => self.link(i, end),
            ';' => TokenKind::Semi,
            '&' => TokenKind::Amp,
            '%' if self.peek_is('%') => self.percent_comment(i, end),
            ':' if self.src.get(i..).is_some_and(|r| r.starts_with(":::")) => {
                self.style_suffix(i, end)
            }
            '[' | '(' | '{' | '>' if self.after_ident(i, c) => return self.bracket(i, end),
            _ => return None,
        })
    }

    /// True when a link (`--`, `==` or `-.-` run) starts at byte `i`.
    fn starts_link(&self, i: usize) -> bool {
        is_link(self.src.get(i..).unwrap_or_default())
    }

    /// The rest of the line from byte `i`. The line end is cached, so a line full of links or
    /// brackets is scanned once, not once per token.
    fn rest_of_line(&mut self, i: usize) -> &'a str {
        if i >= self.eol {
            let rest = self.src.get(i..).unwrap_or_default();
            self.eol = i + rest.find(['\n', '\r']).unwrap_or(rest.len());
        }
        self.src.get(i..self.eol).unwrap_or_default()
    }

    fn advance_to(&mut self, end: usize) {
        while self.it.peek().is_some_and(|&(j, _)| j < end) {
            self.it.next();
        }
    }

    /// True when the bracket char `c` at `i` directly follows an identifier (a keyword only
    /// counts for `[`, `(` and `>`, so `graph{` stays a block opener).
    fn after_ident(&self, i: usize, c: char) -> bool {
        self.prev_ident.0 == i && !(c == '{' && self.prev_ident.1)
    }

    /// `id[Label]`-style text from the opener at `i` through its closer.
    fn bracket(&mut self, i: usize, end: &mut usize) -> Option<TokenKind> {
        let line = self.rest_of_line(i);
        let mut pick = None;
        for (open, close) in FORMS {
            if !line.starts_with(open) {
                continue;
            }
            let body = line.get(open.len()..)?;
            if let Some(p) = find_close(body, open, close) {
                let len = open.len() + p + close.len();
                pick = Some((open, body.get(..p)?.to_string(), len, true));
                break;
            }
            if pick.is_none() {
                let trimmed = line.trim_end();
                pick = Some((open, body.trim_end().to_string(), trimmed.len(), false));
            }
        }
        let (open, text, len, closed) = pick?;
        if open == ">" && !closed {
            return None;
        }
        *end = i + len;
        self.advance_to(*end);
        Some(TokenKind::Bracket { open, text, closed })
    }

    /// `%% text` is a comment; `%%{ directive }%%` has no `//` equivalent, so no fix.
    fn percent_comment(&mut self, i: usize, end: &mut usize) -> TokenKind {
        let kind = self.comment(i, end);
        let directive = matches!(&kind, TokenKind::Comment(t) if t.starts_with('{'));
        let fixes = if directive {
            Vec::new()
        } else {
            vec![Self::fix(i, i + 2, "//")]
        };
        let msg = "Mermaid `%%` comment; VoxDiagram comments start with `//`";
        self.error_fix(codes::MERMAID_SYNTAX, i, i + 2, msg.into(), fixes);
        kind
    }

    /// `:::name`.
    fn style_suffix(&mut self, i: usize, end: &mut usize) -> TokenKind {
        let rest = self.src.get(i + 3..).unwrap_or_default();
        let name = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .count();
        *end = i + 3 + name;
        self.advance_to(*end);
        TokenKind::StyleSuffix
    }

    /// After a word: a styling head swallows its whole line; any other identifier is
    /// remembered so a bracket directly after it is recognised.
    pub(super) fn after_word(&mut self, word: TokenKind, end: &mut usize) -> TokenKind {
        let TokenKind::Ident(w) = &word else {
            return word;
        };
        if self.fresh && HEADS.contains(&w.as_str()) {
            let tail = self.rest_of_line(*end);
            let body = tail.trim_start();
            let acc = w.starts_with("acc");
            let blocked = body.is_empty()
                || body.starts_with("->")
                || body.starts_with(['.', '&'])
                || (!acc && body.starts_with(':'))
                || is_link(body);
            if !blocked {
                *end += tail.trim_end().len();
                self.advance_to(*end);
                return TokenKind::Styling;
            }
        }
        self.prev_ident = (*end, RESERVED.contains(&w.as_str()));
        word
    }

    /// Consume the link that starts at `start` (its first char is already consumed).
    pub(super) fn link(&mut self, start: usize, end: &mut usize) -> TokenKind {
        let line = self.rest_of_line(start);
        let line_end = start + line.len();
        let first = line.chars().next().unwrap_or('-');
        let dotted = line.starts_with("-.");
        let (run_from, run) = if dotted { (2, '-') } else { (0, first) };
        let tail = line.get(run_from..).unwrap_or_default();
        let mut len = run_from + tail.chars().take_while(|&c| c == run).count();
        let (mut exact, mut label) = (false, None);
        let tail = line.get(len..).unwrap_or_default();
        let after = tail.chars().nth(1);
        if tail.starts_with('>') {
            len += 1;
            exact = first == '-' && !dotted;
        } else if first == '-' && !dotted {
            if matches!(tail.chars().next(), Some('x' | 'o'))
                && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
            {
                len += 1;
            } else if len == 2 && tail.starts_with(' ') && start >= self.dash_fail_until {
                // `-- text -->`
                match tail.find("-->") {
                    Some(p) => {
                        label = Some(tail.get(..p).unwrap_or_default().trim().to_string());
                        len += p + 3;
                        exact = true;
                    }
                    None => self.dash_fail_until = line_end,
                }
            }
        }
        if label.is_none() {
            label = self.pipe_label(start, line, &mut len);
        }
        *end = start + len;
        while self.it.peek().is_some_and(|&(j, _)| j < *end) {
            self.it.next();
        }
        TokenKind::MArrow { exact, label }
    }

    /// `|text|` after the link (blanks allowed before it); extends `len` through the closing
    /// pipe. A pipe with no closer on the line is left alone.
    fn pipe_label(&mut self, start: usize, line: &str, len: &mut usize) -> Option<String> {
        let after = line.get(*len..).unwrap_or_default();
        let body = after.trim_start_matches([' ', '\t']);
        let inner = body.strip_prefix('|')?;
        if start >= self.pipe_fail_until {
            let Some(p) = inner.find('|') else {
                self.pipe_fail_until = start + line.len();
                return None;
            };
            *len += after.len() - inner.len() + p + 1;
            return Some(inner.get(..p).unwrap_or_default().trim().to_string());
        }
        None
    }
}
