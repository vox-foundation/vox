use super::*;
use crate::ast::expr::{BinOp, Expr};
use crate::ast::stmt::Stmt;
use crate::lexer::cursor::lex;

fn parse_str(source: &str) -> Module {
    let tokens = lex(source);
    parse(tokens).unwrap_or_else(|e| panic!("Parse errors: {e:?}"))
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
fn test_parse_component() {
    let m = parse_str("@component fn Chat() to Element { ret 0 }");
    assert!(matches!(&m.declarations[0], Decl::Component(_)));
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
    let m = parse_str("@component fn App() to Element { <input value=\"test\" /> }");
    if let Decl::Component(c) = &m.declarations[0] {
        if let Stmt::Expr {
            expr: Expr::JsxSelfClosing(_),
            ..
        } = &c.func.body[0]
        {
            // ok
        } else {
            panic!("Expected self-closing JSX");
        }
    }
}

#[test]
fn test_parse_jsx_with_children() {
    let m = parse_str("@component fn A() to Element { <div><span>hello</span></div> }");
    if let Decl::Component(c) = &m.declarations[0] {
        if let Stmt::Expr {
            expr: Expr::Jsx(el),
            ..
        } = &c.func.body[0]
        {
            assert_eq!(el.tag, "div");
            assert_eq!(el.children.len(), 1);
        } else {
            panic!("Expected JSX element");
        }
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
fn test_parse_v0_prompt() {
    let m = parse_str("@v0 \"A dashboard with charts\" fn Dashboard() to Element");
    if let Decl::V0Component(v) = &m.declarations[0] {
        assert_eq!(v.name, "Dashboard");
        assert_eq!(v.prompt, "A dashboard with charts");
        assert!(v.image_path.is_none());
    } else {
        panic!("Expected V0Component, got {:?}", m.declarations[0]);
    }
}

#[test]
fn test_parse_v0_from_image() {
    let m = parse_str("@v0 from \"design.png\" fn Dashboard() to Element");
    if let Decl::V0Component(v) = &m.declarations[0] {
        assert_eq!(v.name, "Dashboard");
        assert!(v.prompt.is_empty());
        assert_eq!(v.image_path.as_deref(), Some("design.png"));
    } else {
        panic!("Expected V0Component, got {:?}", m.declarations[0]);
    }
}
