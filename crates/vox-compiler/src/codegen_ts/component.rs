use crate::ast::decl::FnDecl;
use crate::ast::expr::Expr;
use crate::ast::scalar_mapping::VoxScalar;
use crate::ast::stmt::Stmt;
use crate::codegen_ts::jsx::{emit_expr, emit_jsx_element, emit_jsx_self_closing, emit_stmt};
use std::collections::BTreeSet;

/// Mapping from Vox `use_*` snake_case names to React camelCase hook names.
/// This covers all stable React 18/19 built-in hooks.
const REACT_HOOK_REGISTRY: &[(&str, &str)] = &[
    ("use_state", "useState"),
    ("use_effect", "useEffect"),
    ("use_memo", "useMemo"),
    ("use_ref", "useRef"),
    ("use_callback", "useCallback"),
    ("use_context", "useContext"),
    ("use_reducer", "useReducer"),
    ("use_id", "useId"),
    ("use_deferred_value", "useDeferredValue"),
    ("use_transition", "useTransition"),
    ("use_sync_external_store", "useSyncExternalStore"),
    ("use_layout_effect", "useLayoutEffect"),
    ("use_imperative_handle", "useImperativeHandle"),
    ("use_debug_value", "useDebugValue"),
    ("use_action_state", "useActionState"),
    ("use_optimistic", "useOptimistic"),
    ("use_form_status", "useFormStatus"),
];

/// Generate a React component from a Vox @component function declaration.
/// Returns (filename, content) tuple.
pub fn generate_component(func: &FnDecl, has_styles: bool) -> (String, String) {
    let name = &func.name;
    let filename = format!("{name}.tsx");
    let mut out = String::new();

    // Collect hook names referenced in the component body.
    let mut vox_hooks_used: BTreeSet<String> = BTreeSet::new();
    for stmt in &func.body {
        collect_hooks_in_stmt(stmt, &mut vox_hooks_used);
    }

    // Map Vox snake_case hook names → React camelCase imports.
    let mut react_hooks: BTreeSet<&str> = BTreeSet::new();
    for vox_name in &vox_hooks_used {
        if let Some((_, react_name)) = REACT_HOOK_REGISTRY
            .iter()
            .find(|(vox, _)| *vox == vox_name.as_str())
        {
            react_hooks.insert(react_name);
        }
        // Custom use_* hooks (not in registry) are not React built-ins;
        // they are imported separately below if the user imports them.
    }

    // Emit React import — only include hooks actually used.
    if react_hooks.is_empty() {
        out.push_str("import React from \"react\";\n\n");
    } else {
        let hook_list: Vec<&&str> = react_hooks.iter().collect();
        out.push_str(&format!(
            "import React, {{ {} }} from \"react\";\n\n",
            hook_list.iter().map(|s| **s).collect::<Vec<_>>().join(", ")
        ));
    }
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

/// Collect every Vox `use_*` call name referenced in a statement tree.
fn collect_hooks_in_stmt(stmt: &Stmt, out: &mut BTreeSet<String>) {
    match stmt {
        Stmt::Let { value, .. } => collect_hooks_in_expr(value, out),
        Stmt::Assign { target, value, .. } => {
            collect_hooks_in_expr(target, out);
            collect_hooks_in_expr(value, out);
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_hooks_in_expr(v, out);
            }
        }
        Stmt::Expr { expr, .. } => collect_hooks_in_expr(expr, out),
    }
}

/// Collect every Vox `use_*` call name referenced in an expression tree.
fn collect_hooks_in_expr(expr: &Expr, out: &mut BTreeSet<String>) {
    match expr {
        Expr::Call { callee, args, .. } => {
            if let Expr::Ident { name, .. } = callee.as_ref() {
                if name.starts_with("use_") {
                    out.insert(name.clone());
                }
            } else {
                collect_hooks_in_expr(callee, out);
            }
            for a in args {
                collect_hooks_in_expr(&a.value, out);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            collect_hooks_in_expr(object, out);
            for a in args {
                collect_hooks_in_expr(&a.value, out);
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_hooks_in_expr(left, out);
            collect_hooks_in_expr(right, out);
        }
        Expr::Unary { operand, .. } => collect_hooks_in_expr(operand, out),
        Expr::FieldAccess { object, .. } => collect_hooks_in_expr(object, out),
        Expr::Lambda { body, .. } => collect_hooks_in_expr(body, out),
        Expr::Pipe { left, right, .. } => {
            collect_hooks_in_expr(left, out);
            collect_hooks_in_expr(right, out);
        }
        Expr::Spawn { target, .. } => collect_hooks_in_expr(target, out),
        Expr::With {
            operand, options, ..
        } => {
            collect_hooks_in_expr(operand, out);
            collect_hooks_in_expr(options, out);
        }
        Expr::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            collect_hooks_in_expr(condition, out);
            for s in then_body {
                collect_hooks_in_stmt(s, out);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    collect_hooks_in_stmt(s, out);
                }
            }
        }
        Expr::For { iterable, body, .. } => {
            collect_hooks_in_expr(iterable, out);
            collect_hooks_in_expr(body, out);
        }
        Expr::Match { subject, arms, .. } => {
            collect_hooks_in_expr(subject, out);
            for arm in arms {
                collect_hooks_in_expr(&arm.body, out);
            }
        }
        Expr::Block { stmts, .. } => {
            for s in stmts {
                collect_hooks_in_stmt(s, out);
            }
        }
        Expr::ListLit { elements, .. } | Expr::TupleLit { elements, .. } => {
            for e in elements {
                collect_hooks_in_expr(e, out);
            }
        }
        Expr::ObjectLit { fields, .. } => {
            for (_, v) in fields {
                collect_hooks_in_expr(v, out);
            }
        }
        Expr::StringInterp { parts, .. } => {
            for p in parts {
                if let crate::ast::expr::StringPart::Interpolation(e) = p {
                    collect_hooks_in_expr(e, out);
                }
            }
        }
        Expr::Jsx(el) => {
            for ch in &el.children {
                collect_hooks_in_expr(ch, out);
            }
            for attr in &el.attributes {
                collect_hooks_in_expr(&attr.value, out);
            }
        }
        Expr::JsxSelfClosing(el) => {
            for attr in &el.attributes {
                collect_hooks_in_expr(&attr.value, out);
            }
        }
        // Leaf expressions
        Expr::IntLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::StringLit { .. }
        | Expr::Ident { .. } => {}
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

fn emit_component_pattern(pattern: &crate::ast::pattern::Pattern) -> String {
    match pattern {
        crate::ast::pattern::Pattern::Ident { name, .. } => name.clone(),
        crate::ast::pattern::Pattern::Tuple { elements, .. } => {
            let elems: Vec<String> = elements.iter().map(emit_component_pattern).collect();
            format!("[{}]", elems.join(", "))
        }
        crate::ast::pattern::Pattern::Wildcard { .. } => "_".to_string(),
        _ => "_".to_string(),
    }
}

/// Map a Vox type expression to a TypeScript type string.
pub fn map_vox_type_to_ts(ty: &crate::ast::types::TypeExpr) -> String {
    match ty {
        crate::ast::types::TypeExpr::Named { name, .. } => {
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
        crate::ast::types::TypeExpr::Generic { name, args, .. } => {
            let args_str: Vec<String> = args.iter().map(map_vox_type_to_ts).collect();
            match name.as_str() {
                "list" => format!("{}[]", args_str.join(", ")),
                "Result" => format!("Result<{}>", args_str.join(", ")),
                "Option" => format!("{} | undefined", args_str.join(", ")),
                _ => format!("{}<{}>", name, args_str.join(", ")),
            }
        }
        crate::ast::types::TypeExpr::Function {
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
        crate::ast::types::TypeExpr::Tuple { elements, .. } => {
            let elems: Vec<String> = elements.iter().map(map_vox_type_to_ts).collect();
            format!("[{}]", elems.join(", "))
        }
        crate::ast::types::TypeExpr::Unit { .. } => "void".to_string(),
    }
}
