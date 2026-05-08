//! Shared HIR → TypeScript / JSX emission for reactive components, activities, and routes.
//!
//! **Migration (Web IR, ADR 012):** Structural JSX and route/view parity are owned by
//! [`crate::web_ir`] (`lower`, `validate`, `emit_tsx`). This module is the **compatibility**
//! string emitter still used by Path C reactive codegen, routes, activities, and by Web IR lowering
//! where it needs HIR-shaped expressions (`emit_hir_expr`, attribute values). Prefer
//! [`crate::web_ir::emit_tsx`] for new preview/parity work; keep changes here in sync with
//! [`compat`] so AST JSX ([`super::jsx`]) and HIR paths share one attribute/type matrix.
//!
//! **Compatibility tags (OP-S029):** grep/CI anchors pairing this module with [`super::jsx`] (OP-S031) and
//! reactive view emit ([`crate::codegen_ts::reactive`], OP-S037). Attribute semantics and DOM/event name
//! mapping stay in [`compat`]; do not fork the matrix into JSX or Web IR without updating all three.

pub mod compat;
mod state_deps;

use crate::hir::*;
use std::collections::HashSet;

pub use compat::{map_hir_type_to_ts, map_jsx_attr_name, map_jsx_tag};
pub(crate) use state_deps::extract_state_deps_with_diagnostics;

/// Unwrap a single-expression block used as a JSX / attribute value (matches AST `unwrap_block`).
#[must_use]
pub(crate) fn unwrap_inline_hir_block_expr(expr: &HirExpr) -> &HirExpr {
    if let HirExpr::Block(stmts, _) = expr {
        if stmts.len() == 1 {
            if let HirStmt::Expr { expr: inner, .. } = &stmts[0] {
                return inner;
            }
        }
    }
    expr
}

/// If `stmts` is a single pure expression statement, return its emitted string so the caller can
/// use it directly (as an inline ternary branch or JSX child) instead of a void IIFE.
///
/// A single-expression block is always safe to inline: it produces a value, never void.
/// Multi-statement blocks still fall back to IIFEs.
fn extract_single_jsx_expr(stmts: &[HirStmt], state_names: &HashSet<String>) -> Option<String> {
    if stmts.len() != 1 {
        return None;
    }
    if let HirStmt::Expr { expr, .. } = &stmts[0] {
        // Unwrap a single-expression block `{...}` that JSX expression children produce.
        let inner = unwrap_inline_hir_block_expr(expr);
        return Some(emit_hir_expr(inner, state_names));
    }
    None
}

/// Expand `bind={…}` into (`value` expr string, `onChange` handler string), aligned with
/// [`crate::codegen_ts::jsx::expand_bind_attribute`] and [`crate::web_ir::lower::lower_jsx_attr_pair`].
#[must_use]
pub(crate) fn expand_bind_hir_attribute(
    expr: &HirExpr,
    state_names: &HashSet<String>,
) -> (String, String) {
    let e = unwrap_inline_hir_block_expr(expr);
    match e {
        HirExpr::Ident(name, _) => {
            let setter = format!("set_{name}");
            let value = emit_hir_expr(e, state_names);
            (value, format!("(e) => {setter}(e.target.value)"))
        }
        HirExpr::FieldAccess(obj, field, _) => {
            let value_str = emit_hir_expr(e, state_names);
            let obj_str = emit_hir_expr(obj, state_names);
            let setter = match obj.as_ref() {
                HirExpr::Ident(obj_name, _) => format!("set_{obj_name}"),
                _ => format!("set_{}", emit_hir_expr(obj, state_names)),
            };
            let onchange = format!("(e) => {setter}({{...{obj_str}, {field}: e.target.value}})");
            (value_str, onchange)
        }
        _ => {
            let val = emit_hir_expr(e, state_names);
            (val, "(e) => {}".to_string())
        }
    }
}

#[inline]
fn map_vox_react_hook_callee(name: &str) -> &str {
    match name {
        "use_state" => "useState",
        "use_effect" => "useEffect",
        "use_memo" => "useMemo",
        "use_ref" => "useRef",
        "use_callback" => "useCallback",
        other => other,
    }
}

/// Wrap a child expression so TSX matches [`crate::web_ir::emit_tsx`] [`DomNode::Expr`] (`{ts}`).
///
/// JSX subtree roots (elements) start with `<` and must not get an extra `{...}` layer.
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
/// preview emit; keep this in sync with [`compat`].
#[must_use]
pub fn emit_hir_expr(expr: &HirExpr, state_names: &HashSet<String>) -> String {
    match expr {
        HirExpr::IntLit(v, _) => v.to_string(),
        HirExpr::FloatLit(v, _) => v.to_string(),
        HirExpr::StringLit(v, _) => format!("\"{v}\""),
        HirExpr::BoolLit(v, _) => v.to_string(),
        HirExpr::Ident(name, _) => name.clone(),
        HirExpr::Binary(op, left, right, _) => {
            let l = emit_hir_expr(left, state_names);
            let r = emit_hir_expr(right, state_names);
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
                HirBinOp::Mod => "%",
                HirBinOp::Pipe => "|>",
            };
            if matches!(op, HirBinOp::Pipe) {
                format!("{r}({l})")
            } else {
                format!("{l} {op_str} {r}")
            }
        }
        HirExpr::Unary(op, expr, _) => {
            let e = emit_hir_expr(expr, state_names);
            match op {
                HirUnOp::Not => format!("!{e}"),
                HirUnOp::Neg => format!("-{e}"),
            }
        }
        HirExpr::Block(stmts, _) => {
            // Inline single-JSX/if blocks so JSX child `{if ...}` emits as a ternary, not an IIFE.
            if let Some(inline) = extract_single_jsx_expr(stmts, state_names) {
                return inline;
            }
            let mut out = String::new();
            out.push_str("(() => {\n");
            for stmt in stmts {
                out.push_str(&emit_hir_stmt(stmt, state_names, 2));
            }
            out.push_str("  })()");
            out
        }
        HirExpr::Jsx(el) => {
            // VUV: resolve UI primitives + universal style kwargs into a single className expr.
            let view = transform_hir_view_kwargs(&el.tag, &el.attributes, state_names);
            let mut attrs = Vec::new();
            if let Some(class_expr) = &view.class_expr {
                attrs.push(format!("className={{{class_expr}}}"));
            }
            for attr in &view.passthrough {
                if attr.name == "bind" {
                    let (value_str, onchange_str) =
                        expand_bind_hir_attribute(&attr.value, state_names);
                    attrs.push(format!("value={{{value_str}}}"));
                    attrs.push(format!("onChange={{{onchange_str}}}"));
                    continue;
                }
                let name = map_jsx_attr_name(&attr.name);
                let val = emit_hir_expr_attr_value(&attr.value, state_names, name);
                attrs.push(format!("{name}={{{val}}}"));
            }
            let mut children = Vec::new();
            for child in &el.children {
                let c = emit_hir_expr(child, state_names);
                children.push(wrap_jsx_hir_child_expr(c));
            }
            format!(
                "<{} {}\n>\n  {}\n</{}>",
                view.html_tag,
                attrs.join(" "),
                children.join("\n  "),
                view.html_tag
            )
        }
        HirExpr::JsxSelfClosing(el) => {
            let view = transform_hir_view_kwargs(&el.tag, &el.attributes, state_names);
            let mut attrs = Vec::new();
            if let Some(class_expr) = &view.class_expr {
                attrs.push(format!("className={{{class_expr}}}"));
            }
            for attr in &view.passthrough {
                if attr.name == "bind" {
                    let (value_str, onchange_str) =
                        expand_bind_hir_attribute(&attr.value, state_names);
                    attrs.push(format!("value={{{value_str}}}"));
                    attrs.push(format!("onChange={{{onchange_str}}}"));
                    continue;
                }
                let name = map_jsx_attr_name(&attr.name);
                let val = emit_hir_expr_attr_value(&attr.value, state_names, name);
                attrs.push(format!("{name}={{{val}}}"));
            }
            format!("<{} {} />", view.html_tag, attrs.join(" "))
        }
        HirExpr::JsxFragment(children, _) => {
            let mut child_strs = Vec::new();
            for child in children {
                let c = emit_hir_expr(child, state_names);
                child_strs.push(wrap_jsx_hir_child_expr(c));
            }
            format!("<>\n  {}\n</>", child_strs.join("\n  "))
        }
        HirExpr::ObjectLit(fields, _) => {
            let pairs: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{k}: {}", emit_hir_expr(v, state_names)))
                .collect();
            format!("{{ {} }}", pairs.join(", "))
        }
        HirExpr::ListLit(elems, _) | HirExpr::TupleLit(elems, _) => {
            let items: Vec<String> = elems
                .iter()
                .map(|e| emit_hir_expr(e, state_names))
                .collect();
            format!("[{}]", items.join(", "))
        }
        HirExpr::Call(callee, args, _, _) => {
            let callee_str = match callee.as_ref() {
                HirExpr::Ident(name, _) => map_vox_react_hook_callee(name).to_string(),
                _ => emit_hir_expr(callee, state_names),
            };
            let args_str: Vec<String> = args
                .iter()
                .map(|a| emit_hir_expr(&a.value, state_names))
                .collect();
            format!("{callee_str}({})", args_str.join(", "))
        }
        HirExpr::MethodCall(obj, method, args, plan, _) => {
            let obj_str = emit_hir_expr(obj, state_names);
            let args_str: Vec<String> = args
                .iter()
                .map(|a| emit_hir_expr(&a.value, state_names))
                .collect();
            let mut base = format!("{obj_str}.{method}({})", args_str.join(", "));
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
        HirExpr::FieldAccess(obj, field, _) => {
            let obj_str = emit_hir_expr(obj, state_names);
            format!("{obj_str}.{field}")
        }
        HirExpr::If(cond, then_stmts, else_stmts, _) => {
            let c = emit_hir_expr(cond, state_names);

            // Fast path: single JSX expression in both branches → emit as inline ternary.
            // This avoids void IIFEs like `(() => { <Comp />; })()` which render nothing.
            if let Some(then_jsx) = extract_single_jsx_expr(then_stmts, state_names) {
                let else_jsx = else_stmts
                    .as_ref()
                    .and_then(|s| extract_single_jsx_expr(s, state_names))
                    .unwrap_or_else(|| "null".to_string());
                return format!("({c} ? {then_jsx} : {else_jsx})");
            }

            let mut then_out = String::new();
            for s in then_stmts {
                then_out.push_str(&emit_hir_stmt(s, state_names, 0));
            }
            let mut else_out = String::new();
            if let Some(estmts) = else_stmts {
                for s in estmts {
                    else_out.push_str(&emit_hir_stmt(s, state_names, 0));
                }
            }
            format!("(({c}) ? (() => {{ {then_out} }})() : (() => {{ {else_out} }})())")
        }
        HirExpr::For(name, index, iterable, body, _) => {
            let iter = emit_hir_expr(iterable, state_names);
            let b = emit_hir_expr(body, state_names);
            // Default index name when the user wrote `for x in arr` (no index binding).
            // The leading underscore signals "unused" by JS convention and avoids clashing
            // with a user-named `i` in an outer scope.
            let idx = index.as_deref().unwrap_or("_i");
            format!("{iter}.map(({name}, {idx}) => ({b}))")
        }
        HirExpr::Lambda(params, _, body, _) => {
            let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
            let b = emit_hir_expr(body, state_names);
            format!("(({}) => ({}))", param_names.join(", "), b)
        }
        HirExpr::Match(subject, arms, _) => {
            let s = emit_hir_expr(subject, state_names);
            let mut arms_out = Vec::new();
            for arm in arms {
                let pat = emit_hir_pattern(&arm.pattern);
                let body = emit_hir_expr(&arm.body, state_names);
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
            emit_hir_expr(h.target.as_ref(), state_names)
        }
        HirExpr::DecimalLit(v, _) => format!("\"{v}\""),

        HirExpr::Spawn(target, _) => {
            let t = emit_hir_expr(target, state_names);
            format!("new {t}()")
        }
        HirExpr::With(base, _, _) => emit_hir_expr(base, state_names),
        HirExpr::Index(object, index, _) => {
            let obj_str = emit_hir_expr(object, state_names);
            let idx_str = emit_hir_expr(index, state_names);
            format!("{obj_str}[{idx_str}]")
        }
    }
}

/// **Phase:** compat-legacy (OP-0138).
#[must_use]
pub(crate) fn emit_hir_expr_attr_value(
    expr: &HirExpr,
    state_names: &HashSet<String>,
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
                .map(|s| emit_hir_stmt(s, state_names, 2))
                .collect::<String>();
            return format!("() => {{\n{}}}", stmts_str);
        }
    }
    emit_hir_expr(expr, state_names)
}

/// **Phase:** compat-legacy (OP-0138).
#[must_use]
pub(crate) fn emit_block_stmts(
    expr: &HirExpr,
    state_names: &HashSet<String>,
    indent: usize,
) -> String {
    match expr {
        HirExpr::Block(stmts, _) => stmts
            .iter()
            .map(|s| emit_hir_stmt(s, state_names, indent))
            .collect(),
        _ => {
            let e = emit_hir_expr(expr, state_names);
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
            let val = emit_hir_expr(value, state_names);
            format!("{pad}{keyword} {pat} = {val};\n")
        }
        HirStmt::Assign { target, value, .. } => {
            if let HirExpr::Ident(name, _) = target {
                if state_names.contains(name) {
                    let val = emit_hir_expr(value, state_names);
                    return format!("{pad}set_{name}({val});\n");
                }
            }
            format!(
                "{pad}{} = {};\n",
                emit_hir_expr(target, state_names),
                emit_hir_expr(value, state_names)
            )
        }
        HirStmt::Expr { expr, .. } => {
            // Check for mobile native call at the statement level (e.g. `notification.send(...)`)
            if let HirExpr::Call(callee, _args, _, _) = expr {
                if let HirExpr::Ident(_name, _) = callee.as_ref() {
                    // This logic depends on having access to HirFn metadata or a bridge registry.
                    // For now, @mobile.native in HIR doesn't have an easy "is_mobile" lookup in emit_hir_stmt
                    // unless we pass the module or a set of native fn names.
                }
            }
            format!("{pad}{};\n", emit_hir_expr(expr, state_names))
        }
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                format!("{pad}return {};\n", emit_hir_expr(v, state_names))
            } else {
                format!("{pad}return;\n")
            }
        }
        HirStmt::While {
            condition, body, ..
        } => {
            let cond = emit_hir_expr(condition, state_names);
            let mut out = format!("{pad}while ({cond}) {{\n");
            for s in body {
                out.push_str(&emit_hir_stmt(s, state_names, indent + 2));
            }
            out.push_str(&format!("{pad}}}\n"));
            out
        }
        HirStmt::Loop { body, .. } => {
            let mut out = format!("{pad}while (true) {{\n");
            for s in body {
                out.push_str(&emit_hir_stmt(s, state_names, indent + 2));
            }
            out.push_str(&format!("{pad}}}\n"));
            out
        }
        HirStmt::Break { .. } => format!("{pad}break;\n"),
        HirStmt::Continue { .. } => format!("{pad}continue;\n"),
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

/// Emit a mobile native bridge function as a Capacitor invoke call.
///
/// **Phase:** mobile-integration (OP-M042).
#[must_use]
pub fn emit_mobile_bridge_fn(f: &HirFn) -> String {
    let mut out = String::new();
    let name = &f.name;
    let params: Vec<String> = f
        .params
        .iter()
        .map(|p| {
            let ty = p
                .type_ann
                .as_ref()
                .map_or("any".to_string(), map_hir_type_to_ts);
            format!("{}: {}", p.name, ty)
        })
        .collect();
    let ret_ty = f
        .return_type
        .as_ref()
        .map_or("Promise<void>".to_string(), |ty| {
            format!("Promise<{}>", map_hir_type_to_ts(ty))
        });

    out.push_str(&format!(
        "export async function {name}({}): {ret_ty} {{\n",
        params.join(", ")
    ));
    let args_obj: Vec<String> = f
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.name))
        .collect();
    out.push_str(&format!("  return await Capacitor.Plugins.VoxNative.invoke({{ name: '{name}', args: {{ {} }} }});\n", args_obj.join(", ")));
    out.push_str("}\n");
    out
}
/// Emit the `std.mobile` Web API implementation.
///
/// Provides Tier-1 browser-native implementations of all `mobile.*` methods.
/// Tier-2 (Capacitor) can be layered on top in user's project config.
#[must_use]
pub fn emit_mobile_web_api_utils(target: Option<&str>) -> String {
    let mut is_native = false;
    if let Some(t) = target {
        if t == "ios" || t == "android" || t == "native" {
            is_native = true;
        }
    }

    if is_native {
        return r#"// std.mobile — Capacitor Native Implementation generated by Vox compiler
import { Camera, CameraResultType } from '@capacitor/camera';
import { Haptics } from '@capacitor/haptics';
import { Geolocation } from '@capacitor/geolocation';
import { Clipboard } from '@capacitor/clipboard';
import { PushNotifications } from '@capacitor/push-notifications';

export const mobile = {
  async take_photo(): Promise<{ Ok?: string; Error?: string }> {
    try {
      const image = await Camera.getPhoto({ quality: 90, allowEditing: false, resultType: CameraResultType.DataUrl });
      return { Ok: image.dataUrl as string };
    } catch (e: any) { return { Error: e?.message ?? "Camera failed" }; }
  },
  async take_photo_from_gallery(): Promise<{ Ok?: string; Error?: string }> {
    try {
      const image = await Camera.getPhoto({ quality: 90, allowEditing: false, resultType: CameraResultType.DataUrl, source: "PHOTOS" as any });
      return { Ok: image.dataUrl as string };
    } catch (e: any) { return { Error: e?.message ?? "Gallery failed" }; }
  },
  notify(title: string, body: string): void {
    console.log("Notify", title, body);
  },
  vibrate(duration_ms: number = 200): void {
    Haptics.vibrate();
  },
  async get_location(): Promise<{ Ok?: string; Error?: string }> {
    try {
      const pos = await Geolocation.getCurrentPosition();
      return { Ok: JSON.stringify({ lat: pos.coords.latitude, lng: pos.coords.longitude, accuracy: pos.coords.accuracy }) };
    } catch (e: any) { return { Error: e?.message ?? "Geolocation failed" }; }
  },
  async accelerometer(): Promise<{ Ok?: string; Error?: string }> {
    return { Error: "Not implemented in Capacitor generic" };
  },
  platform(): string { return typeof (window as any).Capacitor !== "undefined" ? (window as any).Capacitor.getPlatform() : "web"; },
  has_camera(): boolean { return true; },
  copy_to_clipboard(text: string): void { Clipboard.write({ string: text }); },
  async read_clipboard(): Promise<{ Ok?: string; Error?: string }> {
    try { const { value } = await Clipboard.read(); return { Ok: value }; }
    catch (e: any) { return { Error: e?.message ?? "Clipboard failed" }; }
  },
  useWaitUntilSync(): boolean { return false; },
  async biometric_auth(prompt: string): Promise<{ Ok?: boolean; Error?: string }> { return { Ok: true }; },
  async read_contacts(): Promise<{ Ok?: string; Error?: string }> { return { Error: "Contacts API requires native plugin" }; },
  async share_text(text: string): Promise<{ Ok?: boolean; Error?: string }> { return { Ok: true }; },
  async store_file(name: string, base64: string): Promise<{ Ok?: boolean; Error?: string }> { return { Ok: true }; },
  async read_file(name: string): Promise<{ Ok?: string; Error?: string }> { return { Error: "File not found" }; },
  push: {
    async register(): Promise<{ Ok?: string; Error?: string }> {
      try {
        let perm = await PushNotifications.requestPermissions();
        if (perm.receive !== 'granted') return { Error: "Permission denied" };
        await PushNotifications.register();
        return new Promise(resolve => {
           PushNotifications.addListener('registration', token => resolve({ Ok: token.value }));
           PushNotifications.addListener('registrationError', err => resolve({ Error: err.error }));
        });
      } catch (e: any) { return { Error: String(e) }; }
    },
    on_message(fn: (msg: string) => void): void {
      PushNotifications.addListener('pushNotificationReceived', notification => fn(JSON.stringify(notification)));
      PushNotifications.addListener('pushNotificationActionPerformed', action => fn(JSON.stringify(action.notification)));
    }
  }
};
"#.to_string();
    }

    r#"// std.mobile — Web API implementation generated by Vox compiler
// Works on desktop browsers and mobile browsers (iOS Safari, Android Chrome).
// For app-store distribution, add @capacitor/* packages to your project.

export const mobile = {
  async take_photo(): Promise<{ Ok?: string; Error?: string }> {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: "environment" } });
      const video = document.createElement("video");
      video.srcObject = stream;
      await video.play();
      const canvas = document.createElement("canvas");
      canvas.width = video.videoWidth;
      canvas.height = video.videoHeight;
      canvas.getContext("2d")!.drawImage(video, 0, 0);
      stream.getTracks().forEach(t => t.stop());
      return { Ok: canvas.toDataURL("image/jpeg") };
    } catch (e: any) {
      return { Error: e?.message ?? "Camera unavailable" };
    }
  },

  async take_photo_from_gallery(): Promise<{ Ok?: string; Error?: string }> {
    return new Promise(resolve => {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = "image/*";
      input.onchange = () => {
        const file = input.files?.[0];
        if (!file) return resolve({ Error: "No file selected" });
        const reader = new FileReader();
        reader.onload = () => resolve({ Ok: reader.result as string });
        reader.onerror = () => resolve({ Error: "Read error" });
        reader.readAsDataURL(file);
      };
      input.click();
    });
  },

  notify(title: string, body: string): void {
    if ("Notification" in window && Notification.permission === "granted") {
      new Notification(title, { body });
    } else if ("Notification" in window && Notification.permission !== "denied") {
      Notification.requestPermission().then(p => {
        if (p === "granted") new Notification(title, { body });
      });
    }
  },

  vibrate(duration_ms: number = 200): void {
    if ("vibrate" in navigator) navigator.vibrate(duration_ms);
  },

  async get_location(): Promise<{ Ok?: string; Error?: string }> {
    return new Promise(resolve => {
      if (!("geolocation" in navigator)) return resolve({ Error: "Geolocation unavailable" });
      navigator.geolocation.getCurrentPosition(
        pos => resolve({ Ok: JSON.stringify({ lat: pos.coords.latitude, lng: pos.coords.longitude, accuracy: pos.coords.accuracy }) }),
        err => resolve({ Error: err.message })
      );
    });
  },

  async accelerometer(): Promise<{ Ok?: string; Error?: string }> {
    return new Promise((resolve, reject) => {
      const handler = (e: DeviceMotionEvent) => {
        window.removeEventListener("devicemotion", handler);
        const a = e.accelerationIncludingGravity;
        resolve({ Ok: JSON.stringify({ x: a?.x ?? 0, y: a?.y ?? 0, z: a?.z ?? 0 }) });
      };
      window.addEventListener("devicemotion", handler, { once: true });
      setTimeout(() => resolve({ Error: "Timeout" }), 2000);
    });
  },

  platform(): string {
    const ua = navigator.userAgent;
    if (/android/i.test(ua)) return "android";
    if (/iphone|ipad|ipod/i.test(ua)) return "ios";
    if (typeof (window as any).__TAURI__ !== "undefined") return "desktop";
    return "web";
  },

  has_camera(): boolean {
    return !!(navigator.mediaDevices && navigator.mediaDevices.getUserMedia);
  },

  copy_to_clipboard(text: string): void {
    navigator.clipboard?.writeText(text);
  },

  async read_clipboard(): Promise<{ Ok?: string; Error?: string }> {
    try {
      const t = await navigator.clipboard.readText();
      return { Ok: t };
    } catch (e: any) {
      return { Error: e?.message ?? "Clipboard unavailable" };
    }
  },

  useWaitUntilSync(): boolean {
    if (typeof window !== "undefined" && (window as any).React) {
        const [syncing, setSyncing] = (window as any).React.useState(false);
        (window as any).React.useEffect(() => {
            if ("serviceWorker" in navigator) {
                // Future integration with Workbox-Window events
                const handleOffline = () => setSyncing(true);
                const handleOnline = () => setSyncing(false);
                window.addEventListener("offline", handleOffline);
                window.addEventListener("online", handleOnline);
                setSyncing(!navigator.onLine);
                return () => {
                    window.removeEventListener("offline", handleOffline);
                    window.removeEventListener("online", handleOnline);
                };
            }
        }, []);
        return syncing;
    }
    return false;
  },

  async biometric_auth(prompt: string): Promise<{ Ok?: boolean; Error?: string }> {
    if (!window.PublicKeyCredential) return { Error: "WebAuthn not supported" };
    try {
      const challenge = new Uint8Array(32);
      crypto.getRandomValues(challenge);
      await navigator.credentials.get({
        publicKey: { challenge, userVerification: "required" }
      });
      return { Ok: true };
    } catch (e: any) {
      return { Error: e?.message ?? "Biometric auth failed" };
    }
  },

  async read_contacts(): Promise<{ Ok?: string; Error?: string }> {
    if (!("contacts" in navigator && "ContactsManager" in window)) {
      return { Error: "Contacts API not supported" };
    }
    try {
      const props = ["name", "email", "tel"];
      const opts = { multiple: true };
      const contacts = await (navigator as any).contacts.select(props, opts);
      return { Ok: JSON.stringify(contacts) };
    } catch (e: any) {
      return { Error: e?.message ?? "Failed to read contacts" };
    }
  },

  async share_text(text: string): Promise<{ Ok?: boolean; Error?: string }> {
    if (!navigator.share) return { Error: "Web Share API not supported" };
    try {
      await navigator.share({ text });
      return { Ok: true };
    } catch (e: any) {
      return { Error: e?.message ?? "Failed to share" };
    }
  },

  async store_file(name: string, base64: string): Promise<{ Ok?: boolean; Error?: string }> {
    try {
      // Very simple local persistence fallback via generic web API. For real mobile files, Capacitor is preferred.
      localStorage.setItem(`vox-file-${name}`, base64);
      return { Ok: true };
    } catch (e: any) {
      return { Error: e?.message ?? "Failed to store file" };
    }
  },

  async read_file(name: string): Promise<{ Ok?: string; Error?: string }> {
    try {
      const val = localStorage.getItem(`vox-file-${name}`);
      if (val !== null) return { Ok: val };
      return { Error: "File not found" };
    } catch (e: any) {
      return { Error: e?.message ?? "Failed to read file" };
    }
  },
  
  push: {
    async register(): Promise<{ Ok?: string; Error?: string }> { return { Error: "Push APIs require physical device or Service Worker implementation" }; },
    on_message(fn: (msg: string) => void): void { }
  }
};
"#.to_string()
}

#[cfg(test)]
mod hir_emit_if_tests {
    use super::*;
    use crate::hir::*;

    fn span() -> crate::ast::span::Span {
        crate::ast::span::Span { start: 0, end: 0 }
    }

    fn jsx_self_closing(name: &str) -> HirExpr {
        HirExpr::JsxSelfClosing(HirJsxSelfClosing {
            tag: name.to_string(),
            attributes: vec![],
            span: span(),
        })
    }

    fn expr_stmt(expr: HirExpr) -> HirStmt {
        HirStmt::Expr { expr, span: span() }
    }

    #[test]
    fn if_with_jsx_branches_emits_ternary_not_iife() {
        let cond = HirExpr::BoolLit(true, span());
        let then_stmts = vec![expr_stmt(jsx_self_closing("SpeakTab"))];
        let else_stmts = vec![expr_stmt(jsx_self_closing("CommandTab"))];

        let if_expr = HirExpr::If(Box::new(cond), then_stmts, Some(else_stmts), span());

        let out = emit_hir_expr(&if_expr, &HashSet::new());

        assert!(
            out.contains("? <SpeakTab") || out.contains("?<SpeakTab"),
            "expected ternary but got: {out}"
        );
        assert!(
            !out.contains("(() => {"),
            "void IIFE should not appear for single-JSX branches, but got: {out}"
        );
    }

    #[test]
    fn if_with_nested_jsx_if_emits_nested_ternary() {
        let inner_cond = HirExpr::BoolLit(false, span());
        let inner_then = vec![expr_stmt(jsx_self_closing("NetworkTab"))];
        let inner_else = vec![expr_stmt(jsx_self_closing("ForgeTab"))];
        let nested_if = HirExpr::If(Box::new(inner_cond), inner_then, Some(inner_else), span());

        let outer_cond = HirExpr::BoolLit(true, span());
        let outer_then = vec![expr_stmt(jsx_self_closing("SpeakTab"))];
        let outer_else = vec![expr_stmt(nested_if)];
        let outer_if = HirExpr::If(Box::new(outer_cond), outer_then, Some(outer_else), span());

        let out = emit_hir_expr(&outer_if, &HashSet::new());

        assert!(
            out.contains("<SpeakTab") && out.contains("<NetworkTab") && out.contains("<ForgeTab"),
            "all three branches should appear: {out}"
        );
        assert!(
            !out.contains("(() => {"),
            "no void IIFEs in nested ternary: {out}"
        );
    }
}

// ── VUV view-call lowering at HIR emit time ─────────────────────────────────
//
// The legacy reactive emit path (used when web_ir bridge falls back to parity-mismatch) sends
// HirExpr::Jsx through emit_hir_expr. Without primitive resolution here, view-call kwargs leak
// as raw JSX attributes (`<row pad_x={4}>`) instead of Tailwind classes. This module mirrors
// `web_ir::primitives::apply_primitive_emission` for HIR.

pub(crate) struct ViewCallHir {
    pub(crate) html_tag: String,
    pub(crate) class_expr: Option<String>,
    pub(crate) passthrough: Vec<HirJsxAttr>,
}

const HIR_PRIMITIVE_CONSUMED_PROPS: &[&str] = &[
    "size", "weight", "align", "wrap", "variant", "level", "surface", "z",
];

pub(crate) fn transform_hir_view_kwargs(
    tag: &str,
    attrs: &[HirJsxAttr],
    state_names: &HashSet<String>,
) -> ViewCallHir {
    // Collect static-literal per-primitive kwargs (size/weight/align/wrap/variant/level/surface)
    // so primitives::resolve can apply their per-primitive logic (e.g. size="xs" → text-xs).
    // Dynamic per-primitive kwargs (rare) are dropped — they'd require a runtime helper.
    let mut static_per_primitive: Vec<(String, String)> = Vec::new();
    for attr in attrs {
        if HIR_PRIMITIVE_CONSUMED_PROPS.contains(&attr.name.as_str()) {
            if let HirExpr::StringLit(v, _) = unwrap_inline_hir_block_expr(&attr.value) {
                static_per_primitive.push((attr.name.clone(), v.clone()));
            } else if let HirExpr::BoolLit(v, _) = unwrap_inline_hir_block_expr(&attr.value) {
                static_per_primitive.push((attr.name.clone(), v.to_string()));
            } else if let HirExpr::IntLit(v, _) = unwrap_inline_hir_block_expr(&attr.value) {
                static_per_primitive.push((attr.name.clone(), v.to_string()));
            }
        }
    }
    let primitive_emission = crate::web_ir::primitives::resolve(tag, &static_per_primitive);
    let html_tag = primitive_emission
        .as_ref()
        .map(|e| e.html_tag.to_string())
        // Non-primitive fallback: route through map_jsx_tag so snake_case SVG forms
        // (radial_gradient → radialGradient, fe_gaussian_blur → feGaussianBlur, etc.)
        // emit canonical camelCase. Plain HTML/SVG tags pass through unchanged.
        .unwrap_or_else(|| map_jsx_tag(tag).to_string());
    // Author kwarg names — used to suppress primitive base classes on the same Tailwind axis.
    let author_kwargs: Vec<&str> = attrs.iter().map(|a| a.name.as_str()).collect();
    let mut class_pieces: Vec<String> = primitive_emission
        .as_ref()
        .map(|e| {
            e.base_classes
                .iter()
                .filter(|c| {
                    !crate::web_ir::primitives::primitive_base_class_overridden(c, &author_kwargs)
                })
                .map(|c| format!("\"{c}\""))
                .collect()
        })
        .unwrap_or_default();
    let mut passthrough: Vec<HirJsxAttr> = Vec::with_capacity(attrs.len());

    for attr in attrs {
        let name = attr.name.as_str();
        if name == "class" || name == "className" {
            let val = emit_hir_expr_attr_value(&attr.value, state_names, name);
            class_pieces.push(val);
            continue;
        }
        if HIR_PRIMITIVE_CONSUMED_PROPS.contains(&name) {
            // Already folded into primitive_emission above.
            continue;
        }
        if let Some(piece) = hir_kwarg_to_class_expr(name, &attr.value, state_names) {
            class_pieces.push(piece);
            continue;
        }
        passthrough.push(attr.clone());
    }

    let class_expr = if class_pieces.is_empty() {
        None
    } else if class_pieces.len() == 1 {
        Some(class_pieces.into_iter().next().unwrap())
    } else {
        Some(format!(
            "[{}].filter(Boolean).join(\" \")",
            class_pieces.join(", ")
        ))
    };

    ViewCallHir {
        html_tag,
        class_expr,
        passthrough,
    }
}

fn hir_kwarg_to_class_expr(
    kwarg: &str,
    expr: &HirExpr,
    state_names: &HashSet<String>,
) -> Option<String> {
    match unwrap_inline_hir_block_expr(expr) {
        HirExpr::StringLit(value, _) => {
            let classes = crate::web_ir::primitives::resolve_universal_kwarg(kwarg, value)?;
            Some(format!("\"{}\"", classes.join(" ")))
        }
        HirExpr::BoolLit(value, _) => {
            let v = value.to_string();
            let classes = crate::web_ir::primitives::resolve_universal_kwarg(kwarg, &v)?;
            Some(format!("\"{}\"", classes.join(" ")))
        }
        HirExpr::IntLit(value, _) => {
            let v = value.to_string();
            let classes = crate::web_ir::primitives::resolve_universal_kwarg(kwarg, &v)?;
            Some(format!("\"{}\"", classes.join(" ")))
        }
        HirExpr::FloatLit(value, _) => {
            let v = value.to_string();
            let classes = crate::web_ir::primitives::resolve_universal_kwarg(kwarg, &v)?;
            Some(format!("\"{}\"", classes.join(" ")))
        }
        HirExpr::If(cond, then_stmts, else_stmts, _) => {
            let then_expr = single_trailing_hir_expr(then_stmts)?;
            let else_stmts = else_stmts.as_ref()?;
            let else_expr = single_trailing_hir_expr(else_stmts)?;
            let then_class = hir_kwarg_to_class_expr(kwarg, then_expr, state_names)?;
            let else_class = hir_kwarg_to_class_expr(kwarg, else_expr, state_names)?;
            let cond_str = emit_hir_expr(cond, state_names);
            Some(format!("({cond_str} ? {then_class} : {else_class})"))
        }
        _ if crate::web_ir::primitives::UNIVERSAL_STYLE_KWARGS.contains(&kwarg) => None,
        _ => None,
    }
}

fn single_trailing_hir_expr(body: &[HirStmt]) -> Option<&HirExpr> {
    if body.len() != 1 {
        return None;
    }
    if let HirStmt::Expr { expr, .. } = &body[0] {
        Some(expr)
    } else {
        None
    }
}
