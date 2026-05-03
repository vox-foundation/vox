//! AST-only declaration checks not represented in HIR (`@search_index`, `@index`).

use crate::ast::decl::{ComponentDecl, Decl, FnDecl, Module, SearchIndexDecl, TableDecl};
use crate::ast::expr::{Expr, StringPart};
use crate::ast::stmt::Stmt;
use crate::ast::types::TypeExpr;
use crate::react_bridge::{for_each_vox_hook_call_in_stmt, legacy_hook_lint_suppressed};
use crate::typeck::diagnostics::{Diagnostic, DiagnosticCategory, TypeckSeverity};
use crate::typeck::env::{Binding, BindingKind, TypeEnv};
use crate::typeck::ty::Ty;

fn relative_luminance(hex: &str) -> Option<f32> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 { return None; }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;

    let f = |c: f32| -> f32 {
        if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };

    Some(0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b))
}

fn contrast_ratio(l1: f32, l2: f32) -> f32 {
    let lighter = l1.max(l2);
    let darker = l1.min(l2);
    (lighter + 0.05) / (darker + 0.05)
}

fn check_theme_decl(t: &crate::ast::decl::ui::ThemeDecl, diags: &mut Vec<Diagnostic>) {
    for (variant_name, variant_props) in [("light", &t.light), ("dark", &t.dark)] {
        let get_color = |name: &str| -> Option<f32> {
            variant_props.iter().find(|(k, _)| k == name).and_then(|(_, v)| relative_luminance(v))
        };
        
        let check_contrast = |bg_key: &str, fg_key: &str, alt_fg_key: &str, diags: &mut Vec<Diagnostic>| {
            let bg = get_color(bg_key);
            let fg = get_color(fg_key).or_else(|| get_color(alt_fg_key));
            if let (Some(b), Some(f)) = (bg, fg) {
                let ratio = contrast_ratio(b, f);
                if ratio < 4.5 {
                    diags.push(Diagnostic {
                        message: format!("@theme '{}' ({}) has poor contrast ({:.2}:1) between {} and {}. WCAG minimum is 4.5:1.", t.name, variant_name, ratio, bg_key, fg_key),
                        span: t.span,
                        severity: TypeckSeverity::Warning,
                        expected_type: None, found_type: None, context: None,
                        suggestions: vec![format!("Adjust {} or {} to improve readability.", bg_key, fg_key)],
                        category: DiagnosticCategory::Lint,
                        code: Some("lint.theme_contrast".into()),
                        fixes: vec![], line_col: None, missing_cases: vec![], ast_node_kind: None,
                    });
                }
            }
        };

        check_contrast("--color-bg", "--color-text", "--color-fg", diags);
        check_contrast("--color-btn-bg", "--color-btn-text", "--color-btn-fg", diags);
    }
}
fn resolve_type(te: &TypeExpr, env: &TypeEnv) -> Ty {
    match te {
        TypeExpr::Named { name, .. } => {
            if let Some(ty) = env.lookup_type(name) {
                return ty;
            }
            match name.as_str() {
                "int" => Ty::Int,
                "float" => Ty::Float,
                "str" => Ty::Str,
                "bool" => Ty::Bool,
                "Unit" => Ty::Unit,
                "Element" => Ty::Element,
                other => Ty::Named(other.to_string()),
            }
        }
        TypeExpr::Generic { name, args, .. } => {
            let inner_args: Vec<Ty> = args.iter().map(|a| resolve_type(a, env)).collect();
            match name.as_str() {
                "list" | "List" => Ty::List(Box::new(
                    inner_args.into_iter().next().unwrap_or(Ty::TypeVar(0)),
                )),
                "Option" => Ty::Option(Box::new(
                    inner_args.into_iter().next().unwrap_or(Ty::TypeVar(0)),
                )),
                "Result" => Ty::Result(Box::new(
                    inner_args.into_iter().next().unwrap_or(Ty::TypeVar(0)),
                )),
                _ => Ty::Named(name.clone()),
            }
        }
        TypeExpr::Function {
            params,
            return_type,
            ..
        } => {
            let param_tys: Vec<Ty> = params.iter().map(|p| resolve_type(p, env)).collect();
            let ret_ty = resolve_type(return_type, env);
            Ty::Fn(param_tys, Box::new(ret_ty))
        }
        TypeExpr::Tuple { elements, .. } => {
            Ty::Tuple(elements.iter().map(|e| resolve_type(e, env)).collect())
        }
        TypeExpr::Unit { .. } => Ty::Unit,
        TypeExpr::Infer { .. } => Ty::Error,
        TypeExpr::Decimal { .. } => Ty::Decimal,
    }
}

fn register_table(env: &mut TypeEnv, t: &TableDecl) {
    let field_types: Vec<(String, Ty)> = t
        .fields
        .iter()
        .map(|f| (f.name.clone(), resolve_type(&f.type_ann, env)))
        .collect();

    env.define(
        t.name.clone(),
        Binding {
            ty: Ty::Table(t.name.clone(), field_types),
            mutable: false,
            kind: BindingKind::Table,
            is_deprecated: t.is_deprecated,
        },
    );
}

fn check_search_index_decl(env: &TypeEnv, si: &SearchIndexDecl, diags: &mut Vec<Diagnostic>) {
    let Some(binding) = env.lookup(&si.table_name) else {
        diags.push(Diagnostic {
            message: format!(
                "search_index '{}' references unknown table '{}'",
                si.index_name, si.table_name
            ),
            span: si.span,
            severity: TypeckSeverity::Error,
            expected_type: None,
            found_type: None,
            context: None,
            suggestions: vec![],
            category: DiagnosticCategory::Lint,
            code: Some("lint.search_index_unknown_table".into()),
            fixes: vec![],
            line_col: None,
            missing_cases: vec![],
            ast_node_kind: None,
        });
        return;
    };

    let Ty::Table(_table_name, fields) = &binding.ty else {
        diags.push(Diagnostic {
            message: format!(
                "search_index '{}' target '{}' is not a table",
                si.index_name, si.table_name
            ),
            span: si.span,
            severity: TypeckSeverity::Error,
            expected_type: None,
            found_type: None,
            context: None,
            suggestions: vec![],
            category: DiagnosticCategory::Lint,
            code: Some("lint.search_index_not_table".into()),
            fixes: vec![],
            line_col: None,
            missing_cases: vec![],
            ast_node_kind: None,
        });
        return;
    };

    let Some((_, field_ty)) = fields.iter().find(|(n, _)| n == &si.search_field) else {
        diags.push(Diagnostic {
            message: format!(
                "search_index '{}': table '{}' has no field '{}'",
                si.index_name, si.table_name, si.search_field
            ),
            span: si.span,
            severity: TypeckSeverity::Error,
            expected_type: None,
            found_type: None,
            context: None,
            suggestions: vec![],
            category: DiagnosticCategory::Lint,
            code: Some("lint.search_index_unknown_field".into()),
            fixes: vec![],
            line_col: None,
            missing_cases: vec![],
            ast_node_kind: None,
        });
        return;
    };

    if *field_ty != Ty::Str {
        diags.push(Diagnostic {
            message: format!(
                "search_index '{}': field '{}' must be type 'str', found {:?}",
                si.index_name, si.search_field, field_ty
            ),
            span: si.span,
            severity: TypeckSeverity::Error,
            expected_type: Some("str".into()),
            found_type: Some(format!("{field_ty:?}")),
            context: None,
            suggestions: vec![],
            category: DiagnosticCategory::Lint,
            code: Some("lint.search_index_field_type".into()),
            fixes: vec![],
            line_col: None,
            missing_cases: vec![],
            ast_node_kind: None,
        });
    }
}

fn visit_fn_decl_in_decl(decl: &Decl, visit: &mut impl FnMut(&FnDecl)) {
    match decl {
        Decl::Function(f) => visit(f),
        Decl::Component(c) => visit(&c.func),
        Decl::McpTool(m) => visit(&m.func),
        Decl::Test(t) => visit(&t.func),
        Decl::Forall(f) => visit(&f.func),
        Decl::ServerFn(s) => visit(&s.func),
        Decl::Query(q) => visit(&q.func),
        Decl::Mutation(m) => visit(&m.func),
        Decl::Skill(s) => visit(&s.func),
        Decl::AgentDef(ad) => visit(&ad.func),
        Decl::Scheduled(s) => visit(&s.func),
        Decl::Hook(h) => visit(&h.func),
        Decl::Provider(p) => visit(&p.func),
        Decl::Fixture(f) => visit(&f.func),
        Decl::Layout(l) => visit(&l.func),
        Decl::Loading(l) => visit(&l.func),
        Decl::NotFound(n) => visit(&n.func),
        Decl::ErrorBoundary(e) => visit(&e.func),
        Decl::Mock(m) => visit(&m.func),
        Decl::McpResource(m) => visit(&m.func),
        _ => {}
    }
}

/// Shallow `@pure` hint: obvious I/O-ish calls (`print`, `sleep`, `spawn`) are rejected.
fn first_shallow_pure_violation_in_expr(e: &Expr) -> Option<crate::ast::span::Span> {
    match e {
        Expr::Call { callee, args, span } => {
            if matches!(
                callee.as_ref(),
                Expr::Ident { name, .. } if name == "print" || name == "sleep"
            ) {
                return Some(*span);
            }
            first_shallow_pure_violation_in_expr(callee).or_else(|| {
                args.iter()
                    .find_map(|a| first_shallow_pure_violation_in_expr(&a.value))
            })
        }
        Expr::Spawn { span, .. } => Some(*span),
        Expr::Unary { operand, .. } => first_shallow_pure_violation_in_expr(operand),
        Expr::Binary { left, right, .. } => first_shallow_pure_violation_in_expr(left)
            .or_else(|| first_shallow_pure_violation_in_expr(right)),
        Expr::MethodCall { object, args, .. } => first_shallow_pure_violation_in_expr(object)
            .or_else(|| {
                args.iter()
                    .find_map(|a| first_shallow_pure_violation_in_expr(&a.value))
            }),
        Expr::FieldAccess { object, .. } => first_shallow_pure_violation_in_expr(object),
        Expr::Match { subject, arms, .. } => {
            first_shallow_pure_violation_in_expr(subject).or_else(|| {
                arms.iter().find_map(|arm| {
                    arm.guard
                        .as_ref()
                        .and_then(|g| first_shallow_pure_violation_in_expr(g))
                        .or_else(|| first_shallow_pure_violation_in_expr(arm.body.as_ref()))
                })
            })
        }
        Expr::If {
            condition,
            then_body,
            else_body,
            ..
        } => first_shallow_pure_violation_in_expr(condition)
            .or_else(|| {
                then_body
                    .iter()
                    .find_map(first_shallow_pure_violation_in_stmt)
            })
            .or_else(|| {
                else_body
                    .as_ref()
                    .and_then(|b| b.iter().find_map(first_shallow_pure_violation_in_stmt))
            }),
        Expr::For { iterable, body, .. } => first_shallow_pure_violation_in_expr(iterable)
            .or_else(|| first_shallow_pure_violation_in_expr(body)),
        Expr::Lambda { body, .. } => first_shallow_pure_violation_in_expr(body),
        Expr::Pipe { left, right, .. } => first_shallow_pure_violation_in_expr(left)
            .or_else(|| first_shallow_pure_violation_in_expr(right)),
        Expr::Try { target, .. } => first_shallow_pure_violation_in_expr(target),
        Expr::With {
            operand, options, ..
        } => first_shallow_pure_violation_in_expr(operand)
            .or_else(|| first_shallow_pure_violation_in_expr(options)),
        Expr::Block { stmts, .. } => stmts.iter().find_map(first_shallow_pure_violation_in_stmt),
        Expr::ObjectLit { fields, .. } => fields
            .iter()
            .find_map(|(_, v)| first_shallow_pure_violation_in_expr(v)),
        Expr::ListLit { elements, .. } | Expr::TupleLit { elements, .. } => elements
            .iter()
            .find_map(first_shallow_pure_violation_in_expr),
        Expr::StringInterp { parts, .. } => parts.iter().find_map(|p| match p {
            StringPart::Literal(_) => None,
            StringPart::Interpolation(inner) => first_shallow_pure_violation_in_expr(inner),
        }),
        Expr::Jsx(el) => el
            .attributes
            .iter()
            .find_map(|a| first_shallow_pure_violation_in_expr(&a.value))
            .or_else(|| {
                el.children
                    .iter()
                    .find_map(first_shallow_pure_violation_in_expr)
            }),
        Expr::JsxSelfClosing(el) => el
            .attributes
            .iter()
            .find_map(|a| first_shallow_pure_violation_in_expr(&a.value)),
        Expr::JsxFragment { children, .. } => children
            .iter()
            .find_map(first_shallow_pure_violation_in_expr),
        Expr::IntLit { .. }
        | Expr::FloatLit { .. }
        | Expr::StringLit { .. }
        | Expr::BoolLit { .. }
        | Expr::DecimalLit { .. }
        | Expr::Ident { .. } => None,
    }
}

fn first_shallow_pure_violation_in_stmt(s: &Stmt) -> Option<crate::ast::span::Span> {
    match s {
        Stmt::Let { value, .. } => first_shallow_pure_violation_in_expr(value),
        Stmt::Assign { target, value, .. } => first_shallow_pure_violation_in_expr(target)
            .or_else(|| first_shallow_pure_violation_in_expr(value)),
        Stmt::Return { value, .. } => value
            .as_ref()
            .and_then(first_shallow_pure_violation_in_expr),
        Stmt::Expr { expr, .. } => first_shallow_pure_violation_in_expr(expr),
        Stmt::While {
            condition, body, ..
        } => first_shallow_pure_violation_in_expr(condition)
            .or_else(|| body.iter().find_map(first_shallow_pure_violation_in_stmt)),
        Stmt::Loop { body, .. } => body.iter().find_map(first_shallow_pure_violation_in_stmt),
        Stmt::Break { .. } | Stmt::Continue { .. } => None,
    }
}

fn lint_component_react_hooks(comp: &ComponentDecl) -> Vec<Diagnostic> {
    if legacy_hook_lint_suppressed() {
        return Vec::new();
    }
    let mut diags = Vec::new();
    let f = &comp.func;
    for stmt in &f.body {
        for_each_vox_hook_call_in_stmt(stmt, &mut |name, span| {
            diags.push(Diagnostic {
                severity: TypeckSeverity::Warning,
                message: format!(
                    "React-style hook `{name}` in @component — prefer Path C `component` / `@component Name(...) {{ state ... }}` for lower K-complexity"
                ),
                span,
                expected_type: None,
                found_type: None,
                context: None,
                suggestions: vec![
                    "Use `state x = ...`, `derived`, `effect`, `mount`, `cleanup`, and `view:` instead of hooks where possible.".into(),
                    "Keep advanced React-only logic in `@island` TypeScript under islands/.".into(),
                ],
                category: DiagnosticCategory::Lint,
                code: Some("lint.component_react_hook".into()),
                fixes: vec![],
            line_col: None, missing_cases: vec![], ast_node_kind: None,
});
        });
    }
    diags
}

/// Run `@table` / `@index` / `@search_index` validation that stays on the AST surface.
#[must_use]
pub fn lint_ast_declarations(module: &Module, source: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut env = TypeEnv::new();

    for decl in &module.declarations {
        if let Decl::Table(t) = decl {
            for field in &t.fields {
                if field.name == "id" {
                    diags.push(Diagnostic {
                        message: format!(
                            "Table '{}' declares a column `id`; Vox `@table` already adds a surrogate `_id` primary key. \
                             Rename this field (e.g. `user_key`, `external_id`) to avoid LLM confusion with `_id`.",
                            t.name
                        ),
                        span: field.span,
                        severity: TypeckSeverity::Warning,
                        expected_type: None,
                        found_type: None,
                        context: None,
                        suggestions: vec![
                            "Omit a separate `id` and use `_id` in generated Rust/Turso rows.".into(),
                            "Or rename to a non-reserved column name that is not `id`.".into(),
                        ],
                        category: DiagnosticCategory::Lint,
                        code: Some("lint.table_id_column".into()),
                        fixes: vec![],
                    line_col: None, missing_cases: vec![], ast_node_kind: None,
});
                }
            }
            register_table(&mut env, t);
        }
    }

    for decl in &module.declarations {
        if let Decl::Component(c) = decl {
            diags.push(Diagnostic {
                severity: TypeckSeverity::Error,
                message: "Classic `@component fn` syntax is retired. Use Path C `component Name() { ... }` instead.".to_string(),
                span: c.func.span,
                expected_type: None,
                found_type: None,
                context: Some(Diagnostic::capture_context(source, c.func.span)),
                suggestions: vec![
                    format!("component {}() {{ view: ... }}", c.func.name),
                ],
                category: DiagnosticCategory::Lint,
                code: Some("lint.legacy_component_fn".into()),
                fixes: vec![],
                line_col: None, missing_cases: vec![], ast_node_kind: None,
            });
            diags.extend(lint_component_react_hooks(c));
        }
    }

    for decl in &module.declarations {
        let retired: Option<(&str, crate::ast::span::Span, &str)> = match decl {
            Decl::Context(c) => Some((
                "`context` declarations are retired. Define React Context in user-owned `app/App.tsx` (see react-interop migration charter).",
                c.span,
                "lint.retired_context",
            )),
            Decl::Hook(h) => Some((
                "`@hook fn` is retired. Prefer Path C `component`, islands, or plain TS under `islands/`.",
                h.func.span,
                "lint.retired_hook_fn",
            )),
            Decl::Provider(p) => Some((
                "`@provider fn` is retired. Add providers in user-owned `app/App.tsx`.",
                p.span,
                "lint.retired_provider_fn",
            )),
            Decl::Page(pg) => Some((
                "`page:` / static Page declarations are retired for the web stack. Use `routes { ... }` and Path C components.",
                pg.span,
                "lint.retired_page_decl",
            )),
            _ => None,
        };
        if let Some((message, span, code)) = retired {
            diags.push(Diagnostic {
                severity: TypeckSeverity::Error,
                message: message.to_string(),
                span,
                expected_type: None,
                found_type: None,
                context: None,
                suggestions: vec![],
                category: DiagnosticCategory::Lint,
                code: Some(code.to_string()),
                fixes: vec![],
                line_col: None,
                missing_cases: vec![],
                ast_node_kind: None,
            });
        }
    }

    for decl in &module.declarations {
        match decl {
            Decl::Index(idx) => {
                if env.lookup(&idx.table_name).is_none() {
                    diags.push(Diagnostic {
                        message: format!("@index references unknown table '{}'", idx.table_name),
                        span: idx.span,
                        severity: TypeckSeverity::Error,
                        expected_type: None,
                        found_type: None,
                        context: None,
                        suggestions: vec![],
                        category: DiagnosticCategory::Lint,
                        code: Some("lint.index_unknown_table".into()),
                        fixes: vec![],
                        line_col: None,
                        missing_cases: vec![],
                        ast_node_kind: None,
                    });
                }
            }
            Decl::SearchIndex(si) => check_search_index_decl(&env, si, &mut diags),
            Decl::Theme(t) => check_theme_decl(t, &mut diags),
            _ => {}
        }
    }

    for decl in &module.declarations {
        visit_fn_decl_in_decl(decl, &mut |f: &FnDecl| {
            if !f.is_pure {
                return;
            }
            let span = f
                .params
                .iter()
                .find_map(|p| {
                    p.default
                        .as_ref()
                        .and_then(first_shallow_pure_violation_in_expr)
                })
                .or_else(|| f.body.iter().find_map(first_shallow_pure_violation_in_stmt));
            if let Some(span) = span {
                diags.push(Diagnostic {
                    message: "@pure function may not call print/sleep/spawn (shallow purity lint)."
                        .to_string(),
                    span,
                    severity: TypeckSeverity::Warning,
                    expected_type: None,
                    found_type: None,
                    context: None,
                    suggestions: vec![
                        "Remove the call or drop @pure if side effects are intentional.".into(),
                    ],
                    category: DiagnosticCategory::Lint,
                    code: Some("lint.pure_shallow_violation".into()),
                    fixes: vec![],
                    line_col: None,
                    missing_cases: vec![],
                    ast_node_kind: None,
                });
            }
        });
    }

    diags
}
