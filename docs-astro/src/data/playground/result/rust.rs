async fn handle_q_parse_port(
    Extension(vox_rid): Extension<Option<String>>,
    Extension(db): Extension<Arc<Codex>>,
    Query(q): Query<std::collections::BTreeMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let raw = match q.get("raw") {
        None => serde_json::Value::Null,
        Some(__s) => match serde_json::from_str::<serde_json::Value>(__s) {
            Ok(v) => v,
            Err(__e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(vox_http_client::envelope::error_json(
                        "BAD_REQUEST",
                        format!("Invalid JSON for query parameter \"raw\": {}", __e),
                        vox_rid.clone(),
                        Some(serde_json::json!({"param": "raw"})),
                    )),
                ));
            }
        },
    };
    if ((raw) < (1024i64)) {
        return Ok(Json(
            serde_json::json!({ "Err": format!("{}{}", format!("{}{}", "Port must be >= 1024 (got ".to_string(), vox_actor_runtime::builtins::vox_display(&serde_json::to_value(&(raw)).unwrap_or_default())), ")".to_string()) }),
        ));
    };
    return Ok(Json(serde_json::json!({ "Ok": raw })));
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<i64>,
    pub host: String,
    pub port: i64,
}
