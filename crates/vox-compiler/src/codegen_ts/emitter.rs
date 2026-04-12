//! HIR → TypeScript file bundle (production path). **WebIR bridge (OP-S025):** after assembling
//! artifacts, [`maybe_web_ir_validate`] may lower + validate [`crate::web_ir::WebIrModule`] when
//! **`VOX_WEBIR_VALIDATE`** (default **on**): CI and local builds fail codegen when
//! [`validate_web_ir`](crate::web_ir::validate::validate_web_ir) returns diagnostics. Set to `0` / `false` /
//! `no` / `off` to skip the gate (escape hatch).
//!
//! **Escape hatch:** disabling validation skips the structural gate; `routes.manifest.ts` is still emitted
//! only after the validator runs when the gate is enabled (so a failing gate never writes the manifest).
//!
//! **Style + route printer bridge (OP-S059 / S091 / S111 / S137 / S171 / S199):** classic CSS emission and
//! TanStack route files are still assembled here alongside [`super::routes`]; migrating printers to consume
//! only validated [`crate::web_ir::WebIrModule`] slices is tracked in the internal Web IR blueprint.

use crate::app_contract::project_app_contract;
use crate::codegen_ts::activity::{generate_activity_hir, generate_activity_runner};
use crate::codegen_ts::adt::generate_types;
use crate::codegen_ts::component::{generate_component, generate_component_from_web_ir};
use crate::codegen_ts::island_emit::collect_island_names;
use crate::codegen_ts::reactive::generate_reactive_component;
use crate::codegen_ts::route_manifest::{
    ROUTE_MANIFEST_FILENAME, try_emit_route_manifest_from_web_ir,
};
use crate::codegen_ts::routes::generate_routes;
use crate::codegen_ts::tanstack_query_emit::vox_tanstack_query_tsx;
use crate::codegen_ts::vox_client::{VOX_CLIENT_FILENAME, emit_vox_client};
use crate::hir::{HirFn, HirModule};

/// Output from the TypeScript code generator.
pub struct CodegenOutput {
    /// List of (filename, content) pairs.
    pub files: Vec<(String, String)>,
}

/// Options for [`generate_with_options`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CodegenOptions {
    /// Legacy flag (TanStack Start tree emission). **Ignored** — `routes:` now emits [`routes.manifest.ts`] only.
    pub tanstack_start: bool,
    /// Build Target
    pub target: Option<String>,
}

impl CodegenOptions {
    /// Reads **`VOX_WEB_TANSTACK_START`** for callers that still thread the flag (**ignored** for TS emit —
    /// route output is always [`route_manifest`] + components).
    #[must_use]
    pub fn from_env() -> Self {
        let tanstack_start_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxWebTanstackStart);
        Self {
            tanstack_start: tanstack_start_resolved.expose()
                .is_some_and(|v: &str| v == "1" || v.eq_ignore_ascii_case("true")),
            target: None,
        }
    }
}

/// Generate TypeScript files from a Vox module (options from [`CodegenOptions::from_env`]).
pub fn generate(hir: &HirModule) -> Result<CodegenOutput, String> {
    generate_with_options(hir, CodegenOptions::from_env())
}

/// Generate TypeScript with explicit options (callers such as `vox build` should pass config here).
pub fn generate_with_options(
    hir: &HirModule,
    options: CodegenOptions,
) -> Result<CodegenOutput, String> {
    let mut files = Vec::new();
    let island_names = collect_island_names(hir);
    let app_contract = project_app_contract(hir);

    // Generate type definitions
    let types_content = generate_types(hir);
    if !types_content.is_empty() {
        files.push(("types.ts".to_string(), types_content));
    }
    if let Ok(contract_json) = serde_json::to_string_pretty(&app_contract) {
        files.push(("vox-app-contract.json".to_string(), contract_json));
    }

    files.push((
        "vox-tanstack-query.tsx".to_string(),
        vox_tanstack_query_tsx(),
    ));

    let web_projection_cache = if hir.reactive_components.is_empty()
        && hir.components.is_empty()
        && hir.loadings.is_empty()
        && hir.client_routes.is_empty()
    {
        None
    } else {
        Some(crate::web_ir::lower::project_web_from_core(hir))
    };
    let web_projection_ref = web_projection_cache.as_ref();

    // Generate components
    for hir_comp in &hir.components {
        let comp = &hir_comp.0;
        let (filename, content) = web_projection_ref
            .and_then(|web| {
                generate_component_from_web_ir(&comp.func, !comp.styles.is_empty(), web)
            })
            .unwrap_or_else(|| {
                generate_component(&comp.func, !comp.styles.is_empty(), &island_names)
            });
        files.push((filename, content));
    }

    // Generate reactive components (Path C). Optional `VOX_WEBIR_EMIT_REACTIVE_VIEWS=1` uses Web IR
    // preview emit for `view:` when validate is clean and whitespace-normalized JSX matches legacy.
    for rc in &hir.reactive_components {
        let (filename, content) =
            generate_reactive_component(hir, rc, &island_names, web_projection_ref);
        files.push((filename, content));
    }

    // Route loading / suspense UI (`@loading fn … to Element`) — TanStack `pendingComponent`
    for hir_loading in &hir.loadings {
        let (filename, content) = web_projection_ref
            .and_then(|web| generate_component_from_web_ir(&hir_loading.0.func, false, web))
            .unwrap_or_else(|| generate_component(&hir_loading.0.func, false, &island_names));
        files.push((filename, content));
    }

    // Generate v0 component placeholders
    for hir_v0 in &hir.v0_components {
        let v0 = &hir_v0.0;
        let filename = format!("{}.tsx", v0.name);

        let comment = if let Some(ref img) = v0.image_path {
            format!("From image: {img}")
        } else {
            format!("v0 integration ID: {}", v0.v0_id)
        };

        let content = format!(
            "// @v0 generated component\n// {}\n// Note: This file will be overwritten by `npx v0 add` sidecar during build.\n// Install this island (shadcn): npx shadcn@latest add <component-url-or-name>\nimport React from \"react\";\n\nexport function {}(): React.ReactElement {{\n  return <div>{{/* @v0 component pending v0 CLI download */}}</div>;\n}}\n",
            comment, v0.name
        );
        files.push((filename, content));
    }

    // Generate Express server only when explicitly requested (Axum + api.ts is canonical).
    let routes_content = generate_routes(hir);
    let emit_express_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxEmitExpressServer);
    if !routes_content.is_empty()
        && emit_express_resolved.expose()
            .is_some_and(|v: &str| v == "1" || v.eq_ignore_ascii_case("true"))
    {
        files.push(("server.ts".to_string(), routes_content));
    }

    // Generate activities from HIR (canonical)
    if !hir.activities.is_empty() {
        let mut activities_content = String::new();
        activities_content.push_str(&generate_activity_runner());
        activities_content.push('\n');
        for activity in &hir.activities {
            activities_content.push_str(&generate_activity_hir(activity));
        }
        files.push(("activities.ts".to_string(), activities_content));
    }

    // Generate table interfaces + schema from HIR
    if !hir.tables.is_empty() {
        let mut schema = String::new();
        schema.push_str("// Table interfaces generated by Vox compiler\n\n");
        for table in &hir.tables {
            schema.push_str(&format!("export interface {} {{\n", table.name));
            schema.push_str("  _id: number;\n");
            for field in &table.fields {
                let ts_type = crate::codegen_ts::hir_emit::map_hir_type_to_ts(&field.type_ann);
                schema.push_str(&format!("  {}: {};\n", field.name, ts_type));
            }
            schema.push_str("}\n\n");
        }
        files.push(("schema.ts".to_string(), schema));
    }

    // Typed fetch client for `@query` (GET + JSON query values) / `@mutation` / `@server` (POST JSON).
    let has_api_fns =
        !hir.server_fns.is_empty() || !hir.query_fns.is_empty() || !hir.mutation_fns.is_empty();
    if has_api_fns {
        files.push((VOX_CLIENT_FILENAME.to_string(), emit_vox_client(hir)));
    }

    // Generate scoped CSS modules for components with style blocks (classic + Path C reactive).
    for hir_comp in &hir.components {
        let comp = &hir_comp.0;
        if !comp.styles.is_empty() {
            let filename = format!("{}.css", comp.func.name);
            let mut css = String::new();
            for block in &comp.styles {
                css.push_str(&format!("{} {{\n", block.selector));
                for (prop, val) in &block.properties {
                    // Convert Vox camelCase property names to CSS kebab-case
                    let css_prop = prop.chars().fold(String::new(), |mut acc, c| {
                        if c.is_uppercase() {
                            acc.push('-');
                            acc.push(c.to_ascii_lowercase());
                        } else {
                            acc.push(c);
                        }
                        acc
                    });
                    css.push_str(&format!("  {}: {};\n", css_prop, val));
                }
                css.push_str("}\n\n");
            }
            files.push((filename, css));
        }
    }
    for rc in &hir.reactive_components {
        if rc.styles.is_empty() {
            continue;
        }
        let filename = format!("{}.css", rc.name);
        let mut css = String::new();
        for block in &rc.styles {
            css.push_str(&format!("{} {{\n", block.selector));
            for (prop, val) in &block.properties {
                let css_prop = prop.chars().fold(String::new(), |mut acc, c| {
                    if c.is_uppercase() {
                        acc.push('-');
                        acc.push(c.to_ascii_lowercase());
                    } else {
                        acc.push(c);
                    }
                    acc
                });
                css.push_str(&format!("  {}: {};\n", css_prop, val));
            }
            css.push_str("}\n\n");
        }
        files.push((filename, css));
    }

    maybe_web_ir_validate(hir, web_projection_cache.as_ref())?;

    let route_manifest = match web_projection_ref {
        Some(w) => try_emit_route_manifest_from_web_ir(w, hir)?,
        None if !hir.client_routes.is_empty() => {
            let w = crate::web_ir::lower::project_web_from_core(hir);
            try_emit_route_manifest_from_web_ir(&w, hir)?
        }
        _ => None,
    };
    if let Some(manifest) = route_manifest {
        files.push((ROUTE_MANIFEST_FILENAME.to_string(), manifest));
    }

    let island_names: Vec<&str> = hir.islands.iter().map(|i| i.0.name.as_str()).collect();
    if !island_names.is_empty() {
        let mut meta = String::from(
            "// Declared @island names (implementations live under islands/src/<Name>/).\n",
        );
        meta.push_str("export const VOX_ISLAND_NAMES = [");
        meta.push_str(
            &island_names
                .iter()
                .map(|n| format!("{n:?}"))
                .collect::<Vec<_>>()
                .join(", "),
        );
        meta.push_str("] as const;\n");
        meta.push_str("export type VoxIslandName = (typeof VOX_ISLAND_NAMES)[number];\n");
        files.push(("vox-islands-meta.ts".to_string(), meta));
    }

    // Generate mobile native bridge
    let mobile_fns: Vec<&HirFn> = hir
        .functions
        .iter()
        .filter(|f| f.is_mobile_native)
        .collect();
    if !mobile_fns.is_empty() {
        let mut mobile_bridge = String::from("// Mobile native bridge generated by Vox compiler\n");
        mobile_bridge.push_str("import { Capacitor } from \"@capacitor/core\";\n\n");
        for f in mobile_fns {
            mobile_bridge.push_str(&crate::codegen_ts::hir_emit::emit_mobile_bridge_fn(f));
            mobile_bridge.push('\n');
        }
        files.push(("mobile-bridge.ts".to_string(), mobile_bridge));
    }

    let uses_mobile_namespace = hir.imports.iter().any(|imp| {
        (imp.module_path == vec!["std"] && imp.item == "mobile")
            || (imp.module_path.is_empty() && imp.item == "mobile")
    });
    if uses_mobile_namespace {
        files.push((
            "mobile-utils.ts".to_string(),
            crate::codegen_ts::hir_emit::emit_mobile_web_api_utils(options.target.as_deref()),
        ));
    }

    for env in &hir.environments {
        let spec = vox_container::generate::EnvironmentSpec {
            base_image: env
                .base_image
                .clone()
                .unwrap_or_else(|| "debian:bookworm-slim".to_string()),
            packages: env.packages.clone(),
            env_vars: env.env_vars.clone(),
            exposed_ports: env.exposed_ports.clone(),
            volumes: env.volumes.clone(),
            workdir: env.workdir.clone(),
            copy_instructions: env.copy_instructions.clone(),
            run_commands: env.run_commands.clone(),
            cmd: env.cmd.clone(),
            entrypoint: Vec::new(),
        };
        let dockerfile = vox_container::generate::generate_dockerfile_from_spec(&spec);
        files.push((format!("Dockerfile.{}", env.name), dockerfile));
    }

    Ok(CodegenOutput { files })
}

/// WebIR lower + validate gate (OP-0113, OP-0124). **On by default;** set `VOX_WEBIR_VALIDATE=0` / `false` /
/// `no` / `off` to skip.
fn maybe_web_ir_validate(
    hir: &HirModule,
    cached_web: Option<&crate::web_ir::WebIrModule>,
) -> Result<(), String> {
    if !crate::web_migration_env::web_ir_validate_gate_enabled() {
        return Ok(());
    }
    let fallback;
    let web: &crate::web_ir::WebIrModule = match cached_web {
        Some(w) => w,
        None => {
            fallback = crate::web_ir::lower::project_web_from_core(hir);
            &fallback
        }
    };
    let diags = crate::web_ir::validate::validate_web_ir(web);
    if diags.is_empty() {
        return Ok(());
    }
    Err(format!(
        "VOX_WEBIR_VALIDATE: {}",
        crate::web_ir::validate::format_web_ir_validate_failure(&diags)
    ))
}
