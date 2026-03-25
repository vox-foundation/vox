use crate::ast::scalar_mapping::VoxScalar;
use crate::hir::{HirIndex, HirModule, HirTable, HirType};

use super::types::emit_type;

fn hir_type_to_sql(ty: &HirType) -> &'static str {
    match ty {
        HirType::Named(n) => {
            if let Some(s) = VoxScalar::parse(n) {
                s.as_sqlite_affinity()
            } else {
                "TEXT" // Fallback: serialize as JSON text
            }
        }
        HirType::Generic(n, _) => match n.as_str() {
            "Id" => "INTEGER",  // Foreign key reference
            "Option" => "TEXT", // Nullable, type handled at Rust level
            _ => "TEXT",        // Complex types serialized as JSON
        },
        _ => "TEXT",
    }
}

/// Generate a Rust struct for a @table type w/ methods (tests and tooling).
pub fn emit_table_struct(table: &HirTable) -> String {
    let mut out = String::new();
    out.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    out.push_str(&format!("pub struct {} {{\n", table.name));
    out.push_str("    #[serde(skip_serializing_if = \"Option::is_none\")]\n");
    out.push_str("    pub _id: Option<i64>,\n");
    for field in &table.fields {
        out.push_str(&format!(
            "    pub {}: {},\n",
            field.name,
            emit_type(&field.type_ann)
        ));
    }
    out.push_str("}\n\n");

    // Helper to check if a type needs JSON serialization for SQL
    let is_json = |ty: &HirType| -> bool {
        match ty {
            HirType::Named(n) => VoxScalar::parse(n).is_none(),
            HirType::Generic(n, args) => {
                if n == "Option" {
                    // Option<T>: if T is simple, it's simple. If T is complex, it's JSON.
                    match &args[0] {
                        HirType::Named(sub) => VoxScalar::parse(sub).is_none(),
                        _ => true,
                    }
                } else if n == "Id" {
                    false
                } else {
                    true // List, etc.
                }
            }
            _ => true, // Unit, tuple etc.
        }
    };

    out.push_str(&format!("impl {} {{\n", table.name));

    let col_names: Vec<&String> = table.fields.iter().map(|f| &f.name).collect();
    let placeholders: Vec<String> = (1..=col_names.len()).map(|i| format!("?{}", i)).collect();
    let tn = table.name.to_lowercase();

    // -- insert
    out.push_str(
        "    pub async fn insert(db: &Codex, item: &Self) -> Result<i64, turso::Error> {\n",
    );
    for field in &table.fields {
        if is_json(&field.type_ann) {
            out.push_str(&format!(
                "        let {}_json = serde_json::to_string(&item.{}).map_err(|e| turso::Error::ConversionFailure(format!(\"serde_json: {{}}\", e)))?;\n",
                field.name, field.name
            ));
        }
    }
    out.push_str(&format!(
        "        db.connection().execute(\n            \"INSERT INTO {} ({}) VALUES ({})\",\n            turso::params![",
        tn,
        col_names
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        placeholders.join(", ")
    ));
    for (i, field) in table.fields.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        if is_json(&field.type_ann) {
            out.push_str(&format!("{}_json", field.name));
        } else {
            out.push_str(&format!("item.{}.clone()", field.name));
        }
    }
    out.push_str("],\n        ).await?;\n");
    out.push_str("        Ok(db.connection().last_insert_rowid())\n");
    out.push_str("    }\n\n");

    // -- get
    out.push_str(
        "    pub async fn get(db: &Codex, id: i64) -> Result<Option<Self>, turso::Error> {\n",
    );
    out.push_str(&format!(
        "        let mut rows = db.connection().query(\"SELECT * FROM {} WHERE _id = ?1\", turso::params![id]).await?;\n",
        tn
    ));
    out.push_str("        Ok(match rows.next().await? {\n");
    out.push_str("            Some(row) => Some(Self::from_row(&row)?),\n");
    out.push_str("            None => None,\n");
    out.push_str("        })\n");
    out.push_str("    }\n\n");

    // -- query (clause appended to SELECT * FROM table …)
    out.push_str(
        "    pub async fn query(db: &Codex, clause: &str) -> Result<Vec<Self>, turso::Error> {\n",
    );
    out.push_str(&format!(
        "        let sql = format!(\"SELECT * FROM {} {{}}\", clause);\n",
        tn
    ));
    out.push_str(
        "        let mut rows = db.connection().query(&sql, ()).await?;\n        let mut out = Vec::new();\n        while let Some(row) = rows.next().await? {\n            out.push(Self::from_row(&row)?);\n        }\n        Ok(out)\n",
    );
    out.push_str("    }\n\n");

    // -- delete
    out.push_str("    pub async fn delete(db: &Codex, id: i64) -> Result<usize, turso::Error> {\n");
    out.push_str(&format!(
        "        let n = db.connection().execute(\"DELETE FROM {} WHERE _id = ?1\", turso::params![id]).await?;\n",
        tn
    ));
    out.push_str("        Ok(n as usize)\n");
    out.push_str("    }\n\n");

    // -- from_row (column order: _id, then fields in declaration order)
    out.push_str("    fn from_row(row: &turso::Row) -> Result<Self, turso::Error> {\n");
    out.push_str("        let _id_val: i64 = row.get(0)?;\n");
    let mut col_idx: usize = 1;
    out.push_str("        Ok(Self {\n");
    out.push_str("            _id: Some(_id_val),\n");
    for field in &table.fields {
        if is_json(&field.type_ann) {
            out.push_str(&format!(
                "            {}: {{\n                let s: String = row.get({})?;\n                serde_json::from_str(&s).map_err(|e| turso::Error::ConversionFailure(format!(\"JSON decode on {}.{} row _id {{}}: {{}}\", _id_val, e)))?\n            }},\n",
                field.name, col_idx, tn, field.name
            ));
        } else {
            out.push_str(&format!(
                "            {}: row.get({})?,\n",
                field.name, col_idx
            ));
        }
        col_idx += 1;
    }
    out.push_str("        })\n");
    out.push_str("    }\n");

    out.push_str("}\n\n");
    out
}

/// Generate `CREATE TABLE IF NOT EXISTS` DDL for a @table.
pub fn emit_table_ddl(table: &HirTable) -> String {
    let table_name = table.name.to_lowercase();
    let mut cols = vec!["_id INTEGER PRIMARY KEY AUTOINCREMENT".to_string()];
    for field in &table.fields {
        let sql_type = hir_type_to_sql(&field.type_ann);
        let not_null = if matches!(&field.type_ann, HirType::Generic(n, _) if n == "Option") {
            ""
        } else {
            " NOT NULL"
        };
        cols.push(format!("    {} {}{}", field.name, sql_type, not_null));
    }
    format!(
        "CREATE TABLE IF NOT EXISTS {} (\n{}\n);",
        table_name,
        cols.join(",\n")
    )
}

/// Generate `CREATE INDEX IF NOT EXISTS` DDL for a @index.
pub fn emit_index_ddl(index: &HirIndex) -> String {
    let table_name = index.table_name.to_lowercase();
    format!(
        "CREATE INDEX IF NOT EXISTS idx_{table}_{name} ON {table} ({cols});",
        table = table_name,
        name = index.index_name,
        cols = index.columns.join(", "),
    )
}

/// Generate the DB initialization code for `main()` (Turso / libSQL).
///
/// Opens **Codex** via `vox_db::DbConfig::resolve_standalone` (VOX_DB_*, legacy TURSO_*, or local file).
pub(super) fn emit_db_setup(module: &HirModule) -> String {
    let mut out = String::new();
    out.push_str("    // ── Database setup (Codex / vox_db) ──\n");
    out.push_str(
        "    let cfg = vox_db::DbConfig::resolve_standalone().expect(\"resolve Codex DB config (VOX_DB_URL+TOKEN, VOX_DB_PATH, or TURSO_URL+TURSO_AUTH_TOKEN)\");\n",
    );
    out.push_str(
        "    let codex = vox_db::Codex::connect(cfg).await.expect(\"Failed to open Codex database\");\n",
    );
    out.push_str(
        "    codex.connection().execute_batch(\"PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;\").await.expect(\"PRAGMA failed\");\n",
    );
    out.push_str("    codex.connection().execute_batch(r#\"\n");
    for table in &module.tables {
        out.push_str(&emit_table_ddl(table));
        out.push('\n');
    }
    for index in &module.indexes {
        out.push_str(&emit_index_ddl(index));
        out.push('\n');
    }
    out.push_str("#\"#).await.expect(\"schema migration failed\");\n");
    out.push_str("    let db = Arc::new(codex);\n\n");
    out
}
