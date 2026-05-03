//! Express route emission ordering and validation (OP-0166, OP-0170, OP-0171).

use vox_compiler::ast::span::Span;
use vox_compiler::codegen_ts::routes::{
    ExpressRouteEmitCtx, generate_routes, validate_express_route_emit_input,
};
use vox_compiler::hir::{HirHttpMethod, HirModule, HirRoute};
use vox_compiler::web_ir::lower::lower_hir_to_web_ir;

#[test]
fn validate_rejects_duplicate_http_routes_same_method_path() {
    // Two identical literal paths are reported as ambiguous (Phase C upgraded the
    // exact-match HashSet check to segment-aware overlap detection; identical
    // literals collapse to `Overlap::Ambiguous`).
    let sp = Span::new(0, 0);
    let mut m = HirModule::default();
    let r = HirRoute {
        method: HirHttpMethod::Post,
        path: "/dup".into(),
        route_contract: "POST /dup".into(),
        return_type: None,
        body: vec![],
        span: sp,
    };
    m.routes.push(r.clone());
    m.routes.push(r);
    let err = validate_express_route_emit_input(&m).unwrap_err();
    assert!(err.contains("ambiguous"), "{err}");
}

fn route(method: HirHttpMethod, path: &str) -> HirRoute {
    HirRoute {
        method,
        path: path.into(),
        route_contract: format!("{:?} {}", method, path),
        return_type: None,
        body: vec![],
        span: Span::new(0, 0),
    }
}

#[test]
fn validate_rejects_two_param_routes_with_same_method_and_arity() {
    // /:a/:b vs /:x/:y on the same method has no specificity tiebreaker → error.
    let mut m = HirModule::default();
    m.routes.push(route(HirHttpMethod::Get, "/:a/:b"));
    m.routes.push(route(HirHttpMethod::Get, "/:x/:y"));
    let err = validate_express_route_emit_input(&m).unwrap_err();
    assert!(err.contains("ambiguous"), "{err}");
}

#[test]
fn validate_allows_literal_shadowing_param_route() {
    // /users/me is more specific than /users/:id; allowed (precedence by source order).
    let mut m = HirModule::default();
    m.routes.push(route(HirHttpMethod::Get, "/users/me"));
    m.routes.push(route(HirHttpMethod::Get, "/users/:id"));
    assert!(validate_express_route_emit_input(&m).is_ok());
}

#[test]
fn validate_allows_overlapping_routes_on_different_methods() {
    // GET /users/:id and POST /users/:id are not in conflict.
    let mut m = HirModule::default();
    m.routes.push(route(HirHttpMethod::Get, "/users/:id"));
    m.routes.push(route(HirHttpMethod::Post, "/users/:id"));
    assert!(validate_express_route_emit_input(&m).is_ok());
}

#[test]
fn validate_allows_routes_with_disjoint_literal_prefixes() {
    let mut m = HirModule::default();
    m.routes.push(route(HirHttpMethod::Get, "/users/:id"));
    m.routes.push(route(HirHttpMethod::Get, "/posts/:id"));
    assert!(validate_express_route_emit_input(&m).is_ok());
}

#[test]
fn generate_routes_orders_http_paths_lexically() {
    let sp = Span::new(0, 0);
    let mut m = HirModule::default();
    m.routes.push(HirRoute {
        method: HirHttpMethod::Get,
        path: "/zebra".into(),
        route_contract: "GET /zebra".into(),
        return_type: None,
        body: vec![],
        span: sp,
    });
    m.routes.push(HirRoute {
        method: HirHttpMethod::Get,
        path: "/alpha".into(),
        route_contract: "GET /alpha".into(),
        return_type: None,
        body: vec![],
        span: sp,
    });
    let ts = generate_routes(&m);
    assert!(
        ts.contains("class ClaudeActor"),
        "Express boilerplate should include mock actor, got:\n{ts}"
    );
    let a = ts.find("app.get(\"/alpha\"").expect("alpha");
    let z = ts.find("app.get(\"/zebra\"").expect("zebra");
    assert!(a < z, "expected /alpha before /zebra\n{ts}");
}

#[test]
fn express_route_emit_ctx_validates() {
    let m = HirModule::default();
    assert!(ExpressRouteEmitCtx::new(&m).validate().is_ok());
}

#[test]
fn hir_http_route_lowering_populates_web_ir_route_nodes() {
    let sp = Span::new(0, 0);
    let mut m = HirModule::default();
    m.routes.push(HirRoute {
        method: HirHttpMethod::Get,
        path: "/z".into(),
        route_contract: "GET /z".into(),
        return_type: None,
        body: vec![],
        span: sp,
    });
    let web = lower_hir_to_web_ir(&m);
    assert!(
        !web.route_nodes.is_empty(),
        "expected at least one Web IR route envelope for HTTP routes"
    );
}
