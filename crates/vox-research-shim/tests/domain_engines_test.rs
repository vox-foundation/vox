use vox_research_shim::research::domain::codegen::{
    generate_codegen_subqueries, verify_rust_code_in_sandbox,
};
use vox_research_shim::research::domain::shopping::generate_shopping_subqueries;

#[test]
fn test_shopping_subqueries() {
    let qs = generate_shopping_subqueries("Sony WH-1000XM5");
    assert!(
        qs.iter()
            .any(|q| q.contains("specifications") || q.contains("battery"))
    );
    assert!(
        qs.iter()
            .any(|q| q.contains("price") || q.contains("discounts"))
    );
    assert!(
        qs.iter()
            .any(|q| q.contains("complaints") || q.contains("reddit"))
    );
}

#[test]
fn test_codegen_subqueries() {
    let qs = generate_codegen_subqueries("tokio");
    assert!(
        qs.iter()
            .any(|q| q.contains("signatures") || q.contains("docs.rs"))
    );
    assert!(
        qs.iter()
            .any(|q| q.contains("examples") || q.contains("tutorial"))
    );
    assert!(
        qs.iter()
            .any(|q| q.contains("migration") || q.contains("breaking changes"))
    );
}

#[tokio::test]
async fn test_codegen_sandbox_verification() {
    let valid_code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
    let res = verify_rust_code_in_sandbox(valid_code, &[])
        .await
        .expect("check");
    assert!(res.passed, "valid code must pass compiler check");

    let invalid_code = "pub fn bad() { let x: i32 = \"string\"; }";
    let res_bad = verify_rust_code_in_sandbox(invalid_code, &[])
        .await
        .expect("check");
    assert!(!res_bad.passed, "invalid type must fail compiler check");
}
