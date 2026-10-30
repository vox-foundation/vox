//! Public entry points: [`check`] (full report) and [`parse`] (best-effort AST).

use crate::ast::{DiagramKind, Document};
use crate::codes;
use crate::diag::{CheckReport, Collector, Diagnostic, Stats};
use crate::ir::Ir;
use crate::lexer::lex;
use crate::limits::MAX_SOURCE_BYTES;
use crate::parser::parse_tokens;
use crate::resolve::resolve;
use crate::resolve::seq::resolve_sequence;
use crate::span::{LineIndex, Location, Span, to_u32};

/// Result of [`parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    /// The syntax tree; `None` only when the size gate tripped.
    pub ast: Option<Document>,
    /// Syntax diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// One `limit-source` diagnostic when `source` exceeds the cap (checked before any lexing).
fn size_gate(source: &str) -> Option<Diagnostic> {
    if source.len() <= MAX_SOURCE_BYTES {
        return None;
    }
    let mut cut = MAX_SOURCE_BYTES;
    while !source.is_char_boundary(cut) {
        cut -= 1;
    }
    let span = Span {
        start: to_u32(cut),
        end: to_u32(source.len()),
    };
    let loc = LineIndex::new(source).location(source, span);
    let msg = format!(
        "source is {} bytes; the limit is {MAX_SOURCE_BYTES}",
        source.len()
    );
    Some(Diagnostic::error(codes::LIMIT_SOURCE, msg, loc))
}

/// Lex and parse into `sink`; the lexer's diagnostics go in first.
fn parse_into(source: &str, idx: &LineIndex, sink: &mut Collector) -> Document {
    let (toks, lexical) = lex(source, idx);
    sink.extend(lexical);
    parse_tokens(source, idx, &toks, sink)
}

fn eof_location(source: &str, idx: &LineIndex) -> Location {
    let end = to_u32(source.len());
    idx.location(source, Span { start: end, end })
}

/// Lex and parse `source`. Never panics.
pub fn parse(source: &str) -> Parsed {
    if let Some(d) = size_gate(source) {
        return Parsed {
            ast: None,
            diagnostics: vec![d],
        };
    }
    let idx = LineIndex::new(source);
    let mut sink = Collector::new();
    let doc = parse_into(source, &idx, &mut sink);
    Parsed {
        ast: Some(doc),
        diagnostics: sink.finish(eof_location(source, &idx)),
    }
}

/// Check `source` end to end: size gate, lex, parse, resolve. The IR is withheld while any
/// `Error` diagnostic exists. Never panics.
pub fn check(source: &str) -> CheckReport {
    if let Some(d) = size_gate(source) {
        return finish(vec![d], true, Stats::default(), None);
    }
    let idx = LineIndex::new(source);
    let mut sink = Collector::new();
    let doc = parse_into(source, &idx, &mut sink);
    let (resolved, stats, semantic) = match doc.kind {
        DiagramKind::Graph => {
            let (graph, stats, semantic) = resolve(&doc, source, &idx);
            (Ir::Graph(graph), stats, semantic)
        }
        DiagramKind::Sequence => {
            let (seq, stats, semantic) = resolve_sequence(&doc, source, &idx);
            (Ir::Sequence(seq), stats, semantic)
        }
    };
    sink.extend(semantic);
    // Read before `finish`: a dropped Error still invalidates the report.
    let has_error = sink.has_error();
    let diagnostics = sink.finish(eof_location(source, &idx));
    let ir = (!has_error).then_some(resolved);
    finish(diagnostics, has_error, stats, ir)
}

fn finish(
    diagnostics: Vec<Diagnostic>,
    has_error: bool,
    stats: Stats,
    ir: Option<Ir>,
) -> CheckReport {
    let valid = !has_error;
    CheckReport {
        schema_version: 1,
        valid,
        diagnostics,
        stats,
        ir,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Direction, Shape};

    const MINIMAL: &str = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n";

    #[test]
    fn minimal_graph_is_valid_with_ir() {
        let r = check(MINIMAL);
        assert!(r.valid);
        assert_eq!((r.stats.nodes, r.stats.edges), (2, 1));
        let Some(Ir::Graph(g)) = r.ir else {
            panic!("expected graph ir")
        };
        assert_eq!(g.direction, Direction::Down);
        assert_eq!(g.nodes[0].id, "a");
        assert_eq!(g.nodes[0].label, "A");
        assert_eq!(g.nodes[0].shape, Shape::Rect);
        assert_eq!(
            (g.edges[0].from.as_str(), g.edges[0].to.as_str()),
            ("a", "b")
        );
        assert_eq!(g.edges[0].label, None);
    }

    #[test]
    fn malformed_statement_is_one_unexpected_token_without_ir() {
        let r = check("graph {\n    node a \"A\"\n}\n");
        assert!(!r.valid && r.ir.is_none());
        assert_eq!(r.diagnostics.len(), 1);
        assert_eq!(r.diagnostics[0].code, codes::UNEXPECTED_TOKEN);
        assert_eq!(r.diagnostics[0].span.line, 2);
    }

    #[test]
    fn independent_errors_are_all_reported() {
        let r = check("graph {\n    node a \"A\"\n    node b \"B\"\n}\n");
        assert_eq!(r.diagnostics.len(), 2);
        assert_eq!(r.diagnostics[1].span.line, 3);
    }

    #[test]
    fn labelled_edge_and_forward_reference_resolve() {
        let r = check("graph {\n    a -> b: \"go\"\n    node a: \"A\"\n    node b: \"B\"\n}\n");
        let Some(Ir::Graph(g)) = r.ir else {
            panic!("expected graph ir")
        };
        assert_eq!(g.edges[0].label.as_deref(), Some("go"));
    }

    #[test]
    fn oversized_source_is_gated_before_lexing() {
        let big = "é".repeat(40_000); // 80_000 bytes, cut lands mid-char and is pulled back
        let r = check(&big);
        assert_eq!(r.diagnostics.len(), 1);
        assert_eq!(r.diagnostics[0].code, codes::LIMIT_SOURCE);
        assert_eq!(r.diagnostics[0].span.start, 65_536);
        assert!(parse(&big).ast.is_none());
    }

    #[test]
    fn garbage_never_panics() {
        for src in [
            "",
            "}",
            "graph",
            "graph {",
            "graph { a -> }",
            "\"\u{0}",
            "node",
        ] {
            let r = check(src);
            assert!(!r.valid, "{src:?}");
        }
    }
}
