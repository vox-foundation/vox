use crate::hir::{
    HirActivity, HirActor, HirFn, HirModule, HirTable, HirType, HirTypeDef, HirWorkflow,
};
use crate::typeck::env::{
    ActorHandlerSig, AdtDef, Binding, BindingKind, TypeEnv, VariantDef, WorkflowSig,
};
use crate::typeck::ty::Ty;

/// Register all top-level declarations from an HIR module into the type environment.
///
/// This is the "Pass 1" of type checking: it makes every name visible so that
/// forward references work when bodies are checked in Pass 2.
pub fn register_hir_module(env: &mut TypeEnv, module: &HirModule) {
    for td in &module.types {
        register_hir_typedef(env, td);
    }
    for f in &module.functions {
        register_hir_function(env, f);
    }
    for sf in &module.server_fns {
        register_fn_like(env, &sf.params, sf.return_type.as_ref(), &sf.name);
    }
    for t in &module.tests {
        register_hir_function(env, t);
    }
    for t in &module.mcp_tools {
        register_hir_function(env, &t.func);
    }
    for a in &module.actors {
        register_hir_actor(env, a);
    }
    for w in &module.workflows {
        register_hir_workflow(env, w);
    }
    for act in &module.activities {
        register_hir_activity(env, act);
    }
    for t in &module.tables {
        register_hir_table(env, t);
    }
}

/// Convert an HIR type to an internal [`Ty`].
pub fn resolve_hir_type(te: &HirType, env: &TypeEnv) -> Ty {
    match te {
        HirType::Named(name) => {
            if let Some(ty) = env.lookup_type(name) {
                return ty;
            }
            match name.as_str() {
                "int" => Ty::Int,
                "float" | "float64" => Ty::Float,
                "str" => Ty::Str,
                "bool" => Ty::Bool,
                "char" => Ty::Char,
                "never" => Ty::Never,
                "Unit" => Ty::Unit,
                "Element" => Ty::Element,
                other => Ty::Named(other.to_string()),
            }
        }
        HirType::Generic(name, args) => {
            let inner_args: Vec<Ty> = args.iter().map(|a| resolve_hir_type(a, env)).collect();
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
                "Stream" => Ty::Stream(Box::new(
                    inner_args.into_iter().next().unwrap_or(Ty::TypeVar(0)),
                )),
                "Map" => {
                    let mut it = inner_args.into_iter();
                    Ty::Map(
                        Box::new(it.next().unwrap_or(Ty::TypeVar(0))),
                        Box::new(it.next().unwrap_or(Ty::TypeVar(1))),
                    )
                }
                "Set" => Ty::Set(Box::new(
                    inner_args.into_iter().next().unwrap_or(Ty::TypeVar(0)),
                )),
                _ => Ty::Named(name.clone()),
            }
        }
        HirType::Function(params, ret) => Ty::Fn(
            params.iter().map(|p| resolve_hir_type(p, env)).collect(),
            Box::new(resolve_hir_type(ret, env)),
        ),
        HirType::Tuple(elements) => {
            Ty::Tuple(elements.iter().map(|e| resolve_hir_type(e, env)).collect())
        }
        HirType::Unit => Ty::Unit,
    }
}

pub fn register_hir_typedef(env: &mut TypeEnv, td: &HirTypeDef) {
    let variants: Vec<VariantDef> = td
        .variants
        .iter()
        .map(|v| VariantDef {
            name: v.name.clone(),
            fields: v
                .fields
                .iter()
                .map(|f| (f.0.clone(), resolve_hir_type(&f.1, env)))
                .collect(),
        })
        .collect();
    env.register_type(AdtDef {
        name: td.name.clone(),
        variants,
    });
}

pub fn register_hir_function(env: &mut TypeEnv, f: &HirFn) {
    env.push_scope();
    for (i, g) in f.generics.iter().enumerate() {
        env.define_type(g.clone(), Ty::GenericParam(i as u32));
    }
    let param_tys: Vec<Ty> = f
        .params
        .iter()
        .map(|p| {
            p.type_ann
                .as_ref()
                .map_or(Ty::TypeVar(0), |t| resolve_hir_type(t, env))
        })
        .collect();
    let ret_ty = f
        .return_type
        .as_ref()
        .map_or(Ty::Unit, |t| resolve_hir_type(t, env));
    env.pop_scope();
    env.define(
        f.name.clone(),
        Binding {
            ty: Ty::Fn(param_tys, Box::new(ret_ty)),
            mutable: false,
            kind: BindingKind::Function,
            is_deprecated: f.is_deprecated,
        },
    );
}

fn register_fn_like(
    env: &mut TypeEnv,
    params: &[crate::hir::HirParam],
    ret: Option<&HirType>,
    name: &str,
) {
    let param_tys: Vec<Ty> = params
        .iter()
        .map(|p| {
            p.type_ann
                .as_ref()
                .map_or(Ty::TypeVar(0), |t| resolve_hir_type(t, env))
        })
        .collect();
    let ret_ty = ret.map_or(Ty::Unit, |t| resolve_hir_type(t, env));
    env.define(
        name.to_string(),
        Binding::new(
            Ty::Fn(param_tys, Box::new(ret_ty)),
            false,
            BindingKind::Function,
        ),
    );
}

pub fn register_hir_actor(env: &mut TypeEnv, a: &HirActor) {
    let handlers: Vec<ActorHandlerSig> = a
        .handlers
        .iter()
        .map(|h| ActorHandlerSig {
            event_name: h.event_name.clone(),
            params: h
                .params
                .iter()
                .map(|p| {
                    (
                        p.name.clone(),
                        p.type_ann
                            .as_ref()
                            .map_or(Ty::TypeVar(0), |t| resolve_hir_type(t, env)),
                    )
                })
                .collect(),
            return_type: h
                .return_type
                .as_ref()
                .map_or(Ty::Unit, |t| resolve_hir_type(t, env)),
        })
        .collect();
    env.register_actor(a.name.clone(), handlers);
}

pub fn register_hir_workflow(env: &mut TypeEnv, w: &HirWorkflow) {
    env.register_workflow(WorkflowSig {
        name: w.name.clone(),
        params: w
            .params
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    p.type_ann
                        .as_ref()
                        .map_or(Ty::TypeVar(0), |t| resolve_hir_type(t, env)),
                )
            })
            .collect(),
        return_type: w
            .return_type
            .as_ref()
            .map_or(Ty::Unit, |t| resolve_hir_type(t, env)),
    });
}

pub fn register_hir_activity(env: &mut TypeEnv, a: &HirActivity) {
    let param_tys: Vec<Ty> = a
        .params
        .iter()
        .map(|p| {
            p.type_ann
                .as_ref()
                .map_or(Ty::TypeVar(0), |t| resolve_hir_type(t, env))
        })
        .collect();
    let ret_ty = a
        .return_type
        .as_ref()
        .map_or(Ty::Unit, |t| resolve_hir_type(t, env));
    env.define(
        a.name.clone(),
        Binding::new(
            Ty::Fn(param_tys, Box::new(ret_ty)),
            false,
            BindingKind::Activity,
        ),
    );
}

pub fn register_hir_table(env: &mut TypeEnv, t: &HirTable) {
    let fields: Vec<(String, Ty)> = t
        .fields
        .iter()
        .map(|f| (f.name.clone(), resolve_hir_type(&f.type_ann, env)))
        .collect();
    env.define(
        t.name.clone(),
        Binding {
            ty: Ty::Table(t.name.clone(), fields),
            mutable: false,
            kind: BindingKind::Table,
            is_deprecated: t.is_deprecated,
        },
    );
}
