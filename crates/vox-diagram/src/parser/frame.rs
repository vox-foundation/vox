//! M4: a Mermaid `subgraph ... end` frame parses as a container. One diagnostic carries both
//! edits: the header becomes `container id: "Title" {` and the `end` line becomes `}`.

use super::Parser;
use crate::ast::{Block, ContainerDecl, ItemKind, Label};
use crate::codes;
use crate::fixgen::{mermaid_label, replace};
use crate::lexer::TokenKind;
use crate::limits::MAX_DEPTH;
use crate::span::Span;

impl Parser<'_> {
    /// `end` as a whole statement.
    pub(super) fn at_frame_end(&self) -> bool {
        matches!(&self.peek().kind, TokenKind::Ident(s) if s == "end")
            && matches!(
                self.peek_at(1).kind,
                TokenKind::Newline
                    | TokenKind::Eof
                    | TokenKind::RBrace
                    | TokenKind::Comment(_)
                    | TokenKind::Semi
            )
    }

    /// `subgraph ID`, `subgraph ID[Title]` or `subgraph ID["Title"]`, then the body to `end`.
    /// Any other header (`subgraph My Title`) is reported without a fix.
    pub(super) fn subgraph(&mut self) -> Option<ItemKind> {
        let kw = self.bump().span; // `subgraph`
        let id = self.ident("a subgraph id")?;
        let mut title = None;
        if let TokenKind::Bracket { text, .. } = self.peek().kind.clone() {
            self.bump();
            title = Some(mermaid_label(&text));
        }
        let mut exact = true;
        while !matches!(
            self.peek().kind,
            TokenKind::Newline | TokenKind::Eof | TokenKind::Comment(_) | TokenKind::RBrace
        ) {
            if !self.spend_fuel() {
                break;
            }
            exact = false;
            self.bump();
        }
        let end = self.toks[self.pos - 1].span.end;
        let header = Span {
            start: kw.start,
            end,
        };
        let (value, quoted) = title.unwrap_or_else(|| mermaid_label(&id.name));
        let label = Label {
            value,
            span: header,
        };
        let body = if self.depth >= MAX_DEPTH {
            let msg = format!("containers nest at most {MAX_DEPTH} deep; flatten this one");
            self.report(codes::LIMIT_DEPTH, header, msg);
            self.skip_frame_body();
            return Some(Self::frame(id, label, Block::default()));
        } else {
            self.depth += 1;
            let outer = std::mem::replace(&mut self.mframe, true);
            let body = self.block(false, None);
            self.mframe = outer;
            self.depth -= 1;
            body
        };
        let mut fixes = Vec::new();
        if exact {
            let open = format!("container {}: {quoted} {{", id.name);
            fixes.push(replace(header, open));
            fixes.extend(self.frame_end.take().map(|e| replace(e, "}")));
        }
        self.frame_end = None;
        let msg = "Mermaid `subgraph`; write `container id: \"Title\" { ... }`";
        self.report_full(codes::MERMAID_SYNTAX, header, msg.into(), None, fixes);
        Some(Self::frame(id, label, body))
    }

    fn frame(id: crate::ast::Ident, label: Label, body: Block) -> ItemKind {
        ItemKind::Container(ContainerDecl {
            id,
            label,
            body,
            header_trailing: None,
            close_trailing: None,
        })
    }

    /// Skip a frame body past the depth cap, counting nested frames.
    fn skip_frame_body(&mut self) {
        let mut nest = 1usize;
        while nest > 0 && self.spend_fuel() {
            match &self.peek().kind {
                TokenKind::Eof => return,
                TokenKind::Ident(s) if s == "subgraph" => nest += 1,
                TokenKind::Ident(s) if s == "end" && self.at_frame_end() => nest -= 1,
                _ => {}
            }
            self.bump();
        }
    }
}
