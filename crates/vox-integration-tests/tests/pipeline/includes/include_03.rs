// mcp_tool_demo.vox
const MCP_TOOL_SRC: &str = r#"type SearchResult =
    | Found(text: str, score: int)
    | NotFound(query: str)

@mcp.tool "Search the knowledge base" fn search_knowledge(query: str) to SearchResult {
    Found("Result for: " + query, 95)
}

@mcp.tool "Get system status" fn system_status() to str {
    ret "healthy"
}
"#;

#[test]
fn pipeline_mcp_tool_parse() {
    let tokens = lex(MCP_TOOL_SRC);
    let module = parse(tokens).expect("mcp_tool_demo should parse");
    // 1 type + 2 mcp.tool
    assert_eq!(module.declarations.len(), 3);
    assert!(
        matches!(&module.declarations[1], vox_compiler::ast::decl::Decl::McpTool(m) if m.description == "Search the knowledge base"),
        "First tool should have correct description"
    );
}

// pattern_matching.vox
const PATTERN_MATCHING_SRC: &str = r#"type Shape =
    | Circle(radius: int)
    | Rectangle(width: int, height: int)
    | Triangle(base: int, height: int)

fn area(s: Shape) to int {
    match s {
        Circle(radius) -> radius * radius
        Rectangle(width, height) -> width * height
        Triangle(base, height) -> base * height
    }
}

@test fn test_circle_area() to Unit {
    let c = Circle(5)
    let a = area(c)
    assert(a is 25)
}

@test fn test_rectangle_area() to Unit {
    let r = Rectangle(4, 6)
    let a = area(r)
    assert(a is 24)
}
"#;

#[test]
fn pipeline_pattern_matching_parse() {
    let tokens = lex(PATTERN_MATCHING_SRC);
    let module = parse(tokens).expect("pattern_matching should parse");
    // 1 type + 1 fn + 2 tests
    assert_eq!(module.declarations.len(), 4);
}

#[test]
fn pipeline_pattern_matching_codegen() {
    let tokens = lex(PATTERN_MATCHING_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let types = output.files.iter().find(|(n, _)| n == "types.ts").unwrap();
    assert!(
        types.1.contains("export type Shape"),
        "Should export Shape type"
    );
    assert!(types.1.contains("_tag: \"Circle\""), "Should have Circle");
    assert!(
        types.1.contains("_tag: \"Rectangle\""),
        "Should have Rectangle"
    );
    assert!(
        types.1.contains("_tag: \"Triangle\""),
        "Should have Triangle"
    );
}

// multi_route_app.vox
const MULTI_ROUTE_SRC: &str = r#"import react.use_state

type Todo =
    | Active(text: str)
    | Completed(text: str)

@component fn Dashboard() to Element {
    let (visits, set_visits) = use_state(0)
    <div class="dashboard">
        <h1>"Dashboard"</h1>
        <button on_click={fn(_e) set_visits(visits + 1)}>"Track"</button>
    </div>
}

@component fn TodoList() to Element {
    let (items, set_items) = use_state([])
    let (draft, set_draft) = use_state("")
    <div class="todo_list">
        <input bind={draft} placeholder="New task..." />
        <button on_click={fn(_e) set_items(items.append({text: draft}))}>"Add"</button>
    </div>
}

@component fn About() to Element {
    <div class="about">
        <h1>"About"</h1>
    </div>
}

routes {
    "/" to Dashboard
    "/todos" to TodoList
    "/about" to About
}

http get "/api/todos" to str {
    ret "[]"
}

http post "/api/todos" to str {
    ret "created"
}

@server fn get_stats() to int {
    ret 42
}
"#;

#[test]
fn pipeline_multi_route_parse() {
    let tokens = lex(MULTI_ROUTE_SRC);
    let module = parse(tokens).expect("multi_route_app should parse");
    // 1 import + 1 type + 3 components + 1 routes + 2 http + 1 server fn = 9
    assert_eq!(module.declarations.len(), 9);
}

#[test]
fn pipeline_multi_route_codegen() {
    let tokens = lex(MULTI_ROUTE_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        filenames.contains(&"Dashboard.tsx"),
        "Should produce Dashboard.tsx"
    );
    assert!(
        filenames.contains(&"TodoList.tsx"),
        "Should produce TodoList.tsx"
    );
    assert!(filenames.contains(&"About.tsx"), "Should produce About.tsx");
    assert!(
        filenames.contains(&"App.tsx"),
        "Should produce App.tsx for routes:"
    );
    assert!(
        filenames.contains(&"types.ts"),
        "Should produce types.ts for Todo"
    );
}

#[test]
fn pipeline_multi_route_rust_codegen() {
    let tokens = lex(MULTI_ROUTE_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_rust::generate(&hir, "multi_route_app").unwrap();
    let main_rs = output.files.get("src/main.rs").expect("main.rs");
    // Axum uses .route("/path", get(handler)) syntax
    assert!(
        main_rs.contains("\"/api/todos\""),
        "Should have /api/todos route in main.rs, got:\n{}",
        &main_rs[..main_rs.len().min(2000)]
    );
    assert!(
        main_rs.contains("handle_sf_get_stats"),
        "Should have server fn handler for get_stats"
    );
}

/// HTTP `routes` surface plus classic `@component` JSX tails → Web IR summary (OP-0181).
#[test]
fn pipeline_web_ir_lower_summary_counts_http_and_classic() {
    use vox_compiler::web_ir::lower::lower_hir_to_web_ir_with_summary;

    let tokens = lex(MULTI_ROUTE_SRC);
    let module = parse(tokens).expect("multi_route");
    let hir = vox_compiler::hir::lower_module(&module);
    let (_web, summary) = lower_hir_to_web_ir_with_summary(&hir);
    assert!(
        summary.http_loader_contracts >= 1,
        "expected HTTP loader contracts, got {summary:?}"
    );
    assert!(
        summary.classic_component_views_lowered >= 1,
        "expected classic @component views lowered to view_roots, got {summary:?}"
    );
    assert_eq!(
        summary.classic_components_deferred, 0,
        "fixture classic components should fully lower, got {summary:?}"
    );
}

/// Chatbot classic `@component fn Chat` produces a `view_roots` entry and passes `validate_web_ir`.
#[test]
fn pipeline_chat_classic_web_ir_validate_clean() {
    use vox_compiler::web_ir::lower::lower_hir_to_web_ir_with_summary;
    use vox_compiler::web_ir::validate::validate_web_ir;

    let tokens = lex(CHATBOT_SRC);
    let module = parse(tokens).expect("chatbot parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let (web, summary) = lower_hir_to_web_ir_with_summary(&hir);
    assert!(
        summary.classic_component_views_lowered >= 1,
        "Chat should lower into Web IR view_roots, got {summary:?}"
    );
    assert!(
        web.view_roots.iter().any(|(n, _)| n == "Chat"),
        "expected Chat in view_roots, have {:?}",
        web.view_roots.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
    let diags = validate_web_ir(&web);
    assert!(
        diags.is_empty(),
        "validate_web_ir expected clean: {diags:?}"
    );
}

/// OP-S032: integration gate — AST [`vox_compiler::codegen_ts::jsx`] and HIR [`emit_hir`] share [`compat`] DOM edges.
#[test]
fn pipeline_compat_tag_gate_jsx_hir_emit_matrix() {
    let edges = [
        ("for", "htmlFor"),
        ("tab_index", "tabIndex"),
        ("class", "className"),
    ];
    for (vox, react) in edges {
        assert_eq!(
            vox_compiler::codegen_ts::hir_emit::map_jsx_attr_name(vox),
            react
        );
        assert_eq!(vox_compiler::codegen_ts::jsx::map_jsx_attr_name(vox), react);
    }
}

/// OP-S034: Express [`validate_express_route_emit_input`] accepts multi-route HIR (`routes.rs` OP-S033 mapper notes).
#[test]
fn pipeline_express_contract_mapper_fixture_validates_multi_route_hir() {
    let tokens = lex(MULTI_ROUTE_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    vox_compiler::codegen_ts::routes::validate_express_route_emit_input(&hir)
        .expect("MULTI_ROUTE_SRC express validation");
    assert!(
        hir.routes
            .iter()
            .any(|r| r.route_contract.starts_with("GET ") || r.route_contract.starts_with("POST ")),
        "expected HTTP route_contract on hir.routes: {:?}",
        hir.routes.iter().map(|r| &r.route_contract).collect::<Vec<_>>()
    );
}

/// OP-S036: route + component gate — Express validation, Web IR validate clean, `App.tsx` present.
#[test]
fn pipeline_route_component_express_and_web_ir_gate() {
    use vox_compiler::web_ir::validate::validate_web_ir;

    let tokens = lex(MULTI_ROUTE_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    vox_compiler::codegen_ts::routes::validate_express_route_emit_input(&hir).expect("express");
    let (web, summary) =
        vox_compiler::web_ir::lower::lower_hir_to_web_ir_with_summary(&hir);
    assert!(
        summary.client_route_trees >= 1,
        "expected client route trees, got {summary:?}"
    );
    let diags = validate_web_ir(&web);
    assert!(diags.is_empty(), "{diags:?}");
    let out = generate_without_express!(&module);
    assert!(
        out.files.iter().any(|(n, _)| n == "App.tsx"),
        "expected App.tsx in {:?}",
        out.files.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
}

/// OP-S045 / OP-S047 parity chain: routable `@component` + island (same source as `reactive_smoke` / `web_ir_lower_emit`).
const OP_S_PARITY_CHAIN_FIXTURE: &str = r#"
import react.use_state

@island ParityP { label: str }

@component ParityPage() {
    state s: str = "x"
    view: (
        <div class="parity-wrap">
            <ParityP label={s} />
        </div>
    )
}

routes {
    "/" to ParityPage
}
"#;

/// OP-S047: extra parity fixture C — full pipeline emits V1 island mount on classic routed page.
#[test]
fn op_s047_extra_parity_fixture_pipeline_emits_island_mount() {
    let tokens = lex(OP_S_PARITY_CHAIN_FIXTURE);
    let module = parse(tokens).expect("parity parse");
    let output = generate_without_express!(&module);
    let ts = output
        .files
        .iter()
        .find(|(f, _)| f == "ParityPage.tsx")
        .map(|(_, c)| c.as_str())
        .expect("ParityPage.tsx");
    assert!(
        ts.contains("data-vox-island=\"ParityP\""),
        "expected V1 island attr:\n{ts}"
    );
    assert!(ts.contains("data-prop-label="), "expected prop:\n{ts}");
}
