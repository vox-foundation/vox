//! Container-qualified, unique symbol ids (`<path>::<Container>::<name>`).
use std::collections::HashSet;
use std::path::Path;
use vox_graph_reader::ast::{ExtractedGraph, extract_ast_in_module};

fn ids(g: &ExtractedGraph) -> Vec<&str> {
    g.nodes.iter().map(|n| n.id.as_str()).collect()
}

fn kind_of<'a>(g: &'a ExtractedGraph, id: &str) -> &'a str {
    &g.nodes.iter().find(|n| n.id == id).unwrap().kind
}

fn assert_unique(g: &ExtractedGraph) {
    let mut seen = HashSet::new();
    for n in &g.nodes {
        assert!(seen.insert(n.id.as_str()), "duplicate id {}", n.id);
    }
}

const RUST: &str = r#"
pub struct Foo;
pub enum Mode { A, B }
pub type Alias = Foo;
pub trait Greet {
    fn hello(&self);
    fn wave(&self) { helper(); }
}
impl Foo {
    pub fn new() -> Self { Self::build() }
    fn build() -> Self { Foo }
}
impl Greet for Foo {
    fn hello(&self) {}
}
impl<T> std::convert::From<Vec<T>> for Foo { fn from(_: Vec<T>) -> Self { Foo } }
impl From<u8> for Foo { fn from(_: u8) -> Self { Foo } }
impl std::fmt::Display for Foo {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
}
fn helper() { let _ = Foo::new(); }
mod inner {
    pub fn helper() {}
    impl super::Foo { pub fn inner_new() {} }
}
#[cfg(unix)]
fn plat() {}
#[cfg(windows)]
fn plat() { helper(); }
"#;

#[test]
fn rust_methods_are_emitted_with_container_qualified_ids() {
    let g = extract_ast_in_module(Path::new("a.rs"), RUST, "src/a.rs");
    let ids = ids(&g);
    for want in [
        "src/a.rs::Foo",
        "src/a.rs::Mode",
        "src/a.rs::Alias",
        "src/a.rs::Greet",
        "src/a.rs::Greet::hello",
        "src/a.rs::Greet::wave",
        "src/a.rs::Foo::new",
        "src/a.rs::Foo::build",
        "src/a.rs::<Foo as Greet>::hello",
        "src/a.rs::<Foo as From<Vec<T>>>::from",
        "src/a.rs::<Foo as From<u8>>::from",
        "src/a.rs::<Foo as Display>::fmt",
        "src/a.rs::helper",
        "src/a.rs::inner::helper",
        "src/a.rs::inner::Foo::inner_new",
        "src/a.rs::plat",
        "src/a.rs::plat#2",
    ] {
        assert!(ids.contains(&want), "missing {want}; ids: {ids:?}");
    }
    assert_unique(&g);
    assert_eq!(kind_of(&g, "src/a.rs::Foo::new"), "method");
    assert_eq!(kind_of(&g, "src/a.rs::Greet::hello"), "method");
    assert_eq!(kind_of(&g, "src/a.rs::helper"), "fn");
    assert_eq!(kind_of(&g, "src/a.rs::Mode"), "enum");
    assert_eq!(kind_of(&g, "src/a.rs::Greet"), "trait");
    assert_eq!(kind_of(&g, "src/a.rs::Alias"), "type");
}

#[test]
fn rust_method_bodies_emit_call_edges() {
    let g = extract_ast_in_module(Path::new("a.rs"), RUST, "src/a.rs");
    let has = |s: &str, t: &str| g.edges.iter().any(|e| e.source == s && e.target == t);
    // `Self::build()` is resolved to the enclosing impl at extraction time.
    assert!(
        has("src/a.rs::Foo::new", "src/a.rs::Foo::build"),
        "{:?}",
        g.edges
    );
    // `Foo::new()` keeps its type qualifier as a resolution hint.
    assert!(has("src/a.rs::helper", "Foo::new"), "{:?}", g.edges);
    assert!(has("src/a.rs::Greet::wave", "helper"), "{:?}", g.edges);
    // The cfg-duplicate's edges carry its disambiguated id.
    assert!(has("src/a.rs::plat#2", "helper"), "{:?}", g.edges);
}

#[test]
fn free_fn_ids_are_unchanged() {
    let g = extract_ast_in_module(Path::new("m.rs"), "fn a(){ b(); }\nfn b(){}", "m.rs");
    assert_eq!(ids(&g), vec!["m.rs::a", "m.rs::b"]);
}

#[cfg(feature = "tree-sitter-grammars")]
#[test]
fn ts_class_methods_are_container_qualified() {
    let src = "class A { constructor() { go(); } run() {} }\n\
               class B { constructor() {} }\n\
               function go() {}\n";
    let g = extract_ast_in_module(Path::new("x.ts"), src, "ui/x.ts");
    let ids = ids(&g);
    for want in [
        "ui/x.ts::A",
        "ui/x.ts::A::constructor",
        "ui/x.ts::A::run",
        "ui/x.ts::B::constructor",
        "ui/x.ts::go",
    ] {
        assert!(ids.contains(&want), "missing {want}; ids: {ids:?}");
    }
    assert_unique(&g);
    assert_eq!(kind_of(&g, "ui/x.ts::A::run"), "method");
    assert_eq!(kind_of(&g, "ui/x.ts::go"), "fn");
    assert!(
        g.edges
            .iter()
            .any(|e| e.source == "ui/x.ts::A::constructor" && e.target == "go"),
        "{:?}",
        g.edges
    );
}

#[cfg(feature = "tree-sitter-grammars")]
#[test]
fn python_class_methods_are_container_qualified() {
    let src = "class A:\n    def __init__(self):\n        pass\n\n\
               class B:\n    def __init__(self):\n        pass\n\n\
               def top():\n    pass\n";
    let g = extract_ast_in_module(Path::new("c.py"), src, "pkg/c.py");
    let ids = ids(&g);
    for want in [
        "pkg/c.py::A",
        "pkg/c.py::A::__init__",
        "pkg/c.py::B::__init__",
        "pkg/c.py::top",
    ] {
        assert!(ids.contains(&want), "missing {want}; ids: {ids:?}");
    }
    assert_unique(&g);
    assert_eq!(kind_of(&g, "pkg/c.py::A::__init__"), "method");
}
