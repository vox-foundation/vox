//! Shared proptest generators for the formatter, repair and Mermaid-ism suites.
//!
//! S1 `model_graph`: a model-first graph (ids unique by construction). `model_ir_json` builds
//! the expected IR straight from the model, never through the resolver, so it is an
//! independent oracle for the parser. S2 `noisy_source`: prints a model with random layout
//! noise (indent, blank lines, comments, explicit default shapes, multi-line attr blocks,
//! trailing spaces, CRLF) drawn from a shrinkable `Noise`.
#![allow(dead_code)]

use proptest::prelude::*;
use serde_json::{Value, json};
use vox_diagram::ir::{Direction, Shape};

/// One statement of the model.
#[derive(Debug, Clone)]
pub enum Item {
    Node {
        prefix: usize,
        kebab: bool,
        label: String,
        shape: Shape,
    },
    Container {
        prefix: usize,
        kebab: bool,
        label: String,
        items: Vec<Item>,
    },
    /// Endpoints index the document's nodes in declaration order (modulo their count); an
    /// edge in a document with no nodes is dropped.
    Edge {
        from: usize,
        to: usize,
        label: Option<String>,
    },
}

/// A generated graph: an optional root `direction` (with its slot) and root statements.
#[derive(Debug, Clone)]
pub struct Model {
    pub direction: Option<(Direction, usize)>,
    pub items: Vec<Item>,
}

/// Layout noise: a die stream consumed in print order (0 always picks the plainest option, so
/// shrinking converges on clean layout) plus CRLF.
#[derive(Debug, Clone)]
pub struct Noise {
    pub dice: Vec<u32>,
    pub crlf: bool,
}

const PREFIXES: [&str; 8] = ["a", "svc", "db_x", "api", "n", "end", "style", "cls_a"];

/// Unique by the DFS counter `k`; kebab and snake styles, never a reserved word.
fn ident(prefix: usize, kebab: bool, k: usize) -> String {
    let p = PREFIXES[prefix % PREFIXES.len()];
    if kebab {
        format!("{p}-{k}")
    } else {
        format!("{p}{k}")
    }
}

fn allowed(c: char) -> bool {
    c != '\r'
        && !matches!(c, '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{61c}' | '\u{200e}' | '\u{200f}'
            | '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

fn label() -> impl Strategy<Value = String> {
    let ch = prop_oneof![
        8 => proptest::char::range(' ', '~'),
        2 => any::<char>().prop_filter("allowed label char", |c| allowed(*c)),
        1 => Just('\n'),
        1 => Just('\t'),
    ];
    prop::collection::vec(ch, 1..=20).prop_map(|v| v.into_iter().collect())
}

fn shape() -> impl Strategy<Value = Shape> {
    prop_oneof![
        Just(Shape::Rect),
        Just(Shape::Rounded),
        Just(Shape::Diamond),
        Just(Shape::Cylinder),
        Just(Shape::Start),
        Just(Shape::End),
    ]
}

fn direction() -> impl Strategy<Value = Direction> {
    prop_oneof![
        Just(Direction::Down),
        Just(Direction::Right),
        Just(Direction::Up),
        Just(Direction::Left),
    ]
}

fn item() -> impl Strategy<Value = Item> {
    let node =
        (0usize..8, any::<bool>(), label(), shape()).prop_map(|(prefix, kebab, label, shape)| {
            Item::Node {
                prefix,
                kebab,
                label,
                shape,
            }
        });
    let edge = (0usize..1000, 0usize..1000, prop::option::of(label()))
        .prop_map(|(from, to, label)| Item::Edge { from, to, label });
    prop_oneof![3 => node, 2 => edge].prop_recursive(3, 30, 5, |inner| {
        (
            0usize..8,
            any::<bool>(),
            label(),
            prop::collection::vec(inner, 0..5),
        )
            .prop_map(|(prefix, kebab, label, items)| Item::Container {
                prefix,
                kebab,
                label,
                items,
            })
    })
}

/// S1: model-first graphs, containers nested at most 3 deep.
pub fn model_graph() -> impl Strategy<Value = Model> {
    (
        prop::option::of((direction(), 0usize..16)),
        prop::collection::vec(item(), 0..8),
    )
        .prop_map(|(direction, items)| Model { direction, items })
}

/// Random layout noise for [`noisy_source`].
pub fn noise() -> impl Strategy<Value = Noise> {
    (prop::collection::vec(any::<u32>(), 1..=256), any::<bool>())
        .prop_map(|(dice, crlf)| Noise { dice, crlf })
}

fn word<T: serde::Serialize>(v: T) -> String {
    match serde_json::to_value(v) {
        Ok(Value::String(s)) => s,
        other => format!("{other:?}"),
    }
}

#[derive(Default)]
struct Walk {
    ctr: usize,
    nodes: Vec<Value>,
    node_paths: Vec<String>,
    containers: Vec<Value>,
    raw_edges: Vec<(usize, usize, Option<String>)>,
}

impl Walk {
    fn items(&mut self, items: &[Item], parent: Option<&str>, depth: u32) {
        for it in items {
            match it {
                Item::Node {
                    prefix,
                    kebab,
                    label,
                    shape,
                } => {
                    let local = ident(*prefix, *kebab, self.ctr);
                    self.ctr += 1;
                    let id = parent.map_or(local.clone(), |p| format!("{p}.{local}"));
                    self.nodes
                        .push(json!({"id": id, "local_id": local, "label": label,
                        "shape": word(shape), "container": parent}));
                    self.node_paths.push(id);
                }
                Item::Container {
                    prefix,
                    kebab,
                    label,
                    items,
                } => {
                    let local = ident(*prefix, *kebab, self.ctr);
                    self.ctr += 1;
                    let id = parent.map_or(local.clone(), |p| format!("{p}.{local}"));
                    self.containers
                        .push(json!({"id": id, "local_id": local, "label": label,
                        "parent": parent, "depth": depth + 1}));
                    self.items(items, Some(&id), depth + 1);
                }
                Item::Edge { from, to, label } => self.raw_edges.push((*from, *to, label.clone())),
            }
        }
    }
}

fn walk(m: &Model) -> Walk {
    let mut w = Walk::default();
    w.items(&m.items, None, 0);
    w
}

/// The IR the model must produce, built without the parser or resolver, spans omitted.
pub fn model_ir_json(m: &Model) -> Value {
    let w = walk(m);
    let n = w.node_paths.len();
    let edges: Vec<Value> = w
        .raw_edges
        .iter()
        .filter(|_| n > 0)
        .map(
            |(f, t, l)| json!({"from": w.node_paths[f % n], "to": w.node_paths[t % n], "label": l}),
        )
        .collect();
    let dir = m.direction.map_or(Direction::Down, |d| d.0);
    json!({"kind": "graph", "direction": word(dir), "nodes": w.nodes,
        "containers": w.containers, "edges": edges})
}

/// Remove every `span` key recursively.
pub fn strip_spans(v: Value) -> Value {
    match v {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(k, _)| k != "span")
                .map(|(k, v)| (k, strip_spans(v)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.into_iter().map(strip_spans).collect()),
        other => other,
    }
}

pub mod direction;
pub mod hostile;
pub mod mermaid;
mod noisy;
pub mod roundtrip;

/// S2: print `m` with `n`'s layout noise. The text is always valid and denotes `m`.
pub fn noisy_source(m: &Model, n: &Noise) -> String {
    noisy::print(m, n)
}
