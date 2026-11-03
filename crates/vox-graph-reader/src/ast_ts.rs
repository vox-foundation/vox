//! Tree-sitter (TS/JS/Python) helpers for `ast.rs`. Split out to stay under the
//! TOESTUB 500-line cap; no behavior of their own.
#![cfg(feature = "tree-sitter-grammars")]

use std::collections::HashMap;

/// Push `node`'s children so they pop in source order (keeps `#n` ids in source order).
pub(crate) fn push_children<'t>(
    stack: &mut Vec<tree_sitter::Node<'t>>,
    node: tree_sitter::Node<'t>,
    cursor: &mut tree_sitter::TreeCursor<'t>,
) {
    let children: Vec<_> = node.children(cursor).collect();
    stack.extend(children.into_iter().rev());
}

/// A named function definition: fn / method declarations, plus `const f = () => ...` and
/// `const f = function () {...}` declarators (the common TS / React component shape).
pub(crate) fn ts_fn_like(node: tree_sitter::Node) -> bool {
    match node.kind() {
        "function_declaration" | "method_definition" | "function_definition" => true,
        "variable_declarator" => node.child_by_field_name("value").is_some_and(|v| {
            matches!(
                v.kind(),
                "arrow_function" | "function_expression" | "function"
            )
        }),
        _ => false,
    }
}

pub(crate) fn ts_class_like(node: tree_sitter::Node) -> bool {
    matches!(
        node.kind(),
        "class_definition" | "class_declaration" | "abstract_class_declaration"
    )
}

/// Names of `node`'s named enclosing classes / fns, outermost first, and whether the
/// nearest of them is a class (=> `node` is a method).
pub(crate) fn ts_scope(node: tree_sitter::Node, content: &str) -> (Vec<String>, bool) {
    let mut segs = Vec::new();
    let mut nearest_is_class = None;
    let mut cur = node.parent();
    while let Some(n) = cur {
        if ts_fn_like(n) || ts_class_like(n) {
            if let Some(name) = n
                .child_by_field_name("name")
                .and_then(|x| x.utf8_text(content.as_bytes()).ok())
            {
                nearest_is_class.get_or_insert(ts_class_like(n));
                segs.push(name.to_string());
            }
        }
        cur = n.parent();
    }
    segs.reverse();
    (segs, nearest_is_class == Some(true))
}

/// Symbol id of `node`'s nearest enclosing named fn. Calls outside any fn (module top
/// level, anonymous callbacks at top level) have no source and emit no edge.
pub(crate) fn ts_enclosing_fn(
    node: tree_sitter::Node,
    def_ids: &HashMap<usize, String>,
) -> Option<String> {
    let mut cur = node.parent();
    while let Some(n) = cur {
        if ts_fn_like(n) {
            return def_ids.get(&n.id()).cloned();
        }
        cur = n.parent();
    }
    None
}

/// Text of a `string` literal node, stripped of its quote delimiters. Uses the A0d-recorded
/// kind names (string literal == "string").
pub(crate) fn string_literal_value(node: tree_sitter::Node, content: &str) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }
    let raw = node.utf8_text(content.as_bytes()).ok()?;
    let trimmed = raw
        .strip_prefix('\'')
        .or_else(|| raw.strip_prefix('"'))
        .or_else(|| raw.strip_prefix('`'))
        .unwrap_or(raw);
    let trimmed = trimmed
        .strip_suffix('\'')
        .or_else(|| trimmed.strip_suffix('"'))
        .or_else(|| trimmed.strip_suffix('`'))
        .unwrap_or(trimmed);
    Some(trimmed.to_string())
}

/// Value of the positional argument at `idx` in a `call_expression`'s `arguments` node, if it
/// is a string literal. Template-string / computed args → None (honest miss).
pub(crate) fn arg_string_literal(
    args: tree_sitter::Node,
    idx: usize,
    content: &str,
) -> Option<String> {
    let mut cursor = args.walk();
    let arg = args.named_children(&mut cursor).nth(idx)?;
    string_literal_value(arg, content)
}

/// Value of `field`'s string entry inside the object literal positional argument at `idx`.
pub(crate) fn arg_object_string_field(
    args: tree_sitter::Node,
    idx: usize,
    field: &str,
    content: &str,
) -> Option<String> {
    let mut cursor = args.walk();
    let obj = args.named_children(&mut cursor).nth(idx)?;
    if obj.kind() != "object" {
        return None;
    }
    let mut obj_cursor = obj.walk();
    for pair in obj.named_children(&mut obj_cursor) {
        if pair.kind() != "pair" {
            continue;
        }
        let key = pair.child_by_field_name("key")?;
        let key_text = key.utf8_text(content.as_bytes()).ok()?;
        if key_text == field {
            let value = pair.child_by_field_name("value")?;
            return string_literal_value(value, content);
        }
    }
    None
}

pub(crate) fn extract_ts_ast(
    ext: &str,
    content: &str,
    module_id: &str,
    wrappers: &HashMap<String, String>,
    boundary_callees: &[&str],
    nodes: &mut Vec<crate::ast::ExtractedNode>,
    edges: &mut Vec<crate::ast::ExtractedEdge>,
) {
    let language = match ext {
        "ts" | "js" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
        "tsx" | "jsx" => Some(tree_sitter_typescript::LANGUAGE_TSX),
        "py" => Some(tree_sitter_python::LANGUAGE),
        _ => None,
    };
    let Some(lang) = language else {
        return;
    };
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&lang.into()).is_err() {
        return;
    }
    let Some(tree) = parser.parse(content, None) else {
        return;
    };
    let mut cursor = tree.walk();
    let mut stack = vec![tree.root_node()];
    let mut ids = crate::ast::IdAlloc::default();
    let mut def_ids: HashMap<usize, String> = HashMap::new();
    while let Some(node) = stack.pop() {
        let is_fn_def = ts_fn_like(node);
        let is_class = ts_class_like(node);
        let is_call = matches!(node.kind(), "call_expression" | "call");
        let is_jsx = matches!(
            node.kind(),
            "jsx_self_closing_element" | "jsx_opening_element"
        );

        if is_fn_def || is_class {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                    let (scope, in_class) = ts_scope(node, content);
                    let id = ids.alloc(crate::ast::qualify_in(module_id, &scope, name));
                    let kind = if is_class {
                        "struct"
                    } else if in_class || node.kind() == "method_definition" {
                        "method"
                    } else {
                        "fn"
                    };
                    nodes.push(crate::ast::ExtractedNode {
                        id: id.clone(),
                        label: name.to_string(),
                        kind: kind.to_string(),
                    });
                    def_ids.insert(node.id(), id);
                }
            }
        }
        let current_fn = if is_call || is_jsx {
            ts_enclosing_fn(node, &def_ids)
        } else {
            None
        };
        if is_jsx {
            if let Some(ref source_fn) = current_fn {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                        if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                            edges.push(crate::ast::ExtractedEdge {
                                source: source_fn.clone(),
                                target: name.to_string(),
                                confidence: "resolved".into(),
                                kind: "calls".into(),
                            });
                        }
                    }
                }
            }
        }
        if is_call {
            if let Some(ref source_fn) = current_fn {
                if let Some(function_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = function_node.utf8_text(content.as_bytes()) {
                        let bare = callee.rsplit('.').next().unwrap_or(callee);
                        let args = node.child_by_field_name("arguments");
                        if boundary_callees.contains(&bare) {
                            if let Some(arg0) = args.and_then(|a| arg_string_literal(a, 0, content))
                            {
                                if arg0 == "invoke_mcp_tool" {
                                    if let Some(tool) = args.and_then(|a| {
                                        arg_object_string_field(a, 1, "tool", content)
                                    }) {
                                        edges.push(crate::ast::ExtractedEdge {
                                            source: source_fn.clone(),
                                            target: format!("tool:{tool}"),
                                            confidence: "declared".into(),
                                            kind: "calls".into(),
                                        });
                                    }
                                } else {
                                    edges.push(crate::ast::ExtractedEdge {
                                        source: source_fn.clone(),
                                        target: format!("cmd:{arg0}"),
                                        confidence: "declared".into(),
                                        kind: "calls".into(),
                                    });
                                }
                            }
                            push_children(&mut stack, node, &mut cursor);
                            continue;
                        } else if callee.starts_with("voxTransport.") {
                            if let Some(target) = wrappers.get(bare) {
                                edges.push(crate::ast::ExtractedEdge {
                                    source: source_fn.clone(),
                                    target: target.clone(),
                                    confidence: "declared".into(),
                                    kind: "calls".into(),
                                });
                            }
                            push_children(&mut stack, node, &mut cursor);
                            continue;
                        }
                        edges.push(crate::ast::ExtractedEdge {
                            source: source_fn.clone(),
                            target: callee.to_string(),
                            confidence: "resolved".into(),
                            kind: "calls".into(),
                        });
                    }
                }
            }
        }
        push_children(&mut stack, node, &mut cursor);
    }
}
