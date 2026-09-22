use std::collections::HashMap;
use std::path::Path;
use syn::visit::Visit;

/// Frontend callees that mark a backend boundary crossing. The arg0 string literal becomes
/// the target. TODO: a declarative config when a 3rd boundary kind lands — see spec §non-fork.
const BOUNDARY_CALLEES: &[&str] = &["invoke", "callTool"];

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ExtractedNode {
    pub id: String,
    pub label: String,
    pub kind: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ExtractedEdge {
    pub source: String,
    pub target: String,
    #[serde(default = "default_confidence")]
    pub confidence: String,
}

pub(crate) fn default_confidence() -> String {
    "resolved".into()
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ExtractedGraph {
    pub nodes: Vec<ExtractedNode>,
    pub edges: Vec<ExtractedEdge>,
}

/// Bump when the extraction scheme changes (node-id format, edge rules). Folded into the
/// per-file cache key in `rebuild` so unchanged files re-extract instead of returning a
/// graph built under the old scheme.
pub const EXTRACTOR_VERSION: &str = "4";

// Symbol id scheme (EXTRACTOR_VERSION 4). Ids are unique and deterministic per file content:
//
//     <path>::<container>*::<name>[#<n>]
//
// * `<path>` is the repo-relative file path. It never contains `::`, so
//   `id.split("::").next()` is always the file and `id.rsplit("::").next()` the name.
// * `<container>` is zero or more enclosing scopes, outermost first: inline `mod` names,
//   enclosing fns (for nested items), Rust impl self types, trait names, TS/Python classes.
//   - inherent impl: the self type's last path segment without generics
//     (`impl<T> a::Foo<T>` -> `Foo`);
//   - trait impl: `<Self as Trait>`, Trait being the trait's last path segment WITH its
//     generic args (`impl From<u8> for Foo` -> `<Foo as From<u8>>`), so the several
//     `From<_>` impls of one type stay distinct;
//   - non-path self types (`&T`, `[T]`, tuples) render as compact tokens.
//   A rendered container never contains `::` (replaced by `.`), so every `::`-separated
//   segment is exactly one path / container / name.
// * Free top-level fns keep `<path>::<name>`.
// * `#<n>` (n >= 2, source order) disambiguates a residual same-file collision, e.g.
//   cfg-gated duplicate definitions. The first occurrence keeps the bare id.
//
// Kinds: `fn`, `method` (fn directly inside an impl / trait / class), `struct` (Rust
// structs, TS/Python classes), `enum`, `trait`, `type`.

/// Join `module_id`, the container scope, and `name` with `::`. Empty `module_id` yields
/// an unqualified id so the legacy `extract_ast` wrapper keeps bare names.
fn qualify_in(module_id: &str, scope: &[String], name: &str) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(scope.len() + 2);
    if !module_id.is_empty() {
        parts.push(module_id);
    }
    parts.extend(scope.iter().map(String::as_str));
    parts.push(name);
    parts.join("::")
}

/// Per-file id allocator: the n-th (n >= 2) occurrence of an id gets a `#n` suffix.
#[derive(Default)]
struct IdAlloc(HashMap<String, u32>);

impl IdAlloc {
    fn alloc(&mut self, id: String) -> String {
        let n = self.0.entry(id.clone()).or_insert(0);
        *n += 1;
        if *n == 1 { id } else { format!("{id}#{n}") }
    }
}

/// Token text with whitespace dropped except between two identifier characters, and `::`
/// replaced by `.` (see the id scheme above): `From < Vec < T > >` -> `From<Vec<T>>`.
fn compact_tokens(t: &impl quote::ToTokens) -> String {
    let raw = t.to_token_stream().to_string();
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len());
    for (i, &c) in chars.iter().enumerate() {
        if c == ' '
            && !(out.chars().last().is_some_and(is_ident)
                && chars.get(i + 1).copied().is_some_and(is_ident))
        {
            continue;
        }
        out.push(c);
    }
    out.replace("::", ".")
}

fn impl_container(node: &syn::ItemImpl) -> String {
    let self_ty = match &*node.self_ty {
        syn::Type::Path(p) => p
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default(),
        other => compact_tokens(other),
    };
    match node
        .trait_
        .as_ref()
        .and_then(|(_, path, _)| path.segments.last())
    {
        Some(seg) => format!(
            "<{self_ty} as {}{}>",
            seg.ident,
            compact_tokens(&seg.arguments)
        ),
        None => self_ty,
    }
}

struct RustVisitor {
    module_id: String,
    /// Enclosing container segments (see the id scheme).
    scope: Vec<String>,
    /// Qualified prefix of the enclosing impl / trait, for resolving `Self::x()`.
    impl_prefix: Option<String>,
    ids: IdAlloc,
    nodes: Vec<ExtractedNode>,
    edges: Vec<ExtractedEdge>,
    current_fn: Option<String>,
}

impl RustVisitor {
    fn def(&mut self, name: String, kind: &str) -> String {
        let id = self
            .ids
            .alloc(qualify_in(&self.module_id, &self.scope, &name));
        self.nodes.push(ExtractedNode {
            id: id.clone(),
            label: name,
            kind: kind.to_string(),
        });
        id
    }

    /// Define a fn/method and walk its body with it as the edge source and a scope.
    fn def_fn(&mut self, ident: &syn::Ident, kind: &str, walk: impl FnOnce(&mut Self)) {
        let name = ident.to_string();
        let id = self.def(name.clone(), kind);
        let old_fn = self.current_fn.replace(id);
        self.scope.push(name);
        walk(self);
        self.scope.pop();
        self.current_fn = old_fn;
    }

    /// Walk a container (inline mod, impl, trait). `is_impl` makes it the `Self` target.
    fn enter(&mut self, seg: String, is_impl: bool, walk: impl FnOnce(&mut Self)) {
        self.scope.push(seg);
        let old_impl = if is_impl {
            let prefix = self.scope.join("::");
            let prefix = if self.module_id.is_empty() {
                prefix
            } else {
                format!("{}::{prefix}", self.module_id)
            };
            self.impl_prefix.replace(prefix)
        } else {
            self.impl_prefix.clone()
        };
        walk(self);
        self.impl_prefix = old_impl;
        self.scope.pop();
    }
}

impl<'ast> Visit<'ast> for RustVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.def_fn(&node.sig.ident, "fn", |v| {
            syn::visit::visit_item_fn(v, node)
        });
    }
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.def_fn(&node.sig.ident, "method", |v| {
            syn::visit::visit_impl_item_fn(v, node)
        });
    }
    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.def_fn(&node.sig.ident, "method", |v| {
            syn::visit::visit_trait_item_fn(v, node)
        });
    }
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        self.enter(impl_container(node), true, |v| {
            syn::visit::visit_item_impl(v, node)
        });
    }
    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        let name = node.ident.to_string();
        self.def(name.clone(), "trait");
        self.enter(name, true, |v| syn::visit::visit_item_trait(v, node));
    }
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if node.content.is_some() {
            self.enter(node.ident.to_string(), false, |v| {
                syn::visit::visit_item_mod(v, node)
            });
        }
    }
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.def(node.ident.to_string(), "struct");
        syn::visit::visit_item_struct(self, node);
    }
    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.def(node.ident.to_string(), "enum");
        syn::visit::visit_item_enum(self, node);
    }
    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        self.def(node.ident.to_string(), "type");
        syn::visit::visit_item_type(self, node);
    }
    /// Call targets: `f()` -> bare `f` (free fns only); `Self::f()` -> the enclosing
    /// impl's qualified id; `a::Type::f()` (capitalized qualifier) -> `Type::f`, a method
    /// hint. All other paths (`module::f()`) -> bare `f`. Resolved in `rebuild`.
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let (syn::Expr::Path(expr_path), Some(current_fn)) = (&*node.func, &self.current_fn) {
            let segs = &expr_path.path.segments;
            if let Some(last) = segs.last() {
                let name = last.ident.to_string();
                let qualifier = segs.len().checked_sub(2).map(|i| &segs[i].ident);
                let target = match (qualifier, &self.impl_prefix) {
                    (Some(q), Some(prefix)) if q == "Self" => format!("{prefix}::{name}"),
                    (Some(q), _) if q.to_string().starts_with(char::is_uppercase) => {
                        format!("{q}::{name}")
                    }
                    _ => name,
                };
                self.edges.push(ExtractedEdge {
                    source: current_fn.clone(),
                    target,
                    confidence: "resolved".into(),
                });
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

/// Back-compat wrapper: bare ids. Used by `ast_tests` (which assert on `label`, unaffected).
pub fn extract_ast(path: &Path, content: &str) -> ExtractedGraph {
    extract_ast_in_module(path, content, "")
}

/// Per-file AST. Definition ids and edge sources are qualified with `module_id`; edge
/// targets are left bare for global resolution in `rebuild`.
pub fn extract_ast_in_module(path: &Path, content: &str, module_id: &str) -> ExtractedGraph {
    extract_ast_in_module_with_wrappers(path, content, module_id, &HashMap::new())
}

/// Like `extract_ast_in_module`, but with a `voxTransport.<method>` → target map so wrapper
/// calls become declared boundary edges. `invoke`/`callTool` boundary crossings are detected
/// from their arg0 string literal regardless of the map.
pub fn extract_ast_in_module_with_wrappers(
    path: &Path,
    content: &str,
    module_id: &str,
    wrappers: &HashMap<String, String>,
) -> ExtractedGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    if path.extension().map_or(false, |ext| ext == "rs") {
        if let Ok(file) = syn::parse_file(content) {
            let mut visitor = RustVisitor {
                module_id: module_id.to_string(),
                scope: Vec::new(),
                impl_prefix: None,
                ids: IdAlloc::default(),
                nodes: Vec::new(),
                edges: Vec::new(),
                current_fn: None,
            };
            visitor.visit_file(&file);
            nodes = visitor.nodes;
            edges = visitor.edges;
        }
    } else {
        #[cfg(feature = "tree-sitter-grammars")]
        {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let language = match ext {
                    "ts" | "js" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
                    "tsx" | "jsx" => Some(tree_sitter_typescript::LANGUAGE_TSX),
                    "py" => Some(tree_sitter_python::LANGUAGE),
                    _ => None,
                };
                // tree-sitter-typescript 0.23.2 node-kind names (discovered via A0d
                // print-test against LANGUAGE_TSX; use these verbatim in B1/D1):
                //   JSX self-closing element : "jsx_self_closing_element"
                //     - component name        : child_by_field_name("name") -> "identifier"
                //       (Capitalized => component; lowercase => DOM tag)
                //     - opening element       : "jsx_opening_element"
                //   import statement          : "import_statement"
                //     - named import ident    : "import_clause" > "named_imports" >
                //                               "import_specifier"; the bound name is the
                //                               specifier's child_by_field_name("name") -> "identifier"
                //   call_expression           : "call_expression"
                //     - args field            : "arguments" (positional children, named)
                //     - string literal kind   : "string" (text via "string_fragment" child)
                //     - object literal kind    : "object"
                //     - object entry kind      : "pair" (key "property_identifier", value e.g. "string")
                if let Some(lang) = language {
                    let mut parser = tree_sitter::Parser::new();
                    if parser.set_language(&lang.into()).is_ok() {
                        if let Some(tree) = parser.parse(content, None) {
                            let mut cursor = tree.walk();
                            let mut stack = vec![tree.root_node()];
                            let mut ids = IdAlloc::default();
                            // tree-sitter node id -> assigned symbol id, so a call's edge
                            // source is its lexically enclosing fn (`ts_enclosing_fn`).
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
                                            let id = ids.alloc(qualify_in(module_id, &scope, name));
                                            let kind = if is_class {
                                                "struct"
                                            } else if in_class || node.kind() == "method_definition"
                                            {
                                                "method"
                                            } else {
                                                "fn"
                                            };
                                            nodes.push(ExtractedNode {
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
                                // JSX element usage => composition edge (only for
                                // Capitalized names; lowercase = DOM tags).
                                if is_jsx {
                                    if let Some(ref source_fn) = current_fn {
                                        if let Some(name_node) = node.child_by_field_name("name") {
                                            if let Ok(name) =
                                                name_node.utf8_text(content.as_bytes())
                                            {
                                                if name
                                                    .chars()
                                                    .next()
                                                    .is_some_and(|c| c.is_uppercase())
                                                {
                                                    edges.push(ExtractedEdge {
                                                        source: source_fn.clone(),
                                                        target: name.to_string(),
                                                        confidence: "resolved".into(),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                                if is_call {
                                    if let Some(ref source_fn) = current_fn {
                                        if let Some(function_node) =
                                            node.child_by_field_name("function")
                                        {
                                            if let Ok(callee) =
                                                function_node.utf8_text(content.as_bytes())
                                            {
                                                let bare =
                                                    callee.rsplit('.').next().unwrap_or(callee);
                                                let args = node.child_by_field_name("arguments");
                                                if BOUNDARY_CALLEES.contains(&bare) {
                                                    // invoke('cmd') / callTool('cmd'); special-case
                                                    // invoke('invoke_mcp_tool', { tool: '...' }).
                                                    if let Some(arg0) = args.and_then(|a| {
                                                        arg_string_literal(a, 0, content)
                                                    }) {
                                                        if arg0 == "invoke_mcp_tool" {
                                                            if let Some(tool) = args.and_then(|a| {
                                                                arg_object_string_field(
                                                                    a, 1, "tool", content,
                                                                )
                                                            }) {
                                                                edges.push(ExtractedEdge {
                                                                    source: source_fn.clone(),
                                                                    target: format!("tool:{tool}"),
                                                                    confidence: "declared".into(),
                                                                });
                                                            }
                                                        } else {
                                                            edges.push(ExtractedEdge {
                                                                source: source_fn.clone(),
                                                                target: format!("cmd:{arg0}"),
                                                                confidence: "declared".into(),
                                                            });
                                                        }
                                                    }
                                                    // Boundary call handled: do NOT also emit the
                                                    // bare-call edge (prevents the double-count).
                                                    push_children(&mut stack, node, &mut cursor);
                                                    continue;
                                                } else if callee.starts_with("voxTransport.") {
                                                    if let Some(target) = wrappers.get(bare) {
                                                        edges.push(ExtractedEdge {
                                                            source: source_fn.clone(),
                                                            target: target.clone(),
                                                            confidence: "declared".into(),
                                                        });
                                                    }
                                                    push_children(&mut stack, node, &mut cursor);
                                                    continue;
                                                }
                                                edges.push(ExtractedEdge {
                                                    source: source_fn.clone(),
                                                    target: callee.to_string(),
                                                    confidence: "resolved".into(),
                                                });
                                            }
                                        }
                                    }
                                }
                                push_children(&mut stack, node, &mut cursor);
                            }
                        }
                    }
                }
            }
        }
    }

    ExtractedGraph { nodes, edges }
}

/// Push `node`'s children so they pop in source order (keeps `#n` ids in source order).
#[cfg(feature = "tree-sitter-grammars")]
fn push_children<'t>(
    stack: &mut Vec<tree_sitter::Node<'t>>,
    node: tree_sitter::Node<'t>,
    cursor: &mut tree_sitter::TreeCursor<'t>,
) {
    let children: Vec<_> = node.children(cursor).collect();
    stack.extend(children.into_iter().rev());
}

/// A named function definition: fn / method declarations, plus `const f = () => ...` and
/// `const f = function () {...}` declarators (the common TS / React component shape).
#[cfg(feature = "tree-sitter-grammars")]
fn ts_fn_like(node: tree_sitter::Node) -> bool {
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

#[cfg(feature = "tree-sitter-grammars")]
fn ts_class_like(node: tree_sitter::Node) -> bool {
    matches!(
        node.kind(),
        "class_definition" | "class_declaration" | "abstract_class_declaration"
    )
}

/// Names of `node`'s named enclosing classes / fns, outermost first, and whether the
/// nearest of them is a class (=> `node` is a method).
#[cfg(feature = "tree-sitter-grammars")]
fn ts_scope(node: tree_sitter::Node, content: &str) -> (Vec<String>, bool) {
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
#[cfg(feature = "tree-sitter-grammars")]
fn ts_enclosing_fn(node: tree_sitter::Node, def_ids: &HashMap<usize, String>) -> Option<String> {
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
#[cfg(feature = "tree-sitter-grammars")]
fn string_literal_value(node: tree_sitter::Node, content: &str) -> Option<String> {
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
#[cfg(feature = "tree-sitter-grammars")]
fn arg_string_literal(args: tree_sitter::Node, idx: usize, content: &str) -> Option<String> {
    let mut cursor = args.walk();
    let arg = args.named_children(&mut cursor).nth(idx)?;
    string_literal_value(arg, content)
}

/// Value of `field`'s string entry inside the object literal positional argument at `idx`.
#[cfg(feature = "tree-sitter-grammars")]
fn arg_object_string_field(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_tokens_is_stable_and_colon_free() {
        let ty: syn::Type = syn::parse_str("From < std :: vec :: Vec < & 'a str > >").unwrap();
        assert_eq!(compact_tokens(&ty), "From<std.vec.Vec<&'a str>>");
        let dyn_ty: syn::Type = syn::parse_str("Box<dyn Fn(u8) -> u8>").unwrap();
        assert_eq!(compact_tokens(&dyn_ty), "Box<dyn Fn(u8)->u8>");
    }

    #[test]
    fn id_alloc_suffixes_repeats_in_order() {
        let mut ids = IdAlloc::default();
        assert_eq!(ids.alloc("a.rs::f".into()), "a.rs::f");
        assert_eq!(ids.alloc("a.rs::f".into()), "a.rs::f#2");
        assert_eq!(ids.alloc("a.rs::g".into()), "a.rs::g");
        assert_eq!(ids.alloc("a.rs::f".into()), "a.rs::f#3");
    }

    #[test]
    fn extract_ast_keeps_unqualified_ids() {
        let g = extract_ast(Path::new("m.rs"), "struct S; impl S { fn f() {} }");
        let ids: Vec<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, vec!["S", "S::f"]);
    }
}
