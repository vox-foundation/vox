//! Statements of a `sequence { ... }` block: actor declarations and messages. The words `actor`,
//! `reply`, `note` and `over` are contextual: `actor <id>` declares, anything else is
//! `<id> -> <id>`, so a word is still usable as an actor id.

use super::Parser;
use super::stmt::Owner;
use crate::ast::{ActorDecl, Ident, ItemKind, Label, MessageStmt};
use crate::lexer::TokenKind;

impl Parser<'_> {
    /// One statement of a `sequence` block.
    pub(super) fn sequence_statement(&mut self) -> Option<ItemKind> {
        let is_actor = matches!(&self.peek().kind, TokenKind::Ident(s) if s == "actor")
            && matches!(
                self.peek_at(1).kind,
                TokenKind::Ident(_) | TokenKind::BadIdent(_)
            );
        if is_actor {
            self.actor_decl()
        } else {
            self.message(false)
        }
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
        self.expect(&TokenKind::Arrow, "`->`")?;
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
