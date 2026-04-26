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
use crate::codegen_ts::adt::generate_types;

use crate::codegen_ts::island_emit::collect_island_names;
use crate::codegen_ts::routes::generate_routes;
use crate::codegen_ts::tanstack_query_emit::vox_tanstack_query_tsx;
use crate::codegen_ts::vox_client::{VOX_CLIENT_FILENAME, emit_vox_client};
use crate::hir::{HirFn, HirModule};

/// Output from the TypeScript code generator.
pub struct CodegenOutput {
    /// List of (filename, content) pairs.
    pub files: Vec<(String, String)>,
    /// Web IR bridge emit statistics
    pub reactive_stats: crate::codegen_ts::reactive::ReactiveViewBridgeStats,
}

/// Build mode target for codegen.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BuildMode {
    /// Emit app code + components (default)
    #[default]
    App,
    /// Emit UI-agnostic models, schemas, and client fetchers
    Library,
}

/// Options for [`generate_with_options`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CodegenOptions {
    /// Legacy flag (TanStack Start tree emission). **Ignored** — `routes:` now emits [`routes.manifest.ts`] only.
    pub tanstack_start: bool,
    /// Build Target
    pub target: Option<String>,
    /// Build Mode
    pub mode: BuildMode,
}

impl CodegenOptions {
    /// Reads **`VOX_WEB_TANSTACK_START`** for callers that still thread the flag (**ignored** for TS emit —
    /// route output is always [`route_manifest`] + components).
    #[must_use]
    pub fn from_env() -> Self {
        let tanstack_start_resolved =
            vox_clavis::resolve_secret(vox_clavis::SecretId::VoxWebTanstackStart);
        Self {
            tanstack_start: tanstack_start_resolved
                .expose()
                .is_some_and(|v: &str| v == "1" || v.eq_ignore_ascii_case("true")),
            target: None,
            mode: BuildMode::App,
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
    let reactive_stats = crate::codegen_ts::reactive::ReactiveViewBridgeStats::default();
    let _island_names = collect_island_names(hir);
    let app_contract = project_app_contract(hir);

    // Generate type definitions
    let types_content = generate_types(hir);
    let has_types = !types_content.is_empty();
    if has_types {
        files.push(("types.ts".to_string(), types_content));
    }

    // Generate typed URL declarations
    let url_content = crate::codegen_ts::url_emit::emit_url_decls(hir);
    if !url_content.is_empty() {
        files.push(("urls.ts".to_string(), url_content));
    }
    if let Ok(contract_json) = serde_json::to_string_pretty(&app_contract) {
        files.push(("vox-app-contract.json".to_string(), contract_json));
    }

    if options.mode != BuildMode::Library {
        files.push((
            "vox-tanstack-query.tsx".to_string(),
            vox_tanstack_query_tsx(),
        ));
    }

    // Generate Express server only when explicitly requested (Axum + api.ts is canonical).
    if options.mode != BuildMode::Library {
        let routes_content = generate_routes(hir);
        let emit_express_resolved =
            vox_clavis::resolve_secret(vox_clavis::SecretId::VoxEmitExpressServer);
        if !routes_content.is_empty()
            && emit_express_resolved
                .expose()
                .is_some_and(|v: &str| v == "1" || v.eq_ignore_ascii_case("true"))
        {
            files.push(("server.ts".to_string(), routes_content));
        }

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

    let zod_schemas = crate::codegen_ts::zod_emit::generate_zod_schemas(hir);
    let has_schemas = !zod_schemas.is_empty();
    if has_schemas {
        files.push(("schemas.ts".to_string(), zod_schemas));
    }

    // Typed fetch client for `@query` (GET + JSON query values) / `@mutation` / `@server` (POST JSON).
    let has_api_fns = !hir.endpoint_fns.is_empty();
    if has_api_fns {
        files.push((VOX_CLIENT_FILENAME.to_string(), emit_vox_client(hir)));
    }


    // Load vox.tokens.json via TokenRegistry: emits typed CSS + TS and validates token refs.
    let token_registry =
        crate::tokens::TokenRegistry::load_from_str(
            &std::fs::read_to_string("vox.tokens.json").unwrap_or_default(),
        )
        .ok();
    if let Some(ref reg) = token_registry {
        files.push((
            "vox-tokens.css".to_string(),
            crate::codegen_ts::tokens_emit::emit_tokens_css(reg),
        ));
        files.push((
            "tokens.ts".to_string(),
            crate::codegen_ts::tokens_emit::emit_tokens_ts(reg),
        ));
    }

    let web_projection = crate::web_ir::lower::project_web_from_core(hir);
    maybe_web_ir_validate(hir, Some(&web_projection), token_registry.as_ref())?;

    let (manifest_filename, route_manifest) = if options.mode == BuildMode::Library {
        (
            "routes.manifest.json",
            crate::codegen_ts::route_manifest::try_emit_route_manifest_json_from_web_ir(
                &web_projection, hir,
            )?,
        )
    } else {
        (
            "routes.manifest.ts",
            crate::codegen_ts::route_manifest::try_emit_route_manifest_from_web_ir(&web_projection, hir)?,
        )
    };
    if let Some(manifest) = route_manifest {
        files.push((manifest_filename.to_string(), manifest));
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

    if options.mode != BuildMode::Library {
        // Generate mobile native bridge
        let mobile_fns: Vec<&HirFn> = hir
            .functions
            .iter()
            .filter(|f| f.is_mobile_native)
            .collect();
        if !mobile_fns.is_empty() {
            let mut mobile_bridge =
                String::from("// Mobile native bridge generated by Vox compiler\n");
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

    if options.mode == BuildMode::Library {
        let package_json = serde_json::json!({
            "name": "vox-generated-api",
            "version": "0.1.0",
            "type": "module",
            "main": "./index.ts",
            "exports": {
                ".": "./index.ts"
            },
            "peerDependencies": {
                "zod": "^3.22.4"
            }
        });
        files.push((
            "package.json".to_string(),
            serde_json::to_string_pretty(&package_json).unwrap(),
        ));

        let mut index_ts = String::new();
        if has_types {
            index_ts.push_str("export * from \"./types\";\n");
        }
        if has_schemas {
            index_ts.push_str("export * from \"./schemas\";\n");
        }
        if has_api_fns {
            index_ts.push_str("export * from \"./vox-client\";\n");
        }
        files.push(("index.ts".to_string(), index_ts));
    }

    Ok(CodegenOutput {
        files,
        reactive_stats,
    })
}

/// WebIR lower + validate gate (OP-0113, OP-0124). **On by default;** set `VOX_WEBIR_VALIDATE=0` / `false` /
/// `no` / `off` to skip.
///
/// When a [`crate::tokens::TokenRegistry`] is supplied, token reference resolution and
/// WCAG contrast validation run as part of the style stage (TASK-4.4).
fn maybe_web_ir_validate(
    hir: &HirModule,
    cached_web: Option<&crate::web_ir::WebIrModule>,
    registry: Option<&crate::tokens::TokenRegistry>,
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
    let diags = crate::web_ir::validate::validate_web_ir_with_registry(web, registry);
    if diags.is_empty() {
        return Ok(());
    }
    Err(format!(
        "VOX_WEBIR_VALIDATE: {}",
        crate::web_ir::validate::format_web_ir_validate_failure(&diags)
    ))
}
