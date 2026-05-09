//! MVP: Arca schema chunk ↔ compiler `emit_table_ddl` parity for one pinned table.
//!
//! Compares logical column signatures (name, affinity, nullability, PK) with explicit
//! `_id` → `id` mapping. Expand mappings as the shared table-spec pathway grows.

use vox_codegen::codegen_rust::emit::emit_table_ddl;
use vox_compiler::ast::span::Span;
use vox_compiler::hir::{DefId, HirTable, HirTableField, HirType};

/// Column identity for parity (Arca `id` matches compiler `_id`).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct ColSig {
    logical_name: String,
    affinity: String,
    not_null: bool,
    is_pk: bool,
}

fn emit_cols(table: &HirTable) -> Vec<ColSig> {
    let ddl = emit_table_ddl(table);
    parse_create_table_cols(&ddl)
}

fn normalize_ident(sql_col: &str) -> String {
    let s = sql_col.trim();
    let s = s
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(s);
    let logical = if s == "_id" { "id" } else { s };
    logical.to_ascii_lowercase()
}

/// Extract `ColSig` list from a single `CREATE TABLE` statement (no embedded commas in types).
fn parse_create_table_cols(ddl: &str) -> Vec<ColSig> {
    let body_start = ddl.find('(').expect("CREATE TABLE (");
    let body_end = ddl.rfind(')').expect("CREATE TABLE )");
    let body = &ddl[body_start + 1..body_end];
    let mut cols = Vec::new();
    for raw_line in body.split(',') {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        assert!(
            parts.len() >= 2,
            "expected col def line, got {:?}",
            raw_line
        );
        let name = normalize_ident(parts[0]);
        let typ = parts[1].to_ascii_uppercase();
        let mut not_null = false;
        let mut is_pk = false;
        let mut i = 2usize;
        while i < parts.len() {
            if parts[i] == "NOT" && parts.get(i + 1) == Some(&"NULL") {
                not_null = true;
                i += 2;
                continue;
            }
            if parts[i] == "PRIMARY" && parts.get(i + 1) == Some(&"KEY") {
                is_pk = true;
                not_null = true;
                i += 2;
                continue;
            }
            if parts[i] == "AUTOINCREMENT" {
                i += 1;
                continue;
            }
            i += 1;
        }
        cols.push(ColSig {
            logical_name: name,
            affinity: typ,
            not_null,
            is_pk,
        });
    }
    cols
}

fn toestub_suppressions_hir() -> HirTable {
    HirTable {
        id: DefId(1),
        name: "ToestubSuppressions".to_string(),
        fields: vec![
            HirTableField {
                name: "path".to_string(),
                type_ann: HirType::Named("str".to_string()),
                span: Span::new(0, 0),
            },
            HirTableField {
                name: "line".to_string(),
                type_ann: HirType::Named("int".to_string()),
                span: Span::new(0, 0),
            },
            HirTableField {
                name: "rule_id".to_string(),
                type_ann: HirType::Named("str".to_string()),
                span: Span::new(0, 0),
            },
            HirTableField {
                name: "reason".to_string(),
                type_ann: HirType::Generic(
                    "Option".to_string(),
                    vec![HirType::Named("str".to_string())],
                ),
                span: Span::new(0, 0),
            },
            HirTableField {
                name: "created_at".to_string(),
                type_ann: HirType::Named("str".to_string()),
                span: Span::new(0, 0),
            },
        ],
        is_pub: true,
        is_deprecated: false,
        span: Span::new(0, 0),
    }
}

/// Canonical Arca fragment (defaults stripped for comparison — compiler DDL omits `DEFAULT`).
const ARCA_TOESTUB_SUPPRESSIONS_NORMALIZED: &str = "\
CREATE TABLE IF NOT EXISTS toestub_suppressions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
 path TEXT NOT NULL,
 line INTEGER NOT NULL,
 rule_id TEXT NOT NULL,
 reason TEXT,
 created_at TEXT NOT NULL
);";

#[test]
fn arca_compiler_toestub_suppressions_column_parity() {
    let table = toestub_suppressions_hir();
    let compiler_cols = emit_cols(&table);

    let arca_cols =
        parse_create_table_cols(&strip_sql_defaults(ARCA_TOESTUB_SUPPRESSIONS_NORMALIZED));

    assert_eq!(
        compiler_cols, arca_cols,
        "compiler vs Arca column signatures differ"
    );
}

fn strip_sql_defaults(sql: &str) -> String {
    let re = regex::Regex::new(r"\s+DEFAULT\s+\([^)]*\)").expect("regex");
    re.replace_all(sql, "").to_string()
}
