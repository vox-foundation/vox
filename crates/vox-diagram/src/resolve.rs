//! Scope resolution. Pass A declares every node and container (so forward references work),
//! pass B resolves edges and builds the IR in declaration order. Scopes use ordered maps,
//! never hash-based ones. A bare name is searched in the current scope, then each enclosing
//! scope outward; later segments of a dotted path are searched among children only.

use std::collections::BTreeMap;

use crate::ast::{Block, Document, Entry, ItemKind, Ref};
use crate::codes;
use crate::diag::{Diagnostic, Related, Stats, clip};
use crate::ir::{GraphIr, IrContainer, IrEdge, IrNode};
use crate::span::{LineIndex, Span, to_u32};

#[derive(Clone, Copy)]
enum Entity {
    Node(usize),
    /// Index of the container's own scope.
    Container(usize),
}

struct Scope<'a> {
    parent: Option<usize>,
    /// Full dotted id of the owning container (empty for the root).
    path: String,
    /// Declared names with the span of the id that declared each first.
    names: BTreeMap<&'a str, (Entity, Span)>,
    first_child: Option<String>,
}

struct Resolver<'a> {
    src: &'a str,
    idx: &'a LineIndex,
    ir: GraphIr,
    scopes: Vec<Scope<'a>>,
    diags: Vec<Diagnostic>,
    /// Pass B replays pass A's pre-order scope numbering.
    next_scope: usize,
}

/// Resolve `doc` into IR plus semantic diagnostics.
pub(crate) fn resolve<'a>(
    doc: &'a Document,
    src: &'a str,
    idx: &'a LineIndex,
) -> (GraphIr, Stats, Vec<Diagnostic>) {
    let mut r = Resolver {
        src,
        idx,
        ir: GraphIr {
            direction: crate::ir::Direction::default(),
            nodes: Vec::new(),
            containers: Vec::new(),
            edges: Vec::new(),
        },
        scopes: vec![Scope {
            parent: None,
            path: String::new(),
            names: BTreeMap::new(),
            first_child: None,
        }],
        diags: Vec::new(),
        next_scope: 0,
    };
    r.declare(&doc.body, 0, 1);
    r.link(&doc.body, 0);
    let stats = Stats {
        nodes: to_u32(r.ir.nodes.len()),
        containers: to_u32(r.ir.containers.len()),
        edges: to_u32(r.ir.edges.len()),
        max_depth: r.ir.containers.iter().map(|c| c.depth).max().unwrap_or(0),
    };
    (r.ir, stats, r.diags)
}

impl<'a> Resolver<'a> {
    /// Register `name` in `scope`. The first declaration of a name wins; a repeat is a
    /// `duplicate-id`, a keyword is a `reserved-word`, and an id that hides one in an
    /// enclosing scope is a `shadowed-id` at `Severity::Warning` (innermost still wins).
    fn claim(&mut self, scope: usize, id: (&'a str, Span), full: &str, entity: Entity) {
        let (name, span) = id;
        if RESERVED.contains(&name) {
            let msg = format!("`{name}` is a keyword and cannot be used as an id");
            self.report(codes::RESERVED_WORD, span, msg, None);
        }
        if let Some(&(_, first)) = self.scopes[scope].names.get(name) {
            let msg = format!("`{}` is already declared in this scope", clip(name));
            let mut d =
                Diagnostic::error(codes::DUPLICATE_ID, msg, self.idx.location(self.src, span));
            d.related.push(self.related("first declared here", first));
            self.diags.push(d);
        } else {
            self.scopes[scope].names.insert(name, (entity, span));
            if let Some(outer) = self.outer_decl(scope, name) {
                let msg = format!(
                    "`{}` hides an id of the same name in an enclosing scope",
                    clip(name)
                );
                let loc = self.idx.location(self.src, span);
                let mut d = Diagnostic::warning(codes::SHADOWED_ID, msg, loc);
                d.related
                    .push(self.related("the hidden declaration", outer));
                self.diags.push(d);
            }
        }
        self.scopes[scope]
            .first_child
            .get_or_insert_with(|| full.to_string());
    }

    fn related(&self, message: &str, span: Span) -> Related {
        Related {
            message: message.to_string(),
            span: self.idx.location(self.src, span),
        }
    }

    /// Where `name` is declared in a scope enclosing `scope`, nearest first.
    fn outer_decl(&self, scope: usize, name: &str) -> Option<Span> {
        let mut cur = self.scopes[scope].parent;
        while let Some(i) = cur {
            if let Some(&(_, span)) = self.scopes[i].names.get(name) {
                return Some(span);
            }
            cur = self.scopes[i].parent;
        }
        None
    }

    fn declare(&mut self, block: &'a Block, scope: usize, depth: u32) {
        for entry in &block.entries {
            let Entry::Item(item) = entry else { continue };
            let path = &self.scopes[scope].path;
            let enclosing = (!path.is_empty()).then(|| path.clone());
            let loc = self.idx.location(self.src, item.span);
            match &item.kind {
                ItemKind::Node(n) => {
                    let id = qualify(path, &n.id.name);
                    self.claim(
                        scope,
                        (&n.id.name, n.id.span),
                        &id,
                        Entity::Node(self.ir.nodes.len()),
                    );
                    self.ir.nodes.push(IrNode {
                        id,
                        local_id: n.id.name.clone(),
                        label: n.label.value.clone(),
                        shape: n.shape,
                        container: enclosing,
                        span: loc,
                    });
                }
                ItemKind::Container(c) => {
                    let id = qualify(path, &c.id.name);
                    let child = self.scopes.len();
                    self.scopes.push(Scope {
                        parent: Some(scope),
                        path: id.clone(),
                        names: BTreeMap::new(),
                        first_child: None,
                    });
                    self.claim(
                        scope,
                        (&c.id.name, c.id.span),
                        &id,
                        Entity::Container(child),
                    );
                    self.ir.containers.push(IrContainer {
                        id,
                        local_id: c.id.name.clone(),
                        label: c.label.value.clone(),
                        parent: enclosing,
                        depth,
                        span: loc,
                    });
                    self.declare(&c.body, child, depth + 1);
                }
                ItemKind::Direction(_) | ItemKind::Edge(_) => {}
            }
        }
    }

    fn link(&mut self, block: &'a Block, scope: usize) {
        for entry in &block.entries {
            let Entry::Item(item) = entry else { continue };
            match &item.kind {
                ItemKind::Direction(d) if scope == 0 => self.ir.direction = d.value,
                ItemKind::Container(c) => {
                    self.next_scope += 1;
                    let child = self.next_scope;
                    self.link(&c.body, child);
                }
                ItemKind::Edge(e) => {
                    let from = self.resolve_ref(scope, &e.from);
                    let to = self.resolve_ref(scope, &e.to);
                    if let (Some(from), Some(to)) = (from, to) {
                        self.ir.edges.push(IrEdge {
                            from,
                            to,
                            label: e.label.as_ref().map(|l| l.value.clone()),
                            span: self.idx.location(self.src, item.span),
                        });
                    }
                }
                ItemKind::Direction(_) | ItemKind::Node(_) => {}
            }
        }
    }

    fn report(&mut self, code: &'static str, span: Span, message: String, help: Option<String>) {
        let mut d = Diagnostic::error(code, message, self.idx.location(self.src, span));
        d.help = help;
        self.diags.push(d);
    }

    /// Resolve a dotted reference to the full id of a node, reporting why it cannot.
    fn resolve_ref(&mut self, scope: usize, r: &Ref) -> Option<String> {
        let written = |upto: usize| {
            let names: Vec<&str> = r.segments[..=upto]
                .iter()
                .map(|s| s.name.as_str())
                .collect();
            clip(&names.join("."))
        };
        let rest = |from: usize| Span {
            start: r.segments[from].span.start,
            end: r.span.end,
        };
        let mut cur = scope;
        let mut found = loop {
            if let Some((e, _)) = self.scopes[cur].names.get(r.segments[0].name.as_str()) {
                break *e;
            }
            match self.scopes[cur].parent {
                Some(p) => cur = p,
                None => {
                    let msg = format!("node `{}` is not declared", written(0));
                    self.report(codes::UNDECLARED_NODE, rest(0), msg, None);
                    return None;
                }
            }
        };
        for i in 1..r.segments.len() {
            let Entity::Container(sc) = found else {
                let msg = format!("`{}` is a node, not a container", written(i - 1));
                self.report(codes::NOT_A_CONTAINER, rest(i), msg, None);
                return None;
            };
            let Some((next, _)) = self.scopes[sc].names.get(r.segments[i].name.as_str()) else {
                let msg = format!("node `{}` is not declared", written(i));
                self.report(codes::UNDECLARED_NODE, rest(i), msg, None);
                return None;
            };
            found = *next;
        }
        match found {
            Entity::Node(i) => Some(self.ir.nodes[i].id.clone()),
            Entity::Container(sc) => {
                let name = written(r.segments.len() - 1);
                let help = match &self.scopes[sc].first_child {
                    Some(c) => format!("connect to a node inside it, such as `{}`", clip(c)),
                    None => format!("declare a node inside `{name}` and connect to that"),
                };
                let msg = format!("`{name}` is a container; an edge endpoint must be a node");
                self.report(codes::CONTAINER_ENDPOINT, r.span, msg, Some(help));
                None
            }
        }
    }
}

/// Words that can never be declared as ids.
const RESERVED: [&str; 6] = [
    "graph",
    "sequence",
    "direction",
    "node",
    "container",
    "shape",
];

fn qualify(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}.{name}")
    }
}

#[cfg(test)]
mod tests {
    use crate::diag::CheckReport;
    use crate::ir::{Direction, Ir};
    use crate::{check, codes};

    fn graph(r: &CheckReport) -> &crate::ir::GraphIr {
        let Some(Ir::Graph(g)) = &r.ir else {
            panic!("expected graph ir, got {:?}", r.diagnostics)
        };
        g
    }

    #[test]
    fn canonical_shape_dfs_order_and_labels() {
        let r = check(include_str!("../tests/fixtures/valid/canonical.vdg"));
        let g = graph(&r);
        let ids: Vec<_> = g.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(
            ids,
            ["client", "backend.gateway", "backend.engine", "backend.db"]
        );
        assert_eq!(g.nodes[3].container.as_deref(), Some("backend"));
        assert_eq!(g.containers[0].depth, 1);
        assert_eq!(g.containers[0].parent, None);
        assert_eq!(g.containers[0].label, "Orchestrator Host");
        let e: Vec<_> = g
            .edges
            .iter()
            .map(|e| (e.from.as_str(), e.to.as_str()))
            .collect();
        assert_eq!(
            e,
            [
                ("backend.gateway", "backend.engine"),
                ("backend.engine", "backend.db"),
                ("client", "backend.gateway")
            ]
        );
    }

    #[test]
    fn innermost_scope_wins_then_outward() {
        let src = "graph {\n node x: \"X\"\n node y: \"Y\"\n container be: \"B\" {\n node x: \"BX\"\n x -> y\n }\n}\n";
        let r = check(src);
        let g = graph(&r);
        assert_eq!(
            (g.edges[0].from.as_str(), g.edges[0].to.as_str()),
            ("be.x", "y")
        );
    }

    #[test]
    fn forward_reference_and_direction_statement() {
        let src = "graph {\n a -> b\n direction: right\n node a: \"A\"\n node b: \"B\"\n}\n";
        let r = check(src);
        assert_eq!(graph(&r).direction, Direction::Right);
        assert_eq!(graph(&r).edges.len(), 1);
    }

    #[test]
    fn dotted_path_through_nested_containers() {
        let src = "graph {\n container o: \"O\" {\n container i: \"I\" {\n node x: \"X\"\n }\n }\n o.i.x -> o.i.x\n}\n";
        let r = check(src);
        let g = graph(&r);
        assert_eq!(g.nodes[0].id, "o.i.x");
        assert_eq!(g.containers[1].id, "o.i");
        assert_eq!(g.containers[1].depth, 2);
        assert_eq!(g.containers[1].parent.as_deref(), Some("o"));
        assert_eq!(r.stats.max_depth, 2);
    }

    #[test]
    fn path_through_a_node_is_not_a_container() {
        let src = "graph {\n container o: \"O\" {\n node leaf: \"L\"\n }\n node a: \"A\"\n a -> o.leaf.x\n}\n";
        let r = check(src);
        assert_eq!(r.diagnostics.len(), 1);
        let d = &r.diagnostics[0];
        assert_eq!(d.code, codes::NOT_A_CONTAINER);
        assert_eq!((d.span.line, d.span.col, d.span.end_col), (6, 14, 15));
    }

    #[test]
    fn container_as_endpoint_is_rejected_with_help() {
        let src = "graph {\n node client: \"C\"\n container backend: \"B\" {\n node gateway: \"G\"\n }\n client -> backend\n}\n";
        let r = check(src);
        assert_eq!(r.diagnostics.len(), 1);
        let d = &r.diagnostics[0];
        assert_eq!(d.code, codes::CONTAINER_ENDPOINT);
        assert!(d.fixes.is_empty());
        assert!(
            d.help
                .as_deref()
                .is_some_and(|h| h.contains("backend.gateway")),
            "{:?}",
            d.help
        );
        assert_eq!((d.span.line, d.span.col), (6, 12));
    }
}
