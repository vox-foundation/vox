//! Semantic pass over scanned flowchart statements: node bookkeeping, Mermaid's first-closed
//! subgraph membership, caps and the id clashes the DSL cannot express.

use std::collections::{BTreeMap, BTreeSet};

use super::flow::{GContainer, GEdge, GNode, Graph};
use super::lines::{Chain, Ref, Stmt};
use crate::ir::{Direction, Shape};
use crate::limits::{MAX_DEPTH, MAX_EDGES, MAX_NODES};

/// An open subgraph: its container (`None` for a refused header that only balances `end`),
/// header line and the nodes its own body mentions.
struct Frame {
    ctr: Option<usize>,
    line: u32,
    mentions: BTreeSet<usize>,
}

pub(super) struct Built {
    pub graph: Graph,
    pub errors: Vec<(u32, String)>,
    pub dropped: Vec<(u32, String)>,
}

struct Builder {
    g: Graph,
    ix: BTreeMap<String, usize>,
    explicit: Vec<bool>,
    subs: BTreeSet<String>,
    stack: Vec<Frame>,
    errors: Vec<(u32, String)>,
    dropped: Vec<(u32, String)>,
}

impl Builder {
    fn err(&mut self, line: u32, c: &str) {
        self.errors.push((line, c.to_owned()));
    }

    fn mention(&mut self, line: u32, r: Ref, in_edge: bool) -> Option<usize> {
        if self.subs.contains(&r.id) {
            let c = if in_edge && r.decl.is_none() {
                "edge-to-subgraph"
            } else {
                "id-clash"
            };
            self.err(line, c);
            return None;
        }
        let i = if let Some(&i) = self.ix.get(&r.id) {
            i
        } else {
            let i = self.g.nodes.len();
            if i == MAX_NODES {
                self.err(line, "limit-nodes");
            }
            self.ix.insert(r.id.clone(), i);
            self.g.nodes.push(GNode {
                label: r.id.clone(),
                id: r.id,
                shape: Shape::Rect,
            });
            self.g.node_container.push(None);
            self.explicit.push(false);
            i
        };
        if let Some((shape, label)) = r.decl {
            let n = &mut self.g.nodes[i];
            if self.explicit[i] && (n.label != label || n.shape != shape) {
                self.err(line, "redeclared-node");
            } else {
                (n.label, n.shape, self.explicit[i]) = (label, shape, true);
            }
        }
        if let Some(f) = self.stack.last_mut() {
            f.mentions.insert(i);
        }
        Some(i)
    }

    fn chain(&mut self, line: u32, c: Chain) {
        let in_edge = c.groups.len() > 1;
        let groups: Vec<Vec<Option<usize>>> = c
            .groups
            .into_iter()
            .map(|g| {
                g.into_iter()
                    .map(|r| self.mention(line, r, in_edge))
                    .collect()
            })
            .collect();
        for (k, pair) in groups.windows(2).enumerate() {
            for &a in pair[0].iter().flatten() {
                for &b in pair[1].iter().flatten() {
                    if self.g.edges.len() == MAX_EDGES {
                        self.err(line, "limit-edges");
                    }
                    self.g.edges.push(GEdge {
                        from: a,
                        to: b,
                        label: c.arrows[k].clone(),
                    });
                }
            }
        }
    }

    fn open(&mut self, line: u32, id: Option<String>, title: Option<String>) {
        if self.stack.len() == MAX_DEPTH {
            self.err(line, "limit-depth");
        }
        let ctr = id.map(|id| {
            self.g.containers.push(GContainer {
                title: title.unwrap_or_else(|| id.clone()),
                id,
                parent: self.stack.iter().rev().find_map(|f| f.ctr),
            });
            self.g.containers.len() - 1
        });
        self.stack.push(Frame {
            ctr,
            line,
            mentions: BTreeSet::new(),
        });
    }

    /// `direction X` is only legal in a subgraph, and only when it repeats the diagram's own.
    fn direction(&mut self, line: u32, d: Direction) {
        if self.stack.is_empty() {
            self.err(line, "direction");
        } else if d == self.g.direction {
            self.dropped.push((line, "subgraph-direction".to_owned()));
        } else {
            self.err(line, "subgraph-direction");
        }
    }

    fn close(&mut self, line: u32) {
        let Some(f) = self.stack.pop() else {
            self.err(line, "stray-end");
            return;
        };
        if let Some(c) = f.ctr {
            for n in f.mentions {
                self.g.node_container[n].get_or_insert(c);
            }
        }
    }
}

pub(super) fn build(direction: Direction, items: Vec<(u32, Stmt)>) -> Built {
    let mut b = Builder {
        g: Graph {
            direction,
            nodes: Vec::new(),
            containers: Vec::new(),
            node_container: Vec::new(),
            edges: Vec::new(),
        },
        ix: BTreeMap::new(),
        explicit: Vec::new(),
        subs: BTreeSet::new(),
        stack: Vec::new(),
        errors: Vec::new(),
        dropped: Vec::new(),
    };
    for (line, it) in &items {
        if let Stmt::Subgraph { id, .. } = it
            && !b.subs.insert(id.clone())
        {
            b.err(*line, "id-clash");
        }
    }
    for (line, it) in items {
        match it {
            Stmt::Chain(c) => b.chain(line, c),
            Stmt::Subgraph { id, title } => b.open(line, Some(id), title),
            Stmt::End => b.close(line),
            Stmt::BadOpen => b.open(line, None, None),
            Stmt::Direction(d) => b.direction(line, d),
        }
    }
    let open: Vec<u32> = b.stack.iter().map(|f| f.line).collect();
    for line in open {
        b.err(line, "unclosed-subgraph");
    }
    Built {
        graph: b.g,
        errors: b.errors,
        dropped: b.dropped,
    }
}
