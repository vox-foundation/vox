//! Error-recovering parser for the `graph { ... }` block. Recovery re-syncs at the next newline
//! or `}` so independent mistakes are all reported.

use crate::ast::{Block, Comment, Document, Entry, Item, ItemKind};
use crate::codes;
use crate::diag::{Collector, Diagnostic, clip};
use crate::lexer::{Token, TokenKind};
use crate::span::{LineIndex, Span};

mod recover;
mod stmt;
#[cfg(test)]
mod tests;

struct Parser<'a> {
    src: &'a str,
    idx: &'a LineIndex,
    toks: &'a [Token],
    pos: usize,
    diags: &'a mut Collector,
    /// Work budget, see [`Parser::spend_fuel`].
    fuel: usize,
    /// Set when the collector is full or the fuel ran out; parsing stops.
    halted: bool,
    /// A `direction` statement was already seen in the graph block.
    seen_direction: bool,
}

/// Parse a token stream (ending in `Eof`) into a document; diagnostics go to `diags`.
pub(crate) fn parse_tokens(
    src: &str,
    idx: &LineIndex,
    toks: &[Token],
    diags: &mut Collector,
) -> Document {
    run(src, idx, toks, diags, 4 * toks.len() + 1_000)
}

fn run(src: &str, idx: &LineIndex, toks: &[Token], diags: &mut Collector, fuel: usize) -> Document {
    let halted = diags.is_full();
    let mut p = Parser {
        src,
        idx,
        toks,
        pos: 0,
        diags,
        fuel,
        halted,
        seen_direction: false,
    };
    p.document()
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Ident(s) => format!("identifier `{}`", clip(s)),
        TokenKind::Str { value, .. } => format!("string \"{}\"", clip(value)),
        TokenKind::Comment(_) => "a comment".into(),
        TokenKind::BadIdent(s) => format!("`{}`", clip(s)),
        TokenKind::Arrow => "`->`".into(),
        TokenKind::Colon => "`:`".into(),
        TokenKind::Dot => "`.`".into(),
        TokenKind::LBrace => "`{`".into(),
        TokenKind::RBrace => "`}`".into(),
        TokenKind::Comma => "`,`".into(),
        TokenKind::Newline => "end of line".into(),
        TokenKind::Unknown(c) => format!("`{}`", c.escape_default()),
        TokenKind::Eof => "end of input".into(),
    }
}

impl Parser<'_> {
    fn peek(&self) -> &Token {
        // `toks` always ends in Eof and `bump` never moves past it.
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }

    fn peek_at(&self, n: usize) -> &Token {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)]
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        if !matches!(t.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        t
    }

    fn unexpected(&mut self, expected: &str) {
        let tok = self.peek();
        let message = format!("expected {expected}, found {}", describe(&tok.kind));
        let span = tok.span;
        self.error_at(span, message);
    }

    /// An error with `code` over `span` plus a `help` line.
    fn report_help(
        &mut self,
        code: &'static str,
        span: Span,
        message: String,
        help: Option<String>,
    ) {
        let loc = self.idx.location(self.src, span);
        let mut d = Diagnostic::error(code, message, loc);
        d.help = help;
        if self.diags.push(d) {
            self.halted = true;
        }
    }

    /// An `unexpected-token` error over `span`.
    fn error_at(&mut self, span: Span, message: String) {
        self.report(codes::UNEXPECTED_TOKEN, span, message);
    }

    /// An error with `code` over `span`; a full collector halts parsing.
    fn report(&mut self, code: &'static str, span: Span, message: String) {
        self.report_help(code, span, message, None);
    }

    fn expect(&mut self, want: &TokenKind, expected: &str) -> Option<Token> {
        if std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(want) {
            Some(self.bump())
        } else {
            self.unexpected(expected);
            None
        }
    }

    /// A comment on the current line, if the next token is one.
    fn trailing_comment(&mut self) -> Option<Comment> {
        let tok = self.peek().clone();
        let TokenKind::Comment(text) = tok.kind else {
            return None;
        };
        self.bump();
        Some(Comment {
            text,
            blank_before: false,
            span: tok.span,
        })
    }

    /// Own-line comments (and the newlines between them) outside any block. A comment is
    /// `blank_before` iff a blank line separates it from the previous comment.
    fn own_line_comments(&mut self) -> Vec<Comment> {
        let mut out: Vec<Comment> = Vec::new();
        let mut blank = false;
        loop {
            match self.peek().kind {
                TokenKind::Newline => {
                    self.bump();
                    blank = true;
                }
                TokenKind::Comment(_) => {
                    if let Some(mut c) = self.trailing_comment() {
                        c.blank_before = blank && !out.is_empty();
                        out.push(c);
                    }
                    if matches!(self.peek().kind, TokenKind::Newline) {
                        self.bump();
                    }
                    blank = false;
                }
                _ => return out,
            }
        }
    }

    fn document(&mut self) -> Document {
        let mut body = Block::default();
        let leading = self.own_line_comments();
        let start = self.peek().span.start;
        let (mut header_trailing, mut close_trailing) = (None, None);
        let head = self.peek().clone();
        match &head.kind {
            TokenKind::Ident(s) if s == "graph" => {
                self.bump();
                let open = if matches!(self.peek().kind, TokenKind::LBrace) {
                    Some(self.bump().span)
                } else {
                    self.unexpected("`{`");
                    None
                };
                header_trailing = self.trailing_comment();
                body = self.block(true, open);
                close_trailing = self.trailing_comment();
            }
            TokenKind::Ident(s) => {
                let msg = format!(
                    "unknown diagram kind `{}`; only `graph` is supported (`sequence` arrives in a later release)",
                    clip(s)
                );
                self.report(codes::UNKNOWN_DIAGRAM_KIND, head.span, msg);
                self.skip_rest();
            }
            _ => {
                self.unexpected("`graph`");
                self.skip_rest();
            }
        }
        // Anything after the closing brace except newlines and comments is one error.
        let trailing = self.own_line_comments();
        if !self.halted && !matches!(self.peek().kind, TokenKind::Eof) {
            self.trailing_content();
        }
        Document {
            leading,
            body,
            header_trailing,
            close_trailing,
            trailing,
            span: Span {
                start,
                end: self.peek().span.end,
            },
        }
    }

    /// One `trailing-content` error over everything up to EOF, then consume it.
    fn trailing_content(&mut self) {
        let start = self.peek().span.start;
        let last = self.toks.len().saturating_sub(1);
        let mut end = start;
        for t in self.toks.get(self.pos..last).unwrap_or_default() {
            if !matches!(t.kind, TokenKind::Newline) {
                end = t.span.end;
            }
        }
        let msg = "unexpected content after the closing `}` of the graph block; remove it";
        self.report(codes::TRAILING_CONTENT, Span { start, end }, msg.into());
        self.skip_rest();
    }

    /// Entries up to and including the closing `}`; `open` is the span of this block's `{`
    /// (reported as `unclosed-brace` when EOF comes first).
    fn block(&mut self, root: bool, open: Option<Span>) -> Block {
        let mut entries = Vec::new();
        if matches!(self.peek().kind, TokenKind::Newline) {
            self.bump();
        }
        let mut blank = false;
        let mut last_pos = usize::MAX;
        loop {
            if self.halted || !self.spend_fuel() {
                break;
            }
            debug_assert!(self.pos != last_pos, "parser stalled at token {}", self.pos);
            last_pos = self.pos;
            // A blank line never marks the first entry of a block.
            let blank_before = blank && !entries.is_empty();
            match self.peek().kind {
                TokenKind::Newline => {
                    self.bump();
                    blank = true;
                }
                TokenKind::RBrace => {
                    self.bump();
                    break;
                }
                TokenKind::Eof => {
                    if let Some(open) = open {
                        self.unclosed(open);
                    }
                    break;
                }
                TokenKind::Comment(_) => {
                    if let Some(mut c) = self.trailing_comment() {
                        c.blank_before = blank_before;
                        entries.push(Entry::Comment(c));
                    }
                    blank = false;
                    self.end_of_statement();
                }
                _ => {
                    if let Some((mut kind, span)) = self.statement(root) {
                        // After a container this is the comment on its closing `}` line.
                        let trailing = self.trailing_comment();
                        let item_trailing = match &mut kind {
                            ItemKind::Container(c) => {
                                c.close_trailing = trailing;
                                None
                            }
                            _ => trailing,
                        };
                        entries.push(Entry::Item(Item {
                            kind,
                            blank_before,
                            trailing: item_trailing,
                            span,
                        }));
                    } else {
                        self.skip_to_stmt_end();
                    }
                    blank = false;
                    self.end_of_statement();
                }
            }
        }
        Block { entries }
    }

    /// A statement must end at a newline, `}` or EOF.
    fn end_of_statement(&mut self) {
        match self.peek().kind {
            TokenKind::Newline => {
                self.bump();
            }
            TokenKind::RBrace | TokenKind::Eof => {}
            _ => {
                self.unexpected("end of line");
                self.skip_to_stmt_end();
                if matches!(self.peek().kind, TokenKind::Newline) {
                    self.bump();
                }
            }
        }
    }
}
