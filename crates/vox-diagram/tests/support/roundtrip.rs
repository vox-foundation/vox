//! Round-trip test support: the Rust canonical sets (built from the IR alone, never from
//! Mermaid text, so they are independent of the Node oracle) and the golden-file helper.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vox_diagram::ir::{GraphIr, Shape};
use vox_diagram::sequence_ir::{SeqEvent, SequenceIr};

/// `tests/fixtures` of this crate.
pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Sorted file stems in `dir` with extension `ext` (no dot); empty when `dir` is missing.
pub fn stems(dir: &Path, ext: &str) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(ext))
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(str::to_owned))
        .collect();
    out.sort();
    out
}

fn shape_word(s: Shape) -> String {
    match serde_json::to_value(s) {
        Ok(Value::String(w)) => w,
        _ => String::new(),
    }
}

fn container_titles(g: &GraphIr, start: Option<&str>) -> Vec<String> {
    let mut titles = Vec::new();
    let mut cur = start.map(str::to_owned);
    for _ in 0..g.containers.len() {
        let Some(id) = cur.take() else { break };
        let Some(c) = g.containers.iter().find(|c| c.id == id) else {
            break;
        };
        titles.push(c.label.trim().to_owned());
        cur = c.parent.clone();
    }
    titles.reverse();
    titles
}

fn sorted(mut v: Vec<Value>) -> Value {
    v.sort_by_key(|x| serde_json::to_string(x).unwrap_or_default());
    Value::Array(v)
}

/// Canonical id-free sets of a graph IR (see the 84-01 plan, "Canonical sets").
pub fn canon_graph(g: &GraphIr) -> Value {
    let key_of = |id: &str| -> Value {
        match g.nodes.iter().find(|n| n.id == id) {
            Some(n) => json!([
                n.label.trim(),
                shape_word(n.shape),
                container_titles(g, n.container.as_deref())
            ]),
            None => json!([id, "rect", []]),
        }
    };
    let nodes = g.nodes.iter().map(|n| key_of(&n.id)).collect();
    let edges = g
        .edges
        .iter()
        .map(|e| {
            json!([
                key_of(&e.from),
                key_of(&e.to),
                e.label.as_deref().unwrap_or("").trim()
            ])
        })
        .collect();
    let containers = g
        .containers
        .iter()
        .map(|c| json!(container_titles(g, Some(&c.id))))
        .collect();
    let direction = match serde_json::to_value(g.direction) {
        Ok(Value::String(w)) => w,
        _ => String::new(),
    };
    json!({
        "kind": "graph",
        "direction": direction,
        "nodes": sorted(nodes),
        "edges": sorted(edges),
        "containers": sorted(containers),
    })
}

/// Canonical sets of a sequence IR: actor labels by participant index (declared actors in
/// order, then ids used by events but never declared, labelled with the id) and ordered events
/// as `["message", from, to, dashed, text]` / `["note", [idx, ..], text]`. Texts are trimmed; an
/// id used twice resolves to its first occurrence; a note keeps its first two ids.
pub fn canon_sequence(s: &SequenceIr) -> Value {
    let mut ids: Vec<String> = s.actors.iter().map(|a| a.id.clone()).collect();
    let mut labels: Vec<String> = s.actors.iter().map(|a| a.label.trim().to_owned()).collect();
    let mut at = |id: &str| -> usize {
        if let Some(i) = ids.iter().position(|x| x == id) {
            return i;
        }
        ids.push(id.to_owned());
        labels.push(id.trim().to_owned());
        ids.len() - 1
    };
    let events: Vec<Value> = s
        .events
        .iter()
        .map(|e| match e {
            SeqEvent::Message {
                from,
                to,
                text,
                dashed,
                ..
            } => {
                let (f, t) = (at(from), at(to));
                json!(["message", f, t, dashed, text.trim()])
            }
            SeqEvent::Note { over, text, .. } => {
                let mut who: Vec<usize> = over.iter().take(2).map(|id| at(id)).collect();
                // Mermaid lists `Note over A,A` as a note over A.
                who.dedup();
                json!(["note", who, text.trim()])
            }
        })
        .collect();
    json!({"kind": "sequence", "actors": labels, "events": events})
}

/// `*.sets.json` text: pretty JSON plus a trailing newline.
pub fn sets_json(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).unwrap_or_default();
    s.push('\n');
    s
}

/// Compare `actual` with the committed golden at `path`; `UPDATE_GOLDEN=1` (re)writes it.
pub fn assert_golden(path: &Path, actual: &str) {
    if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(path, actual) {
            panic!("cannot write golden {}: {e}", path.display());
        }
        return;
    }
    let Ok(have) = std::fs::read_to_string(path) else {
        panic!(
            "golden {} is missing. Create it with: UPDATE_GOLDEN=1 cargo nextest run -p vox-diagram -E 'binary(mermaid_roundtrip)'",
            path.display()
        );
    };
    assert!(
        have == actual,
        "golden {} is stale. Regenerate with: UPDATE_GOLDEN=1 cargo nextest run -p vox-diagram -E 'binary(mermaid_roundtrip)'",
        path.display()
    );
}
