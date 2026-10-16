//! CommonMark fence scanning cases for `vox_diagram::fence`.
use vox_diagram::fence::{extract, normalize};

const INFO: &str = "vox-diagram";

#[test]
fn two_fences_in_order() {
    let md = "intro\n\n```vox-diagram\none\n```\n\ntext\n\n```vox-diagram\ntwo\n```\n";
    let f = extract(md, INFO);
    assert_eq!(f.len(), 2);
    assert_eq!((f[0].open_line, f[0].body.as_str()), (3, "one\n"));
    assert_eq!((f[1].open_line, f[1].body.as_str()), (9, "two\n"));
}

#[test]
fn inline_code_mention_is_not_a_fence() {
    let md = "Write ```vox-diagram in prose.\n\nOr `\\`\\`\\`vox-diagram` inline.\n";
    assert!(extract(md, INFO).is_empty());
}

#[test]
fn nested_in_four_backtick_block_is_ignored() {
    let md = "````md\n```vox-diagram\ngraph {}\n```\n````\n";
    assert!(extract(md, INFO).is_empty());
    assert_eq!(extract(md, "md").len(), 1);
}

#[test]
fn tilde_fence_matches() {
    let md = "~~~vox-diagram\ngraph {}\n~~~\n";
    let f = extract(md, INFO);
    assert_eq!(f.len(), 1);
    assert!(f[0].closed);
    assert_eq!(f[0].body, "graph {}\n");
}

#[test]
fn list_item_indentation_is_stripped() {
    let md = "- item\n\n  ```vox-diagram\n  graph {\n    a\n  }\n  ```\n";
    let f = extract(md, INFO);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].body, "graph {\n  a\n}\n");
    assert_eq!(f[0].open_line, 3);
    assert!(f[0].closed);
}

#[test]
fn blockquote_prefix_is_stripped() {
    let md = "> ```vox-diagram\n> graph {}\n> ```\n";
    let f = extract(md, INFO);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].body, "graph {}\n");
    assert!(f[0].closed);
}

#[test]
fn unclosed_vs_closed() {
    let open = extract("```vox-diagram\ngraph {}", INFO);
    assert_eq!(open.len(), 1);
    assert!(!open[0].closed);
    let shorter_closer = extract("````vox-diagram\nx\n```\n", INFO);
    assert_eq!(shorter_closer.len(), 1);
    assert!(!shorter_closer[0].closed);
    let closed = extract("```vox-diagram\nx\n```", INFO);
    assert!(closed[0].closed);
}

#[test]
fn empty_body_closed() {
    let f = extract("```vox-diagram\n```\n", INFO);
    assert_eq!(f.len(), 1);
    assert!(f[0].closed);
    assert_eq!(f[0].body, "");
    assert!(f[0].body_range.is_empty());
}

#[test]
fn crlf_matches_lf_after_normalize() {
    let lf = "x\n\n```vox-diagram\ngraph {\n  a\n}\n```\n";
    let crlf = lf.replace('\n', "\r\n");
    let a = extract(lf, INFO);
    let b = extract(&crlf, INFO);
    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
    assert_eq!(normalize(&a[0].body), normalize(&b[0].body));
    assert_eq!(b[0].open_line, 3);
    assert!(b[0].closed);
}

#[test]
fn mdx_frontmatter_and_import() {
    let md = "---\ntitle: T\n---\n\nimport X from './x'\n\n```vox-diagram\ngraph {}\n```\n";
    let f = extract(md, INFO);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].open_line, 7);
    assert_eq!(f[0].body, "graph {}\n");
}

#[test]
fn normalize_examples() {
    assert_eq!(normalize("a\r\nb\r\n\r\n"), "a\nb");
}
