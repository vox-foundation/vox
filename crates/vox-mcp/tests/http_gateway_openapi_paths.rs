#![allow(missing_docs)]

#[test]
fn http_gateway_openapi_paths_match_router() {
    let spec = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/mcp/http-gateway.openapi.yaml");
    let raw = std::fs::read_to_string(&spec).expect("read gateway OpenAPI spec");
    let y: serde_yaml::Value = serde_yaml::from_str(&raw).expect("parse gateway OpenAPI yaml");

    let paths = y["paths"].as_mapping().expect("OpenAPI paths object");
    let mut keys: Vec<String> = paths
        .keys()
        .filter_map(|k| k.as_str().map(str::to_string))
        .collect();
    keys.sort();

    let expected = vec![
        "/health".to_string(),
        "/v1/info".to_string(),
        "/v1/mobile".to_string(),
        "/v1/mobile/status".to_string(),
        "/v1/tools".to_string(),
        "/v1/tools/call".to_string(),
        "/v1/ws".to_string(),
    ];
    assert_eq!(
        keys, expected,
        "update contracts/mcp/http-gateway.openapi.yaml or http_gateway router paths"
    );

    let openapi = y["openapi"].as_str().expect("OpenAPI version");
    assert!(
        openapi.starts_with("3."),
        "expected OpenAPI 3.x, got {openapi:?}"
    );
    let title = y["info"]["title"].as_str().expect("OpenAPI info.title");
    assert!(
        !title.trim().is_empty(),
        "OpenAPI info.title must not be empty"
    );
    for (path_key, path_val) in paths {
        let path_key = path_key.as_str().expect("path key string");
        let path_map = path_val
            .as_mapping()
            .unwrap_or_else(|| panic!("path {path_key} must be mapping"));
        assert!(
            !path_map.is_empty(),
            "path {path_key} must declare at least one operation"
        );
    }
}
