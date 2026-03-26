//! Internal **WebIR** schema — **ADR 012** (normative types, lowering, validation, TSX preview emit).
//!
//! - [`lower::lower_hir_to_web_ir`] — HIR → `WebIrModule` (views, routes, behaviors).
//! - [`validate::validate_web_ir`] — structural checks before target emission.
//! - [`emit_tsx::emit_component_view_tsx`] — JSX string from a lowered view root (parity / tests).

pub mod emit_tsx;
pub mod lower;
pub mod validate;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Module root
// ---------------------------------------------------------------------------

/// Increment when the schema gains breaking layout changes. ADR 012 CP-002.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebIrVersion {
    /// Initial frozen placeholder (Phase 0).
    V0_1,
}

impl Default for WebIrVersion {
    fn default() -> Self {
        Self::V0_1
    }
}

/// Opaque id into [`SourceSpanTable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSpanId(pub u32);

/// Phase 0 span payload (byte offsets into a resolved source file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file_id: u32,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceSpanTable {
    pub spans: Vec<SourceSpan>,
}

impl SourceSpanTable {
    pub fn push_span(&mut self, span: SourceSpan) -> SourceSpanId {
        let id = SourceSpanId(self.spans.len() as u32);
        self.spans.push(span);
        id
    }

    pub fn get(&self, id: SourceSpanId) -> Option<&SourceSpan> {
        self.spans.get(id.0 as usize)
    }
}

/// Classify optionality for validator + emit boundary (ADR 012 nullability policy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldOptionality {
    Required,
    Optional,
    Defaulted,
}

/// Stable node id within the dom node arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DomNodeId(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebIrModule {
    pub dom_nodes: Vec<DomNode>,
    /// Reactive component name → root [`DomNodeId`] into [`Self::dom_nodes`] (Phase 1 lowering).
    #[serde(default)]
    pub view_roots: Vec<(String, DomNodeId)>,
    pub behavior_nodes: Vec<BehaviorNode>,
    pub style_nodes: Vec<StyleNode>,
    pub route_nodes: Vec<RouteNode>,
    pub interop_nodes: Vec<InteropNode>,
    pub diagnostic_nodes: Vec<WebIrDiagnostic>,
    pub spans: SourceSpanTable,
    pub version: WebIrVersion,
}

impl Default for WebIrModule {
    fn default() -> Self {
        Self {
            dom_nodes: Vec::new(),
            view_roots: Vec::new(),
            behavior_nodes: Vec::new(),
            style_nodes: Vec::new(),
            route_nodes: Vec::new(),
            interop_nodes: Vec::new(),
            diagnostic_nodes: Vec::new(),
            spans: SourceSpanTable::default(),
            version: WebIrVersion::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// DOM
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomNode {
    Element {
        id: DomNodeId,
        tag: String,
        /// Attribute name → IR-level value text (lowering fills escaping policy).
        attrs: Vec<(String, String)>,
        children: Vec<DomNodeId>,
        span: Option<SourceSpanId>,
    },
    Text {
        content: String,
        span: Option<SourceSpanId>,
    },
    Fragment {
        children: Vec<DomNodeId>,
        span: Option<SourceSpanId>,
    },
    Slot {
        name: Option<String>,
        span: Option<SourceSpanId>,
    },
    Conditional {
        /// Predicate expression placeholder (HIR/lowering supplies concrete form).
        predicate: String,
        then_children: Vec<DomNodeId>,
        else_children: Vec<DomNodeId>,
        span: Option<SourceSpanId>,
    },
    Loop {
        /// Iterator binding placeholder.
        iterator: String,
        body: Vec<DomNodeId>,
        span: Option<SourceSpanId>,
    },
    IslandMount {
        island_name: String,
        /// Maps to `data-prop-*` compatibility surface (Phase 1..2).
        props: Vec<(String, String)>,
        /// Nested JSX children under an `@island` tag are ignored by hydration (same as `hir_emit`).
        ignored_child_count: u32,
        span: Option<SourceSpanId>,
    },
    /// Dynamic TypeScript/JSX fragment already in render position (lowering fallback / expression leaf).
    Expr {
        ts: String,
        span: Option<SourceSpanId>,
    },
}

// ---------------------------------------------------------------------------
// Behavior
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BehaviorNode {
    StateDecl {
        name: String,
        initial: Option<String>,
        optionality: FieldOptionality,
        span: Option<SourceSpanId>,
    },
    DerivedDecl {
        name: String,
        expr: String,
        span: Option<SourceSpanId>,
    },
    EffectDecl {
        deps: Vec<String>,
        body: String,
        span: Option<SourceSpanId>,
    },
    EventHandler {
        target_dom: Option<DomNodeId>,
        event: String,
        handler: String,
        span: Option<SourceSpanId>,
    },
    Action {
        name: String,
        payload_expr: Option<String>,
        span: Option<SourceSpanId>,
    },
}

// ---------------------------------------------------------------------------
// Style
// ---------------------------------------------------------------------------

/// Typed-ish CSS value bucket (Phase 2 deepens this per blueprint CP-031).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StyleDeclarationValue {
    Raw(String),
    TokenRef(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StyleNode {
    Rule {
        selector: StyleSelector,
        declarations: Vec<(String, StyleDeclarationValue)>,
        span: Option<SourceSpanId>,
    },
    Selector(StyleSelector),
    Declaration {
        property: String,
        value: StyleDeclarationValue,
        important: bool,
        span: Option<SourceSpanId>,
    },
    TokenRef {
        name: String,
        span: Option<SourceSpanId>,
    },
    AtRule {
        name: String,
        prelude: String,
        block: Option<String>,
        span: Option<SourceSpanId>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StyleSelector {
    Class(String),
    Id(String),
    Compound(Vec<StyleSelector>),
    Pseudo { base: Box<StyleSelector>, pseudo: String },
}

// ---------------------------------------------------------------------------
// Route / data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouteNode {
    RouteTree {
        routes: Vec<RouteContract>,
        span: Option<SourceSpanId>,
    },
    LoaderContract {
        route_id: String,
        contract: String,
        span: Option<SourceSpanId>,
    },
    ServerFnContract(ServerFnContract),
    MutationContract(MutationContract),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteContract {
    pub id: String,
    pub pattern: String,
    pub meta: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFnContract {
    pub name: String,
    pub export_path: String,
    pub signature: String,
    pub span: Option<SourceSpanId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationContract {
    pub name: String,
    pub payload_type: String,
    pub span: Option<SourceSpanId>,
}

// ---------------------------------------------------------------------------
// Interop
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteropNode {
    ReactComponentRef {
        component: String,
        import_source: String,
        props: Vec<(String, String)>,
        span: Option<SourceSpanId>,
    },
    ExternalModuleRef {
        specifier: String,
        named: Option<String>,
        span: Option<SourceSpanId>,
    },
    EscapeHatchExpr {
        /// Raw target-language fragment; validator enforces policy (ADR 012 Phase 3).
        expr: String,
        reason: String,
        span: Option<SourceSpanId>,
    },
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebIrDiagnostic {
    pub code: String,
    pub message: String,
    pub span: Option<SourceSpanId>,
}

#[cfg(test)]
mod smoke_tests {
    use super::*;
    use crate::web_ir::validate::validate_web_ir;

    #[test]
    fn web_ir_module_default_validates() {
        let module = WebIrModule::default();
        assert!(validate_web_ir(&module).is_empty());
    }
}
