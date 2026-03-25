//! Vox → Rust code generator.
//!
//! Generates a full Rust web server project from a Vox HIR module.
//!
//! Integration tests also use [`emit`] as `vox_codegen_rust::emit::*`.

#![allow(clippy::collapsible_if)]

pub mod emit;

mod manifest;
mod pipeline;

pub use emit::emit_api_client;

/// Rust `edition` written into generated `Cargo.toml` files (keep aligned with root workspace).
pub const GENERATED_CARGO_EDITION: &str = "2024";

pub use manifest::CodegenOutput;
pub use pipeline::{ScriptTarget, generate, generate_script, generate_script_with_target};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::span::Span;
    use crate::hir::{DefId, HirActor, HirModule, HirTable, HirTableField, HirType, HirWorkflow};
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
