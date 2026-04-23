#![allow(missing_docs)]

//! OpenAPI path keys must match the Axum router in `vox_populi::transport::router`.

#[test]
fn openapi_spec_parses_as_openapiv3() {
    let spec = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/populi/control-plane.openapi.yaml");
    let raw = std::fs::read_to_string(&spec).expect("read OpenAPI spec");
    let openapi: openapiv3::OpenAPI =
        serde_yaml::from_str(&raw).expect("YAML should deserialize to OpenAPI 3.x");
    assert!(
        !openapi.paths.paths.is_empty(),
        "OpenAPI paths must be non-empty"
    );
    assert!(
        openapi.openapi.starts_with("3."),
        "expected OpenAPI 3.x, got {:?}",
        openapi.openapi
    );
}

#[test]
fn openapi_paths_match_transport_router() {
    let spec = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/populi/control-plane.openapi.yaml");
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
        "/v1/populi/a2a/lease-renew".to_string(),
        "/v1/populi/admin/exec-lease/revoke".to_string(),
        "/v1/populi/admin/maintenance".to_string(),
        "/v1/populi/admin/quarantine".to_string(),
        "/v1/populi/bootstrap/exchange".to_string(),
        "/v1/populi/exec/lease/grant".to_string(),
        "/v1/populi/exec/lease/release".to_string(),
        "/v1/populi/exec/lease/renew".to_string(),
        "/v1/populi/exec/leases".to_string(),
        "/v1/populi/heartbeat".to_string(),
        "/v1/populi/join".to_string(),
        "/v1/populi/leave".to_string(),
        "/v1/populi/nodes".to_string(),
    ];
    assert_eq!(
        keys, expected,
        "update contracts/populi/control-plane.openapi.yaml or transport::router"
    );

    let ver = y["openapi"].as_str().expect("OpenAPI version");
    assert!(ver.starts_with("3."), "expected OpenAPI 3.x, got {ver:?}");
    let info = y["info"].as_mapping().expect("OpenAPI info");
    assert!(
        info.get("title").and_then(|t| t.as_str()).is_some(),
        "OpenAPI info.title required"
    );
    for (path_key, path_val) in paths {
        let path_key = path_key.as_str().expect("path key must be string");
        let path_obj = path_val
            .as_mapping()
            .unwrap_or_else(|| panic!("path {path_key} must be a mapping"));
        assert!(
            !path_obj.is_empty(),
            "path {path_key} must declare at least one operation"
        );
    }
}

#[test]
fn openapi_declares_remote_exec_lease_schemas() {
    let spec = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/populi/control-plane.openapi.yaml");
    let raw = std::fs::read_to_string(&spec).expect("read OpenAPI spec");
    let y: serde_yaml::Value = serde_yaml::from_str(&raw).expect("parse yaml");
    let schemas = y["components"]["schemas"]
        .as_mapping()
        .expect("OpenAPI components.schemas");
    let names: std::collections::HashSet<String> = schemas
        .keys()
        .filter_map(|k| k.as_str().map(str::to_string))
        .collect();
    for req in [
        "AdminExecLeaseRevokeRequest",
        "RemoteExecLeaseGrantRequest",
        "RemoteExecLeaseGrantResponse",
        "RemoteExecLeaseListItem",
        "RemoteExecLeaseListResponse",
        "RemoteExecLeaseRenewRequest",
        "RemoteExecLeaseReleaseRequest",
    ] {
        assert!(names.contains(req), "missing components.schemas.{req}");
    }
}

#[test]
fn openapi_declares_gpu_truth_layering_fields() {
    let spec = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/populi/control-plane.openapi.yaml");
    let raw = std::fs::read_to_string(&spec).expect("read OpenAPI spec");
    let y: serde_yaml::Value = serde_yaml::from_str(&raw).expect("parse yaml");
    let node_props = y["components"]["schemas"]["NodeRecord"]["properties"]
        .as_mapping()
        .expect("NodeRecord properties");
    for key in [
        "gpu_allocatable_count",
        "gpu_truth_layer",
        "maintenance_until_unix_ms",
    ] {
        let k = serde_yaml::Value::String(key.to_string());
        assert!(
            node_props.contains_key(&k),
            "missing NodeRecord.{key} in OpenAPI"
        );
    }
}

#[cfg(feature = "transport")]
#[test]
fn a2a_inbox_request_json_keys_match_openapi_contract() {
    use vox_populi::transport::A2AInboxRequest;
    let req = A2AInboxRequest {
        receiver_agent_id: "7".into(),
        claimer_node_id: Some("node-a".into()),
        max_messages: Some(10),
        before_message_id: Some(55),
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["receiver_agent_id"], "7");
    assert_eq!(v["claimer_node_id"], "node-a");
    assert_eq!(v["max_messages"], 10);
    assert_eq!(v["before_message_id"], 55);
}

/// Serde JSON keys for exec-lease DTOs stay aligned with the OpenAPI property names (snake_case).
#[cfg(feature = "transport")]
#[test]
fn remote_exec_lease_dto_json_keys_match_openapi_contract() {
    use vox_populi::transport::{
        RemoteExecLeaseGrantRequest, RemoteExecLeaseGrantResponse, RemoteExecLeaseListItem,
        RemoteExecLeaseListResponse, RemoteExecLeaseReleaseRequest, RemoteExecLeaseRenewRequest,
    };
    let g = RemoteExecLeaseGrantRequest {
        claimer_node_id: "n1".into(),
        scope_key: "wf:1".into(),
    };
    let gv = serde_json::to_value(&g).unwrap();
    assert_eq!(gv["claimer_node_id"], "n1");
    assert_eq!(gv["scope_key"], "wf:1");

    let gr = RemoteExecLeaseGrantResponse {
        lease_id: "7".into(),
        scope_key: "wf:1".into(),
        holder_node_id: "n1".into(),
        expires_unix_ms: 99,
    };
    let grv = serde_json::to_value(&gr).unwrap();
    assert_eq!(grv["lease_id"], "7");
    assert_eq!(grv["holder_node_id"], "n1");
    assert_eq!(grv["expires_unix_ms"], 99);

    let rn = RemoteExecLeaseRenewRequest {
        lease_id: "7".into(),
        claimer_node_id: "n1".into(),
    };
    let rnv = serde_json::to_value(&rn).unwrap();
    assert_eq!(rnv["lease_id"], "7");
    assert_eq!(rnv["claimer_node_id"], "n1");

    let rl = RemoteExecLeaseReleaseRequest {
        lease_id: "7".into(),
        claimer_node_id: "n1".into(),
    };
    let rlv = serde_json::to_value(&rl).unwrap();
    assert_eq!(rlv["lease_id"], "7");
    assert_eq!(rlv["claimer_node_id"], "n1");

    let lr = RemoteExecLeaseListResponse {
        leases: vec![RemoteExecLeaseListItem {
            lease_id: "7".into(),
            scope_key: "wf:1".into(),
            holder_node_id: "n1".into(),
            expires_unix_ms: 100,
        }],
    };
    let lv = serde_json::to_value(&lr).unwrap();
    assert_eq!(lv["leases"][0]["lease_id"], "7");
    assert_eq!(lv["leases"][0]["scope_key"], "wf:1");

    use vox_populi::transport::AdminExecLeaseRevokeRequest;
    let ar = AdminExecLeaseRevokeRequest {
        lease_id: "99".into(),
    };
    let arv = serde_json::to_value(&ar).unwrap();
    assert_eq!(arv["lease_id"], "99");
}
