//! Sequence resolution. Pass A declares every actor (so a message may precede the declaration
//! of its actors), pass B resolves messages and notes into events in source order. Actor lookup
//! uses an ordered map, never a hash-based one.

use std::collections::{BTreeMap, BTreeSet};

use super::{RESERVED, braces};
use crate::ast::{Document, Entry, Ident, Item, ItemKind};
use crate::codes;
use crate::diag::{Diagnostic, Fix, Related, Stats, clip};
use crate::fixgen::{declarable, insert_after_line, insert_decl};
use crate::sequence_ir::{Actor, SeqEvent, SequenceIr};
use crate::span::{LineIndex, Span, to_u32};

struct Resolver<'a> {
    src: &'a str,
    idx: &'a LineIndex,
    names: BTreeMap<&'a str, Span>,
    diags: Vec<Diagnostic>,
    /// The block's `{` and `}`; a declare-fix needs both.
    braces: Option<(Span, Span)>,
    /// Start of the last `actor` statement: new lanes are declared after its line.
    last_actor: Option<u32>,
    /// Ids whose declare-fix is already owned by an earlier diagnostic.
    fixed: BTreeSet<String>,
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
        braces: braces(&doc.body),
        last_actor: None,
        fixed: BTreeSet::new(),
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
    r.last_actor = items
        .iter()
        .rev()
        .find(|i| matches!(i.kind, ItemKind::Actor(_)))
        .map(|i| i.span.start);
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
        if RESERVED.contains(&id.name.as_str()) {
            let msg = format!("`{}` is a keyword and cannot be used as an id", id.name);
            let loc = self.idx.location(self.src, id.span);
            self.diags
                .push(Diagnostic::error(codes::RESERVED_WORD, msg, loc));
        }
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
        let mut d = Diagnostic::error(codes::UNDECLARED_ACTOR, msg, loc);
        d.fixes = self.declare_fix(&id.name);
        self.diags.push(d);
        None
    }

    /// Declare `actor <name>: "<name>"`. Only the first diagnostic per id carries the fix, so
    /// lanes keep first-mention order; none without both braces (an unclosed block is closed
    /// first and the declaration comes next round) or for an id that cannot be declared as is.
    /// The line goes after the last actor statement, unless the `}` shares that line.
    fn declare_fix(&mut self, name: &str) -> Vec<Fix> {
        let Some((open, close)) = self.braces else {
            return Vec::new();
        };
        if !declarable(name) || self.fixed.contains(name) {
            return Vec::new();
        }
        let decl = format!("actor {name}: \"{name}\"");
        let shares_close = |at: u32| {
            let between = self.src.get(at as usize..close.start as usize);
            between.is_none_or(|t| !t.contains('\n'))
        };
        let fix = match self.last_actor {
            Some(at) if !shares_close(at) => insert_after_line(self.src, at, &decl),
            _ => insert_decl(self.src, open, close, &decl),
        };
        match fix {
            Some(f) => {
                self.fixed.insert(name.to_string());
                vec![f]
            }
            None => Vec::new(),
        }
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
