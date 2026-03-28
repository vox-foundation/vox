//! App-surface contract IR derived from semantic HIR.
//!
//! This module defines a serde-stable contract consumed by codegen and tooling so route/RPC
//! ownership does not remain split across ad-hoc emitter logic.

use serde::{Deserialize, Serialize};

use crate::ast::types::TypeExpr;
use crate::hir::{HirHttpMethod, HirModule};
use crate::typeck::env::TypeEnv;
use crate::typeck::registration::{resolve_hir_type, type_signature_from_hir};

/// Versioned schema for [`AppContractModule`].
pub const APP_CONTRACT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppContractModule {
    pub schema_version: u32,
    pub http_routes: Vec<AppHttpRouteContract>,
    pub server_fns: Vec<AppServerFnContract>,
    pub query_fns: Vec<AppServerFnContract>,
    pub mutation_fns: Vec<AppMutationContract>,
    pub client_routes: Vec<AppClientRouteContract>,
    pub islands: Vec<AppIslandContract>,
    pub server_config: AppServerConfigContract,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppHttpRouteContract {
    pub method: String,
    pub path: String,
    pub route_contract: String,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppServerFnContract {
    pub name: String,
    pub route_path: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMutationContract {
    pub name: String,
    pub route_path: String,
    pub signature: String,
    pub wraps_db_transaction: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppClientRouteContract {
    pub path: String,
    pub component_name: String,
    pub redirect: Option<String>,
    pub is_wildcard: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIslandPropContract {
    pub name: String,
    pub ty: String,
    pub is_optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIslandContract {
    pub name: String,
    pub props: Vec<AppIslandPropContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppServerConfigContract {
    pub bind_host: String,
    pub default_port: u16,
    pub port_env_var: String,
    pub dev_proxy_env_var: String,
    pub static_assets_embed_dir: String,
}

fn method_to_string(method: HirHttpMethod) -> String {
    match method {
        HirHttpMethod::Get => "GET".to_string(),
        HirHttpMethod::Post => "POST".to_string(),
        HirHttpMethod::Put => "PUT".to_string(),
        HirHttpMethod::Delete => "DELETE".to_string(),
    }
}

fn fn_signature(params: &[crate::hir::HirParam], ret: Option<&crate::hir::HirType>) -> String {
    let env = TypeEnv::new();
    type_signature_from_hir(params, ret, &env)
}

fn type_expr_signature(te: &TypeExpr) -> String {
    match te {
        TypeExpr::Named { name, .. } => name.clone(),
        TypeExpr::Generic { name, args, .. } => {
            let args = args
                .iter()
                .map(type_expr_signature)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}[{args}]")
        }
        TypeExpr::Function {
            params,
            return_type,
            ..
        } => {
            let params = params
                .iter()
                .map(type_expr_signature)
                .collect::<Vec<_>>()
                .join(", ");
            format!("fn({params}) -> {}", type_expr_signature(return_type))
        }
        TypeExpr::Tuple { elements, .. } => {
            let elems = elements
                .iter()
                .map(type_expr_signature)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({elems})")
        }
        TypeExpr::Unit { .. } => "Unit".to_string(),
    }
}

#[must_use]
pub fn project_app_contract(module: &HirModule) -> AppContractModule {
    let env = TypeEnv::new();
    let http_routes = module
        .routes
        .iter()
        .map(|r| AppHttpRouteContract {
            method: method_to_string(r.method),
            path: r.path.clone(),
            route_contract: r.route_contract.clone(),
            return_type: r
                .return_type
                .as_ref()
                .map(|t| resolve_hir_type(t, &env).signature()),
        })
        .collect();

    let server_fns = module
        .server_fns
        .iter()
        .map(|sf| AppServerFnContract {
            name: sf.name.clone(),
            route_path: sf.route_path.clone(),
            signature: fn_signature(&sf.params, sf.return_type.as_ref()),
        })
        .collect();

    let query_fns = module
        .query_fns
        .iter()
        .map(|qf| AppServerFnContract {
            name: qf.name.clone(),
            route_path: qf.route_path.clone(),
            signature: fn_signature(&qf.params, qf.return_type.as_ref()),
        })
        .collect();

    let wraps_db_transaction = !module.tables.is_empty();
    let mutation_fns = module
        .mutation_fns
        .iter()
        .map(|mf| AppMutationContract {
            name: mf.name.clone(),
            route_path: mf.route_path.clone(),
            signature: fn_signature(&mf.params, mf.return_type.as_ref()),
            wraps_db_transaction,
        })
        .collect();

    let client_routes = module
        .client_routes
        .iter()
        .flat_map(|r| {
            r.0.entries.iter().map(|entry| AppClientRouteContract {
                path: entry.path.clone(),
                component_name: entry.component_name.clone(),
                redirect: entry.redirect.clone(),
                is_wildcard: entry.is_wildcard,
            })
        })
        .collect();

    let islands = module
        .islands
        .iter()
        .map(|i| AppIslandContract {
            name: i.0.name.clone(),
            props: i
                .0
                .props
                .iter()
                .map(|p| AppIslandPropContract {
                    name: p.name.clone(),
                    ty: type_expr_signature(&p.ty),
                    is_optional: p.is_optional,
                })
                .collect(),
        })
        .collect();

    AppContractModule {
        schema_version: APP_CONTRACT_SCHEMA_VERSION,
        http_routes,
        server_fns,
        query_fns,
        mutation_fns,
        client_routes,
        islands,
        server_config: AppServerConfigContract {
            bind_host: "127.0.0.1".to_string(),
            default_port: 3000,
            port_env_var: "VOX_PORT".to_string(),
            dev_proxy_env_var: "VOX_SSR_DEV_URL".to_string(),
            static_assets_embed_dir: "public/".to_string(),
        },
    }
}

/// Canonical JSON bytes for stable app-contract hashing (sorted object keys at every depth).
pub fn canonical_app_contract_bytes(
    module: &AppContractModule,
) -> Result<Vec<u8>, serde_json::Error> {
    let mut v = serde_json::to_value(module)?;
    crate::syntax_k::sort_json_value_keys(&mut v);
    serde_json::to_vec(&v)
}
