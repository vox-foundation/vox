//! Line-oriented flowchart scanner: one trimmed line becomes one statement or one construct
//! name. Only text outside quotes, brackets and edge text can be a construct; the scanner is
//! a single left-to-right pass over borrowed slices.

use super::text::{self, NODE_SPECIAL};
use crate::ir::Shape;
use crate::mermaid::ImportOpts;

pub(super) type Res<T> = Result<T, String>;

/// A node reference: an id with an optional inline declaration (shape and label).
pub(super) struct Ref {
    pub id: String,
    pub decl: Option<(Shape, String)>,
}

/// `groups[k]` joined by `arrows[k]` to `groups[k + 1]`; one group alone declares nodes.
pub(super) struct Chain {
    pub groups: Vec<Vec<Ref>>,
    pub arrows: Vec<Option<String>>,
}

pub(super) enum Stmt {
    Chain(Chain),
    Subgraph {
        id: String,
        title: Option<String>,
    },
    End,
    /// A `subgraph` header that was refused; it still opens a frame so its `end` balances.
    BadOpen,
}

pub(super) struct Scanned {
    pub stmt: Stmt,
    pub dropped: Vec<String>,
}

/// A refused line. `opens` marks a `subgraph` header, so its `end` still balances.
pub(super) struct Bad {
    pub construct: String,
    pub opens: bool,
}

fn bad<T>(c: impl Into<String>) -> Res<T> {
    Err(c.into())
}

/// Longest id-shaped prefix: alphanumerics and `_`, plus `-` only between id characters.
fn id_run(s: &str) -> (&str, &str) {
    let mut end = 0;
    let mut it = s.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        let inner_dash = c == '-'
            && end > 0
            && it
                .peek()
                .is_some_and(|&(_, n)| n.is_alphanumeric() || n == '_');
        if c.is_alphanumeric() || c == '_' || inner_dash {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    s.split_at(end)
}

/// The construct for a token that cannot continue a statement.
fn stop(s: &str) -> String {
    if s.starts_with(';') {
        "semicolon"
    } else if s.starts_with("%%") {
        "comment"
    } else {
        "statement"
    }
    .to_owned()
}

/// Text up to `closer` after the cursor, quoted or not, decoded; the cursor moves past `closer`.
fn bracket_text(cur: &mut &str, closer: &str, special: &str) -> Res<String> {
    let rest = *cur;
    if let Some(q) = rest.strip_prefix('"') {
        let end = q.find('"').ok_or("label-syntax")?;
        *cur = q[end + 1..].strip_prefix(closer).ok_or("label-syntax")?;
        return Ok(text::decode(&q[..end], true, "")?);
    }
    let pos = rest.find(|c| special.contains(c)).ok_or("label-syntax")?;
    *cur = rest[pos..].strip_prefix(closer).ok_or("label-syntax")?;
    Ok(text::decode(&rest[..pos], false, special)?)
}

fn reference(cur: &mut &str) -> Res<Ref> {
    let (id, after) = id_run(cur);
    if id.is_empty() {
        return bad(stop(cur));
    }
    text::valid_id(id)?;
    *cur = after;
    let mut decl = None;
    if let Some(r) = cur.strip_prefix('[') {
        *cur = r;
        let label = bracket_text(cur, "]", NODE_SPECIAL)?;
        if label.is_empty() {
            return bad("empty-label");
        }
        decl = Some((Shape::Rect, label));
    }
    Ok(Ref {
        id: id.to_owned(),
        decl,
    })
}

fn arrow(cur: &mut &str) -> Res<Option<String>> {
    let Some(r) = cur.strip_prefix("-->") else {
        return bad(stop(cur));
    };
    *cur = r;
    let t = r.trim_start();
    let Some(p) = t.strip_prefix('|') else {
        return Ok(None);
    };
    *cur = p;
    let end = p.find('|').ok_or("edge-label")?;
    let label = text::decode(&p[..end], false, NODE_SPECIAL)?;
    *cur = &p[end + 1..];
    Ok((!label.is_empty()).then_some(label))
}

fn chain(line: &str) -> Res<Stmt> {
    let mut cur = line;
    let mut groups = Vec::new();
    let mut arrows = Vec::new();
    loop {
        groups.push(vec![reference(&mut cur)?]);
        cur = cur.trim_start();
        if cur.is_empty() {
            break;
        }
        arrows.push(arrow(&mut cur)?);
        cur = cur.trim_start();
    }
    Ok(Stmt::Chain(Chain { groups, arrows }))
}

fn subgraph(rest: &str) -> Res<Stmt> {
    let (id, after) = id_run(rest);
    if id.is_empty() {
        return bad("subgraph-title");
    }
    text::valid_id(id)?;
    let mut cur = after.trim_start();
    let mut title = None;
    if let Some(r) = cur.strip_prefix('[') {
        cur = r;
        let t = bracket_text(&mut cur, "]", NODE_SPECIAL)?;
        if t.is_empty() {
            return bad("empty-label");
        }
        title = Some(t);
    }
    if !cur.trim().is_empty() {
        return bad("subgraph-title");
    }
    Ok(Stmt::Subgraph {
        id: id.to_owned(),
        title,
    })
}

/// Scan one trimmed, non-blank line.
pub(super) fn scan(line: &str, _opts: ImportOpts) -> Result<Scanned, Bad> {
    let word = line.split_whitespace().next().unwrap_or_default();
    let opens = word == "subgraph";
    let res = if line == "end" {
        Ok(Stmt::End)
    } else if opens {
        subgraph(line[word.len()..].trim_start())
    } else {
        chain(line)
    };
    match res {
        Ok(stmt) => Ok(Scanned {
            stmt,
            dropped: Vec::new(),
        }),
        Err(construct) => Err(Bad { construct, opens }),
    }
}
