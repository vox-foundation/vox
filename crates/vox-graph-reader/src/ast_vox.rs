//! First-class `.vox` declaration and call-edge extractor for `vox-graph-reader`.
//!
//! Kept in Layer 1 without depending on `vox-compiler` (`vox-parser` / `vox-lexer`) so the
//! graph reader remains lightweight and cycle-free. Recognizes canonical Vox grammar keywords
//! (`component`, `table`, `query`, `mutation`, `server`, `tool`, `resource`, `form`, `index`,
//! `fn`, `type`, `struct`, `enum`, `trait`, `actor`, `workflow`, `activity`, `state_machine`,
//! `module`, `routes`) and extracts call, JSX/view composition, `implements`, and `imports` edges.

use std::collections::HashMap;

use crate::ast::{ExtractedEdge, ExtractedGraph, ExtractedNode};

const VOX_DECL_KEYWORDS: &[&str] = &[
    "component",
    "table",
    "query",
    "mutation",
    "server",
    "tool",
    "resource",
    "form",
    "index",
    "fn",
    "type",
    "struct",
    "enum",
    "trait",
    "actor",
    "workflow",
    "activity",
    "state_machine",
    "module",
    "routes",
];

const VOX_CALL_KEYWORDS: &[&str] = &[
    "if",
    "else",
    "match",
    "while",
    "for",
    "loop",
    "return",
    "let",
    "mut",
    "pub",
    "async",
    "await",
    "spawn",
    "with",
    "in",
    "on",
    "as",
    "true",
    "false",
    "Some",
    "None",
    "Ok",
    "Err",
    "component",
    "table",
    "query",
    "mutation",
    "server",
    "tool",
    "resource",
    "form",
    "index",
    "fn",
    "type",
    "struct",
    "enum",
    "trait",
    "actor",
    "workflow",
    "activity",
    "state_machine",
    "module",
    "routes",
    "import",
];

fn qualify_vox_id(module_id: &str, scope: &[String], name: &str) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(scope.len() + 2);
    if !module_id.is_empty() {
        parts.push(module_id);
    }
    parts.extend(scope.iter().map(String::as_str));
    parts.push(name);
    parts.join("::")
}

#[derive(Default)]
struct VoxIdAlloc(HashMap<String, u32>);

impl VoxIdAlloc {
    fn alloc(&mut self, id: String) -> String {
        let n = self.0.entry(id.clone()).or_insert(0);
        *n += 1;
        if *n == 1 { id } else { format!("{id}#{n}") }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Strip `//` line comments and string literals from a source line while keeping brace counts
/// and top-level identifiers intact.
fn strip_comments_and_strings(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut in_str: Option<char> = None;
    while i < chars.len() {
        let c = chars[i];
        if let Some(delim) = in_str {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == delim {
                in_str = None;
            }
            i += 1;
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            break;
        }
        if c == '"' || c == '\'' {
            in_str = Some(c);
            out.push(' ');
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Extract the first identifier token from `s`, returning `(ident, rest)`.
fn next_ident(s: &str) -> Option<(&str, &str)> {
    let trimmed = s.trim_start();
    let mut chars = trimmed.char_indices();
    let (_, first) = chars.next()?;
    if !is_ident_start(first) {
        return None;
    }
    let mut end = first.len_utf8();
    for (idx, ch) in chars {
        if is_ident_continue(ch) {
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    Some((&trimmed[..end], &trimmed[end..]))
}

#[derive(Clone, Debug)]
struct ScopeEntry {
    name: String,
    kind: String,
    node_id: String,
    brace_depth: i32,
    is_container: bool,
    is_callable: bool,
}

/// Extract `.vox` declarations and call/composition/import/implements edges.
pub fn extract_vox_ast_in_module(content: &str, module_id: &str) -> ExtractedGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut ids = VoxIdAlloc::default();
    let mut scope_stack: Vec<ScopeEntry> = Vec::new();
    let mut brace_depth: i32 = 0;

    for raw_line in content.lines() {
        let cleaned = strip_comments_and_strings(raw_line);
        let trimmed = cleaned.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Skip leading decorator `@foo(...)` prefix if on the same line as a declaration,
        // or skip standalone decorator lines.
        let mut cursor = trimmed;
        while cursor.starts_with('@') {
            let after_at = &cursor[1..];
            if let Some((_, rest)) = next_ident(after_at) {
                let rest_trim = rest.trim_start();
                if rest_trim.starts_with('(') {
                    if let Some(close_idx) = rest_trim.find(')') {
                        cursor = rest_trim[close_idx + 1..].trim_start();
                        continue;
                    }
                } else {
                    cursor = rest_trim;
                    continue;
                }
            }
            cursor = "";
            break;
        }

        // Strip `pub` / `async` modifiers.
        loop {
            if let Some((kw, rest)) = next_ident(cursor) {
                if kw == "pub" || kw == "async" {
                    cursor = rest.trim_start();
                    continue;
                }
            }
            break;
        }

        let mut declared_on_line: Option<ScopeEntry> = None;

        if let Some((kw, rest)) = next_ident(cursor) {
            if kw == "import" {
                extract_vox_imports(rest, module_id, &mut edges);
            } else if kw == "impl" {
                if let Some(entry) =
                    parse_vox_impl_header(rest, module_id, &scope_stack, brace_depth, &mut edges)
                {
                    declared_on_line = Some(entry);
                }
            } else if VOX_DECL_KEYWORDS.contains(&kw) {
                let decl_name = if kw == "routes" {
                    Some("routes".to_string())
                } else {
                    next_ident(rest).map(|(n, _)| n.to_string())
                };
                if let Some(name) = decl_name {
                    let container_scope: Vec<String> = scope_stack
                        .iter()
                        .filter(|s| s.is_container)
                        .map(|s| s.name.clone())
                        .collect();
                    let in_type_container = scope_stack
                        .iter()
                        .rev()
                        .find(|s| s.is_container)
                        .is_some_and(|s| matches!(s.kind.as_str(), "actor" | "trait" | "impl"));
                    let kind = if kw == "fn" && in_type_container {
                        "method"
                    } else {
                        kw
                    };
                    let id = ids.alloc(qualify_vox_id(module_id, &container_scope, &name));
                    nodes.push(ExtractedNode {
                        id: id.clone(),
                        label: name.clone(),
                        kind: kind.to_string(),
                    });
                    let is_container = matches!(kw, "module" | "actor" | "trait" | "state_machine");
                    let is_callable = matches!(
                        kw,
                        "component"
                            | "query"
                            | "mutation"
                            | "server"
                            | "tool"
                            | "resource"
                            | "form"
                            | "fn"
                            | "workflow"
                            | "activity"
                            | "routes"
                    );
                    if cleaned.contains('{') {
                        declared_on_line = Some(ScopeEntry {
                            name,
                            kind: kind.to_string(),
                            node_id: id,
                            brace_depth,
                            is_container,
                            is_callable,
                        });
                    }
                }
            }
        }

        // Determine current callable scope for call-edge extraction on this line.
        let active_callable = declared_on_line
            .as_ref()
            .filter(|e| e.is_callable)
            .map(|e| e.node_id.clone())
            .or_else(|| {
                scope_stack
                    .iter()
                    .rev()
                    .find(|s| s.is_callable)
                    .map(|s| s.node_id.clone())
            });

        if let Some(ref caller_id) = active_callable {
            let call_scan_slice = if declared_on_line.is_some() {
                // Only scan after the opening `{` of a declaration header line so the
                // declaration's own parameter list `fn foo(...)` is not mistaken for a call.
                cleaned
                    .find('{')
                    .map(|idx| &cleaned[idx + 1..])
                    .unwrap_or("")
            } else {
                cleaned.as_str()
            };
            extract_vox_calls_in_slice(call_scan_slice, caller_id, &mut edges);
        }

        let opens = cleaned.chars().filter(|&c| c == '{').count() as i32;
        let closes = cleaned.chars().filter(|&c| c == '}').count() as i32;
        if let Some(entry) = declared_on_line {
            if opens > closes {
                scope_stack.push(entry);
            }
        }
        brace_depth += opens - closes;
        while scope_stack
            .last()
            .is_some_and(|top| brace_depth <= top.brace_depth)
        {
            scope_stack.pop();
        }
    }

    ExtractedGraph { nodes, edges }
}

fn extract_vox_imports(rest: &str, module_id: &str, edges: &mut Vec<ExtractedEdge>) {
    let source = if module_id.is_empty() {
        "<vox>".to_string()
    } else {
        module_id.to_string()
    };
    for part in rest.split(',') {
        let token = part
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim();
        let target = token.rsplit('.').next().unwrap_or(token).trim();
        if let Some((ident, _)) = next_ident(target) {
            if !VOX_CALL_KEYWORDS.contains(&ident) {
                edges.push(ExtractedEdge {
                    source: source.clone(),
                    target: ident.to_string(),
                    confidence: "resolved".into(),
                    kind: "imports".into(),
                });
            }
        }
    }
}

fn parse_vox_impl_header(
    rest: &str,
    module_id: &str,
    scope_stack: &[ScopeEntry],
    brace_depth: i32,
    edges: &mut Vec<ExtractedEdge>,
) -> Option<ScopeEntry> {
    let (first_ident, after_first) = next_ident(rest)?;
    let after_trim = after_first.trim_start();
    let container_scope: Vec<String> = scope_stack
        .iter()
        .filter(|s| s.is_container)
        .map(|s| s.name.clone())
        .collect();
    if let Some((kw_for, after_for)) = next_ident(after_trim) {
        if kw_for == "for" {
            if let Some((self_ty, _)) = next_ident(after_for) {
                let source = qualify_vox_id(module_id, &container_scope, self_ty);
                edges.push(ExtractedEdge {
                    source,
                    target: first_ident.to_string(),
                    confidence: "resolved".into(),
                    kind: "implements".into(),
                });
                let container = format!("<{self_ty} as {first_ident}>");
                let id = qualify_vox_id(module_id, &container_scope, &container);
                return Some(ScopeEntry {
                    name: container,
                    kind: "impl".to_string(),
                    node_id: id,
                    brace_depth,
                    is_container: true,
                    is_callable: false,
                });
            }
        }
    }
    let id = qualify_vox_id(module_id, &container_scope, first_ident);
    Some(ScopeEntry {
        name: first_ident.to_string(),
        kind: "impl".to_string(),
        node_id: id,
        brace_depth,
        is_container: true,
        is_callable: false,
    })
}

fn extract_vox_calls_in_slice(slice: &str, caller_id: &str, edges: &mut Vec<ExtractedEdge>) {
    let bytes = slice.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        // JSX-like view tag `<Component` inside a `.vox` component/view block.
        if ch == '<' {
            if let Some((tag, _)) = next_ident(&slice[i + 1..]) {
                if tag.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                    edges.push(ExtractedEdge {
                        source: caller_id.to_string(),
                        target: tag.to_string(),
                        confidence: "resolved".into(),
                        kind: "calls".into(),
                    });
                }
            }
            i += 1;
            continue;
        }
        if is_ident_start(ch) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_continue(bytes[i] as char) {
                i += 1;
            }
            let ident = &slice[start..i];
            // Check for `Type.method(` or `Type::method(` or bare `callee(`.
            let mut j = i;
            while j < bytes.len() && (bytes[j] as char).is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] as char == '(' {
                if !VOX_CALL_KEYWORDS.contains(&ident) {
                    edges.push(ExtractedEdge {
                        source: caller_id.to_string(),
                        target: ident.to_string(),
                        confidence: "resolved".into(),
                        kind: "calls".into(),
                    });
                }
            } else if j + 1 < bytes.len()
                && ((bytes[j] as char == '.')
                    || (bytes[j] as char == ':' && bytes[j + 1] as char == ':'))
            {
                let sep_len = if bytes[j] as char == ':' { 2 } else { 1 };
                if let Some((member, rest)) = next_ident(&slice[j + sep_len..]) {
                    if rest.trim_start().starts_with('(') && !VOX_CALL_KEYWORDS.contains(&member) {
                        let target = if ident.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                        {
                            format!("{ident}::{member}")
                        } else {
                            member.to_string()
                        };
                        edges.push(ExtractedEdge {
                            source: caller_id.to_string(),
                            target,
                            confidence: "resolved".into(),
                            kind: "calls".into(),
                        });
                    }
                }
            }
            continue;
        }
        i += ch.len_utf8();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_vox_declarations_and_calls() {
        let src = r#"
import std.db

table Task {
    title: str
}

@island
component TaskCard(title: str) {
    <Badge />
}

query list_tasks() -> List[Task] {
    helper_fetch()
}

mutation create_task(title: str) {
    list_tasks()
}

fn helper_fetch() {}

actor Worker {
    fn process() {
        helper_fetch()
    }
}
"#;
        let g = extract_vox_ast_in_module(src, "apps/tasks/main.vox");
        let by_id: HashMap<&str, &str> = g
            .nodes
            .iter()
            .map(|n| (n.id.as_str(), n.kind.as_str()))
            .collect();
        assert_eq!(by_id.get("apps/tasks/main.vox::Task"), Some(&"table"));
        assert_eq!(
            by_id.get("apps/tasks/main.vox::TaskCard"),
            Some(&"component")
        );
        assert_eq!(by_id.get("apps/tasks/main.vox::list_tasks"), Some(&"query"));
        assert_eq!(
            by_id.get("apps/tasks/main.vox::create_task"),
            Some(&"mutation")
        );
        assert_eq!(by_id.get("apps/tasks/main.vox::helper_fetch"), Some(&"fn"));
        assert_eq!(by_id.get("apps/tasks/main.vox::Worker"), Some(&"actor"));
        assert_eq!(
            by_id.get("apps/tasks/main.vox::Worker::process"),
            Some(&"method")
        );

        assert!(g.edges.iter().any(|e| {
            e.source == "apps/tasks/main.vox::list_tasks"
                && e.target == "helper_fetch"
                && e.kind == "calls"
        }));
        assert!(g.edges.iter().any(|e| {
            e.source == "apps/tasks/main.vox::TaskCard" && e.target == "Badge" && e.kind == "calls"
        }));
        assert!(
            g.edges
                .iter()
                .any(|e| e.kind == "imports" && e.target == "db")
        );
    }
}
