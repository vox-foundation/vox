//! Error-recovering parser for the `graph { ... }` block. Recovery re-syncs at the next newline
//! or `}` so independent mistakes are all reported.

use crate::ast::{Block, Comment, Document, Entry, Item, ItemKind};
use crate::codes;
use crate::diag::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::span::{LineIndex, Span};

mod stmt;
#[cfg(test)]
mod tests;

struct Parser<'a> {
    src: &'a str,
    idx: &'a LineIndex,
    toks: &'a [Token],
    pos: usize,
    diags: Vec<Diagnostic>,
}

/// Parse a token stream (ending in `Eof`) into a document plus syntax diagnostics.
pub(crate) fn parse_tokens(
    src: &str,
    idx: &LineIndex,
    toks: &[Token],
) -> (Document, Vec<Diagnostic>) {
    let mut p = Parser {
        src,
        idx,
        toks,
        pos: 0,
        diags: Vec::new(),
    };
    let doc = p.document();
    (doc, p.diags)
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Ident(s) => format!("identifier `{}`", s.chars().take(20).collect::<String>()),
        TokenKind::Str { value, .. } => {
            format!("string \"{}\"", value.chars().take(20).collect::<String>())
        }
        TokenKind::Comment(_) => "a comment".into(),
        TokenKind::BadIdent(s) => format!("`{}`", s.chars().take(20).collect::<String>()),
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
        let loc = self.idx.location(self.src, span);
        self.diags
            .push(Diagnostic::error(codes::UNEXPECTED_TOKEN, message, loc));
    }

    /// Advance to the next newline, `}` or EOF without consuming it.
    fn skip_to_stmt_end(&mut self) {
        while !matches!(
            self.peek().kind,
            TokenKind::Newline | TokenKind::RBrace | TokenKind::Eof
        ) {
            self.bump();
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
        if !matches!(self.peek().kind, TokenKind::Eof) {
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
        loop {
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
