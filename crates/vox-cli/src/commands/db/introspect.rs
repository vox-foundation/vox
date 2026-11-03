//! Live app-plane SQL schema introspection (`vox db introspect`).

use anyhow::Result;
use vox_sql::AnySqlBackend;
use vox_sql::schema_model::IntrospectedSchema;

/// Output of `vox db introspect --emit`.
///
/// `json` is the default and matches the previous stdout bytes (pretty or compact).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum IntrospectEmit {
    #[default]
    Json,
    Vox,
}

pub async fn introspect(url: Option<&str>, compact: bool, emit: IntrospectEmit) -> Result<()> {
    let backend = if let Some(url) = url {
        AnySqlBackend::connect_from_url(url).await?
    } else {
        AnySqlBackend::connect_from_app_env().await?
    };
    let schema = backend.introspect_schema().await?;
    print!("{}", render(&schema, compact, emit)?);
    Ok(())
}

/// Render an already-introspected schema. JSON keeps a trailing newline, matching
/// the previous `println!` of `serde_json`. `--compact` applies only to JSON.
fn render(schema: &IntrospectedSchema, compact: bool, emit: IntrospectEmit) -> Result<String> {
    match emit {
        IntrospectEmit::Json => {
            let body = if compact {
                serde_json::to_string(schema)?
            } else {
                serde_json::to_string_pretty(schema)?
            };
            Ok(format!("{body}\n"))
        }
        IntrospectEmit::Vox => Ok(schema.to_vox_source()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_compiler::ast::decl::Decl;
    use vox_sql::type_map::UnsupportedTypePolicy;
    use vox_sql::{BackendKind, LibsqlBackend, SqlBackend};

    async fn fixture_schema() -> vox_sql::schema_model::IntrospectedSchema {
        // `DbConfig::Memory` already has the migrated Vox schema (`users`, …).
        // Use dedicated names, matching `p3_introspect_smoke`, and keep only those
        // tables so the round-trip is about the schema we just created.
        let libsql = LibsqlBackend::connect_from_config(vox_db::DbConfig::Memory)
            .await
            .expect("connect in-memory");
        for ddl in [
            "DROP TABLE IF EXISTS dbi01_users",
            "DROP TABLE IF EXISTS dbi01_blog_posts",
            "CREATE TABLE dbi01_users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, nickname TEXT, score REAL NOT NULL, avatar BLOB)",
            "CREATE TABLE dbi01_blog_posts (post_id INTEGER PRIMARY KEY, author_id INTEGER NOT NULL, body TEXT)",
        ] {
            libsql.execute(ddl, &[]).await.expect("create table");
        }
        let mut schema = AnySqlBackend::Libsql(libsql)
            .introspect_schema()
            .await
            .expect("introspect");
        schema
            .tables
            .retain(|t| t.name == "dbi01_users" || t.name == "dbi01_blog_posts");
        schema.tables.sort_by(|a, b| a.name.cmp(&b.name));
        schema
    }

    /// DBI-01: introspect a sqlite schema, render Vox `table` decls, parse them
    /// with vox-compiler, and run the same declared-vs-live comparison as
    /// `vox db verify` (strict). No drift.
    #[tokio::test]
    async fn introspected_sqlite_schema_round_trips_through_verify() {
        let schema = fixture_schema().await;
        let src = schema.to_vox_source();
        let module = vox_compiler::parser::parse(vox_compiler::lexer::cursor::lex(&src))
            .unwrap_or_else(|e| panic!("emitted Vox must parse: {e:?}\n{src}"));
        let tables: Vec<_> = module
            .declarations
            .iter()
            .filter_map(|d| match d {
                Decl::Table(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(tables.len(), 2, "{src}");
        let findings = super::super::verify::compare_tables(
            &tables,
            &schema,
            BackendKind::Libsql,
            UnsupportedTypePolicy::Reject,
            true,
        );
        assert!(findings.is_empty(), "{findings:?}\n{src}");
    }

    /// DBI-01: default JSON output is unchanged by `--emit`.
    #[tokio::test]
    async fn emit_json_output_is_unchanged() {
        let schema = fixture_schema().await;
        assert_eq!(
            render(&schema, false, IntrospectEmit::Json).unwrap(),
            format!("{}\n", serde_json::to_string_pretty(&schema).unwrap())
        );
        assert_eq!(
            render(&schema, true, IntrospectEmit::Json).unwrap(),
            format!("{}\n", serde_json::to_string(&schema).unwrap())
        );
    }

    #[tokio::test]
    async fn emit_vox_matches_to_vox_source() {
        let schema = fixture_schema().await;
        assert_eq!(
            render(&schema, true, IntrospectEmit::Vox).unwrap(),
            schema.to_vox_source()
        );
    }
}
