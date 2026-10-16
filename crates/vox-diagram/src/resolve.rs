//! Scope resolution: declare every node first (so forward references work), then resolve edges
//! and build the IR in declaration order. Scopes use ordered maps, never hash-based ones.

use std::collections::BTreeMap;

use crate::ast::{Document, Entry, ItemKind};
use crate::codes;
use crate::diag::{Diagnostic, Stats};
use crate::ir::{Direction, GraphIr, IrEdge, IrNode};
use crate::span::{LineIndex, Span, to_u32};

/// Resolve `doc` into IR plus semantic diagnostics.
pub(crate) fn resolve(
    doc: &Document,
    src: &str,
    idx: &LineIndex,
) -> (GraphIr, Stats, Vec<Diagnostic>) {
    let mut ir = GraphIr {
        direction: Direction::default(),
        nodes: Vec::new(),
        containers: Vec::new(),
        edges: Vec::new(),
    };
    let mut diags = Vec::new();
    // ponytail: root scope only and first-wins on duplicates (no duplicate-id diagnostic yet);
    // plan 03 adds nested scopes and duplicate/shadow reporting.
    let mut scope: BTreeMap<&str, usize> = BTreeMap::new();

    for Entry::Item(item) in &doc.body.entries {
        if let ItemKind::Node(n) = &item.kind {
            scope.entry(n.id.name.as_str()).or_insert(ir.nodes.len());
            ir.nodes.push(IrNode {
                id: n.id.name.clone(),
                local_id: n.id.name.clone(),
                label: n.label.value.clone(),
                shape: n.shape,
                container: None,
                span: idx.location(src, item.span),
            });
        }
    }
    for Entry::Item(item) in &doc.body.entries {
        if let ItemKind::Edge(e) = &item.kind {
            let mut ends: Vec<String> = Vec::new();
            for r in [&e.from, &e.to] {
                let path = r
                    .segments
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                if scope.contains_key(path.as_str()) {
                    ends.push(path);
                } else {
                    let span = idx.location(
                        src,
                        Span {
                            start: r.span.start,
                            end: r.span.end,
                        },
                    );
                    let msg = format!("node `{path}` is not declared");
                    diags.push(Diagnostic::error(codes::UNDECLARED_NODE, msg, span));
                }
            }
            if let [from, to] = ends.as_slice() {
                ir.edges.push(IrEdge {
                    from: from.clone(),
                    to: to.clone(),
                    label: e.label.as_ref().map(|l| l.value.clone()),
                    span: idx.location(src, item.span),
                });
            }
        }
    }
    let stats = Stats {
        nodes: to_u32(ir.nodes.len()),
        containers: 0,
        edges: to_u32(ir.edges.len()),
        max_depth: 0,
    };
    (ir, stats, diags)
}
