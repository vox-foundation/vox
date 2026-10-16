//! Trivia and attribute-block behavior of the parser.

use crate::ast::{Document, Entry, ItemKind};
use crate::ir::Shape;
use crate::{check, parse};

fn doc(src: &str) -> Document {
    let p = parse(src);
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let Some(d) = p.ast else { panic!("no ast") };
    d
}

fn shape_of(src: &str) -> Shape {
    let d = doc(src);
    let Some(Entry::Item(item)) = d.body.entries.first() else {
        panic!("no item")
    };
    let ItemKind::Node(n) = &item.kind else {
        panic!("not a node")
    };
    n.shape
}

#[test]
fn attr_block_single_line_multi_line_and_explicit_rect() {
    let one = "graph {\n node db: \"DB\" { shape: cylinder }\n}\n";
    let multi = "graph {\n node db: \"DB\" {\n  shape: cylinder\n }\n}\n";
    let rect = "graph {\n node db: \"DB\" { shape: rect }\n}\n";
    assert_eq!(shape_of(one), Shape::Cylinder);
    assert_eq!(shape_of(multi), Shape::Cylinder);
    assert_eq!(shape_of(rect), Shape::Rect);
}

#[test]
fn all_six_shapes_and_self_loop_are_valid() {
    let mut src = String::from("graph {\n");
    for (i, s) in ["rect", "rounded", "diamond", "cylinder", "start", "end"]
        .iter()
        .enumerate()
    {
        src.push_str(&format!(" node n{i}: \"N\" {{ shape: {s} }}\n"));
    }
    src.push_str(" n0 -> n0\n}\n");
    let r = check(&src);
    assert!(r.valid, "{:?}", r.diagnostics);
    assert_eq!(r.stats.nodes, 6);
}

#[test]
fn comments_attach_to_every_trivia_position() {
    let src = "// lead\ngraph { // h\n    // own\n\n    node a: \"A\" // t\n    container c: \"C\" { // ch\n        // inner\n        node b: \"B\"\n    } // cc\n} // dc\n// after\n";
    let d = doc(src);
    assert_eq!(d.leading.len(), 1);
    assert_eq!(d.leading[0].text, " lead");
    assert_eq!(
        d.header_trailing.as_ref().map(|c| c.text.as_str()),
        Some(" h")
    );
    let Entry::Comment(own) = &d.body.entries[0] else {
        panic!("expected own-line comment")
    };
    assert_eq!((own.text.as_str(), own.blank_before), (" own", false));
    let Entry::Item(a) = &d.body.entries[1] else {
        panic!("expected node")
    };
    assert!(a.blank_before);
    assert_eq!(a.trailing.as_ref().map(|c| c.text.as_str()), Some(" t"));
    let Entry::Item(c) = &d.body.entries[2] else {
        panic!("expected container")
    };
    assert!(c.trailing.is_none());
    let ItemKind::Container(cd) = &c.kind else {
        panic!("expected container")
    };
    assert_eq!(
        cd.header_trailing.as_ref().map(|c| c.text.as_str()),
        Some(" ch")
    );
    assert_eq!(
        cd.close_trailing.as_ref().map(|c| c.text.as_str()),
        Some(" cc")
    );
    assert!(matches!(&cd.body.entries[0], Entry::Comment(c) if c.text == " inner"));
    assert_eq!(
        d.close_trailing.as_ref().map(|c| c.text.as_str()),
        Some(" dc")
    );
    assert_eq!(d.trailing.len(), 1);
    assert_eq!(d.trailing[0].text, " after");
}

#[test]
fn blank_lines_collapse_to_a_flag_and_never_mark_the_first_entry() {
    let d = doc("graph {\n\n    node a: \"A\"\n\n\n    node b: \"B\"\n    node c: \"C\"\n}\n");
    let flags: Vec<bool> = d
        .body
        .entries
        .iter()
        .map(|e| matches!(e, Entry::Item(i) if i.blank_before))
        .collect();
    assert_eq!(flags, [false, true, false]);
}
