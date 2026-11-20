async fn handle_q_get_tasks(
    Extension(vox_rid): Extension<Option<String>>,
    Extension(db): Extension<Arc<Codex>>,
    Query(q): Query<std::collections::BTreeMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    return Ok(Json(
        serde_json::to_value((Task::all(&*db).await.expect("vox codegen: db all")).len() as i64)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(vox_http_client::envelope::error_json(
                        "SERIALIZATION_ERROR",
                        format!("{}", e),
                        vox_rid.clone(),
                        None,
                    )),
                )
            })?,
    ));
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<i64>,
    pub title: String,
    pub done: bool,
}
pub fn complete_task(id: i64) -> Result<String, String> {
    Task::delete(&*db, id)
        .await
        .expect("vox codegen: db delete");
    return Ok("done".to_string());
}
