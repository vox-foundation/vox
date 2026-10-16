//! Mermaid-isms. They parse leniently into the canonical AST and report
//! `diagram/mermaid-syntax` (an Error, so the IR is withheld) with the exact canonical rewrite
//! as `fixes[]` wherever one exists; where a rewrite would lose information there is no fix.

use super::Parser;
use crate::codes;
use crate::fixgen::replace;
use crate::span::Span;

impl Parser<'_> {
    /// M1: a `-{2,}>` link becomes `->`; other link styles have no VoxDiagram equivalent.
    pub(super) fn mermaid_arrow(&mut self, span: Span, exact: bool, labelled: bool) {
        let fixes = if exact && !labelled {
            vec![replace(span, "->")]
        } else {
            Vec::new()
        };
        let msg = if exact {
            "Mermaid link; VoxDiagram edges are written `->`"
        } else {
            "this link style has no VoxDiagram equivalent; edges are written `->`"
        };
        self.report_full(codes::MERMAID_SYNTAX, span, msg.into(), None, fixes);
    }
}
