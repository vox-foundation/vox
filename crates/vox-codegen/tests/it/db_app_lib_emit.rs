//! App-mode (`emit_lib`, used by the Tauri and Axum shells) db codegen.
//!
//! Reproduces the four errors the generated `apps/vox-mental-tracker` app hit once the
//! mobile E2E lanes reached `cargo tauri build` (2026-10-08):
//! - a plain helper fn using `db.*` referenced a `db` binding only handlers create;
//! - `match db.T.op() { Ok(..) => .., Error(..) => .. }` matched Ok/Err on a value the
//!   emitter had already unwrapped with `.expect`, though Vox types the op `Result[T, str]`;
//! - an object literal returned from a fn declared `to <Table>` was emitted as a bare
//!   `serde_json::json!` instead of being converted to the table struct.
//!
//! Fast: asserts on emitted strings (no crate compile).

use vox_codegen::codegen_rust::emit::emit_lib;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::lex;
use vox_compiler::parser::parse_script;
use vox_compiler::typeck::typecheck_hir_module;

const TABLE: &str = "table Note {\n    title: str\n    body: str\n}\n";

fn lib(src: &str) -> String {
    let full = format!("{TABLE}{src}");
    let module = parse_script(lex(&full)).expect("parse");
    let mut hir = lower_module(&module);
    let _ = typecheck_hir_module(&full, &mut hir);
    emit_lib(&hir)
}

/// The emitted text of one function (from its `fn name(` to the next top-level `fn`).
fn fn_body<'a>(lib: &'a str, name: &str) -> &'a str {
    let start = lib
        .find(&format!("fn {name}("))
        .unwrap_or_else(|| panic!("fn {name} not emitted:\n{lib}"));
    let rest = &lib[start..];
    let end = rest[1..].find("\nfn ").map(|i| i + 1).unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn helper_fn_reaches_the_app_db_through_the_global_handle() {
    let out = lib(
        "fn note_count() to int {\n    match db.Note.all() {\n        Ok(rows) => len(rows)\n        Error(_) => 0\n    }\n}\n",
    );
    assert!(
        out.contains("pub static VOX_APP_DB"),
        "a lib with tables must declare the app db handle; got:\n{out}"
    );
    let body = fn_body(&out, "note_count");
    assert!(
        body.contains("VOX_APP_DB.get()") && !body.contains("(&*db") && !body.contains("(&db"),
        "a helper fn has no `db` binding; it must use VOX_APP_DB; got:\n{body}"
    );
}

#[test]
fn match_on_a_db_result_keeps_the_result() {
    let out = lib(
        "fn note_count() to int {\n    match db.Note.all() {\n        Ok(rows) => len(rows)\n        Error(_) => 0\n    }\n}\n",
    );
    let body = fn_body(&out, "note_count");
    assert!(
        body.contains(".map_err(") && !body.contains(".expect(\"vox codegen: db all\")"),
        "`match db.T.all() {{ Ok.. Error.. }}` needs a Result scrutinee; got:\n{body}"
    );
}

#[test]
fn match_on_a_db_insert_keeps_the_result() {
    let out = lib(
        "fn add(t: str) to int {\n    match db.Note.insert({ title: t, body: \"\" }) {\n        Ok(_) => 1\n        Error(_) => 0\n    }\n}\n",
    );
    let body = fn_body(&out, "add");
    assert!(
        body.contains(".map_err(") && !body.contains(".expect(\"vox codegen: db insert\")"),
        "`match db.T.insert(..) {{ Ok.. Error.. }}` needs a Result scrutinee; got:\n{body}"
    );
}

#[test]
fn unwrapping_a_db_result_still_yields_the_value() {
    // The existing value-style uses (`?`, `.unwrap()`, plain binding) must not change.
    let out = lib(
        "fn first_title() to int {\n    let rows = db.Note.all().unwrap()\n    return len(rows)\n}\n",
    );
    let body = fn_body(&out, "first_title");
    assert!(
        !body.contains(".map_err("),
        "a non-matched db op keeps the unwrapped value form; got:\n{body}"
    );
}

#[test]
fn object_literal_returned_as_a_table_is_converted() {
    let out = lib("fn mk(t: str) to Note {\n    return { title: t, body: \"x\" }\n}\n");
    let body = fn_body(&out, "mk");
    assert!(
        body.contains("serde_json::from_value"),
        "an object literal returned as a table struct must be converted; got:\n{body}"
    );
}

#[test]
fn rows_from_a_db_op_are_table_structs() {
    // Typeck typed rows as an anonymous record, so field access emitted JSON indexing
    // (`r["title"]`) on what codegen produces as a `Note` struct (E0608).
    let out = lib(
        "fn titles() to int {\n    let n = 0\n    match db.Note.all() {\n        Ok(rows) => {\n            for r in rows {\n                if r.title == \"x\" { n = n + 1 }\n            }\n        }\n        Error(_) => {}\n    }\n    return n\n}\n",
    );
    let body = fn_body(&out, "titles");
    assert!(
        body.contains(".title") && !body.contains("[\"title\"]"),
        "row field access must be struct access; got:\n{body}"
    );
}
