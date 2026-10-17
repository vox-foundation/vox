//! Canonical printer: the AST in one fixed layout. Printing is a pure function of the AST, so
//! `fmt(fmt(x)) == fmt(x)` follows from `parse(fmt(x))` having the same AST as `parse(x)`.
//! Layout: 4-space indent per level, one statement per line, declaration order kept, `//`
//! comments kept at their attachment points, single blank lines kept (runs collapsed, none at
//! block edges), LF only and exactly one trailing newline. O(n).

use std::fmt::Write as _;

use crate::ast::{
    Block, Comment, ContainerDecl, DiagramKind, Document, Entry, ItemKind, Label, Ref,
};
use crate::check::{check, parse};
use crate::ir::{Direction, Shape};
use crate::sequence_ir::{SeqEvent, SequenceIr};

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
    out.push_str(match doc.kind {
        DiagramKind::Graph => "graph {",
        DiagramKind::Sequence => "sequence {",
    });
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
    push_quoted(out, &l.value);
}

/// Append `s` as a quoted DSL label, escaping `\`, `"` and newlines.
pub(crate) fn push_quoted(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
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

pub(crate) fn shape_word(s: Shape) -> &'static str {
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
                    ItemKind::Actor(a) => {
                        let _ = write!(out, "actor {}: ", a.id.name);
                        label(out, &a.label);
                    }
                    ItemKind::Message(m) => {
                        let reply = if m.reply { "reply " } else { "" };
                        let _ = write!(out, "{reply}{} -> {}", m.from.name, m.to.name);
                        if let Some(l) = &m.label {
                            out.push_str(": ");
                            label(out, l);
                        }
                    }
                    ItemKind::Note(n) => {
                        let who: Vec<&str> = n.over.iter().map(|i| i.name.as_str()).collect();
                        let _ = write!(out, "note over {}: ", who.join(", "));
                        label(out, &n.label);
                    }
                }
                trailing(out, item.trailing.as_ref());
                out.push('\n');
            }
        }
    }
}

/// Canonical `sequence { ... }` text for `ir`: actors in order, a blank line, then the events.
/// Labels are quoted by `push_quoted`; an empty message text prints no `: "..."` part.
pub(crate) fn sequence_source(ir: &SequenceIr) -> String {
    let mut out = String::from("sequence {\n");
    for a in &ir.actors {
        let _ = write!(out, "    actor {}: ", a.id);
        push_quoted(&mut out, &a.label);
        out.push('\n');
    }
    if !ir.actors.is_empty() && !ir.events.is_empty() {
        out.push('\n');
    }
    for e in &ir.events {
        match e {
            SeqEvent::Message {
                from,
                to,
                text,
                dashed,
                ..
            } => {
                let reply = if *dashed { "reply " } else { "" };
                let _ = write!(out, "    {reply}{from} -> {to}");
                if !text.is_empty() {
                    out.push_str(": ");
                    push_quoted(&mut out, text);
                }
            }
            SeqEvent::Note { over, text, .. } => {
                let _ = write!(out, "    note over {}: ", over.join(", "));
                push_quoted(&mut out, text);
            }
        }
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Ir;
    use crate::span::Location;

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
    fn push_quoted_escapes_backslash_quote_and_newline() {
        let mut out = String::new();
        push_quoted(&mut out, "a\\b\"c\nd");
        assert_eq!(out, "\"a\\\\b\\\"c\\nd\"");
    }

    #[test]
    fn errors_refuse_and_warnings_format() {
        assert_eq!(fmt("graph {\n    node a \"A\"\n}\n"), None);
        let shadow = "graph {\n    node a: \"A\"\n    container c: \"C\" {\n        node a: \"A2\"\n    }\n}\n";
        assert_eq!(fmt(shadow).as_deref(), Some(shadow));
    }

    #[test]
    fn sequence_source_prints_every_event_kind() {
        let actor = |id: &str, label: &str| crate::sequence_ir::Actor {
            id: id.into(),
            label: label.into(),
            span: Location::default(),
        };
        let msg = |from: &str, to: &str, text: &str, dashed: bool| SeqEvent::Message {
            from: from.into(),
            to: to.into(),
            text: text.into(),
            dashed,
            span: Location::default(),
        };
        let note = |over: &[&str], text: &str| SeqEvent::Note {
            over: over.iter().map(|s| (*s).into()).collect(),
            text: text.into(),
            span: Location::default(),
        };
        let ir = SequenceIr {
            actors: vec![actor("a", "A \"q\""), actor("b", "B")],
            events: vec![
                msg("a", "b", "hi", false),
                msg("b", "a", "", false),
                msg("b", "a", "back", true),
                msg("a", "a", "self", false),
                note(&["a"], "one"),
                note(&["a", "b"], "two"),
            ],
        };
        let want = "sequence {\n    actor a: \"A \\\"q\\\"\"\n    actor b: \"B\"\n\n    a -> b: \"hi\"\n    b -> a\n    reply b -> a: \"back\"\n    a -> a: \"self\"\n    note over a: \"one\"\n    note over a, b: \"two\"\n}\n";
        let text = sequence_source(&ir);
        assert_eq!(text, want);
        assert_eq!(fmt(&text).as_deref(), Some(want));
        let Some(Ir::Sequence(back)) = check(&text).ir else {
            panic!("does not check: {text}");
        };
        let strip = |s: &SequenceIr| {
            let mut s = s.clone();
            s.actors
                .iter_mut()
                .for_each(|a| a.span = Location::default());
            for e in &mut s.events {
                match e {
                    SeqEvent::Message { span, .. } | SeqEvent::Note { span, .. } => {
                        *span = Location::default();
                    }
                }
            }
            s
        };
        assert_eq!(strip(&back), strip(&ir));
    }
}
