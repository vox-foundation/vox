fn bad_serialize(api_key: &str) -> String {
    serde_json::to_string(&serde_json::json!({
        "api_key": api_key,
        "kind": "provider"
    }))
    .expect("serialize")
}
