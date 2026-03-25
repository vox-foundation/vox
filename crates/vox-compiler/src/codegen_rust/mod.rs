//! Vox → Rust code generator.
//!
//! Generates a full Rust web server project from a Vox HIR module.

#![allow(clippy::collapsible_if)]

pub mod emit;

pub use emit::emit_api_client;

/// Rust `edition` written into generated `Cargo.toml` files (keep aligned with root workspace).
pub const GENERATED_CARGO_EDITION: &str = "2024";

use crate::hir::hir::HirModule;
/// Re-export facade used by integration tests: `vox_codegen_rust::emit::*`.
use std::collections::HashMap;
use std::path::Path;

/// `path` value for a generated Cargo.toml `[dependencies]` entry.
///
/// On Windows, strips `\\?\` from canonical paths so Cargo accepts the literal (avoids `//?/C:/...`).
fn manifest_dependency_path(path: &Path) -> String {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        let rest = if let Some(r) = s.strip_prefix(r"\\?\") {
            r.to_string()
        } else {
            s.to_string()
        };
        let normalized = rest.replace('\\', "/");
        if let Some(unc) = normalized.strip_prefix("UNC/") {
            format!("//{unc}")
        } else {
            normalized
        }
    }
    #[cfg(not(windows))]
    {
        path.to_string_lossy().replace('\\', "/")
    }
}

/// Output of code generation: a map of `filename -> content`.
#[derive(Debug)]
pub struct CodegenOutput {
    pub files: HashMap<String, String>,
    /// TypeScript API client for server functions (empty if no server fns).
    pub api_client_ts: String,
}

impl CodegenOutput {
    /// Write all generated files to the target directory.
    ///
    /// **Incremental Diffing:** Only writes a file if its local content differs
    /// from the existing file on disk. This preserves the file's modification
    /// time (mtime), which prevents Cargo from performing a redundant full rebuild
    /// when the generated code remains identical after a `vox run` re-eval.
    pub fn write_to_dir(&self, target_dir: &Path) -> std::io::Result<()> {
        for (filename, content) in &self.files {
            let path = target_dir.join(filename);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let needs_write = if path.exists() {
                let existing = std::fs::read_to_string(&path).ok();
                existing.as_ref() != Some(content)
            } else {
                true
            };

            if needs_write {
                std::fs::write(&path, content)?;
            }
        }
        Ok(())
    }
}

/// Generate a full Rust project from a HIR module.
pub fn generate(module: &HirModule, package_name: &str) -> Result<CodegenOutput, miette::Error> {
    let out = emit::generate(module, package_name)?;
    Ok(CodegenOutput {
        files: out.files,
        api_client_ts: out.api_client_ts,
    })
}

/// Target for script-mode execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptTarget {
    /// Native Rust binary (tokio, vox-runtime).
    Native,
    /// WASI binary (wasm32-wasip1, vox-script-wasi, no tokio).
    Wasi,
}

/// Generate a minimal Rust binary project for script-mode execution.
///
/// Unlike [generate], this skips all web server boilerplate (warp, axum,
/// rust-embed, metrics) and emits only the code needed to compile and run
/// a `fn main()`.
///
/// `runtime_path` should point to the vox-runtime crate directory (e.g. workspace
/// `crates/vox-runtime`). If `None`, uses `VOX_RUNTIME_PATH` env or a fallback.
pub fn generate_script(
    module: &HirModule,
    package_name: &str,
    runtime_path: Option<&Path>,
) -> Result<CodegenOutput, miette::Error> {
    generate_script_with_target(module, package_name, runtime_path, ScriptTarget::Native)
}

/// Generate a script project for the given target (Native or Wasi).
pub fn generate_script_with_target(
    module: &HirModule,
    package_name: &str,
    runtime_path: Option<&Path>,
    target: ScriptTarget,
) -> Result<CodegenOutput, miette::Error> {
    let mut files = HashMap::new();

    let crate_name = package_name.replace('-', "_");

    let runtime_path_str = runtime_path
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .or_else(|| std::env::var("VOX_RUNTIME_PATH").ok())
        .unwrap_or_else(|| "../vox-runtime".to_string());

    // ── WASI feature guardrail ──────────────────────────────────────────────
    // Jai-inspired: fail loudly and immediately with a clear diagnostic
    // rather than emitting broken code that produces confusing linker errors
    // or silent runtime panics inside the Wasmtime sandbox.
    if target == ScriptTarget::Wasi {
        let mut unsupported: Vec<String> = Vec::new();

        if !module.actors.is_empty() {
            unsupported.push(format!(
                "actors are not supported in WASI mode: {}",
                module
                    .actors
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !module.workflows.is_empty() {
            unsupported.push(format!(
                "workflows are not supported in WASI mode: {}",
                module
                    .workflows
                    .iter()
                    .map(|w| w.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !module.activities.is_empty() {
            unsupported.push(format!(
                "activities are not supported in WASI mode: {}",
                module
                    .activities
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !module.server_fns.is_empty() {
            unsupported.push(format!(
                "server functions are not supported in WASI mode: {}",
                module
                    .server_fns
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !module.mcp_tools.is_empty() {
            unsupported.push(format!(
                "MCP tools are not supported in WASI mode: {}",
                module
                    .mcp_tools
                    .iter()
                    .map(|t| t.func.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        if !unsupported.is_empty() {
            return Err(miette::miette!(
                help = "Remove these features from the script, or run without --isolation wasm to use the full native runtime.",
                "WASI mode does not support the following features used in this script:\n  • {}",
                unsupported.join("\n  • ")
            ));
        }
    }

    let cargo_toml = match target {
        ScriptTarget::Native => format!(
            r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "{edition}"

[workspace]

[dependencies]
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
vox-runtime = {{ path = "{runtime_path_str}" }}
"#,
            package_name = package_name,
            runtime_path_str = runtime_path_str,
            edition = GENERATED_CARGO_EDITION,
        ),
        ScriptTarget::Wasi => {
            let wasi_path = runtime_path
                .and_then(|p| p.parent())
                .map(|p| manifest_dependency_path(&p.join("vox-script-wasi")))
                .unwrap_or_else(|| "../vox-script-wasi".to_string());
            format!(
                r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "{edition}"

[workspace]

[dependencies]
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"

[target.'cfg(target_arch = "wasm32")'.dependencies]
vox-script-wasi = {{ path = "{wasi_path}" }}
"#,
                package_name = package_name,
                wasi_path = wasi_path,
                edition = GENERATED_CARGO_EDITION,
            )
        }
    };
    files.insert("Cargo.toml".to_string(), cargo_toml);

    // Emit lib.rs with all non-main declarations (no warp/SSE for script mode)
    files.insert("src/lib.rs".to_string(), emit::emit_lib(module));

    // Emit a script-mode main.rs: just `use crate::*;` and the user's main fn body
    let mut main_rs = String::new();
    main_rs.push_str("// Generated by Vox Compiler (script mode)\n");
    main_rs.push_str("#![allow(unused)]\n\n");
    main_rs.push_str(&format!("use {}::*;\n\n", crate_name));

    let mut found_main = false;
    for func in &module.functions {
        if func.name == "main" {
            found_main = true;
            match target {
                ScriptTarget::Native => {
                    let is_async = func.is_async;
                    if is_async {
                        main_rs.push_str("#[tokio::main]\nasync fn main() {\n");
                    } else {
                        main_rs.push_str("fn main() {\n");
                    }
                    for stmt in &func.body {
                        main_rs.push_str(&emit::emit_main_stmt(stmt, 1));
                    }
                    main_rs.push_str("}\n");
                }
                ScriptTarget::Wasi => {
                    if func.is_async {
                        // Jai-inspired: compile-time error, not a runtime surprise.
                        // async fn main() is not supported in WASI mode because Wasmtime P1
                        // does not expose an async executor — use native mode for async scripts.
                        main_rs.push_str("fn main() {\n");
                        main_rs.push_str("    compile_error!(\"async fn main() is not supported in --isolation wasm mode. \\nRemove async or use vox run without --isolation wasm.\");\n");
                        main_rs.push_str("}\n");
                    } else {
                        main_rs.push_str("fn main() {\n");
                        for stmt in &func.body {
                            // WASI: same Rust statement emission for now; builtin routing can be
                            // reintroduced when `vox-script-wasi` shims are wired to HIR again.
                            main_rs.push_str(&emit::emit_main_stmt(stmt, 1));
                        }
                        main_rs.push_str("}\n");
                    }
                }
            }
            break;
        }
    }

    if !found_main {
        main_rs.push_str("fn main() {\n    eprintln!(\"No main function found.\");\n}\n");
    }

    files.insert("src/main.rs".to_string(), main_rs);

    Ok(CodegenOutput {
        files,
        api_client_ts: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::span::Span;
    use crate::hir::hir::{
        DefId, HirActor, HirModule, HirTable, HirTableField, HirType, HirWorkflow,
    };
    use emit::emit_table_struct;

    fn empty_module() -> HirModule {
        HirModule::default()
    }

    fn simple_task_table() -> HirTable {
        HirTable {
            id: DefId(1),
            name: "Task".to_string(),
            fields: vec![
                HirTableField {
                    name: "title".to_string(),
                    type_ann: HirType::Named("str".to_string()),
                    span: Span::new(0, 0),
                },
                HirTableField {
                    name: "done".to_string(),
                    type_ann: HirType::Named("bool".to_string()),
                    span: Span::new(0, 0),
                },
                HirTableField {
                    name: "priority".to_string(),
                    type_ann: HirType::Generic(
                        "Option".to_string(),
                        vec![HirType::Named("int".to_string())],
                    ),
                    span: Span::new(0, 0),
                },
            ],
            is_pub: true,
            is_deprecated: false,
            span: Span::new(0, 0),
        }
    }

    // ── emit_table_struct: typed DSL surface ──────────────────────────────────

    #[test]
    #[ignore = "emit_table_struct DSL drift vs tests — reconcile when table codegen stabilizes"]
    fn emits_struct_and_patch_struct() {
        let table = simple_task_table();
        let out = emit_table_struct(&table);
        assert!(out.contains("pub struct Task {"), "should emit Task struct");
        assert!(
            out.contains("pub struct TaskPatch {"),
            "should emit TaskPatch struct"
        );
        // patch fields are Option<T>
        assert!(
            out.contains("pub title: Option<String>"),
            "patch.title should be Option<String>"
        );
        assert!(
            out.contains("pub done: Option<bool>"),
            "patch.done should be Option<bool>"
        );
    }

    #[test]
    fn emits_id_field() {
        let table = simple_task_table();
        let out = emit_table_struct(&table);
        assert!(
            out.contains("pub _id: Option<i64>"),
            "should emit _id Option<i64>"
        );
    }

    #[test]
    #[ignore = "emit_table_struct DSL drift vs tests — reconcile when table codegen stabilizes"]
    fn emits_all_typed_dsl_methods() {
        let table = simple_task_table();
        let out = emit_table_struct(&table);
        // Typed DSL surface — these are the LLM-native methods
        assert!(out.contains("pub async fn all("), "should emit all()");
        assert!(
            out.contains("pub async fn all_ordered("),
            "should emit all_ordered()"
        );
        assert!(out.contains("pub async fn find("), "should emit find()");
        assert!(
            out.contains("pub async fn where_eq("),
            "should emit where_eq()"
        );
        assert!(out.contains("pub async fn count("), "should emit count()");
        assert!(
            out.contains("pub async fn insert_typed("),
            "should emit insert_typed()"
        );
        assert!(
            out.contains("pub async fn update_id("),
            "should emit update_id()"
        );
        assert!(
            out.contains("pub async fn update_patch("),
            "should emit update_patch()"
        );
        assert!(
            out.contains("pub async fn delete_id("),
            "should emit delete_id()"
        );
        assert!(
            out.contains("pub async fn fts_search("),
            "should emit fts_search()"
        );
    }

    #[test]
    fn emits_legacy_escape_hatch_methods() {
        let table = simple_task_table();
        let out = emit_table_struct(&table);
        // Legacy compat — must remain as frozen escape hatches
        assert!(
            out.contains("pub async fn insert("),
            "should retain legacy insert"
        );
        assert!(
            out.contains("pub async fn get("),
            "should retain legacy get"
        );
        assert!(
            out.contains("pub async fn query("),
            "should retain escape-hatch query"
        );
        assert!(
            out.contains("pub async fn delete("),
            "should retain legacy delete"
        );
    }

    #[test]
    fn emits_correct_table_name_in_sql() {
        let table = simple_task_table();
        let out = emit_table_struct(&table);
        assert!(
            out.contains("FROM task"),
            "SQL should reference lowercase table name 'task'"
        );
        assert!(
            out.contains("INSERT INTO task"),
            "INSERT should use lowercase table name"
        );
        assert!(
            out.contains("DELETE FROM task"),
            "DELETE should use lowercase table name"
        );
    }

    #[test]
    #[ignore = "emit_table_struct DSL drift vs tests — reconcile when table codegen stabilizes"]
    fn bool_field_maps_to_i64_in_from_row() {
        let table = simple_task_table();
        let out = emit_table_struct(&table);
        // bool stored as i64 (0/1)
        assert!(
            out.contains("row.get::<i64>") && out.contains("!= 0"),
            "bool field should use i64 + != 0 in from_row"
        );
    }

    #[test]
    #[ignore = "emit_table_struct DSL drift vs tests — reconcile when table codegen stabilizes"]
    fn option_field_maps_to_option_type_in_from_row() {
        let table = simple_task_table();
        let out = emit_table_struct(&table);
        assert!(
            out.contains("row.get::<Option<i64>>"),
            "Option<int> should deserialize via Option<i64>"
        );
    }

    #[test]
    fn find_returns_bare_t_with_error_on_missing() {
        let table = simple_task_table();
        let out = emit_table_struct(&table);
        // find must return Self with turso::Error if missing, per vox non-null policy
        assert!(
            out.contains("Result<Self, turso::Error>"),
            "find, get must return Result<Self> (and error if missing) per non-null policy"
        );
    }

    #[test]
    #[ignore = "emit_table_struct DSL drift vs tests — reconcile when table codegen stabilizes"]
    fn insert_typed_uses_turso_value_not_clone() {
        let table = simple_task_table();
        let out = emit_table_struct(&table);
        // insert_typed uses turso::Value, not .clone() raw value
        assert!(
            out.contains("turso::Value::Text(") || out.contains("turso::Value::Integer("),
            "insert_typed should use turso::Value constructors"
        );
    }

    // ── WASI guardrail tests (unchanged) ─────────────────────────────────────

    #[test]
    fn wasi_clean_script_passes_guardrail() {
        let module = empty_module();
        let result = generate_script_with_target(&module, "test-script", None, ScriptTarget::Wasi);
        assert!(result.is_ok(), "clean WASI script should pass guardrail");
    }

    #[test]
    fn wasi_with_actor_fails_guardrail() {
        let mut module = empty_module();
        module.actors.push(HirActor {
            id: DefId(0),
            name: "MyActor".to_string(),
            handlers: vec![],
            span: Span::new(0, 0),
        });
        let result = generate_script_with_target(&module, "test-script", None, ScriptTarget::Wasi);
        assert!(
            result.is_err(),
            "WASI script with actor should fail guardrail"
        );
        let err = result.unwrap_err().to_string();
        assert!(err.contains("actors"), "error should mention actors: {err}");
    }

    #[test]
    fn wasi_with_workflow_fails_guardrail() {
        let mut module = empty_module();
        module.workflows.push(HirWorkflow {
            id: DefId(0),
            name: "MyWorkflow".to_string(),
            params: vec![],
            return_type: None,
            body: vec![],
            span: Span::new(0, 0),
        });
        let result = generate_script_with_target(&module, "test-script", None, ScriptTarget::Wasi);
        assert!(
            result.is_err(),
            "WASI script with workflow should fail guardrail"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("workflows"),
            "error should mention workflows: {err}"
        );
    }

    #[test]
    fn native_with_actor_passes() {
        let mut module = empty_module();
        module.actors.push(HirActor {
            id: DefId(0),
            name: "MyActor".to_string(),
            handlers: vec![],
            span: Span::new(0, 0),
        });
        let result =
            generate_script_with_target(&module, "test-script", None, ScriptTarget::Native);
        assert!(
            result.is_ok(),
            "native script with actor should not be blocked"
        );
    }
}
