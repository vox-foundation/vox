#![allow(missing_docs)]

//! OpenAPI path keys must match the Axum router in `vox_populi::transport::router`.

#[test]
fn openapi_paths_match_transport_router() {
    let spec = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/populi-control-plane.openapi.yaml");
    let raw = std::fs::read_to_string(&spec).expect("read OpenAPI spec");
    let y: serde_yaml::Value = serde_yaml::from_str(&raw).expect("parse yaml");
    let paths = y["paths"].as_mapping().expect("OpenAPI paths");
    let mut keys: Vec<_> = paths
        .keys()
        .filter_map(|k| k.as_str().map(str::to_string))
        .collect();
    keys.sort();
    let expected = vec![
        "/health".to_string(),
        "/v1/populi/a2a/ack".to_string(),
        "/v1/populi/a2a/deliver".to_string(),
        "/v1/populi/a2a/inbox".to_string(),
        "/v1/populi/bootstrap/exchange".to_string(),
        "/v1/populi/heartbeat".to_string(),
        "/v1/populi/join".to_string(),
        "/v1/populi/leave".to_string(),
        "/v1/populi/nodes".to_string(),
    ];
    assert_eq!(
        keys, expected,
        "update schemas/populi-control-plane.openapi.yaml or transport::router"
    );
}
