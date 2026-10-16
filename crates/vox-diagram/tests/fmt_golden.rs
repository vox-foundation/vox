//! Goldens for the canonical formatter `fmt`: comment placement, blank lines, escapes and
//! normalisation. Every case also asserts idempotence.

use vox_diagram::fmt;

const CANONICAL: &str = include_str!("fixtures/valid/canonical.vdg");
const ALL_CONSTRUCTS: &str = include_str!("fixtures/valid/all_constructs.vdg");

/// Format `src`, which must be valid.
fn once(src: &str) -> String {
    fmt(src).unwrap_or_else(|| panic!("expected valid source:\n{src}"))
}

#[test]
fn canonical_example_is_fixed_point() {
    assert_eq!(fmt(CANONICAL).as_deref(), Some(CANONICAL));
}

#[test]
fn comments_around_the_graph_stay_put() {
    let src = "// first\n\n// second\ngraph {\n    node a: \"A\"\n}\n// after\n\n// last\n";
    let a = once(src);
    // A blank line between the last leading comment and `graph` is not kept; blanks between
    // comments are.
    insta::assert_snapshot!(a);
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn header_and_trailing_comments_attach_to_their_line() {
    let src = "graph {   // h\n    node a: \"A\"   // t\n    a -> a  //  spaced  \n}  // c\n";
    let a = once(src);
    assert_eq!(
        a,
        "graph { // h\n    node a: \"A\" // t\n    a -> a //  spaced\n} // c\n"
    );
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn container_header_and_close_comments_attach() {
    let src = "graph {\n  container c: \"C\" {  // open\n    node a: \"A\"\n  }   // close\n}\n";
    let a = once(src);
    assert_eq!(
        a,
        "graph {\n    container c: \"C\" { // open\n        node a: \"A\"\n    } // close\n}\n"
    );
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn comment_text_is_verbatim_apart_from_trailing_space() {
    let a = once("graph {\n//x\n    //   y   \n    node a: \"A\"\n}\n");
    assert_eq!(a, "graph {\n    //x\n    //   y\n    node a: \"A\"\n}\n");
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn own_line_comments_keep_order_and_blank_before() {
    let src = "graph {\n// one\n\n\n// two\nnode a: \"A\"\n\n// three\n// four\nnode b: \"B\"\n}\n";
    let a = once(src);
    insta::assert_snapshot!(a);
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn blank_line_runs_collapse_to_one() {
    let src = "graph {\n    node a: \"A\"\n\n\n\n    node b: \"B\"\n    a -> b\n}\n";
    let a = once(src);
    assert_eq!(
        a,
        "graph {\n    node a: \"A\"\n\n    node b: \"B\"\n    a -> b\n}\n"
    );
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn blank_lines_at_block_edges_are_removed() {
    let src = "graph {\n\n\n    node a: \"A\"\n    container c: \"C\" {\n\n        node b: \"B\"\n\n    }\n\n\n}\n";
    let a = once(src);
    assert_eq!(
        a,
        "graph {\n    node a: \"A\"\n    container c: \"C\" {\n        node b: \"B\"\n    }\n}\n"
    );
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn default_shape_is_omitted_and_attr_block_collapses() {
    let src = "graph {\n    node a: \"A\" { shape: rect }\n    node b: \"B\" {\n        shape: cylinder\n    }\n    node c: \"C\" { shape: rounded , }\n}\n";
    let a = once(src);
    assert_eq!(
        a,
        "graph {\n    node a: \"A\"\n    node b: \"B\" { shape: cylinder }\n    node c: \"C\" { shape: rounded }\n}\n"
    );
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn labels_are_re_escaped_and_tabs_stay_raw() {
    let src = "graph {\n    node a: \"q\\\"uote \\\\ b\\nnl\tTAB\"\n    a -> a: \"x\\ny\"\n}\n";
    let a = once(src);
    assert_eq!(a, src);
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn crlf_input_formats_to_lf() {
    let src = "// c\ngraph {\n    node a: \"A\" // t\n\n    a -> a\n}\n".replace('\n', "\r\n");
    let a = once(&src);
    assert!(!a.contains('\r'));
    assert_eq!(
        a,
        "// c\ngraph {\n    node a: \"A\" // t\n\n    a -> a\n}\n"
    );
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn empty_and_comment_only_containers_print_on_separate_lines() {
    let src =
        "graph {\n container e: \"E\" { }\n container k: \"K\" {\n // only\n\n // two\n }\n}\n";
    let a = once(src);
    insta::assert_snapshot!(a);
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn every_construct_formats_stably() {
    let a = once(ALL_CONSTRUCTS);
    insta::assert_snapshot!(a);
    assert_eq!(fmt(&a).as_deref(), Some(a.as_str()));
}

#[test]
fn errors_refuse_to_format() {
    assert_eq!(fmt("graph {\n    node a \"A\"\n}\n"), None);
    assert_eq!(fmt(""), None);
}
