//! Error recovery: the fuel budget, statement-boundary resync and the line scans used by
//! declare-then-diagnose. Invariant: every scan either consumes what it scanned or ends the
//! statement, so total parser work stays linear in the token count.

use super::Parser;
use crate::lexer::TokenKind;

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

    /// Consume through the first `{` on the current line, returning whether there was one.
    /// With no `{` the rest of the line is skipped (consume-or-end).
    pub(super) fn consume_through_lbrace(&mut self) -> bool {
        let mut i = self.pos;
        loop {
            if !self.spend_fuel() {
                return false;
            }
            match self.toks.get(i).map(|t| &t.kind) {
                Some(TokenKind::LBrace) => {
                    self.pos = i + 1;
                    return true;
                }
                Some(TokenKind::Newline | TokenKind::Eof) | None => break,
                Some(_) => i += 1,
            }
        }
        self.skip_to_stmt_end();
        false
    }

    /// True when the token after the current newline begins a `node`/`container` declaration.
    pub(super) fn next_line_starts_decl(&self) -> bool {
        matches!(&self.peek_at(1).kind, TokenKind::Ident(s) if s == "node" || s == "container")
    }
}
