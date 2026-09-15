use vox_research_shim::research::domain::micro_benchmark::{
    run_rust_micro_benchmark, scaffold_rust_benchmark_source,
};

#[test]
fn test_scaffold_rust_benchmark_source_uses_black_box() {
    let source = scaffold_rust_benchmark_source("let mut x = 0; for i in 0..100 { x += i; }", 50);
    assert!(source.contains("std::hint::black_box"));
    assert!(source.contains("BENCHMARK_RESULT:"));
}

#[tokio::test]
async fn test_run_rust_micro_benchmark_success() {
    let report = run_rust_micro_benchmark("let mut v = Vec::new(); v.push(42);", 20, 5000)
        .await
        .expect("benchmark execution");
    assert!(report.passed);
    assert_eq!(report.iterations, 20);
    assert!(report.mean_ns > 0);
}

#[tokio::test]
async fn test_run_rust_micro_benchmark_compile_failure() {
    let report = run_rust_micro_benchmark("let mut v = ; invalid syntax here", 20, 5000)
        .await
        .expect("benchmark execution");
    assert!(!report.passed);
    assert_eq!(report.iterations, 0);
    assert!(!report.stderr.is_empty());
}
