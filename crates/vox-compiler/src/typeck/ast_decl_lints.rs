//! AST-only declaration checks not represented in HIR (`@search_index`, `@index`).

use crate::ast::decl::{ComponentDecl, Decl, Module, SearchIndexDecl, TableDecl};
use crate::ast::types::TypeExpr;
use crate::react_bridge::{for_each_vox_hook_call_in_stmt, legacy_hook_lint_suppressed};
use crate::typeck::diagnostics::{Diagnostic, DiagnosticCategory, TypeckSeverity};
use crate::typeck::env::{Binding, BindingKind, TypeEnv};
use crate::typeck::ty::Ty;

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
        });
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
            line_col: None,
});
        });
    }
    diags
}

/// Run `@table` / `@index` / `@search_index` validation that stays on the AST surface.
#[must_use]
pub fn lint_ast_declarations(module: &Module) -> Vec<Diagnostic> {
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
                    line_col: None,
});
                }
            }
            register_table(&mut env, t);
        }
    }

    for decl in &module.declarations {
        if let Decl::Component(c) = decl {
            diags.push(Diagnostic {
                severity: TypeckSeverity::Warning,
                message: "Classic `@component fn` syntax is deprecated. Use Path C `component Name() { ... }` instead.".to_string(),
                span: c.func.span,
                expected_type: None,
                found_type: None,
                context: None,
                suggestions: vec![
                    "Refactor to Path C standard: `component Name() { ... }`".into(),
                ],
                category: DiagnosticCategory::Lint,
                code: Some("lint.legacy_component_fn".into()),
                fixes: vec![],
                line_col: None,
            });
            diags.extend(lint_component_react_hooks(c));
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
                    });
                }
            }
            Decl::SearchIndex(si) => check_search_index_decl(&env, si, &mut diags),
            _ => {}
        }
    }

    diags
}
