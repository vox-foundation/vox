//! Path C reactive components → React TSX via `hir_emit`.
//!
//! **Web IR (OP-0193+):** by default, the `view:` body may be taken from
//! [`crate::web_ir::emit_tsx::emit_component_view_tsx`] after [`lower_hir_to_web_ir`](crate::web_ir::lower::lower_hir_to_web_ir)
//! **only if** [`validate_web_ir`](crate::web_ir::validate::validate_web_ir) is clean and the normalized
//! JSX matches the legacy [`emit_hir_expr`](crate::codegen_ts::hir_emit::emit_hir_expr) string (whitespace-insensitive parity guard).
//! Set `VOX_WEBIR_EMIT_REACTIVE_VIEWS=0` / `false` / `no` / `off` to force legacy `emit_hir_expr` for views.
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
    emit_block_stmts, emit_hir_expr, emit_hir_stmt, extract_state_deps, map_hir_type_to_ts,
};
use crate::hir::*;
use crate::react_bridge::react_exports::{USE_CALLBACK, USE_EFFECT, USE_MEMO, USE_REF, USE_STATE};
use crate::web_ir::WebIrModule;
use std::collections::HashSet;

fn web_ir_reactive_views_env_enabled() -> bool {
    crate::web_migration_env::web_ir_emit_reactive_views_enabled()
}

fn web_ir_reactive_trace_enabled() -> bool {
    std::env::var("VOX_WEBIR_REACTIVE_TRACE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Which codegen path selected the reactive `view:` body (Web IR bridge vs legacy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReactiveViewEmitPathway {
    /// `VOX_WEBIR_EMIT_REACTIVE_VIEWS` explicitly disabled (`0` / `false` / `no` / `off`).
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReactiveViewBridgeStats {
    pub legacy_env_disabled: u64,
    pub legacy_fallback_validate_failed: u64,
    pub legacy_fallback_no_component_tsx: u64,
    pub legacy_fallback_parity_mismatch: u64,
    pub web_ir_view_emitted: u64,
}

impl ReactiveViewBridgeStats {
    pub fn record_pathway(&mut self, p: ReactiveViewEmitPathway) {
        match p {
            ReactiveViewEmitPathway::LegacyEnvDisabled => self.legacy_env_disabled += 1,
            ReactiveViewEmitPathway::LegacyFallbackValidateFailed => {
                self.legacy_fallback_validate_failed += 1
            }
            ReactiveViewEmitPathway::LegacyFallbackNoComponentTsx => {
                self.legacy_fallback_no_component_tsx += 1
            }
            ReactiveViewEmitPathway::LegacyFallbackParityMismatch => {
                self.legacy_fallback_parity_mismatch += 1
            }
            ReactiveViewEmitPathway::WebIrViewEmitted => self.web_ir_view_emitted += 1,
        }
    }
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
    stats: &mut ReactiveViewBridgeStats,
) -> String {
    let Some(view) = &rc.view else {
        return String::new();
    };
    let legacy = emit_hir_expr(view, state_names, island_names);
    if !web_ir_reactive_views_env_enabled() {
        stats.record_pathway(ReactiveViewEmitPathway::LegacyEnvDisabled);
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
        stats.record_pathway(ReactiveViewEmitPathway::LegacyFallbackValidateFailed);
        if web_ir_reactive_trace_enabled() {
            eprintln!(
                "[vox-webir-reactive] component={component_name} pathway=LegacyFallbackValidateFailed"
            );
        }
        return legacy;
    }
    let Some(tsx) = crate::web_ir::emit_tsx::emit_component_view_tsx(web, &rc.name) else {
        stats.record_pathway(ReactiveViewEmitPathway::LegacyFallbackNoComponentTsx);
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
        stats.record_pathway(ReactiveViewEmitPathway::WebIrViewEmitted);
        if web_ir_reactive_trace_enabled() {
            eprintln!("[vox-webir-reactive] component={component_name} pathway=WebIrViewEmitted");
        }
        indent_view_for_return(&tsx)
    } else {
        stats.record_pathway(ReactiveViewEmitPathway::LegacyFallbackParityMismatch);
        if web_ir_reactive_trace_enabled() {
            eprintln!(
                "[vox-webir-reactive] component={component_name} pathway=LegacyFallbackParityMismatch"
            );
        }
        legacy
    }
}

fn scan_hir_expr_for_react_imports(
    e: &HirExpr,
    need_state: &mut bool,
    need_effect: &mut bool,
    need_memo: &mut bool,
    need_ref: &mut bool,
    need_callback: &mut bool,
) {
    match e {
        HirExpr::Call(callee, args, _, _) => {
            if let HirExpr::Ident(name, _) = callee.as_ref() {
                match name.as_str() {
                    "use_state" => *need_state = true,
                    "use_effect" | "use_layout_effect" => *need_effect = true,
                    "use_memo" => *need_memo = true,
                    "use_ref" => *need_ref = true,
                    "use_callback" => *need_callback = true,
                    _ => {}
                }
            }
            for a in args {
                scan_hir_expr_for_react_imports(
                    &a.value,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        HirExpr::Binary(_, l, r, _) => {
            scan_hir_expr_for_react_imports(
                l,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            scan_hir_expr_for_react_imports(
                r,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        HirExpr::Unary(_, x, _) => {
            scan_hir_expr_for_react_imports(
                x,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        HirExpr::MethodCall(recv, _, args, _, _) => {
            scan_hir_expr_for_react_imports(
                recv,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            for a in args {
                scan_hir_expr_for_react_imports(
                    &a.value,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        HirExpr::FieldAccess(b, _, _) => {
            scan_hir_expr_for_react_imports(
                b,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        HirExpr::ListLit(items, _) | HirExpr::TupleLit(items, _) => {
            for x in items {
                scan_hir_expr_for_react_imports(
                    x,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, x) in fields {
                scan_hir_expr_for_react_imports(
                    x,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        HirExpr::Block(stmts, _) => {
            for s in stmts {
                scan_hir_stmt_for_react_imports(
                    s,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        HirExpr::Lambda(_, _, body, _) => {
            scan_hir_expr_for_react_imports(
                body,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        HirExpr::If(cond, then_b, else_b, _) => {
            scan_hir_expr_for_react_imports(
                cond,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            for s in then_b {
                scan_hir_stmt_for_react_imports(
                    s,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
            if let Some(els) = else_b {
                for s in els {
                    scan_hir_stmt_for_react_imports(
                        s,
                        need_state,
                        need_effect,
                        need_memo,
                        need_ref,
                        need_callback,
                    );
                }
            }
        }
        HirExpr::Match(scr, arms, _) => {
            scan_hir_expr_for_react_imports(
                scr,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            for arm in arms {
                if let Some(g) = &arm.guard {
                    scan_hir_expr_for_react_imports(
                        g,
                        need_state,
                        need_effect,
                        need_memo,
                        need_ref,
                        need_callback,
                    );
                }
                scan_hir_expr_for_react_imports(
                    arm.body.as_ref(),
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        HirExpr::For(_, _, it, body, _) => {
            scan_hir_expr_for_react_imports(
                it,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            scan_hir_expr_for_react_imports(
                body,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        HirExpr::With(a, b, _) => {
            scan_hir_expr_for_react_imports(
                a,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            scan_hir_expr_for_react_imports(
                b,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        HirExpr::Spawn(x, _) => {
            scan_hir_expr_for_react_imports(
                x,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }

        HirExpr::Try(t) => {
            scan_hir_expr_for_react_imports(
                t.target.as_ref(),
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        HirExpr::JsxSelfClosing(_)
        | HirExpr::Jsx(_)
        | HirExpr::IntLit(_, _)
        | HirExpr::FloatLit(_, _)
        | HirExpr::StringLit(_, _)
        | HirExpr::BoolLit(_, _)
        | HirExpr::DecimalLit(_, _)
        | HirExpr::Ident(_, _) => {}
    }
}

fn scan_hir_stmt_for_react_imports(
    s: &HirStmt,
    need_state: &mut bool,
    need_effect: &mut bool,
    need_memo: &mut bool,
    need_ref: &mut bool,
    need_callback: &mut bool,
) {
    match s {
        HirStmt::Let { value, .. } => {
            scan_hir_expr_for_react_imports(
                value,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        HirStmt::Assign { target, value, .. } => {
            scan_hir_expr_for_react_imports(
                target,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            scan_hir_expr_for_react_imports(
                value,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        HirStmt::Expr { expr, .. } => {
            scan_hir_expr_for_react_imports(
                expr,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                scan_hir_expr_for_react_imports(
                    v,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        HirStmt::While {
            condition, body, ..
        } => {
            scan_hir_expr_for_react_imports(
                condition,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            for x in body {
                scan_hir_stmt_for_react_imports(
                    x,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        HirStmt::Loop { body, .. } => {
            for x in body {
                scan_hir_stmt_for_react_imports(
                    x,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        HirStmt::Break { .. } | HirStmt::Continue { .. } => {}
    }
}

fn collect_reactive_binding_names(members: &[HirReactiveMember]) -> HashSet<String> {
    fn pat_names(pat: &HirPattern, out: &mut HashSet<String>) {
        match pat {
            HirPattern::Ident(n, _) => {
                out.insert(n.clone());
            }
            HirPattern::Tuple(items, _) => {
                for p in items {
                    pat_names(p, out);
                }
            }
            HirPattern::Constructor(_, items, _) => {
                for p in items {
                    pat_names(p, out);
                }
            }
            HirPattern::Wildcard(_) | HirPattern::Literal(_, _) => {}
        }
    }
    fn stmt_names(s: &HirStmt, out: &mut HashSet<String>) {
        match s {
            HirStmt::Let { pattern, .. } => pat_names(pattern, out),
            HirStmt::While { body, .. } | HirStmt::Loop { body, .. } => {
                for x in body {
                    stmt_names(x, out);
                }
            }
            _ => {}
        }
    }

    let mut names = HashSet::new();
    for m in members {
        match m {
            HirReactiveMember::State(s) => {
                names.insert(s.name.clone());
            }
            HirReactiveMember::Stmt(s) => stmt_names(s, &mut names),
            _ => {}
        }
    }
    names
}

fn react_import_line(members: &[HirReactiveMember]) -> String {
    let mut need_state = false;
    let mut need_effect = false;
    let mut need_memo = false;
    let mut need_ref = false;
    let mut need_callback = false;
    for m in members {
        match m {
            HirReactiveMember::State(_) => need_state = true,
            HirReactiveMember::Derived(_) => need_memo = true,
            HirReactiveMember::Effect(_)
            | HirReactiveMember::OnMount(_)
            | HirReactiveMember::OnCleanup(_) => need_effect = true,
            HirReactiveMember::Stmt(s) => {
                scan_hir_stmt_for_react_imports(
                    s,
                    &mut need_state,
                    &mut need_effect,
                    &mut need_memo,
                    &mut need_ref,
                    &mut need_callback,
                );
            }
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
    if need_ref {
        hooks.push(USE_REF);
    }
    if need_callback {
        hooks.push(USE_CALLBACK);
    }
    if hooks.is_empty() {
        return "import React from \"react\";\n\n".to_string();
    }
    format!(
        "import React, {{ {} }} from \"react\";\n\n",
        hooks.join(", ")
    )
}

/// Walk an HIR expression tree and collect uppercase JSX tag names that correspond
/// to known Vox components. Used to emit cross-component import statements.
fn collect_jsx_component_refs(
    expr: &HirExpr,
    known: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    match expr {
        HirExpr::Jsx(el) => {
            if el.tag.starts_with(|c: char| c.is_uppercase()) && known.contains(&el.tag) {
                out.insert(el.tag.clone());
            }
            for child in &el.children {
                collect_jsx_component_refs(child, known, out);
            }
        }
        HirExpr::JsxSelfClosing(el) => {
            if el.tag.starts_with(|c: char| c.is_uppercase()) && known.contains(&el.tag) {
                out.insert(el.tag.clone());
            }
        }
        HirExpr::If(cond, then_stmts, else_stmts, _) => {
            collect_jsx_component_refs(cond, known, out);
            for s in then_stmts {
                collect_jsx_component_refs_stmt(s, known, out);
            }
            if let Some(stmts) = else_stmts {
                for s in stmts {
                    collect_jsx_component_refs_stmt(s, known, out);
                }
            }
        }
        HirExpr::Block(stmts, _) => {
            for s in stmts {
                collect_jsx_component_refs_stmt(s, known, out);
            }
        }
        HirExpr::For(_, _, iter, body, _) => {
            collect_jsx_component_refs(iter, known, out);
            collect_jsx_component_refs(body, known, out);
        }
        _ => {}
    }
}

fn collect_jsx_component_refs_stmt(
    stmt: &HirStmt,
    known: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    match stmt {
        HirStmt::Expr { expr, .. } => collect_jsx_component_refs(expr, known, out),
        HirStmt::Let { value, .. } => collect_jsx_component_refs(value, known, out),
        HirStmt::Assign { value, .. } => collect_jsx_component_refs(value, known, out),
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_jsx_component_refs(v, known, out);
            }
        }
        _ => {}
    }
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
    stats: &mut ReactiveViewBridgeStats,
) -> (String, String) {
    let name = &rc.name;
    let filename = format!("{name}.tsx");
    let mut out = String::new();

    let state_names = collect_reactive_binding_names(&rc.members);

    out.push_str(&react_import_line(&rc.members));

    // Emit import statements for other Vox components referenced in the view.
    let known_components: HashSet<String> =
        hir.components.iter().map(|c| c.name.clone()).collect();
    let mut comp_refs: HashSet<String> = HashSet::new();
    if let Some(view_expr) = &rc.view {
        collect_jsx_component_refs(view_expr, &known_components, &mut comp_refs);
    }
    // Also walk the view inside members (e.g. inline JSX in state initialisers).
    for m in &rc.members {
        if let HirReactiveMember::Stmt(s) = m {
            collect_jsx_component_refs_stmt(s, &known_components, &mut comp_refs);
        }
    }
    let mut sorted_refs: Vec<String> = comp_refs.into_iter().collect();
    sorted_refs.sort();
    for comp in &sorted_refs {
        out.push_str(&format!("import {{ {comp} }} from \"./{comp}\";\n"));
    }
    if !sorted_refs.is_empty() {
        out.push('\n');
    }

    if !rc.styles.is_empty() {
        out.push_str(&format!("import \"./{name}.css\";\n\n"));
    }

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
            HirReactiveMember::Stmt(s) => {
                out.push_str(&emit_hir_stmt(s, &state_names, island_names, 2));
            }
        }
    }

    if rc.view.is_some() {
        let view_js = emit_reactive_view_body(
            name,
            hir,
            rc,
            &state_names,
            island_names,
            web_projection,
            stats,
        );
        out.push_str(&format!("  return (\n{}\n  );\n", view_js));
    }

    out.push_str("}\n");
    (filename, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::lower::lower_module;
    use crate::lexer::lex;
    use crate::parser::parse;

    fn compile(src: &str) -> Vec<(String, String)> {
        let tokens = lex(src);
        let module = parse(tokens).expect("parse error");
        let hir = lower_module(&module);
        let island_names = HashSet::new();
        let mut stats = ReactiveViewBridgeStats::default();
        hir.components
            .iter()
            .map(|rc| generate_reactive_component(&hir, rc, &island_names, None, &mut stats))
            .collect()
    }

    fn get(files: &[(String, String)], name: &str) -> String {
        files
            .iter()
            .find(|(f, _)| f == name)
            .map(|(_, c)| c.clone())
            .unwrap_or_default()
    }

    #[test]
    fn test_cross_component_import_emitted() {
        let files = compile(
            "component Inner() { view: (<panel><text>\"hi\"</text></panel>) }\n\
             component Outer() { view: (<column><Inner /></column>) }",
        );
        let outer = get(&files, "Outer.tsx");
        assert!(
            outer.contains("import { Inner } from \"./Inner\";"),
            "expected import for Inner in Outer.tsx, got:\n{outer}"
        );
    }

    #[test]
    fn test_no_import_for_html_primitives() {
        let files = compile("component Card() { view: (<panel><text>\"x\"</text></panel>) }");
        let card = get(&files, "Card.tsx");
        // 'panel' and 'text' are primitives, must not generate import lines
        assert!(
            !card.contains("import { panel }"),
            "primitive 'panel' should not be imported"
        );
        assert!(
            !card.contains("import { text }"),
            "primitive 'text' should not be imported"
        );
    }

    #[test]
    fn test_import_inside_if_branch() {
        let files = compile(
            "component Badge() { view: (<text>\"x\"</text>) }\n\
             component Host(show: bool) {\n\
               state s: bool = false\n\
               view: ({if s { <Badge /> } else { <text>\"no\"</text> }})\n\
             }",
        );
        let host = get(&files, "Host.tsx");
        assert!(
            host.contains("import { Badge } from \"./Badge\";"),
            "expected Badge import inside if branch:\n{host}"
        );
    }
}
