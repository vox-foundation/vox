//! Canonical printer: the AST in one fixed layout. Printing is a pure function of the AST, so
//! `fmt(fmt(x)) == fmt(x)` follows from `parse(fmt(x))` having the same AST as `parse(x)`.
//! Layout: 4-space indent per level, one statement per line, declaration order kept, `//`
//! comments kept at their attachment points, single blank lines kept (runs collapsed, none at
//! block edges), LF only and exactly one trailing newline. O(n).

use std::fmt::Write as _;

use crate::ast::{Block, Comment, ContainerDecl, Document, Entry, ItemKind, Label, Ref};
use crate::check::{check, parse};
use crate::ir::{Direction, Shape};

/// Canonical formatter. `Some(text)` iff `source` checks with no `Error` (warnings are fine);
/// `None` otherwise, so formatting can never change a broken document's meaning. Never panics.
pub fn fmt(source: &str) -> Option<String> {
    if !check(source).valid {
        return None;
    }
    parse(source).ast.as_ref().map(to_source)
}

/// Print `doc` in canonical form.
pub(crate) fn to_source(doc: &Document) -> String {
    let mut out = String::new();
    own_line_comments(&mut out, &doc.leading, 0);
    out.push_str("graph {");
    trailing(&mut out, doc.header_trailing.as_ref());
    out.push('\n');
    block(&mut out, &doc.body, 1);
    out.push('}');
    trailing(&mut out, doc.close_trailing.as_ref());
    out.push('\n');
    own_line_comments(&mut out, &doc.trailing, 0);
    out
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("    ");
    }
}

fn own_line_comments(out: &mut String, comments: &[Comment], level: usize) {
    for c in comments {
        if c.blank_before {
            out.push('\n');
        }
        indent(out, level);
        let _ = writeln!(out, "//{}", c.text);
    }
}

fn trailing(out: &mut String, c: Option<&Comment>) {
    if let Some(c) = c {
        let _ = write!(out, " //{}", c.text);
    }
}

fn label(out: &mut String, l: &Label) {
    out.push('"');
    for c in l.value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn path(out: &mut String, r: &Ref) {
    for (i, seg) in r.segments.iter().enumerate() {
        if i > 0 {
            out.push('.');
        }
        out.push_str(&seg.name);
    }
}

fn direction_word(d: Direction) -> &'static str {
    match d {
        Direction::Down => "down",
        Direction::Right => "right",
        Direction::Up => "up",
        Direction::Left => "left",
    }
}

fn shape_word(s: Shape) -> &'static str {
    match s {
        Shape::Rect => "rect",
        Shape::Rounded => "rounded",
        Shape::Diamond => "diamond",
        Shape::Cylinder => "cylinder",
        Shape::Start => "start",
        Shape::End => "end",
    }
}

fn container(out: &mut String, c: &ContainerDecl, level: usize) {
    let _ = write!(out, "container {}: ", c.id.name);
    label(out, &c.label);
    out.push_str(" {");
    trailing(out, c.header_trailing.as_ref());
    out.push('\n');
    block(out, &c.body, level + 1);
    indent(out, level);
    out.push('}');
    trailing(out, c.close_trailing.as_ref());
}

fn block(out: &mut String, b: &Block, level: usize) {
    for entry in &b.entries {
        match entry {
            Entry::Comment(c) => own_line_comments(out, std::slice::from_ref(c), level),
            Entry::Item(item) => {
                if item.blank_before {
                    out.push('\n');
                }
                indent(out, level);
                match &item.kind {
                    ItemKind::Direction(d) => {
                        let _ = write!(out, "direction: {}", direction_word(d.value));
                    }
                    ItemKind::Node(n) => {
                        let _ = write!(out, "node {}: ", n.id.name);
                        label(out, &n.label);
                        if n.shape != Shape::Rect {
                            let _ = write!(out, " {{ shape: {} }}", shape_word(n.shape));
                        }
                    }
                    ItemKind::Container(c) => container(out, c, level),
                    ItemKind::Edge(e) => {
                        path(out, &e.from);
                        out.push_str(" -> ");
                        path(out, &e.to);
                        if let Some(l) = &e.label {
                            out.push_str(": ");
                            label(out, l);
                        }
                    }
                }
                trailing(out, item.trailing.as_ref());
                out.push('\n');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANONICAL: &str = include_str!("../tests/fixtures/valid/canonical.vdg");

    #[test]
    fn canonical_example_is_a_fixed_point() {
        assert_eq!(fmt(CANONICAL).as_deref(), Some(CANONICAL));
    }

    #[test]
    fn noisy_minimal_graph_is_normalised() {
        let noisy = "graph {  \n  node a: \"A\"  \n  node b: \"B\"\n  a -> b\n}";
        let want = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n";
        assert_eq!(fmt(noisy).as_deref(), Some(want));
    }

    #[test]
    fn errors_refuse_and_warnings_format() {
        assert_eq!(fmt("graph {\n    node a \"A\"\n}\n"), None);
        let shadow = "graph {\n    node a: \"A\"\n    container c: \"C\" {\n        node a: \"A2\"\n    }\n}\n";
        assert_eq!(fmt(shadow).as_deref(), Some(shadow));
    }
}
