use vox_orchestrator::dei_shim::research::claims::{extract_claims_with_model, Claim};

#[tokio::test]
async fn extract_claims_stub_returns_empty() {
    let claims = extract_claims_with_model("test query", None, None, None, None).await;
    assert!(claims.is_empty(), "Phase 0a stub must return Vec::new()");
}

#[test]
fn claim_default_fields_set() {
    let c = Claim {
        text: "X".into(),
        claim_id: 0,
        is_numeric: false,
        is_recent: false,
        is_named_event: false,
    };
    assert_eq!(c.text, "X");
}
