//! Shared HIR → TypeScript / JSX emission for reactive components, activities, and routes.
//!
//! **Migration (Web IR, ADR 012):** Structural JSX, islands, and route/view parity are owned by
//! [`crate::web_ir`] (`lower`, `validate`, `emit_tsx`). This module is the **compatibility**
//! string emitter still used by Path C reactive codegen, routes, activities, and by Web IR lowering
//! where it needs HIR-shaped expressions (`emit_hir_expr`, attribute values). Prefer
//! [`crate::web_ir::emit_tsx`] for new preview/parity work; keep changes here in sync with
//! [`compat`] so AST JSX ([`super::jsx`]) and HIR paths share one attribute/type matrix.
//!
//! **Deprecation disposition (OP-0142):** island mount strings and fine-grained HIR statement emit are
//! compatibility-only; [`crate::web_ir`] is the structural SSOT. Links: ADR 012,
//! `docs/src/architecture/internal-web-ir-implementation-blueprint.md` (block 09, OP-0129+).
//!
//! **Compatibility tags (OP-S029):** grep/CI anchors pairing this module with [`super::jsx`] (OP-S031) and
//! reactive view emit ([`crate::codegen_ts::reactive`], OP-S037). Attribute semantics and DOM/event name
//! mapping stay in [`compat`]; do not fork the matrix into JSX or Web IR without updating all three.
//!
//! **Wrapper notes B + hir inventory (OP-S079 / S169):** island and event helpers delegate to [`super::island_emit`];
//! statement-level JSX parity stays compatibility-only vs [`crate::web_ir::emit_tsx`].

pub mod compat;
mod state_deps;

use super::island_emit::{
    island_data_prop_attr, island_mount_hir_fragment, island_mount_opening_part,
};
use crate::hir::*;
use std::collections::HashSet;

pub use compat::{map_hir_type_to_ts, map_jsx_attr_name};
pub(crate) use state_deps::extract_state_deps;

/// Wrap a child expression so TSX matches [`crate::web_ir::emit_tsx`] [`DomNode::Expr`] (`{ts}`).
///
/// JSX subtree roots (elements / island mounts) start with `<` and must not get an extra `{...}` layer.
pub(crate) fn wrap_jsx_hir_child_expr(emit: String) -> String {
    let t = emit.trim_start();
    if t.starts_with('<') {
        emit
    } else {
        format!("{{{emit}}}")
    }
}

/// Emit a HIR expression as TypeScript/JSX with optional reactive `state` names (for `set_x` rewriting).
///
/// **Phase:** compat-legacy (OP-0138). Prefer [`crate::web_ir::emit_tsx`] for structural parity and
/// preview emit; keep this in sync with [`compat`] and [`super::island_emit`].
#[must_use]
pub fn emit_hir_expr(
    expr: &HirExpr,
    state_names: &HashSet<String>,
    island_names: &HashSet<String>,
) -> String {
    match expr {
        HirExpr::IntLit(v, _) => v.to_string(),
        HirExpr::FloatLit(v, _) => v.to_string(),
        HirExpr::StringLit(v, _) => format!("\"{v}\""),
        HirExpr::BoolLit(v, _) => v.to_string(),
        HirExpr::Ident(name, _) => name.clone(),
        HirExpr::Binary(op, left, right, _) => {
            let l = emit_hir_expr(left, state_names, island_names);
            let r = emit_hir_expr(right, state_names, island_names);
            let op_str = match op {
                HirBinOp::Add => "+",
                HirBinOp::Sub => "-",
                HirBinOp::Mul => "*",
                HirBinOp::Div => "/",
                HirBinOp::Lt => "<",
                HirBinOp::Gt => ">",
                HirBinOp::Lte => "<=",
                HirBinOp::Gte => ">=",
                HirBinOp::And => "&&",
                HirBinOp::Or => "||",
                HirBinOp::Is => "===",
                HirBinOp::Isnt => "!==",
                HirBinOp::Pipe => "|>",
            };
            if matches!(op, HirBinOp::Pipe) {
                format!("{r}({l})")
            } else {
                format!("{l} {op_str} {r}")
            }
        }
        HirExpr::Unary(op, expr, _) => {
            let e = emit_hir_expr(expr, state_names, island_names);
            match op {
                HirUnOp::Not => format!("!{e}"),
                HirUnOp::Neg => format!("-{e}"),
            }
        }
        HirExpr::Block(stmts, _) => {
            let mut out = String::new();
            out.push_str("(() => {\n");
            for stmt in stmts {
                out.push_str(&emit_hir_stmt(stmt, state_names, island_names, 2));
            }
            out.push_str("  })()");
            out
        }
        HirExpr::Jsx(el) => {
            if let Some(mount) = emit_hir_island_mount_el(
                &el.tag,
                &el.attributes,
                el.children.len(),
                state_names,
                island_names,
            ) {
                return mount;
            }
            let mut attrs = Vec::new();
            for attr in &el.attributes {
                let name = map_jsx_attr_name(&attr.name);
                let val = emit_hir_expr_attr_value(&attr.value, state_names, island_names, name);
                attrs.push(format!("{name}={{{val}}}"));
            }
            let mut children = Vec::new();
            for child in &el.children {
                let c = emit_hir_expr(child, state_names, island_names);
                children.push(wrap_jsx_hir_child_expr(c));
            }
            format!(
                "<{} {}\n>\n  {}\n</{}>",
                el.tag,
                attrs.join(" "),
                children.join("\n  "),
                el.tag
            )
        }
        HirExpr::JsxSelfClosing(el) => {
            if let Some(mount) =
                emit_hir_island_mount_el(&el.tag, &el.attributes, 0, state_names, island_names)
            {
                return mount;
            }
            let mut attrs = Vec::new();
            for attr in &el.attributes {
                let name = map_jsx_attr_name(&attr.name);
                let val = emit_hir_expr_attr_value(&attr.value, state_names, island_names, name);
                attrs.push(format!("{name}={{{val}}}"));
            }
            format!("<{} {} />", el.tag, attrs.join(" "))
        }
        HirExpr::ObjectLit(fields, _) => {
            let pairs: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{k}: {}", emit_hir_expr(v, state_names, island_names)))
                .collect();
            format!("{{ {} }}", pairs.join(", "))
        }
        HirExpr::ListLit(elems, _) | HirExpr::TupleLit(elems, _) => {
            let items: Vec<String> = elems
                .iter()
                .map(|e| emit_hir_expr(e, state_names, island_names))
                .collect();
            format!("[{}]", items.join(", "))
        }
        HirExpr::Call(callee, args, _, _) => {
            let callee_str = emit_hir_expr(callee, state_names, island_names);
            let args_str: Vec<String> = args
                .iter()
                .map(|a| emit_hir_expr(&a.value, state_names, island_names))
                .collect();
            format!("{callee_str}({})", args_str.join(", "))
        }
        HirExpr::DbTableOp {
            table,
            op,
            args,
            select_cols,
            order_by,
            limit,
            plan,
            ..
        } => {
            let method = match op {
                crate::hir::HirDbTableOp::Insert => "insert",
                crate::hir::HirDbTableOp::Get => "get",
                crate::hir::HirDbTableOp::Delete => "delete",
                crate::hir::HirDbTableOp::All => "all",
                crate::hir::HirDbTableOp::FilterRecord => "filter",
                crate::hir::HirDbTableOp::Count => "count",
                crate::hir::HirDbTableOp::UnsafeQueryRawClause => "unsafe_query_raw_clause",
            };
            let args_str: Vec<String> = args
                .iter()
                .map(|a| emit_hir_expr(&a.value, state_names, island_names))
                .collect();
            let mut base = format!("db.{table}.{method}({})", args_str.join(", "));
            if let Some((col, asc)) = order_by {
                let dir = if *asc { "asc" } else { "desc" };
                base.push_str(&format!(".order_by(\"{}\", \"{}\")", col, dir));
            }
            if let Some(lim) = limit {
                base.push_str(&format!(
                    ".limit({})",
                    emit_hir_expr(lim.as_ref(), state_names, island_names)
                ));
            }
            if let Some(cols) = select_cols {
                let cols_js: Vec<String> = cols
                    .iter()
                    .map(|c| format!("\"{}\"", c.replace('\"', "\\\"")))
                    .collect();
                base.push_str(&format!(".select({})", cols_js.join(", ")));
            }
            if let Some(p) = plan {
                if p.capabilities.requires_sync {
                    base.push_str(".sync()");
                }
                if let Some(mode) = p.capabilities.retrieval_mode {
                    let m = match mode {
                        crate::hir::HirDbRetrievalMode::Fts => "fts",
                        crate::hir::HirDbRetrievalMode::Vector => "vector",
                        crate::hir::HirDbRetrievalMode::Hybrid => "hybrid",
                    };
                    base.push_str(&format!(".using(\"{m}\")"));
                }
                if let Some(topic) = &p.capabilities.live_topic {
                    base.push_str(&format!(".live(\"{}\")", topic.replace('\"', "\\\"")));
                }
                if let Some(scope) = &p.capabilities.orchestration_scope {
                    base.push_str(&format!(".scope(\"{}\")", scope.replace('\"', "\\\"")));
                }
            }
            base
        }
        HirExpr::MethodCall(obj, method, args, _) => {
            let obj_str = emit_hir_expr(obj, state_names, island_names);
            let args_str: Vec<String> = args
                .iter()
                .map(|a| emit_hir_expr(&a.value, state_names, island_names))
                .collect();
            format!("{obj_str}.{method}({})", args_str.join(", "))
        }
        HirExpr::FieldAccess(obj, field, _) => {
            let obj_str = emit_hir_expr(obj, state_names, island_names);
            format!("{obj_str}.{field}")
        }
        HirExpr::If(cond, then_stmts, else_stmts, _) => {
            let c = emit_hir_expr(cond, state_names, island_names);
            let mut then_out = String::new();
            for s in then_stmts {
                then_out.push_str(&emit_hir_stmt(s, state_names, island_names, 0));
            }
            let mut else_out = String::new();
            if let Some(estmts) = else_stmts {
                for s in estmts {
                    else_out.push_str(&emit_hir_stmt(s, state_names, island_names, 0));
                }
            }
            format!("(({c}) ? (() => {{ {then_out} }})() : (() => {{ {else_out} }})())")
        }
        HirExpr::For(name, iterable, body, _) => {
            let iter = emit_hir_expr(iterable, state_names, island_names);
            let b = emit_hir_expr(body, state_names, island_names);
            format!("{iter}.map(({name}) => ({b}))")
        }
        HirExpr::Lambda(params, _, body, _) => {
            let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
            let b = emit_hir_expr(body, state_names, island_names);
            format!("(({}) => ({}))", param_names.join(", "), b)
        }
        HirExpr::Match(subject, arms, _) => {
            let s = emit_hir_expr(subject, state_names, island_names);
            let mut arms_out = Vec::new();
            for arm in arms {
                let pat = emit_hir_pattern(&arm.pattern);
                let body = emit_hir_expr(&arm.body, state_names, island_names);
                arms_out.push(format!("case {pat}: return {body};"));
            }
            format!(
                "((_val) => {{ switch(_val) {{ {} }} }})({s})",
                arms_out.join(" ")
            )
        }
        HirExpr::Try(h) => {
            // No direct equivalent of `?` in TS unless it's a specific pattern, but we'll try to emulate or just emit `await`/direct expression for now since it's just TS generation.
            // A common TS code pattern is to just emit the target since actual error bubbling requires explicit branching. For basic TS compat we'll emit the unwrapped expression.
            emit_hir_expr(h.target.as_ref(), state_names, island_names)
        }
        _ => "null".to_string(),
    }
}

/// When the JSX tag matches an `@island` name, emit a `div` mount point for `island-mount.js`
/// (`data-vox-island` + `data-prop-*`), not a React component reference.
///
/// # Deprecation / migration (OP-0132)
///
/// **Do not extend** this string path for new features. Add island behavior through
/// [`crate::web_ir`] (`lower`, `validate`, `emit_tsx`) and keep this emitter aligned with that
/// shape until Path C dual-run is finished.
///
/// Kept for production Path C / routes until dual-run diff vs [`crate::web_ir`] is complete; string
/// shape must match [`super::island_emit`] and [`crate::web_ir::lower::lower_jsx_attr_pair`].
///
/// **Phase:** compat-legacy (OP-0138).
fn emit_hir_island_mount_el(
    tag: &str,
    attributes: &[HirJsxAttr],
    child_count: usize,
    state_names: &HashSet<String>,
    island_names: &HashSet<String>,
) -> Option<String> {
    if !island_names.contains(tag) {
        return None;
    }
    let mut parts = vec![island_mount_opening_part(tag)];
    for attr in attributes {
        if attr.name == "bind" {
            continue;
        }
        let dname = island_data_prop_attr(&attr.name);
        let val = emit_hir_expr_attr_value(&attr.value, state_names, island_names, &dname);
        parts.push(format!("{dname}={{{val}}}"));
    }
    crate::codegen_ts::island_emit::sort_island_mount_data_prop_parts(&mut parts);
    Some(island_mount_hir_fragment(tag, &parts, child_count))
}

/// **Phase:** compat-legacy (OP-0138).
#[must_use]
pub(crate) fn emit_hir_expr_attr_value(
    expr: &HirExpr,
    state_names: &HashSet<String>,
    island_names: &HashSet<String>,
    attr_name: &str,
) -> String {
    let is_event_handler = attr_name.starts_with("on")
        && attr_name.len() > 2
        && attr_name
            .chars()
            .nth(2)
            .map(|c| c.is_uppercase())
            .unwrap_or(false);
    if is_event_handler {
        if let HirExpr::Block(stmts, _) = expr {
            let stmts_str = stmts
                .iter()
                .map(|s| emit_hir_stmt(s, state_names, island_names, 2))
                .collect::<String>();
            return format!("() => {{\n{}}}", stmts_str);
        }
    }
    emit_hir_expr(expr, state_names, island_names)
}

/// **Phase:** compat-legacy (OP-0138).
#[must_use]
pub(crate) fn emit_block_stmts(
    expr: &HirExpr,
    state_names: &HashSet<String>,
    island_names: &HashSet<String>,
    indent: usize,
) -> String {
    match expr {
        HirExpr::Block(stmts, _) => stmts
            .iter()
            .map(|s| emit_hir_stmt(s, state_names, island_names, indent))
            .collect(),
        _ => {
            let e = emit_hir_expr(expr, state_names, island_names);
            let pad = "  ".repeat(indent);
            format!("{pad}{e};\n")
        }
    }
}

/// **Phase:** compat-legacy (OP-0138).
#[must_use]
pub(crate) fn emit_hir_stmt(
    stmt: &HirStmt,
    state_names: &HashSet<String>,
    island_names: &HashSet<String>,
    indent: usize,
) -> String {
    let pad = "  ".repeat(indent);
    match stmt {
        HirStmt::Let {
            pattern,
            value,
            mutable,
            ..
        } => {
            let keyword = if *mutable { "let" } else { "const" };
            let pat = emit_hir_pattern(pattern);
            let val = emit_hir_expr(value, state_names, island_names);
            format!("{pad}{keyword} {pat} = {val};\n")
        }
        HirStmt::Assign { target, value, .. } => {
            if let HirExpr::Ident(name, _) = target {
                if state_names.contains(name) {
                    let val = emit_hir_expr(value, state_names, island_names);
                    return format!("{pad}set_{name}({val});\n");
                }
            }
            format!(
                "{pad}{} = {};\n",
                emit_hir_expr(target, state_names, island_names),
                emit_hir_expr(value, state_names, island_names)
            )
        }
        HirStmt::Expr { expr, .. } => {
            format!("{pad}{};\n", emit_hir_expr(expr, state_names, island_names))
        }
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                format!(
                    "{pad}return {};\n",
                    emit_hir_expr(v, state_names, island_names)
                )
            } else {
                format!("{pad}return;\n")
            }
        }
    }
}

/// **Phase:** compat-legacy (OP-0138).
#[must_use]
pub(crate) fn emit_hir_pattern(pattern: &HirPattern) -> String {
    match pattern {
        HirPattern::Ident(name, _) => name.clone(),
        HirPattern::Tuple(elems, _) => {
            let s: Vec<String> = elems.iter().map(emit_hir_pattern).collect();
            format!("[{}]", s.join(", "))
        }
        HirPattern::Literal(lit, _) => match lit.as_ref() {
            HirExpr::IntLit(v, _) => v.to_string(),
            HirExpr::FloatLit(v, _) => v.to_string(),
            HirExpr::StringLit(s, _) => format!("\"{s}\""),
            HirExpr::BoolLit(b, _) => b.to_string(),
            _ => "_".to_string(),
        },
        HirPattern::Wildcard(_) => "_".to_string(),
        _ => "_".to_string(),
    }
}
