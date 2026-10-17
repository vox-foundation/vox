//! Line-oriented flowchart scanner: one trimmed line becomes one statement or one construct
//! name. Only text outside quotes, brackets and edge text can be a construct; the scanner is
//! a single left-to-right pass over borrowed slices.

use super::ids;
use super::text::{self, DASH_SPECIAL, NODE_SPECIAL};
use crate::ir::{Direction, Shape};
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
    Direction(Direction),
    /// A styling line dropped under `drop_styling`.
    Skip,
}

pub(super) struct Scanned {
    pub stmt: Stmt,
    pub dropped: Vec<String>,
}

/// Per-line scanning context: whether styling is dropped, and what was dropped on this line.
struct Cx {
    drop_styling: bool,
    dropped: Vec<String>,
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
pub(super) fn id_run(s: &str) -> (&str, &str) {
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

/// Supported inline declarations, longest opener first: opener, shape, closer.
const SHAPES: [(&str, Shape, &str); 6] = [
    ("(((", Shape::End, ")))"),
    ("((", Shape::Start, "))"),
    ("(", Shape::Rounded, ")"),
    ("[(", Shape::Cylinder, ")]"),
    ("[", Shape::Rect, "]"),
    ("{", Shape::Diamond, "}"),
];

/// Shapes Mermaid has and VoxDiagram does not.
const OTHER_SHAPES: [&str; 7] = ["([", "[[", "{{", ">", "[/", "[\\", "@{"];

/// Mermaid's link tokens other than `-->`, longest first.
const LINKS: [&str; 12] = [
    "<-->", "--->", "-.->", "-.-", "==>", "===", "--o", "--x", "---", "~~~", "o--o", "x--x",
];

fn reference(cur: &mut &str, cx: &mut Cx) -> Res<Ref> {
    let (id, after) = id_run(cur);
    if id.is_empty() {
        return bad(stop(cur));
    }
    ids::flow_ref(id, after)?;
    *cur = after;
    if let Some(o) = OTHER_SHAPES.iter().find(|o| cur.starts_with(*o)) {
        return bad(format!("shape:{o}"));
    }
    let mut decl = None;
    if let Some(&(open, shape, close)) = SHAPES.iter().find(|(o, _, _)| cur.starts_with(o)) {
        *cur = &cur[open.len()..];
        let label = bracket_text(cur, close, NODE_SPECIAL)?;
        if label.is_empty() {
            return bad("empty-label");
        }
        decl = Some((shape, label));
    }
    if let Some(r) = cur.strip_prefix(":::") {
        if !cx.drop_styling {
            return bad("styling:shorthand");
        }
        let (name, after) = id_run(r);
        if name.is_empty() {
            return bad("statement");
        }
        *cur = after;
        cx.dropped.push("styling:shorthand".to_owned());
    }
    Ok(Ref {
        id: id.to_owned(),
        decl,
    })
}

/// Quoted or plain edge text ending at `end`; empty text means no label.
fn edge_text(raw: &str, quoted: bool, special: &str) -> Res<Option<String>> {
    let label = text::decode(raw, quoted, special)?;
    Ok((!label.is_empty()).then_some(label))
}

/// Mermaid rejects zero-length quoted and pipe text (`""`, `||`) as a parse error.
fn non_empty(raw: &str) -> Res<&str> {
    if raw.is_empty() {
        return bad("edge-label");
    }
    Ok(raw)
}

fn arrow(cur: &mut &str) -> Res<Option<String>> {
    let s = *cur;
    if let Some(r) = s.strip_prefix("-->") {
        let t = r.trim_start();
        let Some(p) = t.strip_prefix('|') else {
            *cur = r;
            return Ok(None);
        };
        if let Some(q) = p.strip_prefix('"') {
            let end = q.find('"').ok_or("edge-label")?;
            *cur = q[end + 1..].strip_prefix('|').ok_or("edge-label")?;
            return edge_text(non_empty(&q[..end])?, true, "");
        }
        let end = p.find('|').ok_or("edge-label")?;
        *cur = &p[end + 1..];
        return edge_text(non_empty(&p[..end])?, false, NODE_SPECIAL);
    }
    if let Some(r) = s
        .strip_prefix("--")
        .filter(|r| r.starts_with(char::is_whitespace))
    {
        let t = r.trim_start();
        if let Some(q) = t.strip_prefix('"') {
            let end = q.find('"').ok_or("edge-label")?;
            let after = q[end + 1..].trim_start().strip_prefix("-->");
            *cur = after.ok_or("edge-label")?;
            return edge_text(non_empty(&q[..end])?, true, "");
        }
        let end = t.find("-->").ok_or("edge-label")?;
        *cur = &t[end + 3..];
        // Unquoted text ends at its first `--` (Mermaid reads the rest as another link), cannot
        // end in `-` (`a--->` is a `--->` link), and cannot be blank.
        let raw = &t[..end];
        if raw.trim().is_empty() || raw.contains("--") || raw.ends_with('-') {
            return bad("edge-label");
        }
        return edge_text(raw, false, DASH_SPECIAL);
    }
    if let Some(t) = LINKS.iter().find(|t| s.starts_with(*t)) {
        return bad(format!("edge-type:{t}"));
    }
    bad(stop(s))
}

/// One reference, then `&`-joined references.
fn group(cur: &mut &str, cx: &mut Cx) -> Res<Vec<Ref>> {
    let mut refs = vec![reference(cur, cx)?];
    loop {
        let spaced = cur.starts_with(char::is_whitespace);
        *cur = cur.trim_start();
        let Some(r) = cur.strip_prefix('&') else {
            return Ok(refs);
        };
        // Mermaid reads `b&c` as one id and rejects `b &c` and `b& c`: only ` & ` fans out.
        if !spaced || !r.starts_with(char::is_whitespace) {
            return bad("statement");
        }
        *cur = r.trim_start();
        refs.push(reference(cur, cx)?);
    }
}

/// Edges one statement may expand to (`&` fan-out and chains).
const MAX_EXPANSION: usize = 100;

fn chain(line: &str, cx: &mut Cx) -> Res<Stmt> {
    let mut cur = line;
    let mut groups: Vec<Vec<Ref>> = Vec::new();
    let mut arrows = Vec::new();
    let mut edges = 0usize;
    loop {
        let g = group(&mut cur, cx)?;
        if let Some(prev) = groups.last() {
            edges = edges.saturating_add(prev.len().saturating_mul(g.len()));
            if edges > MAX_EXPANSION {
                return bad("expansion-cap");
            }
        }
        groups.push(g);
        if cur.is_empty() {
            break;
        }
        arrows.push(arrow(&mut cur)?);
        cur = cur.trim_start();
    }
    if groups.len() == 1 && groups[0].len() > 1 {
        return bad("statement");
    }
    Ok(Stmt::Chain(Chain { groups, arrows }))
}

fn subgraph(rest: &str) -> Res<Stmt> {
    let (id, after) = id_run(rest);
    if id.is_empty() {
        return bad("subgraph-title");
    }
    let mut cur = after.trim_start();
    ids::subgraph_id(id, cur.starts_with('['))?;
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

fn direction(token: &str) -> Res<Stmt> {
    match token {
        "TD" | "TB" => Ok(Stmt::Direction(Direction::Down)),
        "LR" => Ok(Stmt::Direction(Direction::Right)),
        "RL" => Ok(Stmt::Direction(Direction::Left)),
        "BT" => Ok(Stmt::Direction(Direction::Up)),
        _ => bad("direction"),
    }
}

/// Statement heads that start a styling line.
const HEADS: [&str; 8] = [
    "style",
    "classDef",
    "class",
    "linkStyle",
    "click",
    "callback",
    "accTitle",
    "accDescr",
];

/// True when `s` starts like a link or a fan-out, so a head word before it is a plain id.
fn arrowish(s: &str) -> bool {
    s.starts_with(['-', '=', '~', '&']) || s.starts_with("<-")
}

pub(super) fn styling_head(line: &str) -> Option<&'static str> {
    HEADS.into_iter().find(|h| {
        line.strip_prefix(h)
            .is_some_and(|rest| match rest.chars().next() {
                Some(c) if c.is_whitespace() => !arrowish(rest.trim_start()),
                Some(':' | '{') => h.starts_with("acc"),
                _ => false,
            })
    })
}

fn statement(line: &str, word: &str, cx: &mut Cx) -> (Res<Stmt>, bool) {
    if line == "end" {
        return (Ok(Stmt::End), false);
    }
    if let Some(h) = styling_head(line) {
        let c = format!("styling:{h}");
        if !cx.drop_styling {
            return (bad(c), false);
        }
        cx.dropped.push(c);
        return (Ok(Stmt::Skip), false);
    }
    let tail = line[word.len()..].trim_start();
    match word {
        "subgraph" => (subgraph(tail), true),
        "direction" if !tail.is_empty() && !arrowish(tail) => (direction(tail), false),
        _ => (chain(line, cx), false),
    }
}

/// Scan one trimmed, non-blank line.
pub(super) fn scan(line: &str, opts: ImportOpts) -> Result<Scanned, Bad> {
    if line.starts_with("%%{") {
        return Err(Bad {
            construct: "directive".to_owned(),
            opens: false,
        });
    }
    let word = line.split_whitespace().next().unwrap_or_default();
    let mut cx = Cx {
        drop_styling: opts.drop_styling,
        dropped: Vec::new(),
    };
    let (res, opens) = if line.starts_with("%%") {
        (bad("comment"), false)
    } else {
        statement(line, word, &mut cx)
    };
    match res {
        Ok(stmt) => Ok(Scanned {
            stmt,
            dropped: cx.dropped,
        }),
        Err(construct) => Err(Bad { construct, opens }),
    }
}
