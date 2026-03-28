//! HIR → TypeScript file bundle (production path). **WebIR bridge (OP-S025):** after assembling
//! artifacts, [`maybe_web_ir_validate`] may lower + validate [`crate::web_ir::WebIrModule`] when
//! `VOX_WEBIR_VALIDATE=1`, so CI can gate structural errors without routing all emit through preview TSX.
//!
//! **Fallback mode (OP-S027):** when that env var is unset, validation is skipped and codegen follows the
//! historical fast path (WebIR used only by explicit tooling / tests).
//!
//! **Style + route printer bridge (OP-S059 / S091 / S111 / S137 / S171 / S199):** classic CSS emission and
//! TanStack route files are still assembled here alongside [`super::routes`]; migrating printers to consume
//! only validated [`crate::web_ir::WebIrModule`] slices is tracked in the internal Web IR blueprint.

use crate::codegen_ts::activity::{generate_activity_hir, generate_activity_runner};
use crate::codegen_ts::adt::generate_types;
use crate::codegen_ts::component::generate_component;
use crate::codegen_ts::island_emit::collect_island_names;
use crate::codegen_ts::reactive::generate_reactive_component;
use crate::codegen_ts::routes::generate_routes;
use crate::codegen_ts::tanstack_programmatic_routes::push_route_tree_files;
use crate::codegen_ts::tanstack_query_emit::vox_tanstack_query_tsx;
use crate::codegen_ts::tanstack_start::{
    CREATE_SERVER_FN, CREATE_SERVER_FN_PKG, FETCH_CONTENT_TYPE, SERVER_FN_HTTP_METHOD,
    SERVER_FNS_FILENAME,
};
use crate::hir::HirModule;

/// Output from the TypeScript code generator.
pub struct CodegenOutput {
    /// List of (filename, content) pairs.
    pub files: Vec<(String, String)>,
}

/// Options for [`generate_with_options`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CodegenOptions {
    /// When true, `routes:` emits [`VoxTanStackRouter.tsx`] exporting **`voxRouteTree`** (no `RouterProvider`).
    /// Use with TanStack Start so `getRouter()` in the Vite app owns the single router instance.
    /// When false, emits [`App.tsx`] with `RouterProvider` for the SPA + `index.html` shell.
    pub tanstack_start: bool,
}

impl CodegenOptions {
    /// `VOX_WEB_TANSTACK_START=1` or `true` enables TanStack Start route-tree emission.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            tanstack_start: std::env::var("VOX_WEB_TANSTACK_START")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
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

    // Generate type definitions
    let types_content = generate_types(hir);
    if !types_content.is_empty() {
        files.push(("types.ts".to_string(), types_content));
    }

    files.push((
        "vox-tanstack-query.tsx".to_string(),
        vox_tanstack_query_tsx(),
    ));

    // Generate components
    for hir_comp in &hir.components {
        let comp = &hir_comp.0;
        let (filename, content) =
            generate_component(&comp.func, !comp.styles.is_empty(), &island_names);
        files.push((filename, content));
    }

    // Generate reactive components (Path C). Optional `VOX_WEBIR_EMIT_REACTIVE_VIEWS=1` uses Web IR
    // preview emit for `view:` when validate is clean and whitespace-normalized JSX matches legacy.
    let web_projection_cache = if hir.reactive_components.is_empty() {
        None
    } else {
        Some(crate::web_ir::lower::project_web_from_core(hir))
    };
    let web_projection_ref = web_projection_cache.as_ref();
    for rc in &hir.reactive_components {
        let (filename, content) =
            generate_reactive_component(hir, rc, &island_names, web_projection_ref);
        files.push((filename, content));
    }

    // Route loading / suspense UI (`@loading fn … to Element`) — TanStack `pendingComponent`
    for hir_loading in &hir.loadings {
        let (filename, content) = generate_component(&hir_loading.0.func, false, &island_names);
        files.push((filename, content));
    }

    // Generate v0 component placeholders
    for hir_v0 in &hir.v0_components {
        let v0 = &hir_v0.0;
        let filename = format!("{}.tsx", v0.name);

        let prompt_comment = if !v0.prompt.is_empty() {
            format!("Prompt: {}", v0.prompt)
        } else if let Some(ref img) = v0.image_path {
            format!("From image: {}", img)
        } else {
            "No prompt provided".to_string()
        };

        let content = format!(
            "// @v0 generated component\n// {}\nimport React from \"react\";\n\nexport function {}(): React.ReactElement {{\n  return <div>{{/* AI component definition pending API integration */}}</div>;\n}}\n",
            prompt_comment, v0.name
        );
        files.push((filename, content));
    }

    // Generate Express server only when explicitly requested (Axum + api.ts is canonical).
    let routes_content = generate_routes(hir);
    if !routes_content.is_empty()
        && std::env::var("VOX_EMIT_EXPRESS_SERVER")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
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

    // Generate TanStack Start Server Functions from HIR
    let has_api_fns =
        !hir.server_fns.is_empty() || !hir.query_fns.is_empty() || !hir.mutation_fns.is_empty();
    if has_api_fns && options.tanstack_start {
        let mut server_fns_out = String::new();
        server_fns_out
            .push_str("// Server Functions generated by Vox compiler for TanStack Start\n");
        server_fns_out.push_str(&format!(
            "import {{ {CREATE_SERVER_FN} }} from \"{CREATE_SERVER_FN_PKG}\";\n\n",
        ));
        for sf in hir
            .server_fns
            .iter()
            .chain(hir.query_fns.iter())
            .chain(hir.mutation_fns.iter())
        {
            let name = &sf.name;
            let params: Vec<String> = sf
                .params
                .iter()
                .map(|p| {
                    let ty = p.type_ann.as_ref().map_or(
                        "any".to_string(),
                        crate::codegen_ts::hir_emit::map_hir_type_to_ts,
                    );
                    format!("{}: {}", p.name, ty)
                })
                .collect();
            let return_type = sf.return_type.as_ref().map_or(
                "any".to_string(),
                crate::codegen_ts::hir_emit::map_hir_type_to_ts,
            );
            server_fns_out.push_str(&format!(
                "export const {name} = {CREATE_SERVER_FN}({{ method: '{SERVER_FN_HTTP_METHOD}' }}).handler(async (data: {{ {} }}) => {{\n",
                params.join(", ")
            ));
            server_fns_out.push_str(&format!(
                "  const response = await fetch(\"{}\", {{\n",
                sf.route_path
            ));
            server_fns_out.push_str(&format!("    method: '{SERVER_FN_HTTP_METHOD}',\n"));
            server_fns_out.push_str(&format!(
                "    headers: {{ 'Content-Type': '{FETCH_CONTENT_TYPE}' }},\n",
            ));
            server_fns_out.push_str("    body: JSON.stringify(data),\n");
            server_fns_out.push_str("  });\n");
            server_fns_out
                .push_str("  if (!response.ok) throw new Error(\"Server function failed\");\n");
            server_fns_out.push_str(&format!(
                "  return response.json() as Promise<{return_type}>;\n"
            ));
            server_fns_out.push_str("});\n\n");
        }
        files.push((SERVER_FNS_FILENAME.to_string(), server_fns_out));
    }

    // Generate scoped CSS modules for components with style blocks
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

    push_route_tree_files(&mut files, hir, options.tanstack_start);

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

    maybe_web_ir_validate(hir)?;

    Ok(CodegenOutput { files })
}

/// Optional WebIR lower + validate gate (OP-0113, OP-0124). Set `VOX_WEBIR_VALIDATE=1` to fail
/// codegen when [`crate::web_ir::validate::validate_web_ir`] returns diagnostics.
fn maybe_web_ir_validate(hir: &HirModule) -> Result<(), String> {
    let enabled = std::env::var("VOX_WEBIR_VALIDATE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !enabled {
        return Ok(());
    }
    let web = crate::web_ir::lower::project_web_from_core(hir);
    let diags = crate::web_ir::validate::validate_web_ir(&web);
    if diags.is_empty() {
        return Ok(());
    }
    Err(format!(
        "VOX_WEBIR_VALIDATE: {}",
        crate::web_ir::validate::format_web_ir_validate_failure(&diags)
    ))
}
