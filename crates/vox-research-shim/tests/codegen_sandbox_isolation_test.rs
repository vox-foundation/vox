use vox_research_shim::research::domain::codegen::{
    extract_code_snippets_from_markdown, verify_rust_code_in_sandbox, wrap_code_snippet_if_needed,
};

#[test]
fn test_wrap_code_snippet_statements() {
    let statement_code = "let a = 10;\nlet b = 20;\nassert_eq!(a + b, 30);";
    let wrapped = wrap_code_snippet_if_needed(statement_code);
    assert!(wrapped.contains("pub fn __vox_sandbox_probe"));
    assert!(wrapped.contains("#![allow(unused"));
}

#[test]
fn test_extract_code_snippets_markdown() {
    let md = "Here is an example:\n```rust\npub fn hello() -> &'static str { \"world\" }\n```\nAnd another:\n```rust\nlet x = 42;\n```";
    let snippets = extract_code_snippets_from_markdown(md);
    assert_eq!(snippets.len(), 2);
    assert!(snippets[0].contains("pub fn hello"));
    assert!(snippets[1].contains("let x = 42;"));
}

#[tokio::test]
async fn test_verify_rust_code_statement_with_wrapping() {
    let statement_code = "let a: i32 = 10;\nlet b: i32 = 20;\nassert_eq!(a + b, 30);";
    let wrapped = wrap_code_snippet_if_needed(statement_code);
    let res = verify_rust_code_in_sandbox(&wrapped, &[])
        .await
        .expect("compilation probe should succeed");
    assert!(
        res.passed,
        "Statement wrapped in probe function should compile: {}",
        res.stderr
    );
}
