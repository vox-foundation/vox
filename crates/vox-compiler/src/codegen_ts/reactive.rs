//! Path C reactive components → React TSX via `hir_emit`.
//!
//! **Web IR (OP-0193+):** when `VOX_WEBIR_EMIT_REACTIVE_VIEWS=1` or `true`, the `view:` body may be
//! taken from [`crate::web_ir::emit_tsx::emit_component_view_tsx`] after [`lower_hir_to_web_ir`](crate::web_ir::lower::lower_hir_to_web_ir)
//! **only if** [`validate_web_ir`](crate::web_ir::validate::validate_web_ir) is clean and the normalized
//! JSX matches the legacy [`emit_hir_expr`](crate::codegen_ts::hir_emit::emit_hir_expr) string (whitespace-insensitive parity guard).
//!
//! **Diagnostics:** `VOX_WEBIR_REACTIVE_TRACE=1` logs one stderr line per reactive view decision
//! (component name + pathway). Aggregate counts: [`reactive_view_bridge_stats`].
//!
//! **Behavior adapter (OP-S037):** `view:` bodies still flow through [`emit_block_stmts`] /
//! [`emit_hir_expr`] when the Web IR bridge is off or falls back; `behavior_nodes` / preview emit from
//! [`crate::web_ir`] is the structural mirror—keep pathway counters and parity guards updated together.
//!
//! **Route→behavior map (OP-S073) + notes B/C (OP-S163 / S195):** reactive `view:` bodies are keyed by component
//! name for [`crate::web_ir::emit_tsx::emit_component_view_tsx`] selection; do not rename without updating
//! [`crate::web_ir::WebIrModule::view_roots`] lowering.

use crate::codegen_ts::hir_emit::{
    emit_block_stmts, emit_hir_expr, extract_state_deps, map_hir_type_to_ts,
};
use crate::hir::*;
use crate::react_bridge::react_exports::{USE_EFFECT, USE_MEMO, USE_STATE};
use crate::web_ir::WebIrModule;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

fn web_ir_reactive_views_env_enabled() -> bool {
    std::env::var("VOX_WEBIR_EMIT_REACTIVE_VIEWS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn web_ir_reactive_trace_enabled() -> bool {
    std::env::var("VOX_WEBIR_REACTIVE_TRACE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Which codegen path selected the reactive `view:` body (Web IR bridge vs legacy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReactiveViewEmitPathway {
    /// `VOX_WEBIR_EMIT_REACTIVE_VIEWS` unset / not truthy.
    LegacyEnvDisabled,
    /// Env on, but Web IR module validation returned diagnostics.
    LegacyFallbackValidateFailed,
    /// Env on, validate clean, but no preview TSX for this component.
    LegacyFallbackNoComponentTsx,
    /// Env on, clean TSX emitted, but whitespace-normalized string ≠ legacy `emit_hir_expr`.
    LegacyFallbackParityMismatch,
    /// Web IR preview TSX used for the view body.
    WebIrViewEmitted,
}

static CTR_LEGACY_ENV: AtomicU64 = AtomicU64::new(0);
static CTR_VALIDATE_FALLBACK: AtomicU64 = AtomicU64::new(0);
static CTR_NO_TSX_FALLBACK: AtomicU64 = AtomicU64::new(0);
static CTR_PARITY_FALLBACK: AtomicU64 = AtomicU64::new(0);
static CTR_WEB_IR_SELECTED: AtomicU64 = AtomicU64::new(0);

fn record_pathway(p: ReactiveViewEmitPathway) {
    let c = match p {
        ReactiveViewEmitPathway::LegacyEnvDisabled => &CTR_LEGACY_ENV,
        ReactiveViewEmitPathway::LegacyFallbackValidateFailed => &CTR_VALIDATE_FALLBACK,
        ReactiveViewEmitPathway::LegacyFallbackNoComponentTsx => &CTR_NO_TSX_FALLBACK,
        ReactiveViewEmitPathway::LegacyFallbackParityMismatch => &CTR_PARITY_FALLBACK,
        ReactiveViewEmitPathway::WebIrViewEmitted => &CTR_WEB_IR_SELECTED,
    };
    c.fetch_add(1, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReactiveViewBridgeStats {
    pub legacy_env_disabled: u64,
    pub legacy_fallback_validate_failed: u64,
    pub legacy_fallback_no_component_tsx: u64,
    pub legacy_fallback_parity_mismatch: u64,
    pub web_ir_view_emitted: u64,
}

/// Snapshot of [`ReactiveViewEmitPathway`] tallies (process-wide, test with env mutex / single-threaded care).
pub fn reactive_view_bridge_stats() -> ReactiveViewBridgeStats {
    ReactiveViewBridgeStats {
        legacy_env_disabled: CTR_LEGACY_ENV.load(Ordering::Relaxed),
        legacy_fallback_validate_failed: CTR_VALIDATE_FALLBACK.load(Ordering::Relaxed),
        legacy_fallback_no_component_tsx: CTR_NO_TSX_FALLBACK.load(Ordering::Relaxed),
        legacy_fallback_parity_mismatch: CTR_PARITY_FALLBACK.load(Ordering::Relaxed),
        web_ir_view_emitted: CTR_WEB_IR_SELECTED.load(Ordering::Relaxed),
    }
}

/// Zero adoption counters. **`#[doc(hidden)]`**: intended for `tests/reactive_smoke.rs` only (integration tests
/// link this crate **without** `--cfg test`, so a `#[cfg(test)]` helper would not compile there).
#[doc(hidden)]
pub fn reset_reactive_view_bridge_stats_for_tests() {
    CTR_LEGACY_ENV.store(0, Ordering::Relaxed);
    CTR_VALIDATE_FALLBACK.store(0, Ordering::Relaxed);
    CTR_NO_TSX_FALLBACK.store(0, Ordering::Relaxed);
    CTR_PARITY_FALLBACK.store(0, Ordering::Relaxed);
    CTR_WEB_IR_SELECTED.store(0, Ordering::Relaxed);
}

/// Whitespace normalization for the reactive view parity guard (OP-0261 / OP-0179).
#[doc(hidden)]
pub fn normalize_reactive_view_jsx_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join("")
}

fn indent_view_for_return(view: &str) -> String {
    let pad = "    ";
    view.trim_end()
        .lines()
        .map(|line| {
            let t = line.trim_end();
            if t.is_empty() {
                String::new()
            } else {
                format!("{pad}{t}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn emit_reactive_view_body(
    component_name: &str,
    hir: &HirModule,
    rc: &HirReactiveComponent,
    state_names: &HashSet<String>,
    island_names: &HashSet<String>,
    web_projection: Option<&WebIrModule>,
) -> String {
    let Some(view) = &rc.view else {
        return String::new();
    };
    let legacy = emit_hir_expr(view, state_names, island_names);
    if !web_ir_reactive_views_env_enabled() {
        record_pathway(ReactiveViewEmitPathway::LegacyEnvDisabled);
        if web_ir_reactive_trace_enabled() {
            eprintln!("[vox-webir-reactive] component={component_name} pathway=LegacyEnvDisabled");
        }
        return legacy;
    }
    let owned_web;
    let web: &WebIrModule = if let Some(w) = web_projection {
        w
    } else {
        owned_web = crate::web_ir::lower::lower_hir_to_web_ir(hir);
        &owned_web
    };
    if !crate::web_ir::validate::validate_web_ir(web).is_empty() {
        record_pathway(ReactiveViewEmitPathway::LegacyFallbackValidateFailed);
        if web_ir_reactive_trace_enabled() {
            eprintln!(
                "[vox-webir-reactive] component={component_name} pathway=LegacyFallbackValidateFailed"
            );
        }
        return legacy;
    }
    let Some(tsx) = crate::web_ir::emit_tsx::emit_component_view_tsx(web, &rc.name) else {
        record_pathway(ReactiveViewEmitPathway::LegacyFallbackNoComponentTsx);
        if web_ir_reactive_trace_enabled() {
            eprintln!(
                "[vox-webir-reactive] component={component_name} pathway=LegacyFallbackNoComponentTsx"
            );
        }
        return legacy;
    };
    let n_legacy = normalize_reactive_view_jsx_ws(&legacy);
    let n_tsx = normalize_reactive_view_jsx_ws(&tsx);
    if n_legacy == n_tsx {
        record_pathway(ReactiveViewEmitPathway::WebIrViewEmitted);
        if web_ir_reactive_trace_enabled() {
            eprintln!("[vox-webir-reactive] component={component_name} pathway=WebIrViewEmitted");
        }
        indent_view_for_return(&tsx)
    } else {
        record_pathway(ReactiveViewEmitPathway::LegacyFallbackParityMismatch);
        if web_ir_reactive_trace_enabled() {
            eprintln!(
                "[vox-webir-reactive] component={component_name} pathway=LegacyFallbackParityMismatch"
            );
        }
        legacy
    }
}

fn react_import_line(members: &[HirReactiveMember]) -> String {
    let mut need_state = false;
    let mut need_effect = false;
    let mut need_memo = false;
    for m in members {
        match m {
            HirReactiveMember::State(_) => need_state = true,
            HirReactiveMember::Derived(_) => need_memo = true,
            HirReactiveMember::Effect(_)
            | HirReactiveMember::OnMount(_)
            | HirReactiveMember::OnCleanup(_) => need_effect = true,
        }
    }
    let mut hooks = Vec::new();
    if need_state {
        hooks.push(USE_STATE);
    }
    if need_effect {
        hooks.push(USE_EFFECT);
    }
    if need_memo {
        hooks.push(USE_MEMO);
    }
    if hooks.is_empty() {
        return "import React from \"react\";\n\n".to_string();
    }
    format!(
        "import React, {{ {} }} from \"react\";\n\n",
        hooks.join(", ")
    )
}

/// `island_names` should be [`crate::codegen_ts::island_emit::collect_island_names`] for the enclosing [`crate::hir::HirModule`].
///
/// `hir` must be the full module (needed for optional Web IR view bridge).
///
/// When `web_projection` is `Some`, it must be [`crate::web_ir::lower::lower_hir_to_web_ir`]`(hir)` (or
/// [`crate::web_ir::lower::project_web_from_core`]) so reactive emit avoids N× full-module lowers.
pub fn generate_reactive_component(
    hir: &HirModule,
    rc: &HirReactiveComponent,
    island_names: &HashSet<String>,
    web_projection: Option<&WebIrModule>,
) -> (String, String) {
    let name = &rc.name;
    let filename = format!("{name}.tsx");
    let mut out = String::new();

    let mut state_names = HashSet::new();
    for member in &rc.members {
        if let HirReactiveMember::State(s) = member {
            state_names.insert(s.name.clone());
        }
    }

    out.push_str(&react_import_line(&rc.members));

    if !rc.params.is_empty() {
        out.push_str(&format!("export interface {name}Props {{\n"));
        for param in &rc.params {
            let ts_type = param
                .type_ann
                .as_ref()
                .map_or("any".to_string(), map_hir_type_to_ts);
            out.push_str(&format!("  {}: {};\n", param.name, ts_type));
        }
        out.push_str("}\n\n");
    }

    if rc.params.is_empty() {
        out.push_str(&format!(
            "export function {name}(): React.ReactElement {{\n"
        ));
    } else {
        let param_names: Vec<String> = rc.params.iter().map(|p| p.name.clone()).collect();
        out.push_str(&format!(
            "export function {name}({{ {} }}: {name}Props): React.ReactElement {{\n",
            param_names.join(", ")
        ));
    }

    for member in &rc.members {
        match member {
            HirReactiveMember::State(s) => {
                let init = emit_hir_expr(&s.init, &state_names, island_names);
                out.push_str(&format!(
                    "  const [{}, set_{}] = useState({});\n",
                    s.name, s.name, init
                ));
            }
            HirReactiveMember::Derived(d) => {
                let expr = emit_hir_expr(&d.expr, &state_names, island_names);
                let deps = extract_state_deps(&d.expr, &state_names);
                let dep_str = deps.join(", ");
                out.push_str(&format!(
                    "  const {} = useMemo(() => {}, [{}]);\n",
                    d.name, expr, dep_str
                ));
            }
            HirReactiveMember::Effect(e) => {
                let stmts_str = emit_block_stmts(&e.body, &state_names, island_names, 2);
                let deps = extract_state_deps(&e.body, &state_names);
                let dep_str = deps.join(", ");
                out.push_str(&format!(
                    "  useEffect(() => {{\n{}  }}, [{}]);\n",
                    stmts_str, dep_str
                ));
            }
            HirReactiveMember::OnMount(m) => {
                let stmts_str = emit_block_stmts(&m.body, &state_names, island_names, 2);
                out.push_str(&format!("  useEffect(() => {{\n{}  }}, []);\n", stmts_str));
            }
            HirReactiveMember::OnCleanup(c) => {
                let stmts_str = emit_block_stmts(&c.body, &state_names, island_names, 2);
                out.push_str(&format!(
                    "  useEffect(() => () => {{\n{}  }}, []);\n",
                    stmts_str
                ));
            }
        }
    }

    if rc.view.is_some() {
        let view_js =
            emit_reactive_view_body(name, hir, rc, &state_names, island_names, web_projection);
        out.push_str(&format!("  return (\n{}\n  );\n", view_js));
    }

    out.push_str("}\n");
    (filename, out)
}
