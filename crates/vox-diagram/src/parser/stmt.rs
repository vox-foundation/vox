//! Statement parsers: `direction`, `node`, `container` and edge statements. Keywords are
//! contextual: `node`/`container`/`direction` followed by `->` or `.` is an edge endpoint.

use super::Parser;
use crate::ast::{ContainerDecl, DirectionStmt, EdgeStmt, Ident, ItemKind, Label, NodeDecl, Ref};
use crate::ir::{Direction, Shape};
use crate::lexer::TokenKind;
use crate::span::Span;

impl Parser<'_> {
    /// One statement; `root` is true directly inside `graph { }`.
    pub(super) fn statement(&mut self, root: bool) -> Option<(ItemKind, Span)> {
        let start = self.peek().span.start;
        let kw = match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => String::new(),
        };
        let next = &self.peek_at(1).kind;
        let edge_next = matches!(next, TokenKind::Arrow | TokenKind::Dot);
        let kind = match kw.as_str() {
            "node" if !edge_next => self.node_decl()?,
            "container" if !edge_next => self.container_decl()?,
            "direction" if root && matches!(next, TokenKind::Colon) => self.direction_stmt()?,
            _ => self.edge()?,
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
        // A bad identifier was already reported by the lexer; accept it to avoid a second error.
        if let TokenKind::Ident(name) | TokenKind::BadIdent(name) = tok.kind {
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
        if let TokenKind::Str { value, .. } = tok.kind {
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

    fn direction_stmt(&mut self) -> Option<ItemKind> {
        let start = self.bump().span.start; // `direction`
        self.expect(&TokenKind::Colon, "`:`")?;
        let word = self.ident("`down`, `right`, `up` or `left`")?;
        let value = match word.name.as_str() {
            "down" => Direction::Down,
            "right" => Direction::Right,
            "up" => Direction::Up,
            "left" => Direction::Left,
            _ => {
                let msg = format!(
                    "expected `down`, `right`, `up` or `left`, found `{}`",
                    word.name.chars().take(20).collect::<String>()
                );
                self.error_at(word.span, msg);
                return None;
            }
        };
        Some(ItemKind::Direction(DirectionStmt {
            value,
            span: Span {
                start,
                end: word.span.end,
            },
        }))
    }

    fn node_decl(&mut self) -> Option<ItemKind> {
        self.bump(); // `node`
        let id = self.ident("a node id")?;
        self.expect(&TokenKind::Colon, "`:`")?;
        let label = self.label()?;
        let shape = if matches!(self.peek().kind, TokenKind::LBrace) {
            self.attr_block()?
        } else {
            Shape::default()
        };
        Some(ItemKind::Node(NodeDecl { id, label, shape }))
    }

    /// `{ shape: <word> }` — the only attribute in v1; attributes may be separated by `,` or
    /// newlines. Returns the shape (last one wins).
    fn attr_block(&mut self) -> Option<Shape> {
        self.bump(); // `{`
        let mut shape = Shape::default();
        loop {
            while matches!(self.peek().kind, TokenKind::Newline | TokenKind::Comma) {
                self.bump();
            }
            match self.peek().kind {
                TokenKind::RBrace => {
                    self.bump();
                    return Some(shape);
                }
                TokenKind::Eof => {
                    self.unexpected("`}`");
                    return None;
                }
                _ => {}
            }
            let parsed = self.shape_attr();
            let separated = matches!(
                self.peek().kind,
                TokenKind::Newline | TokenKind::Comma | TokenKind::RBrace
            );
            if parsed.is_some() && !separated {
                self.unexpected("`,`, a newline or `}`");
            }
            match parsed {
                Some(s) if separated => shape = s,
                _ => {
                    // Re-sync to the closing brace so it does not end the enclosing block.
                    while !matches!(self.peek().kind, TokenKind::RBrace | TokenKind::Eof) {
                        self.bump();
                    }
                    if matches!(self.peek().kind, TokenKind::RBrace) {
                        self.bump();
                    }
                    return None;
                }
            }
        }
    }

    fn shape_attr(&mut self) -> Option<Shape> {
        let key = self.ident("`shape`")?;
        if key.name != "shape" {
            let span = key.span;
            let msg = format!(
                "expected `shape`, found `{}`",
                key.name.chars().take(20).collect::<String>()
            );
            self.error_at(span, msg);
            return None;
        }
        self.expect(&TokenKind::Colon, "`:`")?;
        let word = self.ident("a shape name")?;
        let shape = match word.name.as_str() {
            "rect" => Shape::Rect,
            "rounded" => Shape::Rounded,
            "diamond" => Shape::Diamond,
            "cylinder" => Shape::Cylinder,
            "start" => Shape::Start,
            "end" => Shape::End,
            other => {
                let span = word.span;
                let msg = format!(
                    "expected one of rect, rounded, diamond, cylinder, start, end, found `{}`",
                    other.chars().take(20).collect::<String>()
                );
                self.error_at(span, msg);
                return None;
            }
        };
        Some(shape)
    }

    fn container_decl(&mut self) -> Option<ItemKind> {
        self.bump(); // `container`
        let id = self.ident("a container id")?;
        self.expect(&TokenKind::Colon, "`:`")?;
        let label = self.label()?;
        self.expect(&TokenKind::LBrace, "`{`")?;
        let header_trailing = self.trailing_comment();
        let body = self.block(false);
        Some(ItemKind::Container(ContainerDecl {
            id,
            label,
            body,
            header_trailing,
            close_trailing: None,
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
