//! Script mode must wire vox-db when a module declares @table types.

use std::path::PathBuf;

use vox_codegen::codegen_rust::generate_script;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse_script;
use vox_compiler::typeck::typecheck_hir_module;

fn runtime_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vox-actor-runtime")
}

fn generate_option_type_script() -> vox_codegen::codegen_rust::CodegenOutput {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/golden/option_type.vox");
    let src = std::fs::read_to_string(&path).expect("read option_type.vox");
    let module = parse_script(lex(&src)).expect("parse");
    let mut hir = lower_module(&module);
    let _ = typecheck_hir_module(&src, &mut hir);
    generate_script(&hir, "vox-script", Some(&runtime_path())).expect("generate_script")
}

#[test]
fn script_with_workflow_enables_sql_and_vox_db() {
    let src = r#"
activity charge(amount: int) to int { return amount }
workflow checkout(amount: int) to int { return charge(amount) }
"#;
    let module = parse_script(lex(src)).expect("parse");
    let hir = lower_module(&module);
    let out = generate_script(&hir, "vox-script", Some(&runtime_path())).expect("generate_script");
    let cargo = out.files.get("Cargo.toml").expect("Cargo.toml present");
    assert!(
        cargo.contains("vox-workflow-runtime")
            && cargo.contains("features")
            && cargo.contains("sql"),
        "workflow scripts must enable vox-workflow-runtime sql; got:\n{cargo}"
    );
    assert!(
        cargo.contains("vox-db") && cargo.contains("path ="),
        "workflow scripts must depend on vox-db for VoxDbTracker; got:\n{cargo}"
    );
}

#[test]
fn script_without_tables_or_workflow_omits_vox_db() {
    let src = "fn add(a: int, b: int) to int { return a + b }";
    let module = parse_script(lex(src)).expect("parse");
    let hir = lower_module(&module);
    let out = generate_script(&hir, "vox-script", Some(&runtime_path())).expect("generate_script");
    let cargo = out.files.get("Cargo.toml").expect("Cargo.toml present");
    assert!(
        !cargo.contains("vox-db"),
        "plain scripts must stay db-free; got:\n{cargo}"
    );
    assert!(
        !cargo.contains("vox-workflow-runtime"),
        "plain scripts must not pull workflow-runtime; got:\n{cargo}"
    );
}

#[test]
fn script_with_tables_includes_vox_db_in_cargo_toml() {
    let out = generate_option_type_script();
    let cargo = out.files.get("Cargo.toml").expect("Cargo.toml present");
    assert!(
        cargo.contains("vox-db") && cargo.contains("path ="),
        "table-bearing scripts must depend on vox-db; got:\n{cargo}"
    );
}

#[test]
fn script_with_tables_emits_db_handle_and_boot() {
    let out = generate_option_type_script();
    let lib_rs = out.files.get("src/lib.rs").expect("lib.rs present");
    assert!(
        lib_rs.contains("VOX_SCRIPT_DB"),
        "script lib must expose VOX_SCRIPT_DB OnceLock; got:\n{lib_rs}"
    );
    assert!(
        lib_rs.contains("vox_script_boot_db"),
        "script lib must expose async db boot; got:\n{lib_rs}"
    );
    assert!(
        lib_rs.contains("find_user"),
        "@query endpoints must be emitted into script lib; got:\n{lib_rs}"
    );
}

#[test]
fn script_with_tables_awaits_async_endpoint_calls() {
    let out = generate_option_type_script();
    let lib = out.files.get("src/lib.rs").expect("lib.rs present");
    assert!(
        lib.contains("find_user(") && lib.contains(".await"),
        "async @query calls must lower with .await; sample:\n{}",
        &lib[..lib.len().min(4000)]
    );
    assert!(
        lib.contains("async fn greet_user"),
        "transitive async callers must be async fn; sample:\n{}",
        &lib[..lib.len().min(4000)]
    );
}
