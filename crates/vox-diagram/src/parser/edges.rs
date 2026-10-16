//! Edge statements, Mermaid flavoured: chains (`a -> b -> c`), `&` fan-out, link labels
//! (`-->|text|`, `-- text -->`) and bracketed endpoints (`A[Label]`). Each parses into the
//! canonical edges and reports the exact rewrite where one exists.

use super::Parser;
use super::stmt::Owner;
use crate::ast::{Bracket, EdgeStmt, ItemKind, Label, NodeDecl, Ref};
use crate::codes;
use crate::diag::Fix;
use crate::fixgen::{indent_of, mermaid_label, quote, replace};
use crate::ir::Shape;
use crate::lexer::TokenKind;
use crate::span::Span;

/// Most edges one chain or fan-out statement expands to; past it the statement gets a
/// diagnostic without a fix (and the lenient AST keeps only the first edges).
// ponytail: fixed cap, raise it if real diagrams fan out wider than this.
const MAX_EXPANSION: usize = 100;

/// One link between two endpoint groups.
struct Hop {
    span: Span,
    /// The head is `->` or `-{2,}>`, so `->` replaces it exactly.
    exact: bool,
    /// Raw `|text|` or `-- text -->` label.
    label: Option<String>,
    /// It already is the canonical `->`.
    plain: bool,
}

/// The shape a bracket opener stands for, when VoxDiagram has an exact equivalent.
fn bracket_shape(open: &str) -> Option<Shape> {
    match open {
        "[" => Some(Shape::Rect),
        "(" => Some(Shape::Rounded),
        "{" => Some(Shape::Diamond),
        "[(" => Some(Shape::Cylinder),
        _ => None,
    }
}

fn dotted(r: &Ref) -> String {
    let names: Vec<&str> = r.segments.iter().map(|s| s.name.as_str()).collect();
    names.join(".")
}

impl Parser<'_> {
    /// An edge statement, or a lone `A[Label]` node declaration. Mermaid chains, `&` fan-out,
    /// link labels and bracketed endpoints are accepted and reported.
    pub(super) fn edge_stmt(&mut self) -> Option<ItemKind> {
        let start = self.peek().span.start;
        let mut groups = vec![self.group()?];
        let mut hops: Vec<Hop> = Vec::new();
        while let Some(hop) = self.hop() {
            hops.push(hop);
            groups.push(self.group()?);
            if !self.spend_fuel() {
                return None;
            }
        }
        if hops.is_empty() {
            return self.lone_endpoint(groups);
        }
        let mut label = None;
        if matches!(self.peek().kind, TokenKind::Colon) {
            // The last token before the colon, so a bracket on the target stays.
            let from = self.toks[self.pos - 1].span.end;
            self.bump();
            label = self.label(false, Owner::Edge { from });
            if label.is_none() {
                self.skip_to_stmt_end();
            }
        }
        let end = self.toks[self.pos - 1].span.end;
        let stmt = Span { start, end };
        self.report_hops(&groups, &hops, label.as_ref(), stmt);
        self.build_edges(&groups, &hops, label, stmt)
    }

    fn hop(&mut self) -> Option<Hop> {
        let t = self.peek();
        let span = t.span;
        let hop = match &t.kind {
            TokenKind::Arrow => Hop {
                span,
                exact: true,
                label: None,
                plain: true,
            },
            TokenKind::MArrow { exact, label } => Hop {
                span,
                exact: *exact,
                label: label.clone(),
                plain: false,
            },
            _ => return None,
        };
        self.bump();
        Some(hop)
    }

    /// `endpoint { & endpoint }`.
    fn group(&mut self) -> Option<Vec<Ref>> {
        let mut g = vec![self.endpoint()?];
        while matches!(self.peek().kind, TokenKind::Amp) {
            self.bump();
            g.push(self.endpoint()?);
            if !self.spend_fuel() {
                return None;
            }
        }
        Some(g)
    }

    /// A reference, optionally with Mermaid bracket text and a `:::class` suffix.
    fn endpoint(&mut self) -> Option<Ref> {
        let mut r = self.reference()?;
        if let TokenKind::Bracket { open, text, closed } = self.peek().kind.clone() {
            let tok = self.bump();
            let last = r.segments.last().map_or(r.span.start, |s| s.span.start);
            let (label, quoted) = mermaid_label(&text);
            r.bracket = Some(Box::new(Bracket {
                label,
                quoted,
                shape: bracket_shape(open),
                form: Span {
                    start: last,
                    end: tok.span.end,
                },
                closed,
            }));
        }
        if matches!(self.peek().kind, TokenKind::StyleSuffix) {
            let span = self.bump().span;
            self.report_styling(span);
        }
        Some(r)
    }

    /// A statement that is just `A[Label]` declares a node; anything else lacks its arrow.
    fn lone_endpoint(&mut self, mut groups: Vec<Vec<Ref>>) -> Option<ItemKind> {
        let mut only = groups.pop().unwrap_or_default();
        if only.len() == 1
            && let Some(r) = only.pop()
            && let (1, Some(b)) = (r.segments.len(), r.bracket.clone())
        {
            return Some(ItemKind::Node(NodeDecl {
                id: r.segments[0].clone(),
                label: Label {
                    value: b.label.clone(),
                    span: b.form,
                },
                shape: b.shape.unwrap_or_default(),
                bracket: Some(b),
            }));
        }
        self.unexpected("`->`");
        None
    }

    /// The lenient AST: one edge per (from, to) pair of every link; the first is returned and
    /// the rest go to `extra`.
    fn build_edges(
        &mut self,
        groups: &[Vec<Ref>],
        hops: &[Hop],
        trailing: Option<Label>,
        stmt: Span,
    ) -> Option<ItemKind> {
        let mut out: Vec<ItemKind> = Vec::new();
        'hops: for (w, hop) in hops.iter().enumerate() {
            let label = match (&trailing, w + 1 == hops.len()) {
                (Some(l), true) => Some(l.clone()),
                _ => hop.label.as_ref().map(|raw| Label {
                    value: mermaid_label(raw).0,
                    span: hop.span,
                }),
            };
            for from in &groups[w] {
                for to in &groups[w + 1] {
                    if out.len() >= MAX_EXPANSION {
                        break 'hops;
                    }
                    out.push(ItemKind::Edge(EdgeStmt {
                        from: from.clone(),
                        to: to.clone(),
                        label: label.clone(),
                    }));
                }
            }
        }
        let mut edges = out.into_iter();
        let first = edges.next();
        self.extra.extend(edges.map(|k| (k, stmt)));
        first
    }

    /// The edge lines a chain or fan-out statement expands to, or `None` when no exact
    /// rewrite exists (bracketed endpoints, inexact links, two labels on one link).
    fn expansion(
        &self,
        groups: &[Vec<Ref>],
        hops: &[Hop],
        trailing: Option<&Label>,
        stmt: Span,
    ) -> Option<String> {
        let plain = groups.iter().flatten().all(|r| r.bracket.is_none());
        if !plain || !hops.iter().all(|h| h.exact) {
            return None;
        }
        let mut lines: Vec<String> = Vec::new();
        for (w, hop) in hops.iter().enumerate() {
            let tail = trailing.filter(|_| w + 1 == hops.len());
            let label = match (&hop.label, tail) {
                (Some(_), Some(_)) => return None,
                (Some(raw), None) => Some(mermaid_label(raw).1),
                (None, Some(l)) => Some(quote(&l.value)),
                (None, None) => None,
            };
            for from in &groups[w] {
                for to in &groups[w + 1] {
                    let mut line = format!("{} -> {}", dotted(from), dotted(to));
                    if let Some(l) = &label {
                        line.push_str(": ");
                        line.push_str(l);
                    }
                    lines.push(line);
                    if lines.len() > MAX_EXPANSION {
                        return None;
                    }
                }
            }
        }
        Some(lines.join(&format!("\n{}", indent_of(self.src, stmt.start))))
    }

    fn report_hops(
        &mut self,
        groups: &[Vec<Ref>],
        hops: &[Hop],
        trailing: Option<&Label>,
        stmt: Span,
    ) {
        let chained = groups.len() > 2;
        let multi = chained || groups.iter().any(|g| g.len() > 1);
        if multi {
            let fixes: Vec<Fix> = self
                .expansion(groups, hops, trailing, stmt)
                .map(|text| replace(stmt, text))
                .into_iter()
                .collect();
            let whole = !fixes.is_empty();
            let (code, msg) = if chained {
                (
                    codes::CHAINED_EDGE,
                    "chained edges; write one edge per line",
                )
            } else {
                (
                    codes::MERMAID_SYNTAX,
                    "`&` fan-out; write one edge per line",
                )
            };
            self.report_full(code, stmt, msg.into(), None, fixes);
            if whole {
                return;
            }
        }
        for hop in hops.iter().filter(|h| !h.plain) {
            let fixes = match (&hop.label, hop.exact) {
                (_, false) => Vec::new(),
                (None, true) => vec![replace(hop.span, "->")],
                (Some(raw), true) if !multi && trailing.is_none() => {
                    // After the bracket text too, so a pending `C(Label)` rewrite stays whole.
                    let to = &groups[1][0];
                    let at = to.bracket.as_ref().map_or(to.span.end, |b| b.form.end);
                    let insert = Span { start: at, end: at };
                    let text = format!(": {}", mermaid_label(raw).1);
                    vec![replace(hop.span, "->"), replace(insert, text)]
                }
                (Some(_), true) => Vec::new(),
            };
            let msg = match (hop.exact, hop.label.is_some()) {
                (false, _) => {
                    "this link style has no VoxDiagram equivalent; edges are written `->`"
                }
                (true, true) => "Mermaid link label; write `a -> b: \"text\"`",
                (true, false) => "Mermaid link; VoxDiagram edges are written `->`",
            };
            self.report_full(codes::MERMAID_SYNTAX, hop.span, msg.into(), None, fixes);
        }
    }
}
