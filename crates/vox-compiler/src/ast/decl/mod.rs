//! Top-level items in a `.vox` file: functions, data definitions, routes, and platform-specific UI.
//!
//! `Decl` is a flat enum: there is no nested module syntax in the AST. Import declarations are
//! included here so the parser can emit a single `Module` per file. Attributes and decorators
//! that the lexer/parser attach often land on fields inside each variant; mutators on `Decl`
//! exist for codegen and CLI tooling that patch metadata after parse.

/// `@config` blocks and typed configuration declarations.
pub mod config;
/// Relational table, collection, and index declarations.
pub mod db;
/// Functions, components, server handlers, MCP, hooks, and tests.
pub mod fundecl;
/// Actors, agents, workflows, activities, and HTTP routes.
pub mod logic;
/// ADTs, traits, impls, and type aliases.
pub mod typedef;
/// Client routing, layouts, themes, and SSG page metadata.
pub mod ui;

use crate::ast::span::Span;

pub use config::*;
pub use db::*;
pub use fundecl::*;
pub use logic::*;
pub use typedef::*;
pub use ui::*;

/// HTTP method for route declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HttpMethod {
    /// HTTP `GET`.
    Get,
    /// HTTP `POST`.
    Post,
    /// HTTP `PUT`.
    Put,
    /// HTTP `DELETE`.
    Delete,
}

/// An import path segment: `react.use_state`
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportPath {
    /// Dot-separated path segments (e.g. `react`, `use_state`).
    pub segments: Vec<String>,
    /// Source span of this path.
    pub span: Span,
}

/// Import declaration: `import react.use_state, network.HTTP`
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportDecl {
    /// Imported paths listed in a single `import` declaration.
    pub paths: Vec<ImportPath>,
    /// Span covering the full `import` syntax.
    pub span: Span,
}

/// Python library import: `@py.import torch` or `@py.import torch as tc`
///
/// Causes the Rust codegen to emit a `VoxPyRuntime` lazy-static bridge for
/// the named Python module. The alias is used as the binding name in Vox code.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PyImportDecl {
    /// The Python module to import (e.g. `"torch"`, `"torch.nn"`).
    pub module: String,
    /// The binding name in Vox scope. Defaults to the module's last segment.
    pub alias: String,
    /// Source span of the `@py.import` directive.
    pub span: Span,
}

/// One item in a [`Module`]: any construct that can appear at column 0 (after indentation) in a file.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Decl {
    /// Top-level or nested function.
    Function(FnDecl),
    /// React-style `@component` UI function.
    Component(ComponentDecl),
    /// Algebraic type, struct, or type alias.
    TypeDef(TypeDefDecl),
    /// ES-module style import list.
    Import(ImportDecl),
    /// Native Python library import (`@py.import module [as alias]`).
    PyImport(PyImportDecl),
    /// Tokio-style actor with mailbox handlers.
    Actor(ActorDecl),
    /// Immutable constant (`const` / `@const`).
    Const(ConstDecl),
    /// Durable workflow state machine.
    Workflow(WorkflowDecl),
    /// Durable activity (side-effect step).
    Activity(ActivityDecl),
    /// HTTP handler bound to a method and path.
    HttpRoute(HttpRouteDecl),
    /// MCP tool exposed to clients.
    McpTool(McpToolDecl),
    /// MCP resource URI handler.
    McpResource(McpResourceDecl),
    /// Unit test entrypoint.
    Test(TestDecl),
    /// Server-only RPC / RSC-style function.
    ServerFn(ServerFnDecl),
    /// Codex table schema.
    Table(TableDecl),
    /// Document collection schema.
    Collection(CollectionDecl),
    /// B-tree style index on columns.
    Index(IndexDecl),
    /// Vector / embedding index.
    VectorIndex(VectorIndexDecl),
    /// Full-text search index.
    SearchIndex(SearchIndexDecl),
    /// v0.dev generated component stub.
    V0Component(V0ComponentDecl),
    /// Client-side route table.
    Routes(RoutesDecl),
    /// Trait (interface) definition.
    Trait(TraitDecl),
    /// Trait implementation block.
    Impl(ImplDecl),
    /// Read-only data access function.
    Query(QueryDecl),
    /// Transactional write function.
    Mutation(MutationDecl),
    /// Orchestrated server action.
    Action(ActionDecl),
    /// Packaged LLM / tool skill.
    Skill(SkillDecl),
    /// Agent definition (capabilities + handlers).
    AgentDef(AgentDefDecl),
    /// Native agent runtime declaration.
    Agent(AgentDecl),
    /// Inter-agent message shape.
    Message(MessageDecl),
    /// Cron / interval scheduled job.
    Scheduled(ScheduledDecl),
    /// Typed configuration block.
    Config(ConfigDecl),
    /// React context provider shape.
    Context(ContextDecl),
    /// React hook binding.
    Hook(HookDecl),
    /// Context provider component.
    Provider(ProviderDecl),
    /// Test fixture setup.
    Fixture(FixtureDecl),
    /// Nested route layout shell.
    Layout(LayoutDecl),
    /// Route loading / suspense UI.
    Loading(LoadingDecl),
    /// 404 handler component.
    NotFound(NotFoundDecl),
    /// Error boundary component.
    ErrorBoundary(ErrorBoundaryDecl),
    /// CSS `@keyframes` block.
    Keyframes(KeyframeDecl),
    /// Design-token theme (light/dark).
    Theme(ThemeDecl),
    /// Test double / stub implementation.
    Mock(MockDecl),
    /// Container / deployment environment spec.
    Environment(EnvironmentDecl),
    /// Static page for SSG.
    Page(PageDecl),
    /// React island stub (props only); implementation lives in `islands/` TSX.
    Island(IslandDecl),
}

impl Decl {
    /// Attaches a human-readable blurb for OpenAPI / schema-style emitters.
    ///
    /// Only [`Decl::Table`], [`Decl::Collection`], [`Decl::McpTool`], and [`Decl::McpResource`]
    /// store this; other variants ignore the call.
    pub fn set_description(&mut self, desc: String) {
        match self {
            Decl::Table(t) => t.description = Some(desc),
            Decl::Collection(c) => c.description = Some(desc),
            Decl::McpTool(m) => m.description = desc,
            Decl::McpResource(m) => m.description = desc,
            _ => {}
        }
    }
    /// Stores an optional JSON-schema–style layout hint on [`Decl::TypeDef`] and [`Decl::Table`].
    pub fn set_json_layout(&mut self, layout: String) {
        match self {
            Decl::TypeDef(t) => t.json_layout = Some(layout),
            Decl::Table(t) => t.json_layout = Some(layout),
            _ => {}
        }
    }

    /// Merges auth provider, role allowlist extension, and CORS hint onto HTTP-facing declarations.
    ///
    /// Applies to functions, components, server handlers, queries, mutations, actions, HTTP routes,
    /// tables, and several UI route variants. Empty `roles` leaves existing roles unchanged; each
    /// field is applied only when `Some` / non-empty as documented in the match arms.
    pub fn set_security(&mut self, auth: Option<String>, roles: Vec<String>, cors: Option<String>) {
        if auth.is_none() && roles.is_empty() && cors.is_none() {
            return;
        }
        match self {
            Decl::Function(f) => {
                if auth.is_some() {
                    f.auth_provider = auth;
                }
                if !roles.is_empty() {
                    f.roles.extend(roles);
                }
                if cors.is_some() {
                    f.cors = cors;
                }
            }
            Decl::Component(c) => {
                if auth.is_some() {
                    c.func.auth_provider = auth;
                }
                if !roles.is_empty() {
                    c.func.roles.extend(roles);
                }
                if cors.is_some() {
                    c.func.cors = cors;
                }
            }
            Decl::ServerFn(s) => {
                if auth.is_some() {
                    s.func.auth_provider = auth;
                }
                if !roles.is_empty() {
                    s.func.roles.extend(roles);
                }
                if cors.is_some() {
                    s.func.cors = cors;
                }
            }
            Decl::Query(q) => {
                if auth.is_some() {
                    q.func.auth_provider = auth;
                }
                if !roles.is_empty() {
                    q.func.roles.extend(roles);
                }
                if cors.is_some() {
                    q.func.cors = cors;
                }
            }
            Decl::Mutation(m) => {
                if auth.is_some() {
                    m.func.auth_provider = auth;
                }
                if !roles.is_empty() {
                    m.func.roles.extend(roles);
                }
                if cors.is_some() {
                    m.func.cors = cors;
                }
            }
            Decl::Action(a) => {
                if auth.is_some() {
                    a.func.auth_provider = auth;
                }
                if !roles.is_empty() {
                    a.func.roles.extend(roles);
                }
                if cors.is_some() {
                    a.func.cors = cors;
                }
            }
            Decl::HttpRoute(h) => {
                if auth.is_some() {
                    h.auth_provider = auth;
                }
                if !roles.is_empty() {
                    h.roles.extend(roles);
                }
                if cors.is_some() {
                    h.cors = cors;
                }
            }
            Decl::Table(t) => {
                if auth.is_some() {
                    t.auth_provider = auth;
                }
                if !roles.is_empty() {
                    t.roles.extend(roles);
                }
                if cors.is_some() {
                    t.cors = cors;
                }
            }
            Decl::Layout(l) => {
                if auth.is_some() {
                    l.func.auth_provider = auth;
                }
                if !roles.is_empty() {
                    l.func.roles.extend(roles);
                }
                if cors.is_some() {
                    l.func.cors = cors;
                }
            }
            Decl::Loading(l) => {
                if auth.is_some() {
                    l.func.auth_provider = auth;
                }
                if !roles.is_empty() {
                    l.func.roles.extend(roles);
                }
                if cors.is_some() {
                    l.func.cors = cors;
                }
            }
            Decl::NotFound(n) => {
                if auth.is_some() {
                    n.func.auth_provider = auth;
                }
                if !roles.is_empty() {
                    n.func.roles.extend(roles);
                }
                if cors.is_some() {
                    n.func.cors = cors;
                }
            }
            Decl::ErrorBoundary(e) => {
                if auth.is_some() {
                    e.func.auth_provider = auth;
                }
                if !roles.is_empty() {
                    e.func.roles.extend(roles);
                }
                if cors.is_some() {
                    e.func.cors = cors;
                }
            }
            Decl::Page(p) => {
                if auth.is_some() {
                    p.func.auth_provider = auth;
                }
                if !roles.is_empty() {
                    p.func.roles.extend(roles);
                }
                if cors.is_some() {
                    p.func.cors = cors;
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Applies boolean flags from `@` decorators (deprecated, pure, traced, LLM, metrics, health, …).
    ///
    /// The flat parameter list matches legacy parser entry points; each flag updates only the
    /// declaration kinds where that concept exists (e.g. `is_layout` only touches [`Decl::Function`]).
    pub fn set_decorators(
        &mut self,
        is_deprecated: bool,
        is_pure: bool,
        is_traced: bool,
        is_llm: bool,
        llm_model: Option<String>,
        is_layout: bool,
        is_metric: bool,
        metric_name: Option<String>,
        is_health: bool,
    ) {
        match self {
            Decl::Function(f) => {
                if is_deprecated {
                    f.is_deprecated = true;
                }
                if is_pure {
                    f.is_pure = true;
                }
                if is_traced {
                    f.is_traced = true;
                }
                if is_llm {
                    f.is_llm = true;
                    f.llm_model = llm_model;
                }
                if is_layout {
                    f.is_layout = true;
                }
                if is_health {
                    f.is_health = true;
                }
            }
            Decl::Component(c) => {
                if is_deprecated {
                    c.func.is_deprecated = true;
                }
                if is_traced {
                    c.func.is_traced = true;
                }
                if is_metric {
                    c.func.is_metric = true;
                    c.func.metric_name = metric_name.clone();
                }
                if is_health {
                    c.func.is_health = true;
                }
            }
            Decl::Test(t) => {
                if is_deprecated {
                    t.func.is_deprecated = true;
                }
                if is_traced {
                    t.func.is_traced = true;
                }
                if is_metric {
                    t.func.is_metric = true;
                    t.func.metric_name = metric_name.clone();
                }
                if is_health {
                    t.func.is_health = true;
                }
            }
            Decl::ServerFn(s) => {
                if is_deprecated {
                    s.func.is_deprecated = true;
                }
                if is_traced {
                    s.func.is_traced = true;
                }
                if is_metric {
                    s.func.is_metric = true;
                    s.func.metric_name = metric_name.clone();
                }
                if is_health {
                    s.func.is_health = true;
                }
            }
            Decl::Query(q) => {
                if is_deprecated {
                    q.func.is_deprecated = true;
                }
                if is_traced {
                    q.func.is_traced = true;
                }
                if is_metric {
                    q.func.is_metric = true;
                    q.func.metric_name = metric_name.clone();
                }
                if is_health {
                    q.func.is_health = true;
                }
            }
            Decl::Mutation(m) => {
                if is_deprecated {
                    m.func.is_deprecated = true;
                }
                if is_traced {
                    m.func.is_traced = true;
                }
                if is_metric {
                    m.func.is_metric = true;
                    m.func.metric_name = metric_name.clone();
                }
                if is_health {
                    m.func.is_health = true;
                }
            }
            Decl::Action(a) => {
                if is_deprecated {
                    a.func.is_deprecated = true;
                }
                if is_traced {
                    a.func.is_traced = true;
                }
                if is_metric {
                    a.func.is_metric = true;
                    a.func.metric_name = metric_name.clone();
                }
                if is_health {
                    a.func.is_health = true;
                }
            }
            Decl::Skill(s) => {
                if is_deprecated {
                    s.func.is_deprecated = true;
                }
                if is_traced {
                    s.func.is_traced = true;
                }
                if is_metric {
                    s.func.is_metric = true;
                    s.func.metric_name = metric_name.clone();
                }
                if is_health {
                    s.func.is_health = true;
                }
            }
            Decl::AgentDef(a) => {
                if is_deprecated {
                    a.func.is_deprecated = true;
                }
                if is_traced {
                    a.func.is_traced = true;
                }
                if is_metric {
                    a.func.is_metric = true;
                    a.func.metric_name = metric_name.clone();
                }
                if is_health {
                    a.func.is_health = true;
                }
            }
            Decl::Scheduled(s) => {
                if is_deprecated {
                    s.func.is_deprecated = true;
                }
                if is_traced {
                    s.func.is_traced = true;
                }
                if is_metric {
                    s.func.is_metric = true;
                    s.func.metric_name = metric_name.clone();
                }
                if is_health {
                    s.func.is_health = true;
                }
            }
            Decl::McpTool(m) => {
                if is_deprecated {
                    m.func.is_deprecated = true;
                }
                if is_traced {
                    m.func.is_traced = true;
                }
                if is_metric {
                    m.func.is_metric = true;
                    m.func.metric_name = metric_name.clone();
                }
                if is_health {
                    m.func.is_health = true;
                }
            }
            Decl::McpResource(m) => {
                if is_deprecated {
                    m.func.is_deprecated = true;
                }
                if is_traced {
                    m.func.is_traced = true;
                }
                if is_metric {
                    m.func.is_metric = true;
                    m.func.metric_name = metric_name.clone();
                }
                if is_health {
                    m.func.is_health = true;
                }
            }
            Decl::Page(p) => {
                if is_deprecated {
                    p.func.is_deprecated = true;
                }
                if is_traced {
                    p.func.is_traced = true;
                }
                if is_metric {
                    p.func.is_metric = true;
                    p.func.metric_name = metric_name.clone();
                }
                if is_health {
                    p.func.is_health = true;
                }
            }
            Decl::Workflow(w) => {
                if is_deprecated {
                    w.is_deprecated = true;
                }
                if is_traced {
                    w.is_traced = true;
                }
            }
            Decl::Activity(a) => {
                if is_deprecated {
                    a.is_deprecated = true;
                }
                if is_traced {
                    a.is_traced = true;
                }
            }
            Decl::HttpRoute(h) => {
                if is_deprecated {
                    h.is_deprecated = true;
                }
                if is_traced {
                    h.is_traced = true;
                }
            }
            Decl::Actor(a) => {
                if is_deprecated {
                    a.is_deprecated = true;
                }
            }
            Decl::Table(t) => {
                if is_deprecated {
                    t.is_deprecated = true;
                }
            }
            Decl::Trait(t) => {
                if is_deprecated {
                    t.is_deprecated = true;
                }
            }
            Decl::TypeDef(t) => {
                if is_deprecated {
                    t.is_deprecated = true;
                }
            }
            Decl::Const(c) => {
                if is_deprecated {
                    c.is_deprecated = true;
                }
            }
            Decl::Config(c) => {
                if is_deprecated {
                    c.is_deprecated = true;
                }
            }
            Decl::Environment(e) => {
                if is_deprecated {
                    e.is_deprecated = true;
                }
            }
            Decl::Agent(a) => {
                if is_deprecated {
                    a.is_deprecated = true;
                }
            }
            Decl::Message(m) => {
                if is_deprecated {
                    m.is_deprecated = true;
                }
            }
            Decl::Layout(l) => {
                if is_deprecated {
                    l.func.is_deprecated = true;
                }
                if is_traced {
                    l.func.is_traced = true;
                }
            }
            Decl::Loading(l) => {
                if is_deprecated {
                    l.func.is_deprecated = true;
                }
                if is_traced {
                    l.func.is_traced = true;
                }
            }
            Decl::NotFound(n) => {
                if is_deprecated {
                    n.func.is_deprecated = true;
                }
                if is_traced {
                    n.func.is_traced = true;
                }
            }
            Decl::ErrorBoundary(e) => {
                if is_deprecated {
                    e.func.is_deprecated = true;
                }
                if is_traced {
                    e.func.is_traced = true;
                }
            }
            _ => {}
        }
    }

    /// Primary source span for this declaration (used for diagnostics).
    pub fn span(&self) -> Span {
        match self {
            Decl::Function(f) => f.span,
            Decl::Component(c) => c.func.span,
            Decl::TypeDef(t) => t.span,
            Decl::Import(i) => i.span,
            Decl::PyImport(p) => p.span,
            Decl::Actor(a) => a.span,
            Decl::Workflow(w) => w.span,
            Decl::Activity(a) => a.span,
            Decl::HttpRoute(h) => h.span,
            Decl::McpTool(m) => m.func.span,
            Decl::Test(t) => t.func.span,
            Decl::ServerFn(s) => s.func.span,
            Decl::Table(t) => t.span,
            Decl::Collection(c) => c.span,
            Decl::Index(i) => i.span,
            Decl::VectorIndex(v) => v.span,
            Decl::SearchIndex(s) => s.span,
            Decl::V0Component(v) => v.span,
            Decl::Routes(r) => r.span,
            Decl::Trait(t) => t.span,
            Decl::Impl(i) => i.span,
            Decl::Query(q) => q.func.span,
            Decl::Mutation(m) => m.func.span,
            Decl::Action(a) => a.func.span,
            Decl::Skill(s) => s.func.span,
            Decl::AgentDef(ad) => ad.func.span,
            Decl::Agent(a) => a.span,
            Decl::Message(m) => m.span,
            Decl::Scheduled(s) => s.func.span,
            Decl::Const(c) => c.span,
            Decl::Config(c) => c.span,
            Decl::Context(c) => c.span,
            Decl::Hook(h) => h.func.span,
            Decl::Provider(p) => p.func.span,
            Decl::Fixture(f) => f.func.span,
            Decl::Layout(l) => l.func.span,
            Decl::Loading(l) => l.func.span,
            Decl::NotFound(n) => n.func.span,
            Decl::ErrorBoundary(e) => e.func.span,
            Decl::Keyframes(k) => k.span,
            Decl::Theme(t) => t.span,
            Decl::Mock(m) => m.func.span,
            Decl::McpResource(m) => m.func.span,
            Decl::Environment(e) => e.span,
            Decl::Page(p) => p.span,
            Decl::Island(i) => i.span,
        }
    }
}

/// Parsed contents of one compilation unit (usually one `.vox` file).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Module {
    /// Declarations in the order they appeared in source.
    pub declarations: Vec<Decl>,
    /// Span covering the whole file (or recovered module) for file-level diagnostics.
    pub span: Span,
}

impl Module {
    /// Returns `true` if the module defines a top-level `fn main` (CLI script entrypoint).
    #[must_use]
    pub fn has_entrypoint(&self) -> bool {
        self.declarations
            .iter()
            .any(|d| matches!(d, Decl::Function(f) if f.name == "main"))
    }
}
