use vox_research_shim::research::persistence::slug_from_query;

#[test]
fn slug_from_query_basic() {
    assert_eq!(slug_from_query("Hello, World! 2026"), "hello-world-2026");
    assert_eq!(slug_from_query(""), "untitled");
    let s = slug_from_query(&"a".repeat(200));
    assert!(s.len() <= 80, "slug capped at 80 chars, got {}", s.len());
}
