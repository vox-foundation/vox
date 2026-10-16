//! Mermaid-isms. They parse leniently into the canonical AST and report
//! `diagram/mermaid-syntax` (an Error, so the IR is withheld) with the exact canonical rewrite
//! as `fixes[]` wherever one exists; where a rewrite would lose information there is no fix.
//! Every loop spends fuel and consumes what it scans, so the parser stays linear.

use super::Parser;
use crate::codes;
use crate::fixgen::{indent_of, replace};
use crate::ir::Direction;
use crate::lexer::TokenKind;
use crate::span::Span;

/// A Mermaid direction word as `(direction, VoxDiagram word)`.
pub(super) fn mermaid_dir(word: &str) -> Option<(Direction, &'static str)> {
    match word {
        "TD" | "TB" => Some((Direction::Down, "down")),
        "BT" => Some((Direction::Up, "up")),
        "LR" => Some((Direction::Right, "right")),
        "RL" => Some((Direction::Left, "left")),
        _ => None,
    }
}

impl Parser<'_> {
    /// M6: `direction LR` / `direction: LR`.
    pub(super) fn mermaid_direction(&mut self, span: Span, word: Option<&'static str>) {
        let fixes = word
            .map(|w| replace(span, format!("direction: {w}")))
            .into_iter()
            .collect();
        let msg = "Mermaid direction; write `direction: down|right|up|left`";
        self.report_full(codes::MERMAID_SYNTAX, span, msg.into(), None, fixes);
    }

    /// M5: a `graph`/`flowchart` header with an optional direction word and `;` but no `{`.
    /// Consumes the header and returns its span and direction word.
    pub(super) fn mermaid_header(&mut self) -> Option<(Span, Option<&'static str>)> {
        let TokenKind::Ident(head) = &self.peek().kind else {
            return None;
        };
        if head != "graph" && head != "flowchart" {
            return None;
        }
        let start = self.peek().span.start;
        let mut n = 1;
        let mut dir = None;
        if let TokenKind::Ident(w) = &self.peek_at(n).kind
            && let Some((_, word)) = mermaid_dir(w)
        {
            dir = Some(word);
            n += 1;
        }
        if matches!(self.peek_at(n).kind, TokenKind::Semi) {
            n += 1;
        }
        let ends = matches!(
            self.peek_at(n).kind,
            TokenKind::Newline | TokenKind::Eof | TokenKind::Comment(_)
        );
        if !ends {
            return None;
        }
        // `graph` then `{` on a later line is a missing-brace typo, not a Mermaid header.
        let mut m = n;
        while matches!(self.peek_at(m).kind, TokenKind::Newline) {
            m += 1;
        }
        if matches!(self.peek_at(m).kind, TokenKind::LBrace) {
            return None;
        }
        let end = self.peek_at(n - 1).span.end;
        self.pos += n;
        Some((Span { start, end }, dir))
    }

    /// The M5 diagnostic: the header becomes `graph {` (+ `direction`) and a `}` closes the
    /// block at the end of the input.
    pub(super) fn report_header(&mut self, header: Span, dir: Option<&'static str>) {
        let open = match dir {
            Some(w) => format!("graph {{\n    direction: {w}"),
            None => "graph {".to_string(),
        };
        let eof = self.toks.last().map_or(0, |t| t.span.start);
        let close = if self.src.ends_with('\n') {
            "}\n"
        } else {
            "\n}\n"
        };
        let fixes = vec![
            replace(header, open),
            replace(
                Span {
                    start: eof,
                    end: eof,
                },
                close,
            ),
        ];
        let msg = "Mermaid header; start with `graph {` and close the block with `}`";
        self.report_full(codes::MERMAID_SYNTAX, header, msg.into(), None, fixes);
    }

    /// M8: `;` ends the statement. Trailing, it is deleted; mid-line, it becomes a line break.
    pub(super) fn semicolon(&mut self) {
        let span = self.bump().span;
        let trailing = matches!(
            self.peek().kind,
            TokenKind::Newline | TokenKind::Eof | TokenKind::RBrace | TokenKind::Comment(_)
        );
        // Mid-line, the blanks after `;` go with it so the next statement keeps the indent.
        let (cut, text) = if trailing {
            (span, String::new())
        } else {
            let end = self.peek().span.start;
            let indent = indent_of(self.src, span.start);
            (
                Span {
                    start: span.start,
                    end,
                },
                format!("\n{indent}"),
            )
        };
        let msg = "Mermaid statement terminator; VoxDiagram statements end at the line break";
        self.report_full(
            codes::MERMAID_SYNTAX,
            span,
            msg.into(),
            None,
            vec![replace(cut, text)],
        );
        if matches!(self.peek().kind, TokenKind::Newline) {
            self.bump();
        }
    }

    /// M9: a styling line, already one token. No fix: theming is not part of the DSL.
    pub(super) fn styling(&mut self) {
        let span = self.bump().span;
        self.report_styling(span);
    }

    pub(super) fn report_styling(&mut self, span: Span) {
        let msg = "styling is theme-controlled; VoxDiagram has no equivalent";
        self.report_full(codes::MERMAID_SYNTAX, span, msg.into(), None, Vec::new());
    }
}
