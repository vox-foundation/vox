//! M3: Mermaid `id[Label]` forms. The brackets declare the node, so pass A declares it here
//! (an edge to it is never `undeclared-node`) and reports the form with the exact rewrite
//! where one exists: `[ ]` rect, `( )` rounded, `{ }` diamond, `[( )]` cylinder.

use super::{Entity, Resolver};
use crate::ast::{Bracket, Ref};
use crate::codes;
use crate::diag::Fix;
use crate::fixgen::{declarable, insert_decl, replace};
use crate::ir::Shape;
use crate::print::shape_word;

/// `node id: "Label"`, with the shape attribute unless it is the default.
fn node_text(id: &str, b: &Bracket) -> String {
    let shape = b.shape.unwrap_or_default();
    let mut text = format!("node {id}: {}", b.quoted);
    if shape != Shape::Rect {
        text.push_str(&format!(" {{ shape: {} }}", shape_word(shape)));
    }
    text
}

impl<'a> Resolver<'a> {
    /// An exact rewrite exists: the form is closed and its shape maps one to one.
    fn exact(b: &Bracket) -> bool {
        b.closed && b.shape.is_some()
    }

    fn report_bracket(&mut self, b: &Bracket, fixes: Vec<Fix>) {
        let msg = if !b.closed {
            "unclosed Mermaid bracket; close it on the same line"
        } else if b.shape.is_none() {
            "this node shape has no VoxDiagram equivalent (rect, rounded, diamond, cylinder only)"
        } else {
            "Mermaid node syntax; write `node id: \"Label\"`"
        };
        self.report_fixes(codes::MERMAID_SYNTAX, b.form, msg.into(), fixes);
    }

    /// A statement-form `A[Label]`, already declared as a node: rewrite it to `node A: ...`.
    pub(super) fn bracket_stmt(&mut self, id: &str, b: &Bracket) {
        let fixes = if Self::exact(b) {
            vec![replace(b.form, node_text(id, b))]
        } else {
            Vec::new()
        };
        self.report_bracket(b, fixes);
    }

    /// An `A[Label]` inside an edge. Same label and shape as the declared node: the bare id
    /// replaces it. Otherwise it declares the node (a clash is `duplicate-id`) and the fix
    /// replaces it with the bare id AND declares the node at the top of the scope (Rule R).
    pub(super) fn bracket_endpoint(&mut self, scope: usize, r: &'a Ref) {
        let Some(b) = &r.bracket else { return };
        let [id] = r.segments.as_slice() else {
            return self.report_bracket(b, Vec::new());
        };
        let shape = b.shape.unwrap_or_default();
        let same = matches!(
            self.scopes[scope].names.get(id.name.as_str()),
            Some((Entity::Node(i), _))
                if self.ir.nodes[*i].label == b.label && self.ir.nodes[*i].shape == shape
        );
        if same && Self::exact(b) {
            return self.report_bracket(b, vec![replace(b.form, id.name.as_str())]);
        }
        let taken = self.scopes[scope].names.contains_key(id.name.as_str());
        self.add_node(scope, id, &b.label, shape, r.span);
        let mut fixes = Vec::new();
        // A clash is `duplicate-id`; declaring the node again would only repeat it.
        if Self::exact(b)
            && !taken
            && declarable(&id.name)
            && let Some((open, close)) = self.scopes[scope].braces
            && let Some(decl) = insert_decl(self.src, open, close, &node_text(&id.name, b))
        {
            fixes = vec![replace(b.form, id.name.as_str()), decl];
        }
        self.report_bracket(b, fixes);
    }
}
