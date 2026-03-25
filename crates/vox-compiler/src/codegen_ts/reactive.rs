use crate::hir::*;
use std::collections::HashSet;

pub fn generate_reactive_component(rc: &HirReactiveComponent) -> (String, String) {
    let name = &rc.name;
    let filename = format!("{name}.tsx");
    let mut out = String::new();

    let mut state_names = HashSet::new();
    for member in &rc.members {
        if let HirReactiveMember::State(s) = member {
            state_names.insert(s.name.clone());
        }
    }

    // Imports
    out.push_str("import React, { useState, useEffect, useMemo } from \"react\";\n\n");

    // Props interface
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

    // Component Function
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

    // Members
    for member in &rc.members {
        match member {
            HirReactiveMember::State(s) => {
                let init = emit_hir_expr(&s.init, &state_names);
                out.push_str(&format!(
                    "  const [{}, set_{}] = useState({});\n",
                    s.name, s.name, init
                ));
            }
            HirReactiveMember::Derived(d) => {
                let expr = emit_hir_expr(&d.expr, &state_names);
                let deps = extract_state_deps(&d.expr, &state_names);
                let dep_str = deps.join(", ");
                out.push_str(&format!(
                    "  const {} = useMemo(() => {}, [{}]);\n",
                    d.name, expr, dep_str
                ));
            }
            HirReactiveMember::Effect(e) => {
                let stmts_str = emit_block_stmts(&e.body, &state_names, 2);
                let deps = extract_state_deps(&e.body, &state_names);
                let dep_str = deps.join(", ");
                out.push_str(&format!(
                    "  useEffect(() => {{\n{}  }}, [{}]);\n",
                    stmts_str, dep_str
                ));
            }
            HirReactiveMember::OnMount(m) => {
                let stmts_str = emit_block_stmts(&m.body, &state_names, 2);
                out.push_str(&format!("  useEffect(() => {{\n{}  }}, []);\n", stmts_str));
            }
            HirReactiveMember::OnCleanup(c) => {
                let stmts_str = emit_block_stmts(&c.body, &state_names, 2);
                out.push_str(&format!(
                    "  useEffect(() => () => {{\n{}  }}, []);\n",
                    stmts_str
                ));
            }
        }
    }

    // View
    if let Some(view) = &rc.view {
        let view_js = emit_hir_expr(view, &state_names);
        out.push_str(&format!("  return (\n    {}\n  );\n", view_js));
    }

    out.push_str("}\n");
    (filename, out)
}

fn emit_hir_expr(expr: &HirExpr, state_names: &HashSet<String>) -> String {
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
            let mut out = String::new();
            out.push_str("(() => {\n");
            for stmt in stmts {
                out.push_str(&emit_hir_stmt(stmt, state_names, 2));
            }
            out.push_str("  })()");
            out
        }
        HirExpr::Jsx(el) => {
            let mut attrs = Vec::new();
            for attr in &el.attributes {
                let name = map_jsx_attr_name(&attr.name);
                let val = emit_hir_expr_attr_value(&attr.value, state_names, name);
                attrs.push(format!("{name}={{{val}}}"));
            }
            let mut children = Vec::new();
            for child in &el.children {
                children.push(emit_hir_expr(child, state_names));
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
            let mut attrs = Vec::new();
            for attr in &el.attributes {
                let name = map_jsx_attr_name(&attr.name);
                let val = emit_hir_expr_attr_value(&attr.value, state_names, name);
                attrs.push(format!("{name}={{{val}}}"));
            }
            format!("<{} {} />", el.tag, attrs.join(" "))
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
            let callee_str = emit_hir_expr(callee, state_names);
            let args_str: Vec<String> = args
                .iter()
                .map(|a| emit_hir_expr(&a.value, state_names))
                .collect();
            format!("{callee_str}({})", args_str.join(", "))
        }
        HirExpr::MethodCall(obj, method, args, _) => {
            let obj_str = emit_hir_expr(obj, state_names);
            let args_str: Vec<String> = args
                .iter()
                .map(|a| emit_hir_expr(&a.value, state_names))
                .collect();
            format!("{obj_str}.{method}({})", args_str.join(", "))
        }
        HirExpr::FieldAccess(obj, field, _) => {
            let obj_str = emit_hir_expr(obj, state_names);
            format!("{obj_str}.{field}")
        }
        HirExpr::If(cond, then_stmts, else_stmts, _) => {
            let c = emit_hir_expr(cond, state_names);
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
        HirExpr::For(name, iterable, body, _) => {
            let iter = emit_hir_expr(iterable, state_names);
            let b = emit_hir_expr(body, state_names);
            format!("{iter}.map(({name}) => ({b}))")
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
        _ => "null".to_string(),
    }
}

/// Emit a JSX attribute value with context awareness.
/// If the mapped attribute name is an event handler (starts with "on"),
/// block expressions are emitted as arrow functions instead of IIFEs.
fn emit_hir_expr_attr_value(
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
        match expr {
            HirExpr::Block(stmts, _) => {
                let stmts_str = stmts
                    .iter()
                    .map(|s| emit_hir_stmt(s, state_names, 2))
                    .collect::<String>();
                return format!("() => {{\n{}}}", stmts_str);
            }
            _ => {}
        }
    }
    emit_hir_expr(expr, state_names)
}

/// Emit the statements inside a Block expression at a given indent level.
/// Returns an empty string if the expression is not a Block.
fn emit_block_stmts(expr: &HirExpr, state_names: &HashSet<String>, indent: usize) -> String {
    match expr {
        HirExpr::Block(stmts, _) => stmts
            .iter()
            .map(|s| emit_hir_stmt(s, state_names, indent))
            .collect(),
        _ => {
            // Fallback: emit as a single expression statement
            let e = emit_hir_expr(expr, state_names);
            let pad = "  ".repeat(indent);
            format!("{pad}{e};\n")
        }
    }
}

fn emit_hir_stmt(stmt: &HirStmt, state_names: &HashSet<String>, indent: usize) -> String {
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
            format!("{pad}{};\n", emit_hir_expr(expr, state_names))
        }
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                format!("{pad}return {};\n", emit_hir_expr(v, state_names))
            } else {
                format!("{pad}return;\n")
            }
        }
    }
}

fn emit_hir_pattern(pattern: &HirPattern) -> String {
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

fn map_hir_type_to_ts(ty: &HirType) -> String {
    match ty {
        HirType::Named(name) => match name.as_str() {
            "int" | "float" => "number".to_string(),
            "str" => "string".to_string(),
            "bool" => "boolean".to_string(),
            other => other.to_string(),
        },
        HirType::Generic(name, args) => {
            let args_str: Vec<String> = args.iter().map(map_hir_type_to_ts).collect();
            format!("{}<{}>", name, args_str.join(", "))
        }
        _ => "any".to_string(),
    }
}

fn extract_state_deps(expr: &HirExpr, state_names: &HashSet<String>) -> Vec<String> {
    let mut deps = HashSet::new();
    collect_deps(expr, state_names, &mut deps);
    let mut sorted: Vec<String> = deps.into_iter().collect();
    sorted.sort();
    sorted
}

fn collect_deps(expr: &HirExpr, state_names: &HashSet<String>, deps: &mut HashSet<String>) {
    match expr {
        HirExpr::Ident(name, _) => {
            if state_names.contains(name) {
                deps.insert(name.clone());
            }
        }
        HirExpr::Binary(_, left, right, _) => {
            collect_deps(left, state_names, deps);
            collect_deps(right, state_names, deps);
        }
        HirExpr::Unary(_, expr, _) => {
            collect_deps(expr, state_names, deps);
        }
        HirExpr::Block(stmts, _) => {
            for stmt in stmts {
                collect_deps_stmt(stmt, state_names, deps);
            }
        }
        HirExpr::Jsx(el) => {
            for attr in &el.attributes {
                collect_deps(&attr.value, state_names, deps);
            }
            for child in &el.children {
                collect_deps(child, state_names, deps);
            }
        }
        HirExpr::JsxSelfClosing(el) => {
            for attr in &el.attributes {
                collect_deps(&attr.value, state_names, deps);
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, val) in fields {
                collect_deps(val, state_names, deps);
            }
        }
        HirExpr::ListLit(elems, _) | HirExpr::TupleLit(elems, _) => {
            for e in elems {
                collect_deps(e, state_names, deps);
            }
        }
        HirExpr::Call(callee, args, _, _) => {
            collect_deps(callee, state_names, deps);
            for arg in args {
                collect_deps(&arg.value, state_names, deps);
            }
        }
        HirExpr::MethodCall(obj, _, args, _) => {
            collect_deps(obj, state_names, deps);
            for arg in args {
                collect_deps(&arg.value, state_names, deps);
            }
        }
        HirExpr::FieldAccess(obj, _, _) => {
            collect_deps(obj, state_names, deps);
        }
        HirExpr::If(cond, then_body, else_body, _) => {
            collect_deps(cond, state_names, deps);
            for stmt in then_body {
                collect_deps_stmt(stmt, state_names, deps);
            }
            if let Some(estmts) = else_body {
                for stmt in estmts {
                    collect_deps_stmt(stmt, state_names, deps);
                }
            }
        }
        HirExpr::For(_, iterable, body, _) => {
            collect_deps(iterable, state_names, deps);
            collect_deps(body, state_names, deps);
        }
        HirExpr::Lambda(_, _, body, _) => {
            collect_deps(body, state_names, deps);
        }
        HirExpr::Match(subject, arms, _) => {
            collect_deps(subject, state_names, deps);
            for arm in arms {
                collect_deps(&arm.body, state_names, deps);
            }
        }
        HirExpr::Pipe(left, right, _) | HirExpr::With(left, right, _) => {
            collect_deps(left, state_names, deps);
            collect_deps(right, state_names, deps);
        }
        HirExpr::Spawn(expr, _) => {
            collect_deps(expr, state_names, deps);
        }
        _ => {}
    }
}

fn collect_deps_stmt(stmt: &HirStmt, state_names: &HashSet<String>, deps: &mut HashSet<String>) {
    match stmt {
        HirStmt::Let { value, .. } => {
            collect_deps(value, state_names, deps);
        }
        HirStmt::Assign { target, value, .. } => {
            collect_deps(target, state_names, deps);
            collect_deps(value, state_names, deps);
        }
        HirStmt::Expr { expr, .. } => {
            collect_deps(expr, state_names, deps);
        }
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_deps(v, state_names, deps);
            }
        }
    }
}

fn map_jsx_attr_name(name: &str) -> &str {
    match name {
        "class" | "className" => "className",
        "on:click" | "on_click" => "onClick",
        "on:change" | "on_change" => "onChange",
        "on:input" | "on_input" => "onInput",
        "on:submit" | "on_submit" => "onSubmit",
        "on:keydown" | "on_keydown" => "onKeyDown",
        "on:keyup" | "on_keyup" => "onKeyUp",
        "on:mouseenter" | "on_mouseenter" => "onMouseEnter",
        "on:mouseleave" | "on_mouseleave" => "onMouseLeave",
        _ => name,
    }
}
