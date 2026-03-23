use vox_db::{VoxDb, DbConfig, Migration, validate_migrations};

#[tokio::test]
async fn test_migration_application() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    
    // Baseline should be 1
    assert_eq!(db.schema_version().await.unwrap(), 1);
    
    let migrations = vec![
        Migration {
            version: 100,
            name: "create_test_mig".to_string(),
            up_sql: "CREATE TABLE test_mig (id INTEGER PRIMARY KEY);".to_string(),
        },
        Migration {
            version: 101,
            name: "alter_test_mig".to_string(),
            up_sql: "ALTER TABLE test_mig ADD COLUMN name TEXT;".to_string(),
        },
    ];
    
    let applied = db.apply_migrations(&migrations).await.unwrap();
    assert_eq!(applied, vec![100, 101]);
    assert_eq!(db.schema_version().await.unwrap(), 101);
    
    // Check if table exists
    let rows = db.query_all("SELECT name FROM test_mig", ()).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_migration_validation_fails_on_descending() {
    let migrations = vec![
        Migration { version: 10, name: "ten".to_string(), up_sql: "".to_string() },
        Migration { version: 5, name: "five".to_string(), up_sql: "".to_string() },
    ];
    let result = validate_migrations(&migrations);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_auto_migration_diff() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    let migrator = db.auto_migrator();
    
    // We can't easily mock AST here without deep integration, 
    // but we can check if it runs without errors.
    let _list = migrator.introspect_tables().await.unwrap();
}
