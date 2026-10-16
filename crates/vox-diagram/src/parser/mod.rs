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

    /// An `unexpected-token` error over `span`.
    fn error_at(&mut self, span: Span, message: String) {
        self.report(codes::UNEXPECTED_TOKEN, span, message);
    }

    /// An error with `code` over `span`; a full collector halts parsing.
    fn report(&mut self, code: &'static str, span: Span, message: String) {
        let loc = self.idx.location(self.src, span);
        if self.diags.push(Diagnostic::error(code, message, loc)) {
            self.halted = true;
        }
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
        let is_graph = matches!(&self.peek().kind, TokenKind::Ident(s) if s == "graph");
        if !is_graph {
            self.unexpected("`graph`");
        } else {
            self.bump();
            if self.expect(&TokenKind::LBrace, "`{`").is_some() {
                header_trailing = self.trailing_comment();
                body = self.block(true);
                close_trailing = self.trailing_comment();
            }
        }
        // Anything after the closing brace except newlines and comments is an error.
        let trailing = self.own_line_comments();
        if !self.halted && !matches!(self.peek().kind, TokenKind::Eof) {
            self.unexpected("end of input");
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

    /// Entries up to and including the closing `}`.
    fn block(&mut self, root: bool) -> Block {
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
                    self.unexpected("`}`");
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
