//! Strict Mermaid `flowchart` import (tracer grammar: `<id> --> <id>` only). Anything not
//! understood is reported by line and the whole block is refused.

use std::collections::BTreeSet;

use super::{ImportOpts, ImportReport, Unsupported};
use crate::limits::MAX_SOURCE_BYTES;
use crate::{fmt, ir::Direction};

mod build;
mod flow;
mod lines;
mod text;

fn refused(line: u32, construct: &str) -> ImportReport {
    ImportReport {
        source: None,
        sequence: None,
        unsupported: vec![Unsupported {
            line,
            construct: construct.to_owned(),
        }],
        dropped: Vec::new(),
    }
}

/// `Some(direction)` for `graph`/`flowchart` with an optional direction word, else `None`.
fn header(line: &str) -> Option<Direction> {
    let mut words = line.split_whitespace();
    if !matches!(words.next()?, "graph" | "flowchart") {
        return None;
    }
    let dir = match words.next() {
        None | Some("TD" | "TB") => Direction::Down,
        Some("LR") => Direction::Right,
        Some("RL") => Direction::Left,
        Some("BT") => Direction::Up,
        Some(_) => return None,
    };
    words.next().is_none().then_some(dir)
}

fn entries(list: Vec<(u32, String)>) -> Vec<Unsupported> {
    let mut list = list;
    list.sort_by_key(|e| e.0);
    let mut seen = BTreeSet::new();
    list.into_iter()
        .filter(|e| seen.insert(e.clone()))
        .map(|(line, construct)| Unsupported { line, construct })
        .collect()
}

/// The flowchart body: every line is scanned (no early exit), then the semantic pass runs.
fn flowchart(dir: Direction, body: Vec<(u32, &str)>, opts: ImportOpts) -> ImportReport {
    let mut errors = Vec::new();
    let mut dropped = Vec::new();
    let mut stmts = Vec::new();
    for (n, text) in body {
        match lines::scan(text, opts) {
            Ok(s) => {
                dropped.extend(s.dropped.into_iter().map(|c| (n, c)));
                stmts.push((n, s.stmt));
            }
            Err(b) => {
                errors.push((n, b.construct));
                if b.opens {
                    stmts.push((n, lines::Stmt::BadOpen));
                }
            }
        }
    }
    let built = build::build(dir, stmts);
    errors.extend(built.errors);
    dropped.extend(built.dropped);
    let (unsupported, dropped) = (entries(errors), entries(dropped));
    let source = if unsupported.is_empty() {
        let raw = flow::to_dsl(&built.graph);
        match fmt(&raw) {
            Some(s) => Some(s),
            None => return refused(1, "internal"),
        }
    } else {
        None
    };
    ImportReport {
        source,
        sequence: None,
        unsupported,
        dropped,
    }
}

/// Import one Mermaid block. Fail-closed: `source` is `Some` only when every line was read.
pub fn import(block: &str, opts: ImportOpts) -> ImportReport {
    if block.len() > MAX_SOURCE_BYTES {
        return refused(1, "limit-source");
    }
    let mut lines = block
        .lines()
        .enumerate()
        .map(|(i, l)| (u32::try_from(i + 1).unwrap_or(u32::MAX), l.trim()))
        .filter(|(_, l)| !l.is_empty());
    let Some((hline, head)) = lines.next() else {
        return refused(1, "header:");
    };
    let word = head.split_whitespace().next().unwrap_or_default();
    match (word.trim_end_matches(';'), header(head)) {
        ("graph" | "flowchart", _) if head.ends_with(';') => refused(hline, "semicolon"),
        ("graph" | "flowchart", Some(dir)) => flowchart(dir, lines.collect(), opts),
        _ => refused(hline, &format!("header:{word}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces_around_the_arrow_are_optional() {
        let a = import("flowchart TD\n  a-->b\n", ImportOpts::default());
        let b = import("flowchart TD\n  a --> b\n", ImportOpts::default());
        assert_eq!(a, b);
        assert!(a.source.is_some());
    }

    #[test]
    fn every_bad_line_is_listed_with_its_line_number() {
        let r = import(
            "graph\n\n  a --> b\n  no pe\n  a --- b\n",
            ImportOpts::default(),
        );
        let lines: Vec<u32> = r.unsupported.iter().map(|u| u.line).collect();
        assert_eq!(lines, vec![4, 5]);
        assert!(r.source.is_none());
    }

    #[test]
    fn reserved_words_and_dangling_hyphens_are_refused() {
        for body in ["node --> b", "a- --> b"] {
            let r = import(&format!("flowchart TD\n  {body}\n"), ImportOpts::default());
            assert!(r.source.is_none(), "{body}");
        }
    }

    #[test]
    fn bad_header_direction_and_empty_block_fail_closed() {
        assert_eq!(
            import("flowchart XY\n", ImportOpts::default()).unsupported[0].construct,
            "header:flowchart"
        );
        assert!(import("", ImportOpts::default()).source.is_none());
    }
}
