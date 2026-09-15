use vox_research_shim::research::domain::polyglot_sandbox::{
    PolyglotLanguage, verify_in_polyglot_sandbox,
};

#[tokio::test]
async fn test_verify_sql_in_memory_sandbox() {
    let valid_sql = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT); INSERT INTO users VALUES (1, 'Alice');";
    let res = verify_in_polyglot_sandbox(valid_sql, PolyglotLanguage::Sql)
        .await
        .expect("sandbox probe");
    assert!(res.passed);

    let invalid_sql = "CRATE TABEL broken syntax error;";
    let res = verify_in_polyglot_sandbox(invalid_sql, PolyglotLanguage::Sql)
        .await
        .expect("sandbox probe");
    assert!(!res.passed);
}

#[tokio::test]
async fn test_verify_python_syntax_sandbox() {
    let valid_py = "def add(a: int, b: int) -> int:\n    return a + b\n";
    let res = verify_in_polyglot_sandbox(valid_py, PolyglotLanguage::Python)
        .await
        .expect("sandbox probe");
    assert!(res.passed);

    let invalid_py = "def broken(\n    return 1\n";
    let res = verify_in_polyglot_sandbox(invalid_py, PolyglotLanguage::Python)
        .await
        .expect("sandbox probe");
    assert!(!res.passed);
}

#[tokio::test]
async fn test_verify_vox_syntax_sandbox() {
    let valid_vox = "fn add(a, b) to int { return a + b }";
    let res = verify_in_polyglot_sandbox(valid_vox, PolyglotLanguage::Vox)
        .await
        .expect("sandbox probe");
    assert!(res.passed);

    let invalid_vox = "fn broken( { return 1 }";
    let res = verify_in_polyglot_sandbox(invalid_vox, PolyglotLanguage::Vox)
        .await
        .expect("sandbox probe");
    assert!(!res.passed);
}

#[tokio::test]
async fn test_verify_sql_postgres_and_mysql_dialects() {
    use vox_research_shim::research::domain::polyglot_sandbox::{
        SqlDialect, verify_sql_in_sandbox,
    };

    let valid_pg = "SELECT id, name FROM users WHERE id = 1 LIMIT 10;";
    let res = verify_sql_in_sandbox(valid_pg, SqlDialect::Postgres)
        .await
        .expect("sandbox probe");
    assert!(res.passed);

    let invalid_pg = "CRATE TABEL broken;";
    let res = verify_sql_in_sandbox(invalid_pg, SqlDialect::Postgres)
        .await
        .expect("sandbox probe");
    assert!(!res.passed);

    let valid_mysql = "SELECT id, name FROM users WHERE id = 1 LIMIT 10;";
    let res = verify_sql_in_sandbox(valid_mysql, SqlDialect::MySql)
        .await
        .expect("sandbox probe");
    assert!(res.passed);

    let invalid_mysql = "SELEC broken syntax;";
    let res = verify_sql_in_sandbox(invalid_mysql, SqlDialect::MySql)
        .await
        .expect("sandbox probe");
    assert!(!res.passed);
}
