//! Rust codegen emission (Axum server, lib, tables, TS client).
//!
//! Split from the historical single `emit.rs` (OP-0204).

use std::collections::HashMap;

use crate::hir::HirModule;

mod client;
mod http;
mod method_emit;
mod stmt_expr;
mod stmt_expr_tail;
pub mod tables;
mod types;
mod with_emit;
mod workflow;

pub use client::{emit_api_client, emit_mcp_server};
pub use http::emit_main;
pub use stmt_expr::{emit_expr, emit_main_stmt};
pub use tables::{
    emit_index_ddl, emit_table_ddl, emit_table_struct, validate_db_projection_suffixes_unique,
};
pub use workflow::{emit_fn, emit_lib};

pub struct CodegenOutput {
    /// Relative path → file contents (e.g. `Cargo.toml`, `src/main.rs`).
    pub files: HashMap<String, String>,
    /// TypeScript API client for server functions (empty if no server fns)
    pub api_client_ts: String,
}

/// Emit a minimal Axum + Turso backend crate from `module` (paths relative to generated root).
pub fn generate(module: &HirModule, package_name: &str) -> Result<CodegenOutput, miette::Error> {
    let mut files = HashMap::new();

    let table_projections = tables::collect_table_select_projections(module);
    for table in &module.tables {
        if let Some(projs) = table_projections.get(&table.name) {
            tables::validate_db_projection_suffixes_unique(&table.name, projs)?;
        }
    }

    // Cargo.toml
    files.insert(
        "Cargo.toml".to_string(),
        emit_cargo_toml(package_name, module),
    );

    // src/main.rs (Entry point + Routes)
    files.insert("src/main.rs".to_string(), emit_main(module, package_name));

    // src/lib.rs (Types, Actors, Workflows, Functions)
    files.insert("src/lib.rs".to_string(), emit_lib(module));

    // TypeScript API client
    let api_client_ts = emit_api_client(module);

    // MCP server (if @mcp.tool declarations are present)
    if !module.mcp_tools.is_empty() {
        files.insert(
            "src/mcp_server.rs".to_string(),
            emit_mcp_server(module, package_name),
        );
    }

    Ok(CodegenOutput {
        files,
        api_client_ts,
    })
}

fn emit_rust_import_dependencies(module: &HirModule) -> String {
    let mut lines = std::collections::BTreeMap::<String, String>::new();
    for dep in &module.rust_imports {
        let crate_name = dep.crate_name.trim();
        if crate_name.is_empty() {
            continue;
        }
        if matches!(
            crate_name,
            "tokio" | "serde" | "serde_json" | "axum" | "tower" | "reqwest" | "tracing"
        ) {
            continue;
        }
        let line = if let Some(path) = &dep.path {
            format!("{crate_name} = {{ path = \"{path}\" }}")
        } else if let Some(git) = &dep.git {
            if let Some(rev) = &dep.rev {
                format!("{crate_name} = {{ git = \"{git}\", rev = \"{rev}\" }}")
            } else {
                format!("{crate_name} = {{ git = \"{git}\" }}")
            }
        } else if let Some(version) = &dep.version {
            format!("{crate_name} = \"{version}\"")
        } else {
            format!("{crate_name} = \"*\"")
        };
        lines.entry(crate_name.to_string()).or_insert(line);
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!(
            "{}\n",
            lines.values().cloned().collect::<Vec<_>>().join("\n")
        )
    }
}

/// `Cargo.toml` body for the generated Rust package `name`.
pub fn emit_cargo_toml(name: &str, module: &HirModule) -> String {
    let rust_import_deps = emit_rust_import_dependencies(module);
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "{edition}"

[workspace]

[dependencies]
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
axum = "0.7"
tower = "0.4"
rust-embed = "8"
mime_guess = "2"
reqwest = {{ version = "0.12", default-features = false, features = ["rustls-tls"] }}
tracing = "0.1"
tracing-subscriber = "0.3"
turso = {{ version = "0.4", default-features = false }}
vox-db = {{ path = "../../crates/vox-db" }}
vox-runtime = {{ path = "../../crates/vox-runtime" }}
vox-oratio = {{ path = "../../crates/vox-oratio" }}
{rust_import_deps}
"#,
        rust_import_deps = rust_import_deps,
        edition = crate::codegen_rust::GENERATED_CARGO_EDITION,
    )
}
