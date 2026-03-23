//! Codegen tests for ADTs and table DDL against current `vox_hir::HirModule`.
use vox_ast::span::Span;
use vox_codegen_rust::emit::{emit_lib, emit_main, emit_table_ddl};
use vox_hir::*;
use vox_test_harness::spans::dummy_span;
use vox_test_harness::hir_builders::minimal_hir_module as minimal_module;


#[test]
fn table_ddl_maps_unknown_vectorish_type_to_text_column() {
    let table = HirTable {
        id: DefId(1),
        name: "Document".to_string(),
        fields: vec![
            HirTableField {
                name: "text".to_string(),
                type_ann: HirType::Named("str".to_string()),
                span: dummy_span(),
            },
            HirTableField {
                name: "embedding".to_string(),
                type_ann: HirType::Named("vec384".to_string()),
                span: dummy_span(),
            },
        ],
        is_pub: false,
        is_deprecated: false,
        span: dummy_span(),
    };

    let ddl = emit_table_ddl(&table);
    assert!(
        ddl.contains("embedding TEXT"),
        "vec384 should fall back to TEXT in SQLite DDL, got:\n{ddl}"
    );
}

#[test]
fn main_with_table_and_route_emits_db_setup_and_codex_connect() {
    let mut module = minimal_module();
    module.tables.push(HirTable {
        id: DefId(1),
        name: "Document".to_string(),
        fields: vec![HirTableField {
            name: "text".to_string(),
            type_ann: HirType::Named("str".to_string()),
            span: dummy_span(),
        }],
        is_pub: false,
        is_deprecated: false,
        span: dummy_span(),
    });
    module.routes.push(HirRoute {
        method: HirHttpMethod::Post,
        path: "/upload".to_string(),
        return_type: None,
        body: vec![],
        span: dummy_span(),
    });

    let main_rs = emit_main(&module, "test_app");
    assert!(
        main_rs.contains("VOX_DB_PATH"),
        "main should honor VOX_DB_PATH, got:\n{}",
        &main_rs[..main_rs.len().min(2500)]
    );
    assert!(
        main_rs.contains("vox_db::Codex::connect"),
        "main should connect via vox_db Codex (Turso-backed), got:\n{}",
        &main_rs[..main_rs.len().min(2500)]
    );
}

#[test]
fn adt_generates_tuple_style_rust_enum_variants() {
    let mut module = minimal_module();

    module.types.push(HirTypeDef {
        id: DefId(1),
        name: "Shape".to_string(),
        variants: vec![
            HirVariant {
                name: "Circle".to_string(),
                fields: vec![("radius".to_string(), HirType::Named("f64".to_string()))],
                span: dummy_span(),
            },
            HirVariant {
                name: "Rect".to_string(),
                fields: vec![
                    ("width".to_string(), HirType::Named("f64".to_string())),
                    ("height".to_string(), HirType::Named("f64".to_string())),
                ],
                span: dummy_span(),
            },
            HirVariant {
                name: "Point".to_string(),
                fields: vec![],
                span: dummy_span(),
            },
        ],
        is_pub: true,
        span: dummy_span(),
    });

    let lib_rs = emit_lib(&module);

    assert!(
        lib_rs.contains("pub enum Shape {"),
        "Should generate pub enum Shape, got:\n{}",
        &lib_rs[..lib_rs.len().min(2000)]
    );
    assert!(
        lib_rs.contains("Circle("),
        "Should emit tuple-style Circle variant, got:\n{}",
        &lib_rs[..lib_rs.len().min(2000)]
    );
    assert!(
        lib_rs.contains("Rect("),
        "Should emit tuple-style Rect variant, got:\n{}",
        &lib_rs[..lib_rs.len().min(2000)]
    );
    assert!(
        lib_rs.contains("Point,"),
        "Should have Point as unit variant"
    );
    assert!(
        lib_rs.contains("#[derive(Debug, Clone, Serialize, Deserialize)]"),
        "Should have serde derive macros"
    );
}
