//! sequenceDiagram import into `SequenceIr`. A line-oriented scanner for the accepted subset
//! (`participant`, `->>`, `-->>`, `Note over`); every other keyword or arrow is refused by
//! name. Spans are `Location`s over each statement's trimmed byte range in the block.

use std::collections::BTreeMap;

use super::ids::{self, SeqAt};
use super::lines::{id_run, styling_head};
use super::{entries, formatted, text};
use crate::limits::{MAX_EDGES, MAX_NODES};
use crate::mermaid::{ImportOpts, ImportReport};
use crate::print::sequence_source;
use crate::sequence_ir::{Actor, SeqEvent, SequenceIr};
use crate::span::{LineIndex, Span, to_u32};

/// Mermaid block keywords (`block:<keyword>`).
const BLOCKS: [&str; 11] = [
    "loop", "alt", "else", "opt", "par", "and", "critical", "option", "break", "rect", "end",
];
/// Arrow tokens, longest first; `Some(dashed)` for the two accepted ones.
const ARROWS: [(&str, Option<bool>); 10] = [
    ("<<-->>", None),
    ("<<->>", None),
    ("-->>", Some(true)),
    ("->>", Some(false)),
    ("--x", None),
    ("--)", None),
    ("-->", None),
    ("-x", None),
    ("-)", None),
    ("->", None),
];

enum Stmt {
    Participant {
        id: String,
        alias: Option<String>,
    },
    Message {
        from: String,
        to: String,
        text: String,
        dashed: bool,
    },
    Note {
        over: Vec<String>,
        text: String,
    },
    Skip,
}

fn id(s: &str, at: SeqAt) -> Result<String, String> {
    ids::seq_id(s, at).map_err(str::to_owned)?;
    Ok(s.to_owned())
}

fn participant(tail: &str) -> Result<Stmt, String> {
    let (name, rest) = id_run(tail);
    if name.is_empty() {
        return Err("statement".to_owned());
    }
    let name = id(name, SeqAt::Declared)?;
    let rest = rest.trim_start();
    let alias = match rest.strip_prefix("as") {
        _ if rest.is_empty() => None,
        Some(r) if r.is_empty() || r.starts_with(char::is_whitespace) => {
            let label = text::free(r, true).map_err(str::to_owned)?;
            if label.is_empty() {
                return Err("empty-label".to_owned());
            }
            Some(label)
        }
        _ => return Err("statement".to_owned()),
    };
    Ok(Stmt::Participant { id: name, alias })
}

fn note(tail: &str) -> Result<Stmt, String> {
    let lower = tail.to_ascii_lowercase();
    if lower.starts_with("left of") || lower.starts_with("right of") {
        return Err("note-placement".to_owned());
    }
    let Some(rest) = lower
        .starts_with("over")
        .then(|| &tail[4..])
        .filter(|r| r.starts_with(char::is_whitespace))
    else {
        return Err("statement".to_owned());
    };
    let Some((ids, body)) = rest.split_once(':') else {
        return Err("statement".to_owned());
    };
    let ids: Vec<&str> = ids.split(',').map(str::trim).collect();
    if ids.len() > 2 {
        return Err("note-actors".to_owned());
    }
    let over = ids
        .into_iter()
        .map(|s| id(s, SeqAt::Receiver))
        .collect::<Result<Vec<_>, _>>()?;
    let text = text::free(body, true).map_err(str::to_owned)?;
    // The DSL has no empty note: `note over a: ""` is an `empty-label` error.
    if text.is_empty() {
        return Err("empty-label".to_owned());
    }
    Ok(Stmt::Note { over, text })
}

fn message(line: &str) -> Result<Stmt, String> {
    let (from, rest) = id_run(line);
    // `title` followed by whitespace is a title statement, whatever follows it, in any case.
    if from.eq_ignore_ascii_case("title") && rest.starts_with(char::is_whitespace) {
        return Err("title".to_owned());
    }
    let rest = rest.trim_start();
    let Some(&(token, dashed)) = ARROWS.iter().find(|(t, _)| rest.starts_with(t)) else {
        return Err("statement".to_owned());
    };
    let from = id(from, SeqAt::Sender)?;
    let Some(dashed) = dashed else {
        return Err(format!("arrow:{token}"));
    };
    let rest = rest[token.len()..].trim_start();
    if rest.starts_with(['+', '-']) {
        return Err("activation".to_owned());
    }
    let (to, rest) = id_run(rest);
    let Some(body) = rest
        .trim_start()
        .strip_prefix(':')
        .filter(|_| !to.is_empty())
    else {
        return Err("message-syntax".to_owned());
    };
    let to = id(to, SeqAt::Receiver)?;
    let text = text::free(body, true).map_err(str::to_owned)?;
    Ok(Stmt::Message {
        from,
        to,
        text,
        dashed,
    })
}

/// The refused-keyword construct for a statement head word, if any.
fn keyword(word: &str) -> Option<String> {
    Some(match word {
        "actor" => "actor".to_owned(),
        "create" | "destroy" => "lifecycle".to_owned(),
        "activate" | "deactivate" => "activation".to_owned(),
        "autonumber" | "title" | "box" => word.to_owned(),
        "link" | "links" | "properties" | "details" => "links".to_owned(),
        w if BLOCKS.contains(&w) => format!("block:{w}"),
        _ => return None,
    })
}

fn scan(line: &str, opts: ImportOpts, dropped: &mut Vec<String>) -> Result<Stmt, String> {
    if line.starts_with("%%{") {
        return Err("directive".to_owned());
    }
    if line.starts_with("%%") {
        return Err("comment".to_owned());
    }
    if let Some(h) = styling_head(line).filter(|h| h.starts_with("acc")) {
        let c = format!("styling:{h}");
        if !opts.drop_styling {
            return Err(c);
        }
        dropped.push(c);
        return Ok(Stmt::Skip);
    }
    let word = line.split_whitespace().next().unwrap_or_default();
    if let Some(c) = keyword(word) {
        return Err(c);
    }
    if text::semicolon(line).unwrap_or(false) {
        return Err("semicolon".to_owned());
    }
    let tail = line[word.len()..].trim_start();
    match word {
        "participant" => participant(tail),
        w if w.eq_ignore_ascii_case("note") => note(tail),
        _ => message(line),
    }
}

#[derive(Default)]
struct Build {
    actors: Vec<Actor>,
    explicit: Vec<bool>,
    index: BTreeMap<String, usize>,
    events: Vec<SeqEvent>,
    errors: Vec<(u32, String)>,
    nodes_full: bool,
    edges_full: bool,
}

impl Build {
    /// Declare or implicitly create `id`; `false` when the actor cap was hit.
    fn add(
        &mut self,
        n: u32,
        id: &str,
        label: &str,
        explicit: bool,
        span: crate::span::Location,
    ) -> bool {
        if self.actors.len() >= MAX_NODES {
            if !self.nodes_full {
                self.nodes_full = true;
                self.errors.push((n, "limit-nodes".to_owned()));
            }
            return false;
        }
        self.index.insert(id.to_owned(), self.actors.len());
        self.explicit.push(explicit);
        self.actors.push(Actor {
            id: id.to_owned(),
            label: label.to_owned(),
            span,
        });
        true
    }

    fn mention(&mut self, n: u32, id: &str, span: &crate::span::Location) -> bool {
        self.index.contains_key(id) || self.add(n, id, id, false, *span)
    }

    fn push(&mut self, n: u32, ev: SeqEvent) {
        if self.events.len() >= MAX_EDGES {
            if !self.edges_full {
                self.edges_full = true;
                self.errors.push((n, "limit-edges".to_owned()));
            }
            return;
        }
        self.events.push(ev);
    }

    fn stmt(&mut self, n: u32, s: Stmt, span: crate::span::Location) {
        match s {
            Stmt::Skip => {}
            Stmt::Participant { id, alias } => match self.index.get(&id) {
                Some(&i) => {
                    let c = if self.explicit[i] {
                        "redeclared-participant"
                    } else {
                        "participant-after-use"
                    };
                    self.errors.push((n, c.to_owned()));
                }
                None => {
                    self.add(n, &id, alias.as_deref().unwrap_or(&id), true, span);
                }
            },
            Stmt::Message {
                from,
                to,
                text,
                dashed,
            } => {
                if self.mention(n, &from, &span) & self.mention(n, &to, &span) {
                    self.push(
                        n,
                        SeqEvent::Message {
                            from,
                            to,
                            text,
                            dashed,
                            span,
                        },
                    );
                }
            }
            Stmt::Note { over, text } => {
                let ok = over
                    .iter()
                    .fold(true, |ok, id| self.mention(n, id, &span) & ok);
                if ok {
                    self.push(n, SeqEvent::Note { over, text, span });
                }
            }
        }
    }
}

/// `body` is every non-blank line after the header: `(line, byte offset of the trimmed text, text)`.
pub(super) fn sequence(
    block: &str,
    body: Vec<(u32, usize, &str)>,
    opts: ImportOpts,
) -> ImportReport {
    let idx = LineIndex::new(block);
    let mut b = Build::default();
    let mut dropped = Vec::new();
    for (n, off, line) in body {
        let mut d = Vec::new();
        match scan(line, opts, &mut d) {
            Ok(s) => {
                dropped.extend(d.into_iter().map(|c| (n, c)));
                let span = Span {
                    start: to_u32(off),
                    end: to_u32(off + line.len()),
                };
                b.stmt(n, s, idx.location(block, span));
            }
            Err(c) => b.errors.push((n, c)),
        }
    }
    let unsupported = entries(b.errors);
    let dropped = entries(dropped);
    if !unsupported.is_empty() {
        return ImportReport {
            source: None,
            sequence: None,
            unsupported,
            dropped,
        };
    }
    let ir = SequenceIr {
        actors: b.actors,
        events: b.events,
    };
    // Fail closed: the IR is reported only together with DSL that `fmt` accepts.
    match formatted(&sequence_source(&ir)) {
        Ok(source) => ImportReport {
            source: Some(source),
            sequence: Some(ir),
            unsupported,
            dropped,
        },
        Err(refusal) => refusal,
    }
}
