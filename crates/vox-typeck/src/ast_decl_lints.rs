//! AST-only declaration checks not represented in HIR (`@search_index`, `@index`).

use crate::diagnostics::{Diagnostic, Severity};
use crate::env::{Binding, BindingKind, TypeEnv};
use crate::ty::Ty;
use vox_ast::decl::{Decl, Module, SearchIndexDecl, TableDecl};
use vox_ast::types::TypeExpr;

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
            severity: Severity::Error,
            expected_type: None,
            found_type: None,
            context: None,
            suggestions: vec![],
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
            severity: Severity::Error,
            expected_type: None,
            found_type: None,
            context: None,
            suggestions: vec![],
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
            severity: Severity::Error,
            expected_type: None,
            found_type: None,
            context: None,
            suggestions: vec![],
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
            severity: Severity::Error,
            expected_type: Some("str".into()),
            found_type: Some(format!("{field_ty:?}")),
            context: None,
            suggestions: vec![],
        });
    }
}

/// Run `@table` / `@index` / `@search_index` validation that stays on the AST surface.
#[must_use]
pub fn lint_ast_declarations(module: &Module) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut env = TypeEnv::new();

    for decl in &module.declarations {
        if let Decl::Table(t) = decl {
            register_table(&mut env, t);
        }
    }

    for decl in &module.declarations {
        match decl {
            Decl::Index(idx) => {
                if env.lookup(&idx.table_name).is_none() {
                    diags.push(Diagnostic {
                        message: format!("@index references unknown table '{}'", idx.table_name),
                        span: idx.span,
                        severity: Severity::Error,
                        expected_type: None,
                        found_type: None,
                        context: None,
                        suggestions: vec![],
                    });
                }
            }
            Decl::SearchIndex(si) => check_search_index_decl(&env, si, &mut diags),
            _ => {}
        }
    }

    diags
}
