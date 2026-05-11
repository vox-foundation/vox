//! Rust codegen emission (Axum server, lib, tables, TS client).
//!
//! Split from the historical single `emit.rs` (OP-0204).

use std::collections::HashMap;

use vox_compiler::app_contract::project_app_contract;
use vox_compiler::hir::HirModule;
use vox_compiler::rust_interop_support::{classify_rust_crate, is_template_managed_app_dependency};

mod client;
mod durability_lower;
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
    if let Ok(contract_json) = serde_json::to_string_pretty(&project_app_contract(module)) {
        files.insert("app_contract.json".to_string(), contract_json);
    }

    // Legacy `api.ts` emission removed — use Contract-IR-driven `vox-client.ts` from TS codegen.
    let api_client_ts = String::new();

    // MCP stdio server when `@mcp.tool` and/or `@mcp.resource` declarations are present.
    if !module.mcp_tools.is_empty() || !module.mcp_resources.is_empty() {
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
        if is_template_managed_app_dependency(crate_name) {
            continue;
        }
        let support = classify_rust_crate(crate_name).as_label();
        let dep_spec = if let Some(path) = &dep.path {
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
        let line = format!("# vox_rust_import support_class={support}\n{dep_spec}");
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
    let mcp_bin = if !module.mcp_tools.is_empty() || !module.mcp_resources.is_empty() {
        r#"

[[bin]]
name = "mcp_server"
path = "src/mcp_server.rs"
"#
    } else {
        ""
    };
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
tower-http = {{ version = "0.5", features = ["cors", "trace", "request-id"] }}
governor = "0.10"
vox-http-envelope = {{ path = "../../crates/vox-http-envelope" }}
rust-embed = "8"
mime_guess = "2"
reqwest = {{ version = "0.12", default-features = false, features = ["rustls-tls"] }}
vox-reqwest-defaults = {{ path = "../../crates/vox-reqwest-defaults" }}
tracing = "0.1"
tracing-subscriber = "0.3"
turso = {{ version = "0.4", default-features = false }}
vox-db = {{ path = "../../crates/vox-db" }}
vox-actor-runtime = {{ path = "../../crates/vox-actor-runtime" }}
vox-oratio = {{ path = "../../crates/vox-oratio" }}
{rust_import_deps}{mcp_bin}"#,
        rust_import_deps = rust_import_deps,
        mcp_bin = mcp_bin,
        edition = crate::codegen_rust::GENERATED_CARGO_EDITION,
    )
}
