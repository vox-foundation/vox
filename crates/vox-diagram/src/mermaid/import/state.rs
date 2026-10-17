//! stateDiagram-v2 import. Folds into the graph model: `[*]` as a source is the `start`
//! terminal, as a target the `end` terminal, transitions are labelled edges. The DSL is
//! printed by `flow::to_dsl`, so state blocks share the scoped builder with flowcharts.

use std::collections::BTreeMap;

use super::flow::{GEdge, GNode, Graph};
use super::lines::{id_run, styling_head};
use super::{entries, finish, text};
use crate::ir::{Direction, Shape};
use crate::limits::{MAX_EDGES, MAX_NODES};
use crate::mermaid::{ImportOpts, ImportReport};

/// One accepted transition: endpoints are `None` for `[*]`.
struct Trans {
    from: Option<String>,
    to: Option<String>,
    label: Option<String>,
}

/// An endpoint: `[*]` or a state id. `start` / `end` are the terminals' ids, so they clash.
fn endpoint(s: &str) -> Result<Option<String>, String> {
    let s = s.trim();
    if s == "[*]" {
        return Ok(None);
    }
    if matches!(s, "start" | "end") {
        return Err("id-clash".to_owned());
    }
    text::valid_id(s).map_err(str::to_owned)?;
    Ok(Some(s.to_owned()))
}

/// The head-keyword constructs that are outside the subset.
fn keyword(line: &str, word: &str) -> Option<&'static str> {
    let tail = line[word.len()..].trim_start();
    let lower = word.to_ascii_lowercase();
    if line == "--" {
        Some("state:concurrency")
    } else if line == "}" {
        Some("state:composite")
    } else if lower == "note" || (lower == "end" && tail.eq_ignore_ascii_case("note")) {
        Some("state:note")
    } else if line.contains("<<") {
        Some("state:pseudo")
    } else if word == "state" {
        Some(if tail.starts_with('"') {
            "state:alias"
        } else if tail.ends_with('{') {
            "state:composite"
        } else {
            "statement"
        })
    } else if word == "direction" && !tail.is_empty() {
        Some("direction")
    } else {
        None
    }
}

fn transition(line: &str) -> Result<Trans, String> {
    let Some((left, right)) = line.split_once("-->") else {
        let (id, rest) = id_run(line);
        let desc = !id.is_empty() && rest.trim_start().starts_with(':');
        return Err(if desc {
            "state:description"
        } else {
            "statement"
        }
        .to_owned());
    };
    let (target, label) = match right.split_once(':') {
        Some((t, l)) => (t, Some(l)),
        None => (right, None),
    };
    let (from, to) = (endpoint(left)?, endpoint(target)?);
    let label = match label {
        Some(l) => Some(text::free(l, false).map_err(str::to_owned)?).filter(|l| !l.is_empty()),
        None => None,
    };
    Ok(Trans { from, to, label })
}

/// Scan one trimmed line: `Ok(None)` for a dropped styling line.
fn scan(line: &str, opts: ImportOpts, dropped: &mut Vec<String>) -> Result<Option<Trans>, String> {
    if line.starts_with("%%{") {
        return Err("directive".to_owned());
    }
    if line.starts_with("%%") {
        return Err("comment".to_owned());
    }
    let styling = styling_head(line).map(|h| format!("styling:{h}"));
    let styling = styling.or_else(|| line.contains(":::").then(|| "styling:shorthand".to_owned()));
    if let Some(c) = styling {
        if !opts.drop_styling {
            return Err(c);
        }
        dropped.push(c);
        return Ok(None);
    }
    let word = line.split_whitespace().next().unwrap_or_default();
    if let Some(c) = keyword(line, word) {
        return Err(c.to_owned());
    }
    if text::semicolon(line).unwrap_or(false) {
        return Err("semicolon".to_owned());
    }
    transition(line).map(Some)
}

/// Node index for a state or terminal, created at its first mention.
struct Nodes {
    nodes: Vec<GNode>,
    index: BTreeMap<String, usize>,
    full: bool,
}

impl Nodes {
    /// `None` when the node cap would be exceeded (reported once by the caller).
    fn get(&mut self, id: &str, label: &str, shape: Shape) -> Option<usize> {
        if let Some(&i) = self.index.get(id) {
            return Some(i);
        }
        if self.nodes.len() >= MAX_NODES {
            return None;
        }
        self.index.insert(id.to_owned(), self.nodes.len());
        self.nodes.push(GNode {
            id: id.to_owned(),
            label: label.to_owned(),
            shape,
        });
        Some(self.nodes.len() - 1)
    }
}

pub(super) fn state(body: Vec<(u32, &str)>, opts: ImportOpts) -> ImportReport {
    let mut errors = Vec::new();
    let mut dropped = Vec::new();
    let mut ns = Nodes {
        nodes: Vec::new(),
        index: BTreeMap::new(),
        full: false,
    };
    let mut edges: Vec<GEdge> = Vec::new();
    let mut edges_full = false;
    for (n, line) in body {
        let mut d = Vec::new();
        let t = match scan(line, opts, &mut d) {
            Ok(t) => t,
            Err(c) => {
                errors.push((n, c));
                continue;
            }
        };
        dropped.extend(d.into_iter().map(|c| (n, c)));
        let Some(t) = t else { continue };
        let from = match &t.from {
            None => ns.get("start", "Start", Shape::Start),
            Some(id) => ns.get(id, id, Shape::Rect),
        };
        let to = match &t.to {
            None => ns.get("end", "End", Shape::End),
            Some(id) => ns.get(id, id, Shape::Rect),
        };
        let (Some(from), Some(to)) = (from, to) else {
            if !ns.full {
                ns.full = true;
                errors.push((n, "limit-nodes".to_owned()));
            }
            continue;
        };
        if edges.len() >= MAX_EDGES {
            if !edges_full {
                edges_full = true;
                errors.push((n, "limit-edges".to_owned()));
            }
            continue;
        }
        edges.push(GEdge {
            from,
            to,
            label: t.label,
        });
    }
    let node_container = vec![None; ns.nodes.len()];
    let g = Graph {
        direction: Direction::Down,
        nodes: ns.nodes,
        containers: Vec::new(),
        node_container,
        edges,
    };
    finish(entries(errors), entries(dropped), &g)
}
