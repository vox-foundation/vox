//! Error-recovering parser for the `graph { ... }` block. Recovery re-syncs at the next newline
//! or `}` so independent mistakes are all reported.

use crate::ast::{Block, Document, Entry, Item};
use crate::codes;
use crate::diag::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::span::{LineIndex, Span};

mod stmt;

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

    fn document(&mut self) -> Document {
        let mut body = Block::default();
        while matches!(self.peek().kind, TokenKind::Newline) {
            self.bump();
        }
        let start = self.peek().span.start;
        let is_graph = matches!(&self.peek().kind, TokenKind::Ident(s) if s == "graph");
        if !is_graph {
            self.unexpected("`graph`");
        } else {
            self.bump();
            if self.expect(&TokenKind::LBrace, "`{`").is_some() {
                body = self.block(true);
            }
        }
        // Anything after the closing brace except newlines is an error.
        while matches!(self.peek().kind, TokenKind::Newline) {
            self.bump();
        }
        if !matches!(self.peek().kind, TokenKind::Eof) {
            self.unexpected("end of input");
        }
        Document {
            leading: Vec::new(),
            body,
            header_trailing: None,
            close_trailing: None,
            trailing: Vec::new(),
            span: Span {
                start,
                end: self.peek().span.end,
            },
        }
    }

    /// Statements up to and including the closing `}`.
    fn block(&mut self, root: bool) -> Block {
        let mut entries = Vec::new();
        if matches!(self.peek().kind, TokenKind::Newline) {
            self.bump();
        }
        let mut blank = false;
        loop {
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
                _ => {
                    if let Some(kind_span) = self.statement(root) {
                        let (kind, span) = kind_span;
                        entries.push(Entry::Item(Item {
                            kind,
                            blank_before: blank,
                            trailing: None,
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
