//! Mermaid-isms inside a `sequence` block: `participant` / `actor ... as ...` declarations,
//! `->>` / `-->>` arrows and `Note over` spellings. Each parses leniently into the canonical
//! AST and reports `diagram/mermaid-syntax` with the exact rewrite as its fix, built only from
//! `fixgen` (quoted labels, token spans). Every loop spends fuel and consumes what it scans.

use super::Parser;
use crate::ast::{ActorDecl, Ident, ItemKind, Label};
use crate::codes;
use crate::fixgen::{mermaid_label, quote, replace};
use crate::lexer::{Token, TokenKind};
use crate::span::Span;

/// Tokens that end a statement (the block loop handles `;`, comments and `}`).
fn is_stmt_end(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Newline
            | TokenKind::Eof
            | TokenKind::RBrace
            | TokenKind::Comment(_)
            | TokenKind::Semi
    )
}

fn word_is(kind: &TokenKind, want: &str) -> bool {
    matches!(kind, TokenKind::Ident(w) if w.eq_ignore_ascii_case(want))
}

impl Parser<'_> {
    /// True when `actor <id>` is followed by `as` or by the end of the statement, which is the
    /// Mermaid spelling; `actor <id>: "<label>"` is canonical.
    pub(super) fn mermaid_actor_ahead(&self) -> bool {
        let after = &self.peek_at(2).kind;
        matches!(after, TokenKind::Ident(w) if w == "as") || is_stmt_end(after)
    }

    /// `participant <id> [as <text>]` or `actor <id> [as <text>]`. The actor is declared even
    /// when the tail is broken, so later messages do not cascade into `undeclared-actor`.
    pub(super) fn mermaid_actor(&mut self) -> Option<ItemKind> {
        let src = self.src;
        let start = self.bump().span.start; // `participant` / `actor`
        let bad = matches!(self.peek().kind, TokenKind::BadIdent(_));
        let id = self.ident("an actor id")?;
        let (mut end, mut raw) = (id.span.end, None);
        if matches!(&self.peek().kind, TokenKind::Ident(w) if w == "as") {
            end = self.bump().span.end;
            raw = self.scan_tail();
            end = raw.map_or(end, |(_, e)| e);
        } else if !is_stmt_end(&self.peek().kind) {
            self.unexpected("`as` or the end of the line");
            self.skip_to_stmt_end();
            return Some(plain_actor(id));
        }
        let text = raw
            .and_then(|(s, e)| src.get(s as usize..e as usize))
            .unwrap_or_default();
        let (decoded, quoted) = mermaid_label(text);
        let (value, quoted) = if decoded.is_empty() {
            (id.name.clone(), quote(&id.name))
        } else {
            (decoded, quoted)
        };
        let span = Span { start, end };
        // A lexically bad id was reported already; rewriting it would only repeat the error. An
        // unterminated alias string is closed by the lexer's own fix first; rewriting the same
        // span in this round would quote the stray `"` into the label.
        let open_str = raw.is_some()
            && matches!(
                self.toks.get(self.pos.wrapping_sub(1)).map(|t| &t.kind),
                Some(TokenKind::Str {
                    terminated: false,
                    ..
                })
            );
        let fixes = if bad || open_str {
            Vec::new()
        } else {
            vec![replace(span, format!("actor {}: {quoted}", id.name))]
        };
        let msg = "Mermaid participant; write `actor <id>: \"<label>\"`";
        self.report_full(codes::MERMAID_SYNTAX, span, msg.into(), None, fixes);
        let label = Label {
            value,
            span: id.span,
        };
        Some(ItemKind::Actor(ActorDecl { id, label }))
    }

    /// Consume tokens up to the end of the statement; the source range they cover.
    fn scan_tail(&mut self) -> Option<(u32, u32)> {
        let mut range: Option<(u32, u32)> = None;
        while self.spend_fuel() && !is_stmt_end(&self.peek().kind) {
            let sp = self.bump().span;
            range = Some((range.map_or(sp.start, |r| r.0), sp.end));
        }
        range
    }

    /// The `>` that directly follows a token ending at `end`, consumed (`->>`, `-->>`).
    fn adjacent_gt(&mut self, end: u32) -> Option<u32> {
        let t = self.peek();
        if matches!(t.kind, TokenKind::Unknown('>')) && t.span.start == end {
            return Some(self.bump().span.end);
        }
        None
    }

    /// The arrow after `from`; returns whether the message is a dashed reply. `->` is
    /// canonical, `->>` is rewritten to it, `-->` / `-->>` become `reply ... ->`, and any other
    /// Mermaid link is reported without a fix (the statement still parses as a message).
    pub(super) fn arrow(&mut self, from: &Ident, reply: bool) -> Option<bool> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Arrow => {
                self.bump();
                if let Some(end) = self.adjacent_gt(tok.span.end) {
                    let span = Span {
                        start: tok.span.start,
                        end,
                    };
                    let msg = "Mermaid message arrow; VoxDiagram writes `->`";
                    let fixes = vec![replace(span, "->")];
                    self.report_full(codes::MERMAID_SYNTAX, span, msg.into(), None, fixes);
                }
                Some(reply)
            }
            TokenKind::MArrow {
                exact: true,
                label: None,
            } => {
                self.bump();
                let end = self.adjacent_gt(tok.span.end).unwrap_or(tok.span.end);
                let span = Span {
                    start: tok.span.start,
                    end,
                };
                let at = from.span.start;
                let mut fixes = Vec::new();
                if !reply {
                    fixes.push(replace(Span { start: at, end: at }, "reply "));
                }
                fixes.push(replace(span, "->"));
                let msg = "Mermaid dashed arrow; write the return as `reply a -> b`";
                self.report_full(codes::MERMAID_SYNTAX, span, msg.into(), None, fixes);
                Some(true)
            }
            TokenKind::MArrow { .. } => {
                self.bump();
                let msg =
                    "this Mermaid link has no VoxDiagram equivalent; use `->` or `reply a -> b`";
                self.report_full(
                    codes::MERMAID_SYNTAX,
                    tok.span,
                    msg.into(),
                    None,
                    Vec::new(),
                );
                Some(reply)
            }
            _ => {
                self.unexpected("`->`");
                None
            }
        }
    }

    /// Lowercase a miscased `Note` / `OVER` (one diagnostic, one fix group).
    pub(super) fn note_case(&mut self, note: &Token, over: &Token) {
        let mut fixes = Vec::new();
        for (tok, want) in [(note, "note"), (over, "over")] {
            if matches!(&tok.kind, TokenKind::Ident(w) if w != want) {
                fixes.push(replace(tok.span, want));
            }
        }
        if !fixes.is_empty() {
            let span = Span {
                start: note.span.start,
                end: over.span.end,
            };
            let msg = "keywords are lowercase; write `note over`";
            self.report_full(codes::MERMAID_SYNTAX, span, msg.into(), None, fixes);
        }
    }

    /// True at `note left of` / `note right of` (any case): a Mermaid side note.
    pub(super) fn note_side_ahead(&self) -> bool {
        let side = &self.peek_at(1).kind;
        (word_is(side, "left") || word_is(side, "right")) && word_is(&self.peek_at(2).kind, "of")
    }

    /// A side note has no equivalent: report it, no fix, and skip the statement.
    pub(super) fn note_side(&mut self) -> Option<ItemKind> {
        let start = self.bump().span.start;
        self.bump();
        let end = self.bump().span.end;
        let msg = "VoxDiagram notes sit over one or two actors: write `note over A`";
        let span = Span { start, end };
        self.report_full(codes::MERMAID_SYNTAX, span, msg.into(), None, Vec::new());
        self.skip_to_stmt_end();
        None
    }

    /// True at `note over` in any case.
    pub(super) fn note_over_ahead(&self) -> bool {
        word_is(&self.peek_at(1).kind, "over")
    }
}

/// An actor whose label is its id, used when the rest of the statement is unusable.
fn plain_actor(id: Ident) -> ItemKind {
    let label = Label {
        value: id.name.clone(),
        span: id.span,
    };
    ItemKind::Actor(ActorDecl { id, label })
}

#[cfg(test)]
mod tests {
    use crate::check::parse;

    #[test]
    fn a_broken_participant_tail_still_declares_the_actor() {
        let p = parse("sequence {\n    participant a \"x\"\n    a -> a\n}\n");
        assert_eq!(p.diagnostics.len(), 1, "{:?}", p.diagnostics);
        assert_eq!(p.diagnostics[0].code, "diagram/unexpected-token");
    }

    #[test]
    fn an_unterminated_alias_is_left_to_the_lexer_fix_first() {
        let p = parse("sequence {\n    participant a as \"open\n}\n");
        let ms: Vec<_> = p
            .diagnostics
            .iter()
            .filter(|d| d.code == "diagram/mermaid-syntax")
            .collect();
        assert!(
            ms.len() == 1 && ms[0].fixes.is_empty(),
            "{:?}",
            p.diagnostics
        );
    }

    #[test]
    fn a_bad_id_gets_no_rewrite() {
        let p = parse("sequence {\n    participant caf\u{e9} as Cafe\n}\n");
        assert!(p.diagnostics.iter().all(|d| d.fixes.is_empty()));
    }
}
