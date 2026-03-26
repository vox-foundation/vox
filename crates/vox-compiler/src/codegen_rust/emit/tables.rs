use std::collections::{HashMap, HashSet};

use crate::ast::scalar_mapping::VoxScalar;
use crate::hir::{HirDbTableOp, HirExpr, HirIndex, HirModule, HirStmt, HirTable, HirType};

use super::types::emit_type;

/// Stable Rust method suffix for `SELECT _id, col1, …` projections (columns in declaration order).
pub(super) fn db_projection_method_suffix(cols: &[String]) -> String {
    cols.join("_")
}

/// Fail codegen when two different column projections collapse to the same Rust helper suffix
/// (`join("_")`), which would emit duplicate/inconsistent `from_row_sel_*` methods.
pub fn validate_db_projection_suffixes_unique(
    table_name: &str,
    projections: &[Vec<String>],
) -> Result<(), miette::Error> {
    use std::collections::HashMap;

    let mut by_suffix: HashMap<String, Vec<String>> = HashMap::new();
    for cols in projections {
        let sfx = db_projection_method_suffix(cols);
        if let Some(prev) = by_suffix.get(&sfx) {
            if prev != cols {
                return Err(miette::miette!(
                    "table '{}': `.select([…])` projections {:?} and {:?} both codegen to suffix '{}'; disambiguate column lists (suffix is columns joined with '_')",
                    table_name,
                    prev,
                    cols,
                    sfx
                ));
            }
        } else {
            by_suffix.insert(sfx, cols.clone());
        }
    }
    Ok(())
}

fn walk_select_projection_stmts(
    stmts: &[HirStmt],
    record: &mut impl FnMut(&str, &[String]),
) {
    for s in stmts {
        walk_select_projection_stmt(s, record);
    }
}

fn walk_select_projection_stmt(
    stmt: &HirStmt,
    record: &mut impl FnMut(&str, &[String]),
) {
    match stmt {
        HirStmt::Let { value, .. } => walk_select_projection_expr(value, record),
        HirStmt::Assign { target, value, .. } => {
            walk_select_projection_expr(target, record);
            walk_select_projection_expr(value, record);
        }
        HirStmt::Return { value: Some(v), .. } => walk_select_projection_expr(v, record),
        HirStmt::Return { value: None, .. } => {}
        HirStmt::Expr { expr, .. } => walk_select_projection_expr(expr, record),
    }
}

fn walk_select_projection_expr(expr: &HirExpr, record: &mut impl FnMut(&str, &[String])) {
    match expr {
        HirExpr::DbTableOp {
            table,
            op,
            args,
            select_cols,
            plan,
            limit,
            ..
        } => {
            if matches!(op, HirDbTableOp::All | HirDbTableOp::FilterRecord) {
                if let Some(cols) = select_cols {
                    record(table, cols);
                }
                if let Some(p) = plan
                    && let Some(cols) = p.projection.as_ref()
                {
                    record(table, cols);
                }
            }
            for a in args {
                walk_select_projection_expr(&a.value, record);
            }
            if let Some(l) = limit {
                walk_select_projection_expr(l.as_ref(), record);
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, v) in fields {
                walk_select_projection_expr(v, record);
            }
        }
        HirExpr::ListLit(items, _) | HirExpr::TupleLit(items, _) => {
            for it in items {
                walk_select_projection_expr(it, record);
            }
        }
        HirExpr::Binary(_, l, r, _) => {
            walk_select_projection_expr(l.as_ref(), record);
            walk_select_projection_expr(r.as_ref(), record);
        }
        HirExpr::Unary(_, o, _) => walk_select_projection_expr(o.as_ref(), record),
        HirExpr::Call(callee, args, _, _) => {
            walk_select_projection_expr(callee.as_ref(), record);
            for a in args {
                walk_select_projection_expr(&a.value, record);
            }
        }
        HirExpr::MethodCall(obj, _, args, _) => {
            walk_select_projection_expr(obj.as_ref(), record);
            for a in args {
                walk_select_projection_expr(&a.value, record);
            }
        }
        HirExpr::FieldAccess(o, _, _) => walk_select_projection_expr(o.as_ref(), record),
        HirExpr::Match(subj, arms, _) => {
            walk_select_projection_expr(subj.as_ref(), record);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    walk_select_projection_expr(g.as_ref(), record);
                }
                walk_select_projection_expr(arm.body.as_ref(), record);
            }
        }
        HirExpr::If(cond, then_b, else_b, _) => {
            walk_select_projection_expr(cond.as_ref(), record);
            walk_select_projection_stmts(then_b, record);
            if let Some(else_stmts) = else_b {
                walk_select_projection_stmts(else_stmts, record);
            }
        }
        HirExpr::For(_, it, body, _) => {
            walk_select_projection_expr(it.as_ref(), record);
            walk_select_projection_expr(body.as_ref(), record);
        }
        HirExpr::Lambda(_, _, body, _) => walk_select_projection_expr(body.as_ref(), record),
        HirExpr::Pipe(l, r, _) => {
            walk_select_projection_expr(l.as_ref(), record);
            walk_select_projection_expr(r.as_ref(), record);
        }
        HirExpr::Spawn(t, _) => walk_select_projection_expr(t.as_ref(), record),
        HirExpr::With(b, o, _) => {
            walk_select_projection_expr(b.as_ref(), record);
            walk_select_projection_expr(o.as_ref(), record);
        }
        HirExpr::Jsx(el) => {
            for a in &el.attributes {
                walk_select_projection_expr(&a.value, record);
            }
            for c in &el.children {
                walk_select_projection_expr(c, record);
            }
        }
        HirExpr::JsxSelfClosing(el) => {
            for a in &el.attributes {
                walk_select_projection_expr(&a.value, record);
            }
        }
        HirExpr::Block(stmts, _) => walk_select_projection_stmts(stmts, record),
        HirExpr::IntLit(_, _)
        | HirExpr::FloatLit(_, _)
        | HirExpr::StringLit(_, _)
        | HirExpr::BoolLit(_, _)
        | HirExpr::Ident(_, _) => {}
    }
}

/// Distinct column projections per table (post-HIR normalization — declaration order).
pub fn collect_table_select_projections(module: &HirModule) -> HashMap<String, Vec<Vec<String>>> {
    let mut sets: HashMap<String, HashSet<Vec<String>>> = HashMap::new();
    let mut record = |table_name: &str, cols: &[String]| {
        if cols.is_empty() {
            return;
        }
        sets.entry(table_name.to_string())
            .or_default()
            .insert(cols.to_vec());
    };

    for f in &module.functions {
        walk_select_projection_stmts(&f.body, &mut record);
    }
    for f in &module.tests {
        walk_select_projection_stmts(&f.body, &mut record);
    }
    for r in &module.routes {
        walk_select_projection_stmts(&r.body, &mut record);
    }
    for w in &module.workflows {
        walk_select_projection_stmts(&w.body, &mut record);
    }
    for a in &module.activities {
        walk_select_projection_stmts(&a.body, &mut record);
    }
    for sf in &module.server_fns {
        walk_select_projection_stmts(&sf.body, &mut record);
    }
    for qf in &module.query_fns {
        walk_select_projection_stmts(&qf.body, &mut record);
    }
    for mf in &module.mutation_fns {
        walk_select_projection_stmts(&mf.body, &mut record);
    }
    for actor in &module.actors {
        for h in &actor.handlers {
            walk_select_projection_stmts(&h.body, &mut record);
        }
    }
    for tool in &module.mcp_tools {
        walk_select_projection_stmts(&tool.func.body, &mut record);
    }

    sets.into_iter()
        .map(|(k, v)| {
            let mut projections: Vec<Vec<String>> = v.into_iter().collect();
            projections.sort();
            (k, projections)
        })
        .collect()
}

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

fn emit_select_projection_helpers(
    table: &HirTable,
    proj: &[String],
    tn: &str,
    is_json: &dyn Fn(&HirType) -> bool,
) -> String {
    let sfx = db_projection_method_suffix(proj);
    let col_list = proj.join(", ");
    let mut out = String::new();

    // -- from_row_sel_<suffix>
    out.push_str(&format!(
        "    fn from_row_sel_{sfx}(row: &turso::Row) -> Result<Self, turso::Error> {{\n        let _id_val: i64 = row.get(0)?;\n        Ok(Self {{\n            _id: Some(_id_val),\n"
    ));
    for field in &table.fields {
        let is_opt = matches!(&field.type_ann, HirType::Generic(n, _) if n == "Option");
        out.push_str(&format!("            {}: ", field.name));
        if let Some(ci) = proj.iter().position(|c| c == &field.name) {
            let idx = ci + 1;
            if is_json(&field.type_ann) {
                out.push_str(&format!(
                    "{{\n                let s: String = row.get({idx})?;\n                serde_json::from_str(&s).map_err(|e| turso::Error::ConversionFailure(format!(\"JSON decode on {}.{} row _id {{}}: {{}}\", _id_val, e)))?\n            }},\n",
                    tn, field.name
                ));
            } else {
                out.push_str(&format!("row.get({idx})?,\n"));
            }
        } else if is_opt {
            out.push_str("None,\n");
        } else {
            out.push_str(
                "{ return Err(turso::Error::ConversionFailure(\"vox: required column omitted from SELECT projection\".into())) },\n",
            );
        }
    }
    out.push_str("        })\n    }\n\n");

    // -- all_proj_<suffix>
    out.push_str(&format!(
        "    pub async fn all_proj_{sfx}(db: &Codex) -> Result<Vec<Self>, turso::Error> {{\n        let mut rows = db.connection().query(\"SELECT _id, {col_list} FROM {tn}\", ()).await?;\n        let mut out = Vec::new();\n        while let Some(row) = rows.next().await? {{\n            out.push(Self::from_row_sel_{sfx}(&row)?);\n        }}\n        Ok(out)\n    }}\n\n"
    ));

    // -- all_order_limit_proj_<suffix>
    out.push_str(&format!(
        "    pub async fn all_order_limit_proj_{sfx}(db: &Codex, order_clause: &str, limit: Option<i64>) -> Result<Vec<Self>, turso::Error> {{\n        let mut sql = \"SELECT _id, {col_list} FROM {tn}\".to_string();\n        if !order_clause.trim().is_empty() {{\n            sql.push_str(\" ORDER BY \");\n            sql.push_str(order_clause);\n        }}\n        if let Some(l) = limit {{\n            sql.push_str(&format!(\" LIMIT {{}}\", l.max(0)));\n        }}\n        let mut rows = db.connection().query(&sql, ()).await?;\n        let mut out = Vec::new();\n        while let Some(row) = rows.next().await? {{\n            out.push(Self::from_row_sel_{sfx}(&row)?);\n        }}\n        Ok(out)\n    }}\n\n"
    ));

    // -- filter_where_proj_<suffix>
    out.push_str(&format!(
        "    pub async fn filter_where_proj_{sfx}(db: &Codex, where_clause: &str, params: impl turso::IntoParams + Send) -> Result<Vec<Self>, turso::Error> {{\n        let sql = format!(\"SELECT _id, {col_list} FROM {tn} WHERE {{}}\", where_clause);\n        let mut rows = db.connection().query(&sql, params).await?;\n        let mut out = Vec::new();\n        while let Some(row) = rows.next().await? {{\n            out.push(Self::from_row_sel_{sfx}(&row)?);\n        }}\n        Ok(out)\n    }}\n\n"
    ));

    // -- filter_where_order_limit_proj_<suffix>
    out.push_str(&format!(
        "    pub async fn filter_where_order_limit_proj_{sfx}(db: &Codex, where_clause: &str, params: impl turso::IntoParams + Send, order_clause: &str, limit: Option<i64>) -> Result<Vec<Self>, turso::Error> {{\n        let mut sql = format!(\"SELECT _id, {col_list} FROM {tn} WHERE {{}}\", where_clause);\n        if !order_clause.trim().is_empty() {{\n            sql.push_str(\" ORDER BY \");\n            sql.push_str(order_clause);\n        }}\n        if let Some(l) = limit {{\n            sql.push_str(&format!(\" LIMIT {{}}\", l.max(0)));\n        }}\n        let mut rows = db.connection().query(&sql, params).await?;\n        let mut out = Vec::new();\n        while let Some(row) = rows.next().await? {{\n            out.push(Self::from_row_sel_{sfx}(&row)?);\n        }}\n        Ok(out)\n    }}\n\n"
    ));

    out
}

/// Generate a Rust struct for a @table type w/ methods (tests and tooling).
///
/// `projections` lists extra `SELECT _id, …` column sets referenced by `.select(...)` in the module.
pub fn emit_table_struct(table: &HirTable, projections: &[Vec<String>]) -> String {
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

    // -- all (safe full scan)
    out.push_str(
        "    pub async fn all(db: &Codex) -> Result<Vec<Self>, turso::Error> {\n",
    );
    out.push_str(&format!(
        "        let mut rows = db.connection().query(\"SELECT * FROM {}\", ()).await?;\n",
        tn
    ));
    out.push_str(
        "        let mut out = Vec::new();\n        while let Some(row) = rows.next().await? {\n            out.push(Self::from_row(&row)?);\n        }\n        Ok(out)\n",
    );
    out.push_str("    }\n\n");

    // -- all_order_limit (safe ORDER BY / LIMIT from compiler-validated clauses)
    out.push_str(
        "    pub async fn all_order_limit(db: &Codex, order_clause: &str, limit: Option<i64>) -> Result<Vec<Self>, turso::Error> {\n",
    );
    out.push_str(&format!(
        "        let mut sql = \"SELECT * FROM {}\".to_string();\n",
        tn
    ));
    out.push_str(
        "        if !order_clause.trim().is_empty() {\n            sql.push_str(\" ORDER BY \");\n            sql.push_str(order_clause);\n        }\n        if let Some(l) = limit {\n            sql.push_str(&format!(\" LIMIT {}\", l.max(0)));\n        }\n",
    );
    out.push_str(
        "        let mut rows = db.connection().query(&sql, ()).await?;\n        let mut out = Vec::new();\n        while let Some(row) = rows.next().await? {\n            out.push(Self::from_row(&row)?);\n        }\n        Ok(out)\n",
    );
    out.push_str("    }\n\n");

    // -- count
    out.push_str(
        "    pub async fn count(db: &Codex) -> Result<i64, turso::Error> {\n",
    );
    out.push_str(&format!(
        "        let mut rows = db.connection().query(\"SELECT COUNT(*) FROM {}\", ()).await?;\n",
        tn
    ));
    out.push_str(
        "        let row = rows.next().await?.ok_or_else(|| turso::Error::SqliteFailure(0, \"count: empty result\".into()))?;\n",
    );
    out.push_str("        let c: i64 = row.get(0)?;\n");
    out.push_str("        Ok(c)\n");
    out.push_str("    }\n\n");

    // -- count_where: parameterized count with WHERE fragment
    out.push_str(
        "    pub async fn count_where(db: &Codex, where_clause: &str, params: impl turso::IntoParams + Send) -> Result<i64, turso::Error> {\n",
    );
    out.push_str(&format!(
        "        let sql = format!(\"SELECT COUNT(*) FROM {} WHERE {{}}\", where_clause);\n",
        tn
    ));
    out.push_str(
        "        let mut rows = db.connection().query(&sql, params).await?;\n        let row = rows.next().await?.ok_or_else(|| turso::Error::SqliteFailure(0, \"count_where: empty result\".into()))?;\n        let c: i64 = row.get(0)?;\n        Ok(c)\n",
    );
    out.push_str("    }\n\n");

    // -- filter_where: parameterized equality predicates (compiler supplies literal column names)
    out.push_str(
        "    /// `WHERE` fragment uses `?1`, `?2`, … placeholders; bind with `turso::params!`.\n",
    );
    out.push_str(
        "    pub async fn filter_where(db: &Codex, where_clause: &str, params: impl turso::IntoParams + Send) -> Result<Vec<Self>, turso::Error> {\n",
    );
    out.push_str(&format!(
        "        let sql = format!(\"SELECT * FROM {} WHERE {{}}\", where_clause);\n",
        tn
    ));
    out.push_str(
        "        let mut rows = db.connection().query(&sql, params).await?;\n        let mut out = Vec::new();\n        while let Some(row) = rows.next().await? {\n            out.push(Self::from_row(&row)?);\n        }\n        Ok(out)\n",
    );
    out.push_str("    }\n\n");

    // -- filter_where_order_limit: parameterized filter + safe ORDER BY / LIMIT
    out.push_str(
        "    pub async fn filter_where_order_limit(db: &Codex, where_clause: &str, params: impl turso::IntoParams + Send, order_clause: &str, limit: Option<i64>) -> Result<Vec<Self>, turso::Error> {\n",
    );
    out.push_str(&format!(
        "        let mut sql = format!(\"SELECT * FROM {} WHERE {{}}\", where_clause);\n",
        tn
    ));
    out.push_str(
        "        if !order_clause.trim().is_empty() {\n            sql.push_str(\" ORDER BY \");\n            sql.push_str(order_clause);\n        }\n        if let Some(l) = limit {\n            sql.push_str(&format!(\" LIMIT {}\", l.max(0)));\n        }\n",
    );
    out.push_str(
        "        let mut rows = db.connection().query(&sql, params).await?;\n        let mut out = Vec::new();\n        while let Some(row) = rows.next().await? {\n            out.push(Self::from_row(&row)?);\n        }\n        Ok(out)\n",
    );
    out.push_str("    }\n\n");

    // -- unsafe_query_raw_clause (caller-controlled fragment after SELECT * FROM t)
    out.push_str(
        "    /// Builds `format!(\"SELECT * FROM <table> {}\", clause)` — use [`Self::all`] or [`Self::get`] when possible.\n",
    );
    out.push_str(
        "    pub async fn unsafe_query_raw_clause(db: &Codex, clause: &str) -> Result<Vec<Self>, turso::Error> {\n",
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

    for proj in projections {
        if proj.is_empty() {
            continue;
        }
        let field_names: Vec<&String> = table.fields.iter().map(|f| &f.name).collect();
        let ok = proj.iter().all(|c| field_names.iter().any(|n| *n == c));
        if !ok || proj.len() > field_names.len() {
            continue;
        }
        out.push_str(&emit_select_projection_helpers(
            table, proj, tn.as_str(), &is_json,
        ));
    }

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
