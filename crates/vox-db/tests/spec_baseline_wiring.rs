//! `schema::spec` DDL is concatenated into [`vox_db::schema::baseline_sql`] — guard against drift.

#[test]
fn spec_ddl_is_substring_of_baseline_sql() {
    let sql = vox_db::schema::baseline_sql();
    assert!(
        sql.contains(vox_db::schema::spec::POPULI_TRAINING_RUN_DDL.trim()),
        "baseline_sql must include POPULI_TRAINING_RUN_DDL from spec"
    );
    assert!(
        sql.contains(vox_db::schema::spec::CODEX_CAPABILITY_MAP_DDL.trim()),
        "baseline_sql must include CODEX_CAPABILITY_MAP_DDL from spec"
    );
    assert!(
        sql.contains("CREATE TABLE IF NOT EXISTS agent_sessions")
            && sql.contains("CREATE TABLE IF NOT EXISTS orchestration_lineage_events"),
        "baseline must include coordination + agents surfaces (multi-agent SSOT)"
    );
}

#[test]
fn orchestrator_digest_is_collections_first_for_usage_surfaces() {
    let d = vox_db::schema::orchestrator_schema_digest();
    assert!(
        d.tables.is_empty(),
        "orchestrator digest should not declare typed SQL tables for document-store surfaces"
    );
    let names: Vec<&str> = d.collections.iter().map(|c| c.name.as_str()).collect();
    assert!(
        names.contains(&"provider_usage"),
        "expected provider_usage collection, got {names:?}"
    );
}
