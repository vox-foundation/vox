//! stateDiagram-v2 import. Folds into the graph model: `[*]` as a source is the `start`
//! terminal, as a target the `end` terminal, transitions are labelled edges. The DSL is
//! printed by `flow::to_dsl`, so state blocks share the scoped builder with flowcharts.

use std::collections::BTreeMap;

use super::flow::{GEdge, GNode, Graph};
use super::ids::{StateAt, state_id};
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

/// One accepted line: a transition, or a state declared alone (`Run:::bad` under `drop_styling`).
enum Item {
    Edge(Trans),
    State(String),
}

/// An endpoint: `[*]` or a state id. `start` / `end` are the terminals' ids, so they clash.
fn endpoint(s: &str, at: StateAt) -> Result<Option<String>, String> {
    let s = s.trim();
    if s == "[*]" {
        return Ok(None);
    }
    if matches!(s, "start" | "end") {
        return Err("id-clash".to_owned());
    }
    state_id(s, at).map_err(str::to_owned)?;
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

/// Split a trailing `:::class` shorthand off an endpoint: `(endpoint, had_class)`.
fn shorthand(s: &str) -> Result<(&str, bool), String> {
    let Some((endpoint, class)) = s.split_once(":::") else {
        return Ok((s, false));
    };
    let (name, rest) = id_run(class.trim());
    if name.is_empty() || !rest.is_empty() {
        return Err("statement".to_owned());
    }
    Ok((endpoint, true))
}

/// One transition. A `:::class` shorthand is only reached under `drop_styling` (the default mode
/// refuses it first); it is stripped from the endpoints and reported in `dropped`.
fn transition(line: &str, dropped: &mut Vec<String>) -> Result<Trans, String> {
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
    // A `:::` right after the target is the shorthand; the label starts at the next lone `:`.
    let (target, label, t_class) = match right.find(':') {
        Some(p) if right[p..].starts_with(":::") => {
            let (name, rest) = id_run(&right[p + 3..]);
            let rest = rest.trim_start();
            match rest.strip_prefix(':') {
                Some(l) if !name.is_empty() => (&right[..p], Some(l), true),
                None if !name.is_empty() && rest.is_empty() => (&right[..p], None, true),
                _ => return Err("statement".to_owned()),
            }
        }
        Some(p) => (&right[..p], Some(&right[p + 1..]), false),
        None => (right, None, false),
    };
    let (left, l_class) = shorthand(left)?;
    if label.is_some_and(|l| l.contains(":::")) {
        return Err("styling:shorthand".to_owned());
    }
    if l_class || t_class {
        dropped.push("styling:shorthand".to_owned());
    }
    let at = if label.is_some() {
        StateAt::Labelled
    } else {
        StateAt::Target
    };
    let (from, to) = (endpoint(left, StateAt::Source)?, endpoint(target, at)?);
    let label = match label {
        // Mermaid rejects a transition whose `:` is followed by nothing.
        Some(l) if l.trim().is_empty() => return Err("empty-label".to_owned()),
        Some(l) => Some(text::free(l, false).map_err(str::to_owned)?).filter(|l| !l.is_empty()),
        None => None,
    };
    Ok(Trans { from, to, label })
}

/// A `drop_styling` line with a class shorthand and no arrow: `Run:::bad` declares the state Run
/// (the pinned Mermaid lists it); `state Run:::bad` declares nothing it lists, so only the line goes.
fn declaration(line: &str, dropped: &mut Vec<String>) -> Result<Option<Item>, String> {
    let (body, keyword) = match line.strip_prefix("state") {
        Some(r) if r.starts_with(char::is_whitespace) => (r.trim_start(), true),
        _ => (line, false),
    };
    let (id, class) = body.split_once(":::").unwrap_or((body, ""));
    let (name, rest) = id_run(class.trim());
    if name.is_empty() {
        return Err("statement".to_owned());
    }
    if rest.trim_start().starts_with(':') {
        return Err("state:description".to_owned());
    }
    if !rest.is_empty() {
        return Err("statement".to_owned());
    }
    let Some(id) = endpoint(id, StateAt::Source)? else {
        return Err("statement".to_owned());
    };
    dropped.push("styling:shorthand".to_owned());
    Ok((!keyword).then_some(Item::State(id)))
}

/// Scan one trimmed line: `Ok(None)` for a dropped styling line.
fn scan(line: &str, opts: ImportOpts, dropped: &mut Vec<String>) -> Result<Option<Item>, String> {
    if line.starts_with("%%{") {
        return Err("directive".to_owned());
    }
    if line.starts_with("%%") {
        return Err("comment".to_owned());
    }
    let styling = styling_head(line).map(|h| format!("styling:{h}"));
    // Under `drop_styling` a `:::class` suffix is stripped by `transition`, which keeps the edge.
    let styling = styling.or_else(|| {
        (line.contains(":::") && !opts.drop_styling).then(|| "styling:shorthand".to_owned())
    });
    if let Some(c) = styling {
        if !opts.drop_styling {
            return Err(c);
        }
        dropped.push(c);
        return Ok(None);
    }
    if opts.drop_styling && line.contains(":::") && !line.contains("-->") {
        return declaration(line, dropped);
    }
    let word = line.split_whitespace().next().unwrap_or_default();
    if let Some(c) = keyword(line, word) {
        return Err(c.to_owned());
    }
    if text::semicolon(line).unwrap_or(false) {
        return Err("semicolon".to_owned());
    }
    transition(line, dropped).map(|t| Some(Item::Edge(t)))
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
        let t = match t {
            None => continue,
            Some(Item::State(id)) => {
                if ns.get(&id, &id, Shape::Rect).is_none() && !ns.full {
                    ns.full = true;
                    errors.push((n, "limit-nodes".to_owned()));
                }
                continue;
            }
            Some(Item::Edge(t)) => t,
        };
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
