use crate::jsx::{emit_expr, emit_jsx_element, emit_jsx_self_closing, emit_stmt};
use vox_ast::decl::FnDecl;
use vox_ast::expr::Expr;
use vox_ast::scalar_mapping::VoxScalar;
use vox_ast::stmt::Stmt;

/// Generate a React component from a Vox @component function declaration.
/// Returns (filename, content) tuple.
pub fn generate_component(func: &FnDecl, has_styles: bool) -> (String, String) {
    let name = &func.name;
    let filename = format!("{name}.tsx");
    let mut out = String::new();

    // Imports — detect hooks from AST (not Debug strings).
    let mut need_state = false;
    let mut need_effect = false;
    let mut need_memo = false;
    let mut need_ref = false;
    let mut need_callback = false;
    for stmt in &func.body {
        scan_stmt_for_hooks(
            stmt,
            &mut need_state,
            &mut need_effect,
            &mut need_memo,
            &mut need_ref,
            &mut need_callback,
        );
    }
    let mut hooks = Vec::new();
    if need_state {
        hooks.push("useState");
    }
    if need_effect {
        hooks.push("useEffect");
    }
    if need_memo {
        hooks.push("useMemo");
    }
    if need_ref {
        hooks.push("useRef");
    }
    if need_callback {
        hooks.push("useCallback");
    }
    if hooks.is_empty() {
        hooks.push("useState");
    }
    out.push_str(&format!(
        "import React, {{ {} }} from \"react\";\n\n",
        hooks.join(", ")
    ));
    if has_styles {
        out.push_str(&format!("import \"./{name}.css\";\n\n"));
    }

    // Props interface
    if !func.params.is_empty() {
        out.push_str(&format!("export interface {name}Props {{\n"));
        for param in &func.params {
            let ts_type = param
                .type_ann
                .as_ref()
                .map_or("any".to_string(), map_vox_type_to_ts);
            let optional = if param.default.is_some() { "?" } else { "" };
            out.push_str(&format!("  {}{optional}: {ts_type};\n", param.name));
        }
        out.push_str("}\n\n");
    }

    // Function component
    if func.params.is_empty() {
        out.push_str(&format!(
            "export function {name}(): React.ReactElement {{\n"
        ));
    } else {
        // Destructure props
        let param_names: Vec<String> = func
            .params
            .iter()
            .map(|p| {
                if let Some(ref default) = p.default {
                    format!("{} = {}", p.name, emit_expr(default))
                } else {
                    p.name.clone()
                }
            })
            .collect();
        out.push_str(&format!(
            "export function {name}({{ {} }}: {name}Props): React.ReactElement {{\n",
            param_names.join(", ")
        ));
    }

    // Body: emit all non-return, non-JSX statements, then find the JSX return
    let mut jsx_return: Option<String> = None;

    for stmt in &func.body {
        match stmt {
            Stmt::Let { .. } | Stmt::Assign { .. } => {
                out.push_str(&emit_component_stmt(stmt));
            }
            Stmt::Expr { expr, .. } => {
                match expr {
                    Expr::Jsx(el) => {
                        // This is the return JSX
                        jsx_return = Some(emit_jsx_element(el, 2));
                    }
                    Expr::JsxSelfClosing(el) => {
                        jsx_return = Some(emit_jsx_self_closing(el, 2));
                    }
                    Expr::Call { .. } | Expr::MethodCall { .. } => {
                        out.push_str(&emit_component_stmt(stmt));
                    }
                    _ => {
                        out.push_str(&emit_component_stmt(stmt));
                    }
                }
            }
            Stmt::Return {
                value: Some(expr), ..
            } => {
                jsx_return = Some(format!("    {}", emit_expr(expr)));
            }
            _ => {}
        }
    }

    // Emit JSX return
    if let Some(jsx) = jsx_return {
        out.push_str(&format!("  return (\n{jsx}  );\n"));
    }

    out.push_str("}\n");

    (filename, out)
}

fn mark_vox_hook(
    name: &str,
    need_state: &mut bool,
    need_effect: &mut bool,
    need_memo: &mut bool,
    need_ref: &mut bool,
    need_callback: &mut bool,
) {
    match name {
        "use_state" => *need_state = true,
        "use_effect" => *need_effect = true,
        "use_memo" => *need_memo = true,
        "use_ref" => *need_ref = true,
        "use_callback" => *need_callback = true,
        _ => {}
    }
}

fn scan_expr_for_hooks(
    expr: &Expr,
    need_state: &mut bool,
    need_effect: &mut bool,
    need_memo: &mut bool,
    need_ref: &mut bool,
    need_callback: &mut bool,
) {
    match expr {
        Expr::Call { callee, args, .. } => {
            if let Expr::Ident { name, .. } = callee.as_ref() {
                mark_vox_hook(
                    name.as_str(),
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            } else {
                scan_expr_for_hooks(
                    callee,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
            for a in args {
                scan_expr_for_hooks(
                    &a.value,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        Expr::MethodCall { object, args, .. } => {
            scan_expr_for_hooks(
                object,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            for a in args {
                scan_expr_for_hooks(
                    &a.value,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        Expr::Binary { left, right, .. } => {
            scan_expr_for_hooks(
                left,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            scan_expr_for_hooks(
                right,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        Expr::Unary { operand, .. } => {
            scan_expr_for_hooks(
                operand,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        Expr::Block { stmts, .. } => {
            for s in stmts {
                scan_stmt_for_hooks(
                    s,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        Expr::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            scan_expr_for_hooks(
                condition,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            for s in then_body {
                scan_stmt_for_hooks(
                    s,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
            if let Some(else_stmts) = else_body {
                for s in else_stmts {
                    scan_stmt_for_hooks(
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
        Expr::For { iterable, body, .. } => {
            scan_expr_for_hooks(
                iterable,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            scan_expr_for_hooks(
                body,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        Expr::Match { subject, arms, .. } => {
            scan_expr_for_hooks(
                subject,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            for arm in arms {
                scan_expr_for_hooks(
                    &arm.body,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        Expr::Lambda { body, .. } => {
            scan_expr_for_hooks(
                body,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        Expr::Pipe { left, right, .. } => {
            scan_expr_for_hooks(
                left,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            scan_expr_for_hooks(
                right,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        Expr::Spawn { target, .. } => {
            scan_expr_for_hooks(
                target,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        Expr::With {
            operand, options, ..
        } => {
            scan_expr_for_hooks(
                operand,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            scan_expr_for_hooks(
                options,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        Expr::StringInterp { parts, .. } => {
            for p in parts {
                if let vox_ast::expr::StringPart::Interpolation(e) = p {
                    scan_expr_for_hooks(
                        e,
                        need_state,
                        need_effect,
                        need_memo,
                        need_ref,
                        need_callback,
                    );
                }
            }
        }
        Expr::ListLit { elements, .. } => {
            for el in elements {
                scan_expr_for_hooks(
                    el,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        Expr::TupleLit { elements, .. } => {
            for el in elements {
                scan_expr_for_hooks(
                    el,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        Expr::ObjectLit { fields, .. } => {
            for (_, v) in fields {
                scan_expr_for_hooks(
                    v,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        Expr::FieldAccess { object, .. } => {
            scan_expr_for_hooks(
                object,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        Expr::Jsx(el) => {
            for ch in &el.children {
                scan_expr_for_hooks(
                    ch,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
            for attr in &el.attributes {
                scan_expr_for_hooks(
                    &attr.value,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        Expr::JsxSelfClosing(el) => {
            for attr in &el.attributes {
                scan_expr_for_hooks(
                    &attr.value,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        _ => {}
    }
}

fn scan_stmt_for_hooks(
    stmt: &Stmt,
    need_state: &mut bool,
    need_effect: &mut bool,
    need_memo: &mut bool,
    need_ref: &mut bool,
    need_callback: &mut bool,
) {
    match stmt {
        Stmt::Let { value, .. } => scan_expr_for_hooks(
            value,
            need_state,
            need_effect,
            need_memo,
            need_ref,
            need_callback,
        ),
        Stmt::Assign { target, value, .. } => {
            scan_expr_for_hooks(
                target,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
            scan_expr_for_hooks(
                value,
                need_state,
                need_effect,
                need_memo,
                need_ref,
                need_callback,
            );
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                scan_expr_for_hooks(
                    v,
                    need_state,
                    need_effect,
                    need_memo,
                    need_ref,
                    need_callback,
                );
            }
        }
        Stmt::Expr { expr, .. } => scan_expr_for_hooks(
            expr,
            need_state,
            need_effect,
            need_memo,
            need_ref,
            need_callback,
        ),
    }
}

/// Emit a statement inside a React component body.
fn emit_component_stmt(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Let { pattern, value, .. } => {
            let pat = emit_component_pattern(pattern);
            let val = emit_component_expr(value);
            format!("  const {pat} = {val};\n")
        }
        Stmt::Expr { expr, .. } => {
            // Check for nested function definitions
            if let Expr::Block { .. } = expr {
                return emit_component_expr(expr);
            }
            format!("  {};\n", emit_component_expr(expr))
        }
        _ => emit_stmt(stmt, 1),
    }
}

/// Emit an expression in component context with React-specific transformations.
fn emit_component_expr(expr: &Expr) -> String {
    match expr {
        Expr::Call { callee, args, .. } => {
            let callee_str = match callee.as_ref() {
                Expr::Ident { name, .. } => {
                    // Map Vox stdlib names to React equivalents
                    match name.as_str() {
                        "use_state" => "useState".to_string(),
                        "use_effect" => "useEffect".to_string(),
                        "use_memo" => "useMemo".to_string(),
                        "use_ref" => "useRef".to_string(),
                        "use_callback" => "useCallback".to_string(),
                        other => other.to_string(),
                    }
                }
                other => emit_expr(other),
            };
            let args_str: Vec<String> = args.iter().map(|a| emit_expr(&a.value)).collect();
            format!("{callee_str}({})", args_str.join(", "))
        }
        Expr::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            let args_str: Vec<String> = args.iter().map(|a| emit_expr(&a.value)).collect();
            if let Expr::Ident { name, .. } = object.as_ref() {
                if name == "Speech" && method == "transcribe" {
                    let path_js = args_str
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "\"\"".to_string());
                    return format!(
                        "((path: string) => {{ throw new Error(\"Speech.transcribe is backend-only (Vox Oratio / Candle Whisper). Use @server or POST /api/audio/transcribe; see examples/oratio/codexAudioTranscribe.ts.\"); }})({path_js} as string)"
                    );
                }
            }
            let obj = emit_component_expr(object);
            if method == "append" && args.len() == 1 {
                return format!("[...{obj}, {}]", args_str[0]);
            }
            format!("{obj}.{method}({})", args_str.join(", "))
        }
        Expr::Match { subject, arms, .. } => {
            // For HTTP.post results in a component, emit try/catch
            let subj = emit_component_expr(subject);
            let mut out = String::new();
            out.push_str(&format!(
                "(async () => {{\n    try {{\n      const _result = await {subj};\n"
            ));
            if let Some(ok_arm) = arms.first() {
                out.push_str(&format!("      {};\n", emit_expr(&ok_arm.body)));
            }
            out.push_str("    } catch (_err) {\n");
            if arms.len() > 1 {
                out.push_str(&format!("      {};\n", emit_expr(&arms[1].body)));
            }
            out.push_str("    }\n  })()");
            out
        }
        _ => emit_expr(expr),
    }
}

fn emit_component_pattern(pattern: &vox_ast::pattern::Pattern) -> String {
    match pattern {
        vox_ast::pattern::Pattern::Ident { name, .. } => name.clone(),
        vox_ast::pattern::Pattern::Tuple { elements, .. } => {
            let elems: Vec<String> = elements.iter().map(emit_component_pattern).collect();
            format!("[{}]", elems.join(", "))
        }
        vox_ast::pattern::Pattern::Wildcard { .. } => "_".to_string(),
        _ => "_".to_string(),
    }
}

/// Map a Vox type expression to a TypeScript type string.
pub fn map_vox_type_to_ts(ty: &vox_ast::types::TypeExpr) -> String {
    match ty {
        vox_ast::types::TypeExpr::Named { name, .. } => {
            if let Some(s) = VoxScalar::parse(name) {
                s.as_ts_primitive().to_string()
            } else {
                match name.as_str() {
                    "Element" => "React.ReactElement".to_string(),
                    "Unit" => "void".to_string(),
                    other => other.to_string(),
                }
            }
        }
        vox_ast::types::TypeExpr::Generic { name, args, .. } => {
            let args_str: Vec<String> = args.iter().map(map_vox_type_to_ts).collect();
            match name.as_str() {
                "list" => format!("{}[]", args_str.join(", ")),
                "Result" => format!("Result<{}>", args_str.join(", ")),
                "Option" => format!("{} | undefined", args_str.join(", ")),
                _ => format!("{}<{}>", name, args_str.join(", ")),
            }
        }
        vox_ast::types::TypeExpr::Function {
            params,
            return_type,
            ..
        } => {
            let params_str: Vec<String> = params
                .iter()
                .enumerate()
                .map(|(i, p)| format!("arg{i}: {}", map_vox_type_to_ts(p)))
                .collect();
            format!(
                "({}) => {}",
                params_str.join(", "),
                map_vox_type_to_ts(return_type)
            )
        }
        vox_ast::types::TypeExpr::Tuple { elements, .. } => {
            let elems: Vec<String> = elements.iter().map(map_vox_type_to_ts).collect();
            format!("[{}]", elems.join(", "))
        }
        vox_ast::types::TypeExpr::Unit { .. } => "void".to_string(),
    }
}
