//! Statements of a `sequence { ... }` block: actor declarations, messages and notes. The words
//! `actor`, `reply`, `note` and `over` are contextual: `actor <id>` declares, `reply <id> ->`
//! is a dashed message, `note over` starts a note, and the same words followed by `->` are
//! plain actor ids.

use super::Parser;
use super::stmt::Owner;
use crate::ast::{ActorDecl, Ident, ItemKind, Label, MessageStmt, NoteStmt};
use crate::lexer::TokenKind;

impl Parser<'_> {
    /// One statement of a `sequence` block.
    pub(super) fn sequence_statement(&mut self) -> Option<ItemKind> {
        let kw = match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => String::new(),
        };
        let next = &self.peek_at(1).kind;
        let ident_next = matches!(next, TokenKind::Ident(_) | TokenKind::BadIdent(_));
        let arrow_next = matches!(next, TokenKind::Arrow | TokenKind::MArrow { .. });
        let note = kw.eq_ignore_ascii_case("note");
        match kw.as_str() {
            "participant" if ident_next => self.mermaid_actor(),
            "actor" if ident_next && self.mermaid_actor_ahead() => self.mermaid_actor(),
            "actor" if ident_next => self.actor_decl(),
            _ if note && self.note_over_ahead() => self.note_stmt(),
            _ if note && self.note_side_ahead() => self.note_side(),
            "reply" if ident_next => {
                self.bump(); // `reply`
                self.message(true)
            }
            "reply" if !arrow_next => {
                self.bump(); // `reply`
                self.unexpected("a message after `reply`");
                None
            }
            _ => self.message(false),
        }
    }

    /// `note over <id>[, <id>]: "<text>"`. A third actor is reported where the `:` belongs.
    fn note_stmt(&mut self) -> Option<ItemKind> {
        let (note, over) = (self.bump(), self.bump()); // `note over`, any case
        self.note_case(&note, &over);
        let mut over = vec![self.ident("an actor id")?];
        if matches!(self.peek().kind, TokenKind::Comma) {
            self.bump();
            over.push(self.ident("an actor id")?);
        }
        if self.expect(&TokenKind::Colon, "`:`").is_none() {
            self.skip_to_stmt_end();
            return None;
        }
        let label = self.label(false, Owner::Note)?;
        Some(ItemKind::Note(NoteStmt { over, label }))
    }

    /// Declare-then-diagnose, like `node_decl`: once `actor <id>` was read the actor is
    /// declared, even when the rest is broken, so later messages do not cascade into
    /// `undeclared-actor`.
    fn actor_decl(&mut self) -> Option<ItemKind> {
        self.bump(); // `actor`
        let id = self.ident("an actor id")?;
        let has_colon = self.expect(&TokenKind::Colon, "`:`").is_some();
        let mut label = None;
        if has_colon || matches!(self.peek().kind, TokenKind::Str { .. }) {
            label = self.label(false, Owner::Id(&id.name));
        }
        if label.is_none() {
            self.skip_to_stmt_end();
        }
        let label = label.unwrap_or_else(|| Label {
            value: id.name.clone(),
            span: id.span,
        });
        Some(ItemKind::Actor(ActorDecl { id, label }))
    }

    /// `<from> -> <to>` with an optional `: "<text>"`; `reply` marks a dashed return.
    pub(super) fn message(&mut self, reply: bool) -> Option<ItemKind> {
        let from: Ident = self.ident("an actor id")?;
        let reply = self.arrow(&from, reply)?;
        let to = self.ident("an actor id")?;
        let mut label = None;
        if matches!(self.peek().kind, TokenKind::Colon) {
            self.bump();
            label = self.label(false, Owner::Edge { from: to.span.end });
            if label.is_none() {
                self.skip_to_stmt_end();
            }
        }
        Some(ItemKind::Message(MessageStmt {
            from,
            to,
            label,
            reply,
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{DiagramKind, Entry, ItemKind};
    use crate::check::parse;

    #[test]
    fn actor_and_message_become_items() {
        let p = parse("sequence {\n    actor a: \"A\"\n    a -> b: \"hi\"\n}\n");
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        let Some(doc) = p.ast else {
            panic!("no ast");
        };
        assert_eq!(doc.kind, DiagramKind::Sequence);
        let kinds: Vec<&ItemKind> = doc
            .body
            .entries
            .iter()
            .filter_map(|e| match e {
                Entry::Item(i) => Some(&i.kind),
                Entry::Comment(_) => None,
            })
            .collect();
        let [ItemKind::Actor(a), ItemKind::Message(m)] = kinds[..] else {
            panic!("unexpected items: {kinds:?}");
        };
        assert_eq!((a.id.name.as_str(), a.label.value.as_str()), ("a", "A"));
        assert_eq!((m.from.name.as_str(), m.to.name.as_str()), ("a", "b"));
        assert!(!m.reply && m.label.as_ref().is_some_and(|l| l.value == "hi"));
    }

    #[test]
    fn broken_actor_is_still_declared() {
        let p = parse("sequence {\n    actor a \"A\"\n}\n");
        assert_eq!(p.diagnostics.len(), 1);
        let Some(doc) = p.ast else {
            panic!("no ast");
        };
        assert!(matches!(
            doc.body.entries.first(),
            Some(Entry::Item(i)) if matches!(&i.kind, ItemKind::Actor(a) if a.label.value == "A")
        ));
    }
}
