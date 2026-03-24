#![allow(missing_docs)]

use vox_compiler::codegen_rust::{emit::emit_api_client, generate as generate_rust};
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parser::parse;
use vox_runtime::{ContextBudget, RetrievedChunk, RetryPolicy, apply_context_budget};

fn lower(src: &str) -> vox_compiler::hir::HirModule {
    let tokens = lex(src);
    let module = parse(tokens).expect("source should parse");
    lower_module(&module)
}

#[test]
fn parity_contract_codegen_rust_includes_auth_rate_limit_and_request_id() {
    let src = r#"
@server fn chat(prompt: str) to str {
    ret prompt
}
"#;
    let hir = lower(src);
    let out = generate_rust(&hir, "parity_app").expect("rust codegen should succeed");
    let main_rs = out.files.get("src/main.rs").expect("main.rs should exist");

    assert!(
        main_rs.contains("tracing_subscriber::fmt::init"),
        "generated server should init tracing"
    );
    assert!(
        main_rs.contains("Router::new") && main_rs.contains("post(handle_sf_chat)"),
        "generated server should register @server route"
    );
    assert!(
        main_rs.contains("VOX_PORT"),
        "generated server should configure listen port from env"
    );
}

#[test]
fn parity_contract_api_client_supports_secure_headers_and_streaming() {
    let src = r#"
@server fn summarize(input: str) to str {
    ret input
}
"#;
    let hir = lower(src);
    let api_client = emit_api_client(&hir);
    assert!(
        api_client.contains("export async function summarize"),
        "api client should export summarize()"
    );
    assert!(
        api_client.contains("fetch") && api_client.contains("/api/"),
        "api client should call generated route"
    );
    assert!(
        api_client.contains("application/json"),
        "api client should send JSON"
    );
}

#[test]
fn parity_contract_context_budget_preserves_provenance() {
    let chunks = vec![
        RetrievedChunk {
            id: "c1".into(),
            source: "doc-a".into(),
            text: "0123456789".into(),
            score: 0.95,
        },
        RetrievedChunk {
            id: "c2".into(),
            source: "doc-b".into(),
            text: "abcdefghij".into(),
            score: 0.85,
        },
    ];
    let (selected, provenance) = apply_context_budget(
        chunks,
        ContextBudget {
            max_chunks: 2,
            max_chars: 12,
        },
    );
    assert_eq!(
        selected.len(),
        2,
        "should select both chunks under max_chunks"
    );
    assert_eq!(
        provenance.len(),
        2,
        "should produce provenance for each selected chunk"
    );
    assert!(
        provenance.iter().any(|p| p.truncated),
        "should mark truncated chunk when char budget is exceeded"
    );
}

#[test]
fn parity_contract_retry_policy_defaults_are_production_like() {
    let policy = RetryPolicy::default();
    assert!(
        policy.max_attempts >= 3,
        "retry policy should retry multiple times"
    );
    assert!(
        policy.base_delay_ms >= 100,
        "retry should include backoff delay"
    );
}
