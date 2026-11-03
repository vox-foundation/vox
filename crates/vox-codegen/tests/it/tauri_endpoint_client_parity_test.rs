//! VUV / full-stack seam: `@query`/`@mutation`/`@server` lower to Tauri Rust commands and to `vox-client.ts`
//! invoke transport (Contract IR), while `CodegenOutput::api_client_ts` stays empty.
//!
//! Dashboard and `native-binary` remain Axum per ADR 024 / ADR 037; this test pins the
//! **generated app** desktop/mobile shell only.

use vox_codegen::codegen_rust::{RustAppShell, generate};
use vox_codegen::codegen_ts::vox_client::emit_vox_client;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;

const ENDPOINT_VOX: &str = r#"query get_count() to int { return 0 }"#;

#[test]
fn tauri_rust_commands_match_vox_client_invoke() {
    let module = parse(lex(ENDPOINT_VOX)).expect("parse");
    let hir = lower_module(&module);

    let rust = generate(&hir, "demo_app", RustAppShell::TauriApp).expect("rust generate");
    assert!(
        rust.api_client_ts.is_empty(),
        "api_client_ts must stay empty; vox-client.ts is the SSOT"
    );

    let main = rust
        .files
        .get("src-tauri/src/main.rs")
        .expect("Tauri main.rs");
    assert!(
        main.contains("#[tauri::command]"),
        "expected #[tauri::command] in generated main.rs"
    );
    assert!(
        main.contains("async fn get_count("),
        "expected get_count command in generated main.rs"
    );

    let client = emit_vox_client(&hir);
    assert!(
        client.contains("isTauri()"),
        "vox-client must branch for Tauri webview"
    );
    assert!(
        client.contains("$tauri") && client.contains("\"get_count\""),
        "vox-client must invoke the same command name as Rust emit"
    );
}

#[test]
fn web_ir_lowers_endpoint_only_module() {
    let module = parse(lex(ENDPOINT_VOX)).expect("parse");
    let hir = lower_module(&module);
    let web = vox_codegen::web_ir::lower::lower_hir_to_web_ir(&hir);
    assert!(
        web.view_roots.is_empty(),
        "endpoint-only fixture should not introduce WebIR view roots"
    );
}

/// mental-tracker (2026-10-08): a command body that called a private helper or another
/// endpoint did not compile in main.rs, and an async command returning a bare value is
/// rejected by Tauri. Endpoints now live in lib.rs; commands are typed wrappers.
#[test]
fn tauri_commands_wrap_endpoint_lib_fns() {
    let src = "fn _double(n: int) to int { return n * 2 }\n\
               query twice(n: int) to int { return _double(n) }\n\
               query four(n: int) to int { return twice(twice(n)) }\n";
    let hir = lower_module(&parse(lex(src)).expect("parse"));
    let rust = generate(&hir, "demo_app", RustAppShell::TauriApp).expect("rust generate");
    let main = &rust.files["src-tauri/src/main.rs"];
    let lib = &rust.files["src-tauri/src/lib.rs"];
    assert!(
        main.contains("async fn twice(request: serde_json::Value) -> Result<i64, String>"),
        "{main}"
    );
    assert!(
        main.contains("let n: i64 = serde_json::from_value(request[\"n\"].clone())"),
        "{main}"
    );
    assert!(main.contains("Ok(demo_app::twice(n))"), "{main}");
    assert!(
        !main.contains("_double"),
        "helper calls stay in lib.rs:\n{main}"
    );
    assert!(
        lib.contains("pub fn twice(") && lib.contains("pub fn four("),
        "{lib}"
    );
}
