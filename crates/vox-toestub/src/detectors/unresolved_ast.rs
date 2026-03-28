//! Syn-backed function definitions, call sites, and `use` imports for [`super::UnresolvedRefDetector`].

use std::collections::HashSet;

use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprPath, File, ForeignItem, Item, TraitItem, UseTree};

#[derive(Debug, Default)]
pub struct AstUnresolvedHints {
    pub defined_fns: HashSet<String>,
    pub call_sites: HashSet<(usize, String)>,
    /// Bound names introduced by `use` (last segment / `as` alias).
    pub use_imports: HashSet<String>,
}

struct Collector {
    defined_fns: HashSet<String>,
    call_sites: HashSet<(usize, String)>,
    use_imports: HashSet<String>,
}

impl Collector {
    fn finish(self) -> AstUnresolvedHints {
        AstUnresolvedHints {
            defined_fns: self.defined_fns,
            call_sites: self.call_sites,
            use_imports: self.use_imports,
        }
    }
}

fn record_use_tree(tree: &UseTree, out: &mut HashSet<String>) {
    match tree {
        UseTree::Path(p) => record_use_tree(&p.tree, out),
        UseTree::Name(n) => {
            out.insert(n.ident.to_string());
        }
        UseTree::Rename(r) => {
            out.insert(r.rename.to_string());
        }
        UseTree::Glob(_) => {
            // Wildcard — handled separately by `file_has_high_fanout_glob_use` heuristics.
        }
        UseTree::Group(g) => {
            for t in &g.items {
                record_use_tree(t, out);
            }
        }
    }
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_item(&mut self, i: &'ast Item) {
        if let Item::Use(u) = i {
            record_use_tree(&u.tree, &mut self.use_imports);
        }
        visit::visit_item(self, i);
    }

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        self.defined_fns.insert(i.sig.ident.to_string());
        visit::visit_item_fn(self, i);
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        self.defined_fns.insert(i.sig.ident.to_string());
        visit::visit_impl_item_fn(self, i);
    }

    fn visit_trait_item(&mut self, i: &'ast TraitItem) {
        if let TraitItem::Fn(f) = i {
            self.defined_fns.insert(f.sig.ident.to_string());
        }
        visit::visit_trait_item(self, i);
    }

    fn visit_foreign_item(&mut self, i: &'ast ForeignItem) {
        if let ForeignItem::Fn(f) = i {
            self.defined_fns.insert(f.sig.ident.to_string());
        }
        visit::visit_foreign_item(self, i);
    }

    fn visit_expr_call(&mut self, c: &'ast ExprCall) {
        if let Expr::Path(ExprPath { path, .. }) = c.func.as_ref()
            && path.segments.len() == 1
            && path.segments[0].arguments.is_empty()
        {
            let seg = &path.segments[0];
            let line = seg.ident.span().start().line;
            self.call_sites.insert((line, seg.ident.to_string()));
        }
        visit::visit_expr_call(self, c);
    }
}

pub fn analyze_rust_ast(file: &File) -> AstUnresolvedHints {
    let mut c = Collector {
        defined_fns: HashSet::new(),
        call_sites: HashSet::new(),
        use_imports: HashSet::new(),
    };
    c.visit_file(file);
    c.finish()
}
