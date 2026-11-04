//! Sequence generators: a model-first `SeqModel` (ids unique by construction), the IR the model
//! must produce (built without the parser or resolver, so an independent oracle), and a noisy
//! printer for valid text. The printer reuses the graph die-stream machinery of `noisy.rs`.
#![allow(dead_code)]

use proptest::prelude::*;
use serde_json::{Value, json};

use super::noisy::Printer;
use super::{Noise, label};

/// One statement after the actor declarations.
#[derive(Debug, Clone)]
pub enum SeqItem {
    /// Endpoints index the actors (modulo their count).
    Message {
        from: usize,
        to: usize,
        label: Option<String>,
        reply: bool,
    },
    /// One or two actor indices (modulo the actor count).
    Note { over: Vec<usize>, label: String },
}

/// A generated sequence: `(prefix, label)` per actor in declaration order, then the events.
#[derive(Debug, Clone)]
pub struct SeqModel {
    pub actors: Vec<(usize, String)>,
    pub events: Vec<SeqItem>,
}

/// Prefixes chosen to sit next to the contextual words of the grammar.
const PREFIXES: [&str; 8] = [
    "note",
    "reply",
    "over",
    "actor",
    "participant",
    "api-gw",
    "db_x",
    "a",
];

/// Unique by the declaration counter `k`; never exactly a contextual word (a digit follows).
pub fn actor_id(prefix: usize, k: usize) -> String {
    format!("{}{k}", PREFIXES[prefix % PREFIXES.len()])
}

/// Model-first sequences: 0-6 actors, 0-12 events (dropped when there is no actor).
pub fn model_sequence() -> impl Strategy<Value = SeqModel> {
    let message = (
        0usize..1000,
        0usize..1000,
        prop::option::of(label()),
        any::<bool>(),
    )
        .prop_map(|(from, to, label, reply)| SeqItem::Message {
            from,
            to,
            label,
            reply,
        });
    let note = (prop::collection::vec(0usize..1000, 1..=2), label())
        .prop_map(|(over, label)| SeqItem::Note { over, label });
    let event = prop_oneof![3 => message, 1 => note];
    (
        prop::collection::vec((0usize..8, label()), 0..=6),
        prop::collection::vec(event, 0..=12),
    )
        .prop_map(|(actors, events)| SeqModel { actors, events })
}

/// The IR the model must produce, spans omitted.
pub fn model_seq_ir_json(m: &SeqModel) -> Value {
    let ids: Vec<String> = (0..m.actors.len())
        .map(|k| actor_id(m.actors[k].0, k))
        .collect();
    let n = ids.len();
    let actors: Vec<Value> = ids
        .iter()
        .zip(&m.actors)
        .map(|(id, (_, label))| json!({"id": id, "label": label}))
        .collect();
    let events: Vec<Value> = m
        .events
        .iter()
        .filter(|_| n > 0)
        .map(|e| match e {
            SeqItem::Message {
                from,
                to,
                label,
                reply,
            } => json!({"type": "message", "from": ids[from % n], "to": ids[to % n],
                "text": label.clone().unwrap_or_default(), "dashed": reply}),
            SeqItem::Note { over, label } => {
                let over: Vec<&String> = over.iter().map(|i| &ids[i % n]).collect();
                json!({"type": "note", "over": over, "text": label})
            }
        })
        .collect();
    json!({"kind": "sequence", "actors": actors, "events": events})
}

impl Printer {
    fn actor_line(&mut self, id: &str, label: &str) {
        self.begin(1);
        let sp = self.sp();
        self.out.push_str(&format!("actor{sp}{id}"));
        self.colon();
        self.label(label);
        self.end_line();
    }

    fn arrow(&mut self) {
        let (a, b) = (self.sp0(), self.sp0());
        self.out.push_str(&format!("{a}->{b}"));
    }

    fn event(&mut self, e: &SeqItem, ids: &[String]) {
        let n = ids.len();
        self.begin(1);
        match e {
            SeqItem::Message {
                from,
                to,
                label,
                reply,
            } => {
                if *reply {
                    let sp = self.sp();
                    self.out.push_str(&format!("reply{sp}"));
                }
                self.out.push_str(&ids[from % n]);
                self.arrow();
                self.out.push_str(&ids[to % n]);
                if let Some(l) = label {
                    self.colon();
                    self.label(l);
                }
            }
            SeqItem::Note { over, label } => {
                let (s1, s2) = (self.sp(), self.sp());
                let list: Vec<&str> = over.iter().map(|i| ids[i % n].as_str()).collect();
                let sep = if list.len() == 2 {
                    let (a, b) = (self.sp0(), self.sp0());
                    format!("{a},{b}")
                } else {
                    String::new()
                };
                self.out
                    .push_str(&format!("note{s1}over{s2}{}", list.join(&sep)));
                self.colon();
                self.label(label);
            }
        }
        self.end_line();
    }
}

/// Print `m` with `n`'s layout noise. The text is always valid and denotes `m`. Actor
/// declarations keep their order but may follow the events that mention them.
pub fn noisy_seq_source(m: &SeqModel, n: &Noise) -> String {
    let ids: Vec<String> = (0..m.actors.len())
        .map(|k| actor_id(m.actors[k].0, k))
        .collect();
    let mut p = Printer::new(n);
    p.comment_lines(3);
    p.blanks();
    let pad = p.sp0();
    p.out.push_str(&format!("sequence{pad}{{"));
    if p.chance(4) {
        let (pad, text) = (p.sp0(), p.comment_text());
        p.out.push_str(&format!("{pad}//{text}"));
    }
    p.out.push('\n');
    let mut next = 0;
    let events: &[SeqItem] = if ids.is_empty() { &[] } else { &m.events };
    for e in events {
        while next < ids.len() && p.chance(3) {
            p.actor_line(&ids[next], &m.actors[next].1);
            next += 1;
        }
        p.event(e, &ids);
    }
    while next < ids.len() {
        p.actor_line(&ids[next], &m.actors[next].1);
        next += 1;
    }
    p.begin(0);
    p.out.push('}');
    p.end_line();
    p.comment_lines(3);
    if p.chance(5) {
        p.out.pop();
    }
    if n.crlf {
        p.out.replace('\n', "\r\n")
    } else {
        p.out
    }
}
