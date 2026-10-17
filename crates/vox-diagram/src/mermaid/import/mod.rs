//! Strict Mermaid `flowchart` import (tracer grammar: `<id> --> <id>` only). Anything not
//! understood is reported by line and the whole block is refused.

use std::collections::BTreeSet;

use super::{ImportOpts, ImportReport, Unsupported};
use crate::limits::MAX_SOURCE_BYTES;
use crate::print::push_quoted;
use crate::resolve::RESERVED;
use crate::{fmt, ir::Direction};

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

fn is_id(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        && !s.ends_with('-')
        && !RESERVED.contains(&s)
}

/// `a --> b` (spaces optional) as its two ids.
fn edge(line: &str) -> Option<(&str, &str)> {
    let (from, to) = line.split_once("-->")?;
    let (from, to) = (from.trim(), to.trim());
    (is_id(from) && is_id(to)).then_some((from, to))
}

fn direction_word(d: Direction) -> Option<&'static str> {
    match d {
        Direction::Down => None,
        Direction::Right => Some("right"),
        Direction::Up => Some("up"),
        Direction::Left => Some("left"),
    }
}

/// Import one Mermaid block. Fail-closed: `source` is `Some` only when every line was read.
pub fn import(block: &str, _opts: ImportOpts) -> ImportReport {
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
    let Some(dir) = header(head) else {
        let word = head.split_whitespace().next().unwrap_or_default();
        return refused(hline, &format!("header:{word}"));
    };
    let mut nodes: Vec<&str> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut edges = Vec::new();
    let mut bad = Vec::new();
    for (n, text) in lines {
        match edge(text) {
            Some((a, b)) => {
                for id in [a, b] {
                    if seen.insert(id) {
                        nodes.push(id);
                    }
                }
                edges.push((a, b));
            }
            None => bad.push(Unsupported {
                line: n,
                construct: "statement".to_owned(),
            }),
        }
    }
    if !bad.is_empty() {
        return ImportReport {
            source: None,
            sequence: None,
            unsupported: bad,
            dropped: Vec::new(),
        };
    }
    let mut raw = String::from("graph {\n");
    if let Some(w) = direction_word(dir) {
        raw.push_str(&format!("    direction: {w}\n"));
    }
    for id in &nodes {
        raw.push_str(&format!("    node {id}: "));
        push_quoted(&mut raw, id);
        raw.push('\n');
    }
    for (a, b) in &edges {
        raw.push_str(&format!("    {a} -> {b}\n"));
    }
    raw.push_str("}\n");
    match fmt(&raw) {
        Some(source) => ImportReport {
            source: Some(source),
            sequence: None,
            unsupported: Vec::new(),
            dropped: Vec::new(),
        },
        None => refused(1, "internal"),
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
            "graph\n\n  a --> b\n  nope\n  a --- b\n",
            ImportOpts::default(),
        );
        let lines: Vec<u32> = r.unsupported.iter().map(|u| u.line).collect();
        assert_eq!(lines, vec![4, 5]);
        assert!(r.source.is_none());
    }

    #[test]
    fn reserved_words_and_chained_edges_are_refused() {
        for body in ["node --> b", "a --> b --> c", "a -->|x| b", "a- --> b"] {
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
