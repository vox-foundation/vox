//! Sequence resolution. Pass A declares every actor (so a message may precede the declaration
//! of its actors), pass B resolves messages and notes into events in source order. Actor lookup
//! uses an ordered map, never a hash-based one.

use std::collections::BTreeMap;

use crate::ast::{Document, Entry, Ident, Item, ItemKind};
use crate::codes;
use crate::diag::{Diagnostic, Related, Stats, clip};
use crate::sequence_ir::{Actor, SeqEvent, SequenceIr};
use crate::span::{LineIndex, Span, to_u32};

struct Resolver<'a> {
    src: &'a str,
    idx: &'a LineIndex,
    names: BTreeMap<&'a str, Span>,
    diags: Vec<Diagnostic>,
}

/// Resolve a `sequence` document into IR plus semantic diagnostics.
pub(crate) fn resolve_sequence(
    doc: &Document,
    src: &str,
    idx: &LineIndex,
) -> (SequenceIr, Stats, Vec<Diagnostic>) {
    let mut r = Resolver {
        src,
        idx,
        names: BTreeMap::new(),
        diags: Vec::new(),
    };
    let items: Vec<&Item> = doc
        .body
        .entries
        .iter()
        .filter_map(|e| match e {
            Entry::Item(i) => Some(i),
            Entry::Comment(_) => None,
        })
        .collect();
    let mut ir = SequenceIr {
        actors: Vec::new(),
        events: Vec::new(),
    };
    for item in &items {
        if let ItemKind::Actor(a) = &item.kind
            && let Some(actor) = r.declare(&a.id, &a.label.value, item.span)
        {
            ir.actors.push(actor);
        }
    }
    for item in &items {
        let span = r.idx.location(r.src, item.span);
        match &item.kind {
            ItemKind::Message(m) => {
                let ends = [r.lookup(&m.from), r.lookup(&m.to)];
                if let [Some(from), Some(to)] = ends {
                    ir.events.push(SeqEvent::Message {
                        from,
                        to,
                        text: m
                            .label
                            .as_ref()
                            .map_or_else(String::new, |l| l.value.clone()),
                        dashed: m.reply,
                        span,
                    });
                }
            }
            ItemKind::Note(n) => {
                let over: Vec<Option<String>> = n.over.iter().map(|i| r.lookup(i)).collect();
                if over.iter().all(Option::is_some) {
                    ir.events.push(SeqEvent::Note {
                        over: over.into_iter().flatten().collect(),
                        text: n.label.value.clone(),
                        span,
                    });
                }
            }
            _ => {}
        }
    }
    let stats = Stats {
        nodes: to_u32(ir.actors.len()),
        edges: to_u32(ir.events.len()),
        ..Stats::default()
    };
    (ir, stats, r.diags)
}

impl<'a> Resolver<'a> {
    /// Declare `id`; the first declaration wins and a repeat is a `duplicate-id`.
    fn declare(&mut self, id: &'a Ident, label: &str, stmt: Span) -> Option<Actor> {
        if let Some(&first) = self.names.get(id.name.as_str()) {
            let msg = format!("`{}` is already declared in this scope", clip(&id.name));
            let loc = self.idx.location(self.src, id.span);
            let mut d = Diagnostic::error(codes::DUPLICATE_ID, msg, loc);
            d.related.push(Related {
                message: "first declared here".to_string(),
                span: self.idx.location(self.src, first),
            });
            self.diags.push(d);
            return None;
        }
        self.names.insert(id.name.as_str(), id.span);
        Some(Actor {
            id: id.name.clone(),
            label: label.to_string(),
            span: self.idx.location(self.src, stmt),
        })
    }

    /// The id of a declared actor, or one `undeclared-actor` error at the identifier.
    fn lookup(&mut self, id: &Ident) -> Option<String> {
        if self.names.contains_key(id.name.as_str()) {
            return Some(id.name.clone());
        }
        let msg = format!("actor `{}` is not declared", clip(&id.name));
        let loc = self.idx.location(self.src, id.span);
        self.diags
            .push(Diagnostic::error(codes::UNDECLARED_ACTOR, msg, loc));
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::check::check;
    use crate::ir::Ir;
    use crate::sequence_ir::SeqEvent;

    #[test]
    fn forward_reference_resolves_and_lanes_follow_declaration_order() {
        let r =
            check("sequence {\n    a -> b: \"go\"\n    actor b: \"B\"\n    actor a: \"A\"\n}\n");
        assert!(r.valid, "{:?}", r.diagnostics);
        let Some(Ir::Sequence(s)) = r.ir else {
            panic!("expected sequence ir");
        };
        let ids: Vec<&str> = s.actors.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["b", "a"]);
        assert!(
            matches!(&s.events[0], SeqEvent::Message { from, to, .. } if from == "a" && to == "b")
        );
        assert_eq!((r.stats.nodes, r.stats.edges), (2, 1));
    }

    #[test]
    fn both_ends_of_a_message_are_checked() {
        let r = check("sequence {\n    x -> y\n}\n");
        assert_eq!(r.diagnostics.len(), 2);
        assert!(r.ir.is_none());
    }
}
