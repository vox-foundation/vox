//! Statement parsers: `direction`, `node`, `container` and edge statements. Keywords are
//! contextual: `node`/`container`/`direction` followed by `->` or `.` is an edge endpoint.

use super::Parser;
use crate::ast::{
    Block, ContainerDecl, DirectionStmt, EdgeStmt, Ident, ItemKind, Label, NodeDecl, Ref,
};
use crate::codes;
use crate::diag::clip;
use crate::ir::{Direction, Shape};
use crate::lexer::TokenKind;
use crate::limits::{MAX_DEPTH, MAX_LABEL_CHARS};
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
            "direction" if matches!(next, TokenKind::Colon) => self.direction_stmt(root)?,
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

    /// A quoted label. Unquoted text is recovered as the label (and reported); `None` only
    /// when there is nothing label-like at all.
    fn label(&mut self, stop_lbrace: bool) -> Option<Label> {
        let tok = self.peek().clone();
        if let TokenKind::Str { value, terminated } = tok.kind {
            self.bump();
            if terminated && value.is_empty() {
                let msg = "label is empty; give it some text".to_string();
                self.report(codes::EMPTY_LABEL, tok.span, msg);
            }
            // Chars never outnumber bytes, so short values skip the count.
            let chars = if value.len() > MAX_LABEL_CHARS {
                value.chars().count()
            } else {
                0
            };
            if chars > MAX_LABEL_CHARS {
                let msg = format!("label has {chars} characters; the limit is {MAX_LABEL_CHARS}");
                self.report(codes::LIMIT_LABEL, tok.span, msg);
            }
            return Some(Label {
                value,
                span: tok.span,
            });
        }
        if let Some(label) = self.unquoted_label(stop_lbrace) {
            return Some(label);
        }
        self.unexpected("a quoted label");
        None
    }

    fn direction_stmt(&mut self, root: bool) -> Option<ItemKind> {
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
                    clip(&word.name)
                );
                let closest = ["down", "right", "up", "left"]
                    .into_iter()
                    .min_by_key(|w| strsim::levenshtein(w, &word.name))
                    .unwrap_or("down");
                let help = Some(format!("did you mean `{closest}`?"));
                self.report_help(codes::INVALID_DIRECTION, word.span, msg, help);
                return None;
            }
        };
        let span = Span {
            start,
            end: word.span.end,
        };
        if !root {
            let msg = "`direction` is only allowed directly inside `graph { }`".to_string();
            self.report(codes::DIRECTION_IN_CONTAINER, span, msg);
        } else if self.seen_direction {
            let msg = "a graph has one `direction`; remove this one".to_string();
            self.report(codes::DUPLICATE_DIRECTION, span, msg);
        }
        self.seen_direction |= root;
        Some(ItemKind::Direction(DirectionStmt { value, span }))
    }

    /// A label made from the id, used when the real one cannot be recovered.
    fn placeholder(id: &Ident) -> Label {
        Label {
            value: id.name.clone(),
            span: id.span,
        }
    }

    /// Declare-then-diagnose: once `node <id>` was read the node is always declared, even
    /// when the rest is broken, so later edges to it do not cascade into `undeclared-node`.
    fn node_decl(&mut self) -> Option<ItemKind> {
        self.bump(); // `node`
        let id = self.ident("a node id")?;
        let has_colon = self.expect(&TokenKind::Colon, "`:`").is_some();
        let mut label = None;
        if has_colon || matches!(self.peek().kind, TokenKind::Str { .. }) {
            label = self.label(true);
            if label.is_none() {
                self.skip_to_stmt_end();
            }
        } else {
            self.skip_to_stmt_end();
        }
        let shape = if matches!(self.peek().kind, TokenKind::LBrace) {
            self.attr_block()
        } else {
            Shape::default()
        };
        let label = label.unwrap_or_else(|| Self::placeholder(&id));
        Some(ItemKind::Node(NodeDecl { id, label, shape }))
    }

    /// `{ shape: <word> }` — the only attribute in v1; attributes may be separated by `,` or
    /// newlines. Returns the shape (last one wins; the default after any error).
    fn attr_block(&mut self) -> Shape {
        let open = self.bump().span; // `{`
        let mut shape = Shape::default();
        loop {
            while matches!(self.peek().kind, TokenKind::Newline | TokenKind::Comma) {
                // A line-leading `node`/`container` means the block was never closed.
                if matches!(self.peek().kind, TokenKind::Newline) && self.next_line_starts_decl() {
                    self.unclosed(open);
                    return shape;
                }
                self.bump();
            }
            match self.peek().kind {
                TokenKind::RBrace => {
                    self.bump();
                    return shape;
                }
                TokenKind::Eof => {
                    self.unclosed(open);
                    return shape;
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
                    self.resync_attr(open);
                    return Shape::default();
                }
            }
        }
    }

    fn shape_attr(&mut self) -> Option<Shape> {
        let key = self.ident("`shape`")?;
        if key.name != "shape" {
            let msg = format!(
                "unknown attribute `{}`; the only attribute is `shape`",
                clip(&key.name)
            );
            self.report(codes::UNKNOWN_ATTRIBUTE, key.span, msg);
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
                let msg = format!(
                    "expected one of rect, rounded, diamond, cylinder, start, end, found `{}`",
                    clip(other)
                );
                self.report(codes::INVALID_SHAPE, word.span, msg);
                return None;
            }
        };
        Some(shape)
    }

    /// Declare-then-diagnose for a container header. A broken header with a `{` later on the
    /// same line still parses its body; with no `{` on the line the container is declared
    /// empty (the body's `}` then surfaces as one acceptable follow-up error).
    fn container_decl(&mut self) -> Option<ItemKind> {
        let kw = self.bump(); // `container`
        let id = self.ident("a container id")?;
        let has_colon = self.expect(&TokenKind::Colon, "`:`").is_some();
        let label = if has_colon || matches!(self.peek().kind, TokenKind::Str { .. }) {
            self.label(true)
        } else {
            None
        };
        let open = if matches!(self.peek().kind, TokenKind::LBrace) {
            Some(self.bump().span)
        } else {
            if label.is_some() {
                self.unexpected("`{`");
            }
            self.consume_through_lbrace()
        };
        let label = label.unwrap_or_else(|| Self::placeholder(&id));
        let (header_trailing, body) = match open {
            Some(open) if self.depth >= MAX_DEPTH => {
                // Skip the body with a brace counter instead of recursing, so nesting depth
                // can never exceed MAX_DEPTH frames however deep the input goes.
                let span = Span {
                    start: kw.span.start,
                    end: open.end,
                };
                let msg = format!("containers nest at most {MAX_DEPTH} deep; flatten this one");
                self.report(codes::LIMIT_DEPTH, span, msg);
                self.skip_container_body();
                (None, Block::default())
            }
            Some(open) => {
                self.depth += 1;
                let parsed = (self.trailing_comment(), self.block(false, Some(open)));
                self.depth -= 1;
                parsed
            }
            None => (None, Block::default()),
        };
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
        let mut label = None;
        if matches!(self.peek().kind, TokenKind::Colon) {
            self.bump();
            label = self.label(false);
            if label.is_none() {
                self.skip_to_stmt_end();
            }
        }
        Some(ItemKind::Edge(EdgeStmt { from, to, label }))
    }
}
