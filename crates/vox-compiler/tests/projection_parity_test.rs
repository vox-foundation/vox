//! Projection parity gate: WebIR + AppContract + RuntimeProjection from one fixture.

use vox_compiler::app_contract::{canonical_app_contract_bytes, project_app_contract};
use vox_compiler::hir::lower_module;
use vox_compiler::parser::parse;
use vox_compiler::runtime_projection::{
    canonical_runtime_projection_bytes, project_runtime_from_hir,
};
use vox_compiler::syntax_k::{canonical_web_ir_bytes, sha3_hex};
use vox_compiler::web_ir::lower::lower_hir_to_web_ir;

fn lower_src(src: &str) -> vox_compiler::hir::TypedCoreIR_v2 {
    let tokens = vox_compiler::lexer::lex(src);
    let module = parse(tokens).expect("parse");
    lower_module(&module)
}

#[test]
fn projection_triplet_is_deterministic_and_schema_versioned() {
    let src = r#"
@table type Task { title: str done: bool }

component Home() {
    state n: int = 0
    derived rows = db.Task.filter({ done: false }).select("title", "done")
    view: <div>{n}</div>
}

routes {
    "/" to Home
}

http get "/api/ping" to int { ret 1 }
@server fn sf_ping() to int { ret 1 }
@query fn list_tasks() to int { ret 0 }
@mutation fn save_task(title: str) to int {
    db.Task.insert({ title: title, done: false })
    ret 1
}
"#;
    let hir = lower_src(src);

    let web = lower_hir_to_web_ir(&hir);
    let app = project_app_contract(&hir);
    let runtime = project_runtime_from_hir(&hir);

    let web_bytes_a = canonical_web_ir_bytes(&web).expect("web bytes");
    let app_bytes_a = canonical_app_contract_bytes(&app).expect("app bytes");
    let rt_bytes_a = canonical_runtime_projection_bytes(&runtime).expect("runtime bytes");

    let web_bytes_b = canonical_web_ir_bytes(&web).expect("web bytes");
    let app_bytes_b = canonical_app_contract_bytes(&app).expect("app bytes");
    let rt_bytes_b = canonical_runtime_projection_bytes(&runtime).expect("runtime bytes");

    assert_eq!(
        web_bytes_a, web_bytes_b,
        "webir canonical bytes must be stable"
    );
    assert_eq!(
        app_bytes_a, app_bytes_b,
        "app contract canonical bytes must be stable"
    );
    assert_eq!(
        rt_bytes_a, rt_bytes_b,
        "runtime projection canonical bytes must be stable"
    );

    assert_eq!(app.schema_version, 2);
    assert_eq!(runtime.schema_version, 1);
    assert!(!app.http_routes.is_empty(), "expected HTTP route contract");
    assert!(!app.server_fns.is_empty(), "expected @server contract");
    assert!(!app.query_fns.is_empty(), "expected @query contract");
    assert!(!app.mutation_fns.is_empty(), "expected @mutation contract");

    // Ensure we can monitor drift of each projection independently in CI logs.
    let web_hash = sha3_hex(&web_bytes_a);
    let app_hash = sha3_hex(&app_bytes_a);
    let runtime_hash = sha3_hex(&rt_bytes_a);
    assert_ne!(web_hash, app_hash);
    assert_ne!(app_hash, runtime_hash);
}
