use vox_db::{DbConfig, VoxDb};

#[tokio::test]
async fn test_research_misguidance_tables_exist() {
    let db = VoxDb::connect(DbConfig::Memory)
        .await
        .expect("connect memory db");
    // AMENDED #1: Use public db.connection() accessor instead of private db.conn
    let mut rows = db
        .connection()
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('research_misguidance_events', 'research_domain_reputation')",
            turso::params![],
        )
        .await
        .expect("query tables");

    let mut found = Vec::new();
    while let Some(row) = rows.next().await.expect("row") {
        found.push(row.get::<String>(0).expect("name"));
    }
    assert!(found.contains(&"research_misguidance_events".to_string()));
    assert!(found.contains(&"research_domain_reputation".to_string()));
}
