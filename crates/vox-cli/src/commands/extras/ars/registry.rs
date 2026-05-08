use std::sync::Arc;

pub(super) async fn make_registry() -> Arc<vox_openclaw_runtime::SkillRegistry> {
    let registry = vox_skills::new_registry_arc();
    if let Ok(db) = vox_db::Codex::connect_default().await {
        let db_arc = Arc::new(db);
        registry.set_db(db_arc.clone());
        let _ = registry.hydrate_from_db().await;
    }
    let _ = vox_skills::install_builtins(registry.as_ref()).await;
    registry
}
