//! `routes { }` → `routes.manifest.ts` (`VoxRoute[]` consumed by a user-owned router adapter).
//! Primary source: validated [`WebIrModule`](crate::web_ir::WebIrModule) route trees (OP-S042).

use std::collections::BTreeSet;

use crate::hir::HirModule;
use crate::web_ir::{RouteContract, RouteNode, WebIrModule};

pub const ROUTE_MANIFEST_FILENAME: &str = "routes.manifest.ts";

fn route_tree_top_contracts(web: &WebIrModule) -> Vec<&RouteContract> {
    let mut top: Vec<&RouteContract> = Vec::new();
    for node in &web.route_nodes {
        if let RouteNode::RouteTree { routes, .. } = node {
            for r in routes {
                top.push(r);
            }
        }
    }
    top
}

/// Fail-fast checks for manifest imports: HIR must define every component/loader/pending referenced
/// by the WebIR route tree when `routes { }` is present.
pub fn validate_manifest_symbols(web: &WebIrModule, hir: &HirModule) -> Result<(), String> {
    if hir.client_routes.is_empty() {
        return Ok(());
    }
    let top = route_tree_top_contracts(web);
    if top.is_empty() {
        return Err(
            "routes { } is present but WebIR has no RouteTree — cannot emit routes.manifest.ts (check lowering / VOX_WEBIR_VALIDATE diagnostics)".to_string(),
        );
    }

    let mut component_names: BTreeSet<String> = BTreeSet::new();
    for c in &hir.components {
        component_names.insert(c.0.func.name.clone());
    }
    for rc in &hir.reactive_components {
        component_names.insert(rc.name.clone());
    }
    for l in &hir.loadings {
        component_names.insert(l.0.func.name.clone());
    }
    for v in &hir.v0_components {
        component_names.insert(v.0.name.clone());
    }

    let query_names: BTreeSet<String> = hir.query_fns.iter().map(|q| q.name.clone()).collect();

    let mut errors: Vec<String> = Vec::new();
    for c in &top {
        validate_contract_branch(c, &component_names, &query_names, &mut errors);
    }

    for block in &hir.client_routes {
        let d = &block.0;
        if let Some(ref n) = d.not_found_component {
            if !component_names.contains(n) {
                errors.push(format!(
                    "route manifest: not_found component `{n}` has no matching generated .tsx"
                ));
            }
        }
        if let Some(ref n) = d.error_component {
            if !component_names.contains(n) {
                errors.push(format!(
                    "route manifest: error component `{n}` has no matching generated .tsx"
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn validate_contract_branch(
    e: &RouteContract,
    component_names: &BTreeSet<String>,
    query_names: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let path = &e.pattern;
    match meta_str(&e.meta, "component").as_deref() {
        None | Some("Unknown") => {
            errors.push(format!(
                "route manifest: route {path:?} has no resolved component in WebIR meta"
            ));
        }
        Some(c) => {
            if !component_names.contains(c) {
                errors.push(format!(
                    "route manifest: component `{c}` (path {path:?}) has no matching generated .tsx"
                ));
            }
        }
    }
    if let Some(l) = meta_str(&e.meta, "loader") {
        if !query_names.contains(&l) {
            errors.push(format!(
                "route manifest: loader `{l}` (path {path:?}) is not declared as @query"
            ));
        }
    }
    if let Some(p) = meta_str(&e.meta, "pending") {
        if !component_names.contains(&p) {
            errors.push(format!(
                "route manifest: pendingComponent `{p}` (path {path:?}) has no matching generated .tsx"
            ));
        }
    }
    for c in &e.children {
        validate_contract_branch(c, component_names, query_names, errors);
    }
}

/// Validate + emit; returns `Ok(None)` when there are no client routes.
pub fn try_emit_route_manifest_from_web_ir(
    web: &WebIrModule,
    hir: &HirModule,
) -> Result<Option<String>, String> {
    if hir.client_routes.is_empty() {
        return Ok(None);
    }
    validate_manifest_symbols(web, hir)?;
    let content = emit_route_manifest_from_web_ir(web, hir).ok_or_else(|| {
        "internal: routes.manifest.ts emit failed after validation (empty WebIR tree?)".to_string()
    })?;
    Ok(Some(content))
}

/// Emit manifest from lowered Web IR + HIR (for `not_found` / global pending from AST-only fields).
#[must_use]
pub fn emit_route_manifest_from_web_ir(web: &WebIrModule, hir: &HirModule) -> Option<String> {
    if hir.client_routes.is_empty() {
        return None;
    }

    let top = route_tree_top_contracts(web);
    if top.is_empty() {
        return None;
    }

    let mut not_found: Option<String> = None;
    let mut error_comp: Option<String> = None;
    let mut global_pending: Option<String> = None;

    for block in &hir.client_routes {
        let d = &block.0;
        if not_found.is_none() {
            not_found.clone_from(&d.not_found_component);
        }
        if error_comp.is_none() {
            error_comp.clone_from(&d.error_component);
        }
    }

    if let Some(l) = hir.loadings.first() {
        global_pending = Some(l.0.func.name.clone());
    }

    let mut import_names: BTreeSet<String> = BTreeSet::new();
    for c in &top {
        collect_contract_component_names(c, &mut import_names);
    }
    if let Some(ref n) = not_found {
        import_names.insert(n.clone());
    }
    if let Some(ref n) = error_comp {
        import_names.insert(n.clone());
    }
    if let Some(n) = &global_pending {
        import_names.insert(n.clone());
    }

    let mut loaders: BTreeSet<String> = BTreeSet::new();
    for c in &top {
        collect_contract_loader_names(c, &mut loaders);
    }

    let mut s = String::new();
    s.push_str("// Generated by Vox — framework-agnostic route manifest.\n");
    s.push_str("// Source: WebIR RouteTree → TS (see valid gates: VOX_WEBIR_VALIDATE).\n");
    s.push_str("// Adapter: import { voxRoutes } from \"./routes.manifest\" in your App.tsx.\n\n");
    s.push_str("import type { ComponentType } from \"react\"\n");

    for name in import_names.iter() {
        s.push_str(&format!("import {{ {name} }} from \"./{name}.tsx\"\n"));
    }
    if !loaders.is_empty() {
        let joined = loaders.iter().cloned().collect::<Vec<_>>().join(", ");
        s.push_str(&format!("import {{ {joined} }} from \"./vox-client\"\n"));
        s.push_str(
            "// TanStack Query: manifest `loader` fns call vox-client directly (no hooks here).\n",
        );
        s.push_str("// Inside route components, use `useVoxServerQuery` from `./vox-tanstack-query` for cache/dedup.\n");
    }

    s.push_str("\nexport type VoxRoute = {\n");
    s.push_str("  path: string\n");
    s.push_str("  component: ComponentType<any>\n");
    s.push_str("  loader?: (ctx: { params: Record<string, string> }) => Promise<unknown>\n");
    s.push_str("  pendingComponent?: ComponentType\n");
    s.push_str("  errorComponent?: ComponentType<{ error: Error }>\n");
    s.push_str("  children?: VoxRoute[]\n");
    s.push_str("  index?: boolean\n");
    s.push_str("}\n\n");

    if let Some(n) = &not_found {
        s.push_str(&format!("export const notFoundComponent = {n}\n"));
    }
    if let Some(n) = &error_comp {
        s.push_str(&format!("export const errorComponent = {n}\n"));
    }
    if let Some(n) = &global_pending {
        s.push_str(&format!("export const globalPendingComponent = {n}\n"));
    }

    s.push_str("export const voxRoutes: VoxRoute[] = [\n");
    for c in &top {
        s.push_str(&emit_contract_route_object(c, ""));
    }
    s.push_str("]\n");
    Some(s)
}

/// Emit a route manifest via WebIR (lowers HIR once). Kept for call sites without a cached [`WebIrModule`].
pub fn emit_route_manifest(hir: &HirModule) -> Result<Option<String>, String> {
    let web = crate::web_ir::lower::lower_hir_to_web_ir(hir);
    try_emit_route_manifest_from_web_ir(&web, hir)
}

fn meta_str(meta: &serde_json::Value, key: &str) -> Option<String> {
    meta.get(key)
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

fn collect_contract_component_names(e: &RouteContract, names: &mut BTreeSet<String>) {
    if let Some(c) = meta_str(&e.meta, "component") {
        if c != "Unknown" {
            names.insert(c);
        }
    }
    if let Some(p) = meta_str(&e.meta, "pending") {
        names.insert(p);
    }
    for c in &e.children {
        collect_contract_component_names(c, names);
    }
}

fn collect_contract_loader_names(e: &RouteContract, names: &mut BTreeSet<String>) {
    if let Some(l) = meta_str(&e.meta, "loader") {
        names.insert(l);
    }
    for c in &e.children {
        collect_contract_loader_names(c, names);
    }
}

fn has_dynamic_params(path: &str) -> bool {
    path.contains(':')
}

fn emit_contract_route_object(e: &RouteContract, indent: &str) -> String {
    let component = meta_str(&e.meta, "component").unwrap_or_else(|| "Unknown".to_string());
    let index = e.pattern.trim() == "/";
    let mut block = String::new();
    block.push_str(&format!("{indent}{{\n"));
    block.push_str(&format!("{indent}  path: {:?},\n", e.pattern));
    block.push_str(&format!("{indent}  component: {component},\n"));
    if let Some(l) = meta_str(&e.meta, "loader") {
        if has_dynamic_params(&e.pattern) {
            let inner = dynamic_param_object_innards(&e.pattern);
            block.push_str(&format!(
                "{indent}  loader: async ({{ params }}) => {l}({{ {inner} }}),\n"
            ));
        } else {
            block.push_str(&format!("{indent}  loader: async () => {l}({{}}),\n"));
        }
    }
    if let Some(p) = meta_str(&e.meta, "pending") {
        block.push_str(&format!("{indent}  pendingComponent: {p},\n"));
    }
    if index {
        block.push_str(&format!("{indent}  index: true,\n"));
    }
    if !e.children.is_empty() {
        block.push_str(&format!("{indent}  children: [\n"));
        for c in &e.children {
            block.push_str(&emit_contract_route_object(c, &format!("{indent}    ")));
        }
        block.push_str(&format!("{indent}  ],\n"));
    }
    block.push_str(&format!("{indent}}},\n"));
    block
}

/// Comma-separated object fields: `id: params["id"] ?? ""`
fn dynamic_param_object_innards(path: &str) -> String {
    let mut parts = Vec::new();
    for seg in path.split('/') {
        if let Some(stripped) = seg.strip_prefix(':') {
            if stripped.is_empty() {
                continue;
            }
            let field = stripped.trim_end_matches('?');
            parts.push(format!("{field}: params[\"{field}\"] ?? \"\""));
        }
    }
    parts.join(", ")
}

/// JSON format for library mode (`routes.manifest.json`).
pub fn try_emit_route_manifest_json_from_web_ir(
    web: &WebIrModule,
    hir: &HirModule,
) -> Result<Option<String>, String> {
    if hir.client_routes.is_empty() {
        return Ok(None);
    }
    validate_manifest_symbols(web, hir)?;
    let content = emit_route_manifest_json(web, hir)
        .ok_or_else(|| "internal: routes.manifest.json emit failed after validation".to_string())?;
    Ok(Some(content))
}

pub fn emit_route_manifest_json(web: &WebIrModule, hir: &HirModule) -> Option<String> {
    if hir.client_routes.is_empty() {
        return None;
    }
    let top = route_tree_top_contracts(web);
    if top.is_empty() {
        return None;
    }

    let mut not_found: Option<String> = None;
    let mut error_comp: Option<String> = None;
    let mut global_pending: Option<String> = None;

    for block in &hir.client_routes {
        let d = &block.0;
        if not_found.is_none() {
            not_found.clone_from(&d.not_found_component);
        }
        if error_comp.is_none() {
            error_comp.clone_from(&d.error_component);
        }
    }

    if let Some(l) = hir.loadings.first() {
        global_pending = Some(l.0.func.name.clone());
    }

    let json_obj = serde_json::json!({
        "notFoundComponent": not_found,
        "errorComponent": error_comp,
        "globalPendingComponent": global_pending,
        "routes": top.iter().map(|c| get_contract_route_json(c)).collect::<Vec<_>>()
    });

    Some(serde_json::to_string_pretty(&json_obj).unwrap())
}

fn get_contract_route_json(e: &RouteContract) -> serde_json::Value {
    let component = meta_str(&e.meta, "component").unwrap_or_else(|| "Unknown".to_string());
    let mut obj = serde_json::Map::new();
    obj.insert("path".to_string(), serde_json::json!(e.pattern));
    obj.insert("component".to_string(), serde_json::json!(component));
    if let Some(l) = meta_str(&e.meta, "loader") {
        obj.insert("loader".to_string(), serde_json::json!(l));
    }
    if let Some(p) = meta_str(&e.meta, "pending") {
        obj.insert("pendingComponent".to_string(), serde_json::json!(p));
    }
    if e.pattern.trim() == "/" {
        obj.insert("index".to_string(), serde_json::json!(true));
    }
    if !e.children.is_empty() {
        let children: Vec<_> = e
            .children
            .iter()
            .map(|c| get_contract_route_json(c))
            .collect();
        obj.insert("children".to_string(), serde_json::json!(children));
    }
    serde_json::Value::Object(obj)
}
