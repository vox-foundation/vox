use vox_cli_research::eval::{GoldenQueryItem, evaluate_single_query_pipeline};
use vox_search::context::SearchRuntimeContext;

#[tokio::test]
async fn test_eval_executes_real_research_pipeline_not_web_lines() {
    // Exercises a real model; skipped where no local LLM is listening (hosted CI runners).
    if std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], 11434)),
        std::time::Duration::from_millis(300),
    )
    .is_err()
    {
        eprintln!("skipped: no local LLM on 127.0.0.1:11434");
        return;
    }
    let current_dir = std::env::current_dir().expect("cwd");
    let ctx = SearchRuntimeContext::new(
        current_dir.clone(),
        None,
        current_dir.clone(),
        current_dir.join("memory.md"),
    );
    let config = vox_research_shim::research::ResearchConfig::default();

    let item = GoldenQueryItem {
        query: "What is the memory safety model of Rust?".into(),
        gold_answer: Some(
            "Rust enforces memory safety through ownership, borrowing, and lifetimes at compile time."
                .into(),
        ),
        domain_mode: None,
        expected_sources: None,
        is_adversarial: None,
    };

    let sample = evaluate_single_query_pipeline("run-001", &item, &ctx, None, &config)
        .await
        .expect("must execute real pipeline");

    assert!(
        !sample.model_answer.is_empty(),
        "model answer must not be empty"
    );
    assert!(
        !sample.model_answer.starts_with("engine:"),
        "model answer must not be raw engine lines"
    );
    assert!(
        sample.evidence.get("total_claims").is_some(),
        "must record verified claims"
    );
}
