//! Error-recovering parser for the `graph { ... }` block. Recovery re-syncs at the next newline
//! or `}` so independent mistakes are all reported.

use crate::ast::{Block, Document, EdgeStmt, Entry, Ident, Item, ItemKind, Label, NodeDecl, Ref};
use crate::codes;
use crate::diag::Diagnostic;
use crate::ir::Shape;
use crate::lexer::{Token, TokenKind};
use crate::span::{LineIndex, Span};

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
        TokenKind::Str(s) => format!("string \"{}\"", s.chars().take(20).collect::<String>()),
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
        let span = self.idx.location(self.src, tok.span);
        self.diags
            .push(Diagnostic::error(codes::UNEXPECTED_TOKEN, message, span));
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
                body = self.block();
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
            body,
            span: Span {
                start,
                end: self.peek().span.end,
            },
        }
    }

    /// Statements up to and including the closing `}`.
    fn block(&mut self) -> Block {
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
                    if let Some(kind_span) = self.statement() {
                        let (kind, span) = kind_span;
                        entries.push(Entry::Item(Item {
                            kind,
                            blank_before: blank,
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

    fn statement(&mut self) -> Option<(ItemKind, Span)> {
        let start = self.peek().span.start;
        let is_node = matches!(&self.peek().kind, TokenKind::Ident(s) if s == "node");
        let kind = if is_node {
            self.node_decl()?
        } else {
            self.edge()?
        };
        Some((
            kind,
            Span {
                start,
                end: self.toks[self.pos - 1].span.end,
            },
        ))
    }

    fn ident(&mut self, expected: &str) -> Option<Ident> {
        let tok = self.peek().clone();
        if let TokenKind::Ident(name) = tok.kind {
            self.bump();
            Some(Ident {
                name,
                span: tok.span,
            })
        } else {
            self.unexpected(expected);
            None
        }
    }

    fn label(&mut self) -> Option<Label> {
        let tok = self.peek().clone();
        if let TokenKind::Str(value) = tok.kind {
            self.bump();
            Some(Label {
                value,
                span: tok.span,
            })
        } else {
            self.unexpected("a quoted label");
            None
        }
    }

    fn node_decl(&mut self) -> Option<ItemKind> {
        self.bump(); // `node`
        let id = self.ident("a node id")?;
        self.expect(&TokenKind::Colon, "`:`")?;
        let label = self.label()?;
        Some(ItemKind::Node(NodeDecl {
            id,
            label,
            shape: Shape::default(),
        }))
    }

    fn reference(&mut self) -> Option<Ref> {
        let first = self.ident("a node reference")?;
        let mut segments = vec![first];
        while matches!(self.peek().kind, TokenKind::Dot) {
            self.bump();
            segments.push(self.ident("an identifier after `.`")?);
        }
        let span = Span {
            start: segments[0].span.start,
            end: segments[segments.len() - 1].span.end,
        };
        Some(Ref { segments, span })
    }

    fn edge(&mut self) -> Option<ItemKind> {
        let from = self.reference()?;
        self.expect(&TokenKind::Arrow, "`->`")?;
        let to = self.reference()?;
        let label = if matches!(self.peek().kind, TokenKind::Colon) {
            self.bump();
            Some(self.label()?)
        } else {
            None
        };
        Some(ItemKind::Edge(EdgeStmt { from, to, label }))
    }
}
