use super::*;
use crate::ast::decl::{ImportPathKind, ReactiveMemberDecl, RoutesParseSummary};
use crate::ast::expr::{BinOp, Expr};
use crate::ast::stmt::Stmt;
use crate::lexer::cursor::lex;
use crate::parser::ParseErrorClass;
use std::sync::Mutex;

/// Serialize tests that toggle `VOX_ALLOW_LEGACY_COMPONENT_FN` (process-global).
static LEGACY_COMPONENT_FN_ENV_LOCK: Mutex<()> = Mutex::new(());

fn parse_str(source: &str) -> Module {
    let tokens = lex(source);
    parse(tokens).unwrap_or_else(|e| panic!("Parse errors: {e:?}"))
}

fn assert_parse_fails(source: &str) {
    let tokens = lex(source);
    assert!(
        parse(tokens).is_err(),
        "expected parse to fail for source: {source:?}"
    );
}

#[test]
fn test_parse_simple_fn() {
    let m = parse_str("fn add(a, b) to int { ret a + b }");
    assert_eq!(m.declarations.len(), 1);
    assert!(matches!(&m.declarations[0], Decl::Function(f) if f.name == "add"));
}

#[test]
fn test_parse_import() {
    let m = parse_str("import react.use_state, network.HTTP");
    assert!(matches!(&m.declarations[0], Decl::Import(i) if i.paths.len() == 2));
}

#[test]
fn test_parse_rust_import() {
    let m = parse_str("import rust:serde_json");
    match &m.declarations[0] {
        Decl::Import(i) => {
            assert_eq!(i.paths.len(), 1);
            assert!(matches!(i.paths[0].kind, ImportPathKind::RustCrate(_)));
        }
        other => panic!("Expected import decl, got {other:?}"),
    }
}

#[test]
fn test_parse_rust_import_with_alias_and_meta() {
    let m = parse_str(
        "import rust:serde_json(version: \"1\", git: \"https://example.invalid/repo\", rev: \"main\") as json",
    );
    match &m.declarations[0] {
        Decl::Import(i) => {
            let p = &i.paths[0];
            assert_eq!(p.alias.as_deref(), Some("json"));
            match &p.kind {
                ImportPathKind::RustCrate(spec) => {
                    assert_eq!(spec.crate_name, "serde_json");
                    assert_eq!(spec.version.as_deref(), Some("1"));
                    assert_eq!(spec.git.as_deref(), Some("https://example.invalid/repo"));
                    assert_eq!(spec.rev.as_deref(), Some("main"));
                }
                _ => panic!("Expected rust crate import"),
            }
        }
        other => panic!("Expected import decl, got {other:?}"),
    }
}

#[test]
fn test_parse_let() {
    let m = parse_str("fn main() { let x = 42\n ret x }");
    if let Decl::Function(f) = &m.declarations[0] {
        assert_eq!(f.body.len(), 2);
        assert!(matches!(&f.body[0], Stmt::Let { .. }));
    } else {
        panic!("Expected function");
    }
}

#[test]
fn classic_component_fn_is_parse_error_without_escape_hatch() {
    let _g = LEGACY_COMPONENT_FN_ENV_LOCK
        .lock()
        .expect("LEGACY_COMPONENT_FN_ENV_LOCK poisoned");
    unsafe {
        std::env::remove_var("VOX_ALLOW_LEGACY_COMPONENT_FN");
    }
    assert_parse_fails("@component fn Chat() to Element { ret 0 }");
}

#[test]
fn classic_component_fn_parses_when_escape_hatch_enabled() {
    let _g = LEGACY_COMPONENT_FN_ENV_LOCK
        .lock()
        .expect("LEGACY_COMPONENT_FN_ENV_LOCK poisoned");
    unsafe {
        std::env::set_var("VOX_ALLOW_LEGACY_COMPONENT_FN", "1");
    }
    let m = parse_str("@component fn Chat() to Element { ret 0 }");
    assert!(matches!(&m.declarations[0], Decl::Component(_)));
    unsafe {
        std::env::remove_var("VOX_ALLOW_LEGACY_COMPONENT_FN");
    }
}

#[test]
fn test_parse_loading_decl() {
    let m = parse_str("@loading fn RouteSpinner() to Element { ret <div/> }");
    assert!(matches!(
        &m.declarations[0],
        Decl::Loading(l) if l.func.name == "RouteSpinner"
    ));
}

#[test]
fn test_parse_at_component_reactive_path_c() {
    let m =
        parse_str("@component Widget(x: int) {\n  state n: int = x\n  view: <span>{n}</span>\n}");
    if let Decl::ReactiveComponent(r) = &m.declarations[0] {
        assert_eq!(r.name, "Widget");
        assert_eq!(r.params.len(), 1);
        assert_eq!(r.members.len(), 1);
        assert!(r.view.is_some());
    } else {
        panic!("Expected Decl::ReactiveComponent for @component Path C form");
    }
}

#[test]
fn test_parse_http_route() {
    let m = parse_str("http post \"/api/chat\" to Result { ret 0 }");
    assert!(matches!(&m.declarations[0], Decl::HttpRoute(r) if r.path == "/api/chat"));
}

#[test]
fn test_parse_match() {
    let m = parse_str("fn f() { match x { Ok(r) -> r\n Error(e) -> e\n } }");
    if let Decl::Function(f) = &m.declarations[0] {
        if let Stmt::Expr {
            expr: Expr::Match { arms, .. },
            ..
        } = &f.body[0]
        {
            assert_eq!(arms.len(), 2);
        } else {
            panic!("Expected match");
        }
    }
}

#[test]
fn test_parse_type_def() {
    let m = parse_str("type Shape =\n    | Circle(r: float)\n    | Point");
    if let Decl::TypeDef(t) = &m.declarations[0] {
        assert_eq!(t.name, "Shape");
        assert_eq!(t.variants.len(), 2);
    } else {
        panic!("Expected type def");
    }
}

#[test]
fn test_parse_operator_precedence() {
    let m = parse_str("fn f() { ret 1 + 2 * 3 }");
    if let Decl::Function(f) = &m.declarations[0] {
        if let Stmt::Return {
            value:
                Some(Expr::Binary {
                    op: BinOp::Add,
                    right,
                    ..
                }),
            ..
        } = &f.body[0]
        {
            assert!(matches!(
                right.as_ref(),
                Expr::Binary { op: BinOp::Mul, .. }
            ));
        } else {
            panic!("Expected add(1, mul(2,3))");
        }
    }
}

#[test]
fn test_parse_pipe() {
    let m = parse_str("fn f() { ret x |> transform |> render }");
    assert!(matches!(&m.declarations[0], Decl::Function(_)));
}

#[test]
fn test_parse_actor() {
    let m = parse_str("actor Worker { on receive(msg) to str { ret msg } }");
    if let Decl::Actor(a) = &m.declarations[0] {
        assert_eq!(a.name, "Worker");
        assert_eq!(a.handlers.len(), 1);
        assert_eq!(a.handlers[0].event_name, "receive");
    } else {
        panic!("Expected actor");
    }
}

#[test]
fn test_parse_workflow() {
    let m = parse_str("workflow process(file: str) to str { ret file }");
    if let Decl::Workflow(w) = &m.declarations[0] {
        assert_eq!(w.name, "process");
        assert_eq!(w.params.len(), 1);
    } else {
        panic!("Expected workflow");
    }
}

#[test]
fn test_parse_lambda() {
    let m = parse_str("fn f() { let add = fn(a, b) a + b\n ret add(1, 2) }");
    if let Decl::Function(f) = &m.declarations[0] {
        assert_eq!(f.body.len(), 2);
        if let Stmt::Let {
            value: Expr::Lambda { params, .. },
            ..
        } = &f.body[0]
        {
            assert_eq!(params.len(), 2);
        } else {
            panic!("Expected lambda let");
        }
    } else {
        panic!("Expected function");
    }
}

#[test]
fn test_parse_if_else() {
    let m = parse_str("fn f(x) { if x { ret 1\n} else { ret 0\n} }");
    if let Decl::Function(f) = &m.declarations[0] {
        if let Stmt::Expr {
            expr:
                Expr::If {
                    then_body,
                    else_body,
                    ..
                },
            ..
        } = &f.body[0]
        {
            assert_eq!(then_body.len(), 1);
            assert!(else_body.is_some());
        } else {
            panic!("Expected if/else");
        }
    }
}

#[test]
fn test_parse_mutable_let() {
    let m = parse_str("fn f() { let mut x = 0\n x = 1\n ret x }");
    if let Decl::Function(f) = &m.declarations[0] {
        if let Stmt::Let { mutable, .. } = &f.body[0] {
            assert!(mutable, "Should be mutable");
        } else {
            panic!("Expected mutable let");
        }
    }
}

#[test]
fn test_parse_method_chain() {
    let m = parse_str("fn f() { ret list.map(fn(x) x).filter(fn(x) x) }");
    if let Decl::Function(f) = &m.declarations[0] {
        if let Stmt::Return {
            value: Some(Expr::MethodCall { method, .. }),
            ..
        } = &f.body[0]
        {
            assert_eq!(method, "filter");
        } else {
            panic!("Expected method chain");
        }
    }
}

#[test]
fn test_parse_jsx_self_closing() {
    let m = parse_str("component App() { view: <input value=\"test\" /> }");
    if let Decl::ReactiveComponent(r) = &m.declarations[0] {
        match &r.view {
            Some(Expr::JsxSelfClosing(_)) => {}
            other => panic!("Expected self-closing JSX in view, got {other:?}"),
        }
    } else {
        panic!("Expected reactive component");
    }
}

#[test]
fn test_parse_jsx_with_children() {
    let m = parse_str("component A() { view: <div><span>hello</span></div> }");
    if let Decl::ReactiveComponent(r) = &m.declarations[0] {
        if let Some(Expr::Jsx(el)) = &r.view {
            assert_eq!(el.tag, "div");
            assert_eq!(el.children.len(), 1);
        } else {
            panic!("Expected JSX element in view");
        }
    } else {
        panic!("Expected reactive component");
    }
}

#[test]
fn test_parse_spawn() {
    let m = parse_str("fn f() { ret spawn(Worker) }");
    if let Decl::Function(f) = &m.declarations[0] {
        if let Stmt::Return {
            value: Some(Expr::Spawn { .. }),
            ..
        } = &f.body[0]
        {
            // ok
        } else {
            panic!("Expected spawn");
        }
    }
}

#[test]
fn test_parse_for_loop() {
    let m = parse_str("fn f() { for x in items { x } }");
    if let Decl::Function(f) = &m.declarations[0] {
        if let Stmt::Expr {
            expr: Expr::For { binding, .. },
            ..
        } = &f.body[0]
        {
            assert_eq!(binding, "x");
        } else {
            panic!("Expected for loop");
        }
    }
}

#[test]
fn test_parse_pub_fn() {
    let m = parse_str("pub fn helper() to int { ret 42 }");
    if let Decl::Function(f) = &m.declarations[0] {
        assert!(f.is_pub);
        assert_eq!(f.name, "helper");
    } else {
        panic!("Expected pub fn");
    }
}

#[test]
fn test_parse_multiple_decls() {
    let src = "import std\n\nfn a() { ret 1 }\n\nfn b() { ret 2 }";
    let m = parse_str(src);
    assert_eq!(m.declarations.len(), 3, "import + 2 functions");
}

#[test]
fn test_parse_activity() {
    let m = parse_str("activity send_email(recipient: str) to str { ret recipient }");
    if let Decl::Activity(a) = &m.declarations[0] {
        assert_eq!(a.name, "send_email");
        assert_eq!(a.params.len(), 1);
        assert_eq!(a.params[0].name, "recipient");
        assert!(a.return_type.is_some());
    } else {
        panic!("Expected activity declaration, got {:?}", m.declarations[0]);
    }
}

#[test]
fn test_parse_with_expression() {
    let m = parse_str("fn f() { ret call() with { timeout: 5 } }");
    if let Decl::Function(f) = &m.declarations[0] {
        if let Stmt::Return {
            value: Some(Expr::With {
                operand, options, ..
            }),
            ..
        } = &f.body[0]
        {
            assert!(matches!(operand.as_ref(), Expr::Call { .. }));
            assert!(matches!(options.as_ref(), Expr::ObjectLit { .. }));
        } else {
            panic!("Expected With expression in return");
        }
    } else {
        panic!("Expected function");
    }
}

#[test]
fn test_parse_table() {
    let m = parse_str("@table type Task { title: str\n done: bool\n priority: int }");
    if let Decl::Table(t) = &m.declarations[0] {
        assert_eq!(t.name, "Task");
        assert_eq!(t.fields.len(), 3);
        assert_eq!(t.fields[0].name, "title");
        assert_eq!(t.fields[1].name, "done");
        assert_eq!(t.fields[2].name, "priority");
    } else {
        panic!("Expected table declaration, got {:?}", m.declarations[0]);
    }
}

#[test]
fn test_parse_index() {
    let m = parse_str("@index Task.by_done on (done, priority)");
    if let Decl::Index(idx) = &m.declarations[0] {
        assert_eq!(idx.table_name, "Task");
        assert_eq!(idx.index_name, "by_done");
        assert_eq!(idx.columns, vec!["done", "priority"]);
    } else {
        panic!("Expected index declaration, got {:?}", m.declarations[0]);
    }
}
#[test]
fn test_parse_v0_component() {
    let m = parse_str("@v0 \"yM1xXq6\" fn Dashboard() -> Element");
    if let Decl::V0Component(v) = &m.declarations[0] {
        assert_eq!(v.name, "Dashboard");
        assert_eq!(v.v0_id, "yM1xXq6");
        assert!(v.image_path.is_none());
    } else {
        panic!("Expected V0Component, got {:?}", m.declarations[0]);
    }
}

#[test]
fn test_parse_v0_component_from_image() {
    let m = parse_str(r#"@v0 from "mock.png" fn Landing() -> Element"#);
    if let Decl::V0Component(v) = &m.declarations[0] {
        assert_eq!(v.name, "Landing");
        assert_eq!(v.v0_id, "");
        assert_eq!(v.image_path.as_deref(), Some("mock.png"));
    } else {
        panic!("Expected V0Component, got {:?}", m.declarations[0]);
    }
}

// WebIR blueprint G1: parser-truth coverage for islands, server fns, routes, reactive surface.

#[test]
fn test_parse_island_optional_prop() {
    let m = parse_str("@island DataChart {\n    title: str\n    data: str\n    width?: int\n}");
    if let Decl::Island(island) = &m.declarations[0] {
        assert_eq!(island.name, "DataChart");
        assert_eq!(island.props.len(), 3);
        assert!(!island.props[0].is_optional);
        assert!(!island.props[1].is_optional);
        assert!(island.props[2].is_optional);
        assert_eq!(island.props[2].name, "width");
    } else {
        panic!("Expected Decl::Island, got {:?}", m.declarations[0]);
    }
}

#[test]
fn test_parse_server_fn_brace_shape() {
    let m = parse_str("@server fn echo(x: str) to str {\n    ret x\n}");
    if let Decl::ServerFn(s) = &m.declarations[0] {
        assert_eq!(s.func.name, "echo");
        assert_eq!(s.func.params.len(), 1);
        assert_eq!(s.func.params[0].name, "x");
    } else {
        panic!("Expected Decl::ServerFn, got {:?}", m.declarations[0]);
    }
}

#[test]
fn test_parse_routes_multiple_entries() {
    let m = parse_str("routes { \"/\" to Home \"/about\" to About }");
    if let Decl::Routes(r) = &m.declarations[0] {
        assert_eq!(r.entries.len(), 2);
        assert_eq!(r.entries[0].path, "/");
        assert_eq!(r.entries[0].component_name, "Home");
        assert_eq!(r.entries[1].path, "/about");
        assert_eq!(r.entries[1].component_name, "About");
    } else {
        panic!("Expected Decl::Routes, got {:?}", m.declarations[0]);
    }
}

/// OP-0022: malformed routes entry (`path` then component ident without `to`) rejects gracefully.
#[test]
fn test_parse_rejects_invalid_route_entry_missing_to() {
    assert_parse_fails(r#"routes { "/" Home }"#);
}

/// OP-0026: root path and multi-segment path literals in one `routes` block.
#[test]
fn test_parse_routes_root_and_nested_path_literals() {
    let m = parse_str(r#"routes { "/" to Home "/blog/post" to Article }"#);
    if let Decl::Routes(r) = &m.declarations[0] {
        assert_eq!(r.entries.len(), 2);
        assert_eq!(r.entries[0].path, "/");
        assert_eq!(r.entries[0].component_name, "Home");
        assert_eq!(r.entries[1].path, "/blog/post");
        assert_eq!(r.entries[1].component_name, "Article");
    } else {
        panic!("Expected Decl::Routes, got {:?}", m.declarations[0]);
    }
}

#[test]
fn test_parse_reactive_effect_mount_cleanup_view() {
    let m = parse_str(
        "@component Demo(x: int) {\n  state n: int = x\n  effect: { }\n  mount: { }\n  cleanup: { }\n  view: <span>{n}</span>\n}",
    );
    if let Decl::ReactiveComponent(r) = &m.declarations[0] {
        assert_eq!(r.name, "Demo");
        assert!(
            r.members
                .iter()
                .any(|m| matches!(m, ReactiveMemberDecl::Effect(_))),
            "expected effect member"
        );
        assert!(
            r.members
                .iter()
                .any(|m| matches!(m, ReactiveMemberDecl::OnMount(_))),
            "expected mount member"
        );
        assert!(
            r.members
                .iter()
                .any(|m| matches!(m, ReactiveMemberDecl::OnCleanup(_))),
            "expected cleanup member"
        );
        assert!(r.view.is_some());
    } else {
        panic!(
            "Expected Decl::ReactiveComponent, got {:?}",
            m.declarations[0]
        );
    }
}

#[test]
fn test_parse_island_prop_requires_colon() {
    assert_parse_fails("@island X {\n    title str\n}");
}

#[test]
fn test_parse_reactive_rejects_misplaced_view_without_colon() {
    assert_parse_fails("@component Bad() {\n  view <div />\n}");
}

/// OP-0014: lexer token stream around optional island prop includes `?` and `:` markers.
#[test]
fn test_island_optional_prop_token_shape() {
    let src = "@island X {\n    title: str\n    width?: int\n}";
    let dbg = lex(src)
        .into_iter()
        .map(|s| format!("{:?}", s.token))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        dbg.contains("Question") && dbg.contains("Colon"),
        "unexpected token dbg: {dbg}"
    );
}

/// OP-0028: [`RoutesDecl::parse_summary`] is stable for multi-entry blocks.
#[test]
fn test_routes_parse_summary_matches_paths() {
    let m = parse_str(r#"routes { "/" to Home "/blog/post" to Article }"#);
    if let Decl::Routes(r) = &m.declarations[0] {
        assert_eq!(
            r.parse_summary(),
            RoutesParseSummary {
                entry_count: 2,
                paths: vec!["/".to_string(), "/blog/post".to_string()],
            }
        );
    } else {
        panic!("Expected Decl::Routes, got {:?}", m.declarations[0]);
    }
}

/// OP-0029: misplaced `view` JSX without `view:` is a hard parse failure (colon expectation).
#[test]
fn test_reactive_body_unknown_token_diagnostic_class() {
    let tokens = lex("@component Bad() {\n  view <div />\n}");
    let err = parse(tokens).expect_err("expected parse failure");
    assert!(
        err.iter().any(
            |e| e.class == ParseErrorClass::ExpectToken || e.class == ParseErrorClass::TopLevel
        ),
        "{err:?}"
    );
}

/// OP-0015: syntax inventory strings remain wired for tooling/docs extraction.
#[test]
fn test_web_surface_syntax_inventory_non_empty() {
    use crate::parser::WEB_SURFACE_SYNTAX_INVENTORY;
    let joined = WEB_SURFACE_SYNTAX_INVENTORY.join("\n");
    assert!(
        joined.contains("@island") && joined.contains("routes {"),
        "{joined}"
    );
}

#[test]
fn test_parse_agent_and_environment() {
    let m = parse_str(
        r#"
agent Assistant {
    version "1.0"
}
environment staging {
    base "node"
}
"#,
    );
    assert_eq!(2, m.declarations.len());
    assert!(matches!(m.declarations[0], Decl::Agent(_)));
    assert!(matches!(m.declarations[1], Decl::Environment(_)));
}
