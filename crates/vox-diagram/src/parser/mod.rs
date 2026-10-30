//! Error-recovering parser for the `graph { ... }` block. Recovery re-syncs at the next newline
//! or `}` so independent mistakes are all reported.
//!
//! Bounded time (DGM-06). The lexer is one pass, O(n). The parser is O(tokens): every
//! statement-loop iteration and every scanned token spends one unit of fuel
//! (`4 * tokens + 1_000`) and every scan either consumes what it scanned or ends the
//! statement, so nothing is rescanned. Recursion is bounded by `MAX_DEPTH`: a container past
//! the cap has its body skipped with a brace counter instead of a recursive call. The
//! resolver is O(edges * depth * log nodes) and `LineIndex` lookups are O(log lines).
//! Diagnostics are capped by the collector, and the parser stops once it is full.

use crate::ast::{Block, Comment, DiagramKind, Document, Entry, Item, ItemKind};
use crate::codes;
use crate::diag::{Collector, Diagnostic, Fix, clip};
use crate::lexer::{Token, TokenKind};
use crate::span::{LineIndex, Span};
use recover::describe;

mod edges;
mod frame;
mod mermaid;
mod recover;
mod seq;
mod seq_mermaid;
mod stmt;
#[cfg(test)]
mod tests;

struct Parser<'a> {
    src: &'a str,
    idx: &'a LineIndex,
    toks: &'a [Token],
    pos: usize,
    diags: &'a mut Collector,
    /// Work budget, see [`Parser::spend_fuel`].
    fuel: usize,
    /// Set when the collector is full or the fuel ran out; parsing stops.
    halted: bool,
    /// A `direction` statement was already seen in the graph block.
    seen_direction: bool,
    /// The header word is `sequence`: statements are actors, messages and notes.
    seq: bool,
    /// Containers currently being parsed; capped at `MAX_DEPTH`, which bounds recursion.
    depth: usize,
    nodes: usize,
    edges: usize,
    /// Spans of the first node / edge past the caps, reported once at the end with totals.
    first_extra_node: Option<Span>,
    first_extra_edge: Option<Span>,
    /// The block being parsed is a Mermaid `subgraph ... end` frame.
    mframe: bool,
    /// Span of the `end` that closed the last frame.
    frame_end: Option<Span>,
    /// Edges of a statement past its first (chains, fan-out); the block loop adds them.
    extra: Vec<(ItemKind, Span)>,
}

/// Parse a token stream (ending in `Eof`) into a document; diagnostics go to `diags`.
pub(crate) fn parse_tokens(
    src: &str,
    idx: &LineIndex,
    toks: &[Token],
    diags: &mut Collector,
) -> Document {
    run(src, idx, toks, diags, 4 * toks.len() + 1_000)
}

fn run(src: &str, idx: &LineIndex, toks: &[Token], diags: &mut Collector, fuel: usize) -> Document {
    let halted = diags.is_full();
    let mut p = Parser {
        src,
        idx,
        toks,
        pos: 0,
        diags,
        fuel,
        halted,
        seen_direction: false,
        seq: false,
        depth: 0,
        nodes: 0,
        edges: 0,
        first_extra_node: None,
        first_extra_edge: None,
        mframe: false,
        frame_end: None,
        extra: Vec::new(),
    };
    p.document()
}

impl Parser<'_> {
    fn peek(&self) -> &Token {
        // `toks` always ends in Eof and `bump` never moves past it.
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }

    fn peek_at(&self, n: usize) -> &Token {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)]
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        if !matches!(t.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        t
    }

    fn unexpected(&mut self, expected: &str) {
        let tok = self.peek();
        let message = format!("expected {expected}, found {}", describe(&tok.kind));
        let span = tok.span;
        self.error_at(span, message);
    }

    /// An error with `code` over `span` plus a `help` line.
    fn report_help(
        &mut self,
        code: &'static str,
        span: Span,
        message: String,
        help: Option<String>,
    ) {
        self.report_full(code, span, message, help, Vec::new());
    }

    /// An error with `code` over `span`, an optional `help` and machine-applicable `fixes`.
    fn report_full(
        &mut self,
        code: &'static str,
        span: Span,
        message: String,
        help: Option<String>,
        fixes: Vec<Fix>,
    ) {
        let loc = self.idx.location(self.src, span);
        let mut d = Diagnostic::error(code, message, loc);
        d.help = help;
        d.fixes = fixes;
        if self.diags.push(d) {
            self.halted = true;
        }
    }

    /// An `unexpected-token` error over `span`.
    fn error_at(&mut self, span: Span, message: String) {
        self.report(codes::UNEXPECTED_TOKEN, span, message);
    }

    /// An error with `code` over `span`; a full collector halts parsing.
    fn report(&mut self, code: &'static str, span: Span, message: String) {
        self.report_help(code, span, message, None);
    }

    fn expect(&mut self, want: &TokenKind, expected: &str) -> Option<Token> {
        if std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(want) {
            Some(self.bump())
        } else {
            self.unexpected(expected);
            None
        }
    }

    /// A comment on the current line, if the next token is one.
    fn trailing_comment(&mut self) -> Option<Comment> {
        let tok = self.peek().clone();
        let TokenKind::Comment(text) = tok.kind else {
            return None;
        };
        self.bump();
        Some(Comment {
            text,
            blank_before: false,
            span: tok.span,
        })
    }

    /// Own-line comments (and the newlines between them) outside any block. A comment is
    /// `blank_before` iff a blank line separates it from the previous comment.
    fn own_line_comments(&mut self) -> Vec<Comment> {
        let mut out: Vec<Comment> = Vec::new();
        let mut blank = false;
        loop {
            match self.peek().kind {
                TokenKind::Newline => {
                    self.bump();
                    blank = true;
                }
                TokenKind::Comment(_) => {
                    if let Some(mut c) = self.trailing_comment() {
                        c.blank_before = blank && !out.is_empty();
                        out.push(c);
                    }
                    if matches!(self.peek().kind, TokenKind::Newline) {
                        self.bump();
                    }
                    blank = false;
                }
                _ => return out,
            }
        }
    }

    fn document(&mut self) -> Document {
        let mut body = Block::default();
        let leading = self.own_line_comments();
        let start = self.peek().span.start;
        let (mut header_trailing, mut close_trailing) = (None, None);
        let head = self.peek().clone();
        match (self.mermaid_header(), &head.kind) {
            (Some((header, dir)), _) => {
                header_trailing = self.trailing_comment();
                body = self.block(true, None);
                close_trailing = self.trailing_comment();
                self.report_header(header, dir);
            }
            (None, TokenKind::Ident(s)) if s == "graph" || s == "sequence" => {
                self.seq = s == "sequence";
                self.bump();
                let open = if matches!(self.peek().kind, TokenKind::LBrace) {
                    Some(self.bump().span)
                } else {
                    self.unexpected("`{`");
                    None
                };
                header_trailing = self.trailing_comment();
                body = self.block(true, open);
                close_trailing = self.trailing_comment();
            }
            (None, TokenKind::Ident(s)) => {
                let msg = format!(
                    "unknown diagram kind `{}`; the supported kinds are `graph` and `sequence`",
                    clip(s)
                );
                self.report(codes::UNKNOWN_DIAGRAM_KIND, head.span, msg);
                self.skip_rest();
            }
            (None, _) => {
                self.unexpected("`graph`");
                self.skip_rest();
            }
        }
        // Anything after the closing brace except newlines and comments is one error.
        let trailing = self.own_line_comments();
        self.report_caps();
        if !self.halted && !matches!(self.peek().kind, TokenKind::Eof) {
            self.trailing_content();
        }
        Document {
            kind: if self.seq {
                DiagramKind::Sequence
            } else {
                DiagramKind::Graph
            },
            leading,
            body,
            header_trailing,
            close_trailing,
            trailing,
            span: Span {
                start,
                end: self.peek().span.end,
            },
        }
    }

    /// Entries up to and including the closing `}`; `open` is the span of this block's `{`
    /// (reported as `unclosed-brace` when EOF comes first).
    fn block(&mut self, root: bool, open: Option<Span>) -> Block {
        let mut entries = Vec::new();
        if matches!(self.peek().kind, TokenKind::Newline) {
            self.bump();
        }
        let mut blank = false;
        let mut last_pos = usize::MAX;
        let mut close = None;
        loop {
            if self.halted || !self.spend_fuel() {
                break;
            }
            debug_assert!(self.pos != last_pos, "parser stalled at token {}", self.pos);
            last_pos = self.pos;
            // A blank line never marks the first entry of a block.
            let blank_before = blank && !entries.is_empty();
            match self.peek().kind {
                TokenKind::Newline => {
                    self.bump();
                    blank = true;
                }
                // A frame never owns the `}` of its parent block.
                TokenKind::RBrace if self.mframe => break,
                TokenKind::RBrace => {
                    close = Some(self.bump().span.start);
                    break;
                }
                TokenKind::Semi => self.semicolon(),
                TokenKind::Eof => {
                    if let Some(open) = open {
                        self.unclosed(open);
                    }
                    break;
                }
                TokenKind::Comment(_) => {
                    if let Some(mut c) = self.trailing_comment() {
                        c.blank_before = blank_before;
                        entries.push(Entry::Comment(c));
                    }
                    blank = false;
                    self.end_of_statement();
                }
                _ => {
                    if self.mframe && self.at_frame_end() {
                        self.frame_end = Some(self.bump().span);
                        break;
                    }
                    if let Some((mut kind, span)) = self.statement(root) {
                        self.count(&kind, span);
                        // After a container this is the comment on its closing `}` line.
                        let trailing = self.trailing_comment();
                        let item_trailing = match &mut kind {
                            ItemKind::Container(c) => {
                                c.close_trailing = trailing;
                                None
                            }
                            _ => trailing,
                        };
                        entries.push(Entry::Item(Item {
                            kind,
                            blank_before,
                            trailing: item_trailing,
                            span,
                        }));
                    } else {
                        self.skip_to_stmt_end();
                    }
                    for (kind, span) in std::mem::take(&mut self.extra) {
                        self.count(&kind, span);
                        entries.push(Entry::Item(Item {
                            kind,
                            blank_before: false,
                            trailing: None,
                            span,
                        }));
                    }
                    blank = false;
                    self.end_of_statement();
                }
            }
        }
        Block {
            entries,
            braces: open.zip(close).map(|(open, close)| (open.start, close)),
        }
    }

    /// A statement must end at a newline, `}` or EOF.
    fn end_of_statement(&mut self) {
        match self.peek().kind {
            TokenKind::Newline => {
                self.bump();
            }
            TokenKind::RBrace | TokenKind::Eof => {}
            TokenKind::Semi => self.semicolon(),
            _ => {
                self.unexpected("end of line");
                self.skip_to_stmt_end();
                if matches!(self.peek().kind, TokenKind::Newline) {
                    self.bump();
                }
            }
        }
    }
}
