//! Guardrail tests for generated `main.rs` and `lib.rs` content.
use vox_ast::span::Span;
use vox_codegen_rust::emit::{emit_main, generate};
use vox_hir::*;
use vox_test_harness::spans::dummy_span;
use vox_test_harness::hir_builders::minimal_hir_module as minimal_module;


#[test]
fn a102_generated_main_uses_vox_port_env_var() {
    let mut module = minimal_module();
    module.routes.push(HirRoute {
        method: HirHttpMethod::Get,
        path: "/ping".to_string(),
        return_type: None,
        body: vec![],
        span: dummy_span(),
    });
    let main_rs = emit_main(&module, "test_app");

    assert!(
        main_rs.contains("VOX_PORT"),
        "Generated main.rs should read port from VOX_PORT env var, got:\n{}",
        &main_rs[..main_rs.len().min(3000)]
    );
    assert!(
        !main_rs.contains("SocketAddr::from(([127, 0, 0, 1], 3000))"),
        "Generated main.rs should NOT hardcode port 3000 in SocketAddr::from"
    );
    assert!(
        main_rs.contains("VOX_SSR_DEV_URL") && main_rs.contains("serve_dispatch"),
        "Generated main.rs should support optional SSR dev proxy via VOX_SSR_DEV_URL, got:\n{}",
        &main_rs[..main_rs.len().min(4000)]
    );
}

#[test]
fn a103_generated_main_uses_vox_db_path_env_var() {
    let mut module = minimal_module();
    module.tables.push(HirTable {
        id: DefId(1),
        name: "Task".to_string(),
        fields: vec![],
        is_pub: false,
        is_deprecated: false,
        span: dummy_span(),
    });
    module.routes.push(HirRoute {
        method: HirHttpMethod::Get,
        path: "/ping".to_string(),
        return_type: None,
        body: vec![],
        span: dummy_span(),
    });
    let main_rs = emit_main(&module, "test_app");

    assert!(
        main_rs.contains("VOX_DB_PATH"),
        "Generated main.rs should document VOX_DB_PATH in Codex resolve message, got:\n{}",
        &main_rs[..main_rs.len().min(3000)]
    );
    assert!(
        main_rs.contains("vox_db::DbConfig::resolve_standalone"),
        "Generated main.rs should resolve DB via vox_db::DbConfig::resolve_standalone"
    );
    assert!(
        main_rs.contains("vox_db::Codex::connect"),
        "Generated main.rs should open Codex with vox_db::Codex::connect"
    );
    assert!(
        !main_rs.contains("unwrap_or_else(|_| \"app.db\""),
        "Generated main.rs should NOT inline 'app.db' literal in db_path assignment"
    );
}

#[test]
fn a104_generated_main_uses_expect_not_unwrap_on_listener() {
    let mut module = minimal_module();
    module.routes.push(HirRoute {
        method: HirHttpMethod::Get,
        path: "/slow".to_string(),
        return_type: None,
        body: vec![],
        span: dummy_span(),
    });
    let main_rs = emit_main(&module, "test_app");

    assert!(
        !main_rs.contains("bind(addr).await.unwrap()"),
        "Generated code should not use .unwrap() on TcpListener::bind"
    );
    assert!(
        !main_rs.contains("serve(listener, app).await.unwrap()"),
        "Generated code should not use .unwrap() on axum::serve"
    );
    assert!(
        main_rs.contains(".expect("),
        "Generated code should use .expect() for error handling on listener/serve, got:\n{}",
        &main_rs[..main_rs.len().min(3000)]
    );
}

#[test]
fn codegen_emits_jsx_placeholder_in_function_body() {
    let mut module = minimal_module();
    module.functions.push(HirFn {
        id: DefId(0),
        name: "foo".to_string(),
        generics: vec![],
        params: vec![],
        return_type: None,
        body: vec![HirStmt::Expr {
            expr: HirExpr::JsxSelfClosing(HirJsxSelfClosing {
                tag: "div".to_string(),
                attributes: vec![],
                span: dummy_span(),
            }),
            span: dummy_span(),
        }],
        is_component: false,
        is_async: false,
        is_pub: false,
        is_deprecated: false,
        span: dummy_span(),
    });

    let output = generate(&module, "test").expect("generate");
    let lib_rs = output.files.get("src/lib.rs").expect("lib.rs");
    assert!(
        lib_rs.contains("panic!(\"JSX cannot be rendered via the Rust backend yet\")"),
        "expected JSX panic stub in emitted lib.rs, got: {lib_rs}"
    );
}

#[test]
fn table_insert_uses_map_err_not_unwrap() {
    let mut module = minimal_module();
    module.tables.push(HirTable {
        id: DefId(1),
        name: "Item".to_string(),
        fields: vec![HirTableField {
            name: "item".to_string(),
            type_ann: HirType::Named("serde_json::Value".to_string()),
            span: dummy_span(),
        }],
        is_pub: false,
        is_deprecated: false,
        span: dummy_span(),
    });

    let output = generate(&module, "test").unwrap();
    let lib_rs = output.files.get("src/lib.rs").expect("lib.rs");
    assert!(
        lib_rs.contains("map_err")
            && !lib_rs.contains("serde_json::to_string(&item.data).unwrap()"),
        "table insert should use map_err for JSON serialization, not unwrap"
    );
}

#[test]
fn codegen_maps_assert_to_assert_eq() {
    let mut module = minimal_module();

    module.tests.push(HirFn {
        id: DefId(100),
        name: "check_math".to_string(),
        generics: vec![],
        params: vec![],
        return_type: None,
        body: vec![HirStmt::Expr {
            expr: HirExpr::Call(
                Box::new(HirExpr::Ident("assert".to_string(), dummy_span())),
                vec![HirArg {
                    name: None,
                    value: HirExpr::Binary(
                        HirBinOp::Is,
                        Box::new(HirExpr::Binary(
                            HirBinOp::Add,
                            Box::new(HirExpr::IntLit(1, dummy_span())),
                            Box::new(HirExpr::IntLit(1, dummy_span())),
                            dummy_span(),
                        )),
                        Box::new(HirExpr::IntLit(2, dummy_span())),
                        dummy_span(),
                    ),
                }],
                false,
                dummy_span(),
            ),
            span: dummy_span(),
        }],
        is_component: false,
        is_async: false,
        is_pub: false,
        is_deprecated: false,
        span: dummy_span(),
    });

    let output = generate(&module, "test").unwrap();
    let lib_rs = output.files.get("src/lib.rs").expect("lib.rs");

    assert!(
        lib_rs.contains("#[test]\nfn check_math()"),
        "Should generate Rust test function, got: {}",
        lib_rs
    );
    assert!(
        lib_rs.contains("assert_eq!"),
        "Should map assert(1 + 1 == 2) to assert_eq!, got: {}",
        lib_rs
    );
    assert!(
        lib_rs.contains("2"),
        "assertion should reference literal 2, got: {}",
        lib_rs
    );
}
