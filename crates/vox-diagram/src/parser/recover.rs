//! Error recovery: the fuel budget, statement-boundary resync and the line scans used by
//! declare-then-diagnose. Invariant: every scan either consumes what it scanned or ends the
//! statement, so total parser work stays linear in the token count.

use super::Parser;
use crate::ast::Label;
use crate::codes;
use crate::diag::clip;
use crate::lexer::TokenKind;
use crate::span::Span;

impl Parser<'_> {
    /// Spend one unit of the work budget (`4 * tokens + 1_000`): once per statement-loop
    /// iteration and per scanned token. On exhaustion the report gets its
    /// `limit-diagnostics` sentinel and the parser halts. Returns whether work may continue.
    pub(super) fn spend_fuel(&mut self) -> bool {
        if self.fuel == 0 {
            if !self.halted {
                self.halted = true;
                self.diags.trip();
            }
            return false;
        }
        self.fuel -= 1;
        true
    }

    /// Advance to the end of the current statement: the next newline, EOF, or a `}` that
    /// belongs to the enclosing block, never consuming it. Braces opened inside the skipped
    /// region are skipped with it, newlines included, unless the next line starts a
    /// `node`/`container` declaration.
    pub(super) fn skip_to_stmt_end(&mut self) {
        let mut depth = 0usize;
        loop {
            if !self.spend_fuel() {
                return;
            }
            match &self.peek().kind {
                TokenKind::Eof => return,
                TokenKind::Newline if depth == 0 || self.next_line_starts_decl() => return,
                TokenKind::RBrace if depth == 0 => return,
                TokenKind::RBrace => depth -= 1,
                TokenKind::LBrace => depth += 1,
                _ => {}
            }
            self.bump();
        }
    }

    /// Consume through the first `{` on the current line, returning its span. With no `{`
    /// the rest of the line is skipped (consume-or-end) and `None` comes back.
    pub(super) fn consume_through_lbrace(&mut self) -> Option<Span> {
        let mut i = self.pos;
        loop {
            if !self.spend_fuel() {
                return None;
            }
            match self.toks.get(i) {
                Some(t) if matches!(t.kind, TokenKind::LBrace) => {
                    self.pos = i + 1;
                    return Some(t.span);
                }
                Some(t) if !matches!(t.kind, TokenKind::Newline | TokenKind::Eof) => i += 1,
                _ => break,
            }
        }
        self.skip_to_stmt_end();
        None
    }

    /// Drop every remaining token without a diagnostic.
    pub(super) fn skip_rest(&mut self) {
        self.pos = self.toks.len().saturating_sub(1);
    }

    /// `unclosed-brace` at the `{` that opened the scope.
    pub(super) fn unclosed(&mut self, open: Span) {
        let msg = "unclosed `{`; add a matching `}`".to_string();
        self.report(codes::UNCLOSED_BRACE, open, msg);
    }

    /// After a broken attribute: skip to the `}` that closes the attribute block and consume
    /// it. Stops without consuming at EOF or at a line-leading `node`/`container` (reporting
    /// the block as unclosed), so one bad attribute cannot swallow the rest of the file.
    pub(super) fn resync_attr(&mut self, open: Span) {
        loop {
            if !self.spend_fuel() {
                return;
            }
            match &self.peek().kind {
                TokenKind::RBrace => {
                    self.bump();
                    return;
                }
                TokenKind::Eof => {
                    self.unclosed(open);
                    return;
                }
                TokenKind::Newline if self.next_line_starts_decl() => {
                    self.unclosed(open);
                    return;
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// Label text without double quotes (`Label text`, `'x'`): tokens up to the end of the
    /// line, or up to `{` when `stop_lbrace`. Reports `expected-quoted-label` and returns the
    /// recovered label; `None` when no such tokens exist.
    pub(super) fn unquoted_label(&mut self, stop_lbrace: bool) -> Option<Label> {
        let start = self.peek().span.start;
        let mut end = None;
        while self.spend_fuel() {
            let t = self.peek();
            let stop = match t.kind {
                TokenKind::Newline | TokenKind::Eof | TokenKind::RBrace | TokenKind::Comment(_) => {
                    true
                }
                TokenKind::LBrace => stop_lbrace,
                _ => false,
            };
            if stop {
                break;
            }
            end = Some(t.span.end);
            self.bump();
        }
        let span = Span { start, end: end? };
        let raw = self.src.get(span.start as usize..span.end as usize);
        let raw = raw.unwrap_or_default();
        let text = raw
            .strip_prefix('\'')
            .and_then(|r| r.strip_suffix('\''))
            .unwrap_or(raw);
        let msg = format!("labels must be double-quoted; write \"{}\"", clip(text));
        self.report(codes::EXPECTED_QUOTED_LABEL, span, msg);
        Some(Label {
            value: text.to_string(),
            span,
        })
    }

    /// True when the token after the current newline begins a `node`/`container` declaration.
    pub(super) fn next_line_starts_decl(&self) -> bool {
        matches!(&self.peek_at(1).kind, TokenKind::Ident(s) if s == "node" || s == "container")
    }
}
