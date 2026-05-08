use crate::ast::decl::*;
use crate::ast::expr;
use crate::ast::types::TypeExpr;
use crate::hir::*;

use super::LowerCtx;

impl LowerCtx {
    pub(crate) fn lower_fn(&mut self, f: &FnDecl, is_component: bool) -> HirFn {
        let id = self.def_map.define(f.name.clone());
        self.def_map.push_scope();
        let params = f.params.iter().map(|p| self.lower_param(p)).collect();
        let mut body = f.body.iter().map(|s| self.lower_stmt(s)).collect();
        body = self.inject_contracts(f, body);
        self.def_map.pop_scope();

        let capabilities = f
            .effects
            .iter()
            .map(|e| match e {
                crate::ast::decl::EffectAnnotation::Net => crate::hir::HirCapability::Net,
                crate::ast::decl::EffectAnnotation::Db => crate::hir::HirCapability::Db,
                crate::ast::decl::EffectAnnotation::Fs => crate::hir::HirCapability::Fs,
                crate::ast::decl::EffectAnnotation::Env => crate::hir::HirCapability::Env,
                crate::ast::decl::EffectAnnotation::Clock => crate::hir::HirCapability::Clock,
                crate::ast::decl::EffectAnnotation::Random => crate::hir::HirCapability::Random,
                crate::ast::decl::EffectAnnotation::Spawn => crate::hir::HirCapability::Spawn,
                crate::ast::decl::EffectAnnotation::Mcp(t) => {
                    crate::hir::HirCapability::Mcp(t.clone())
                }
                crate::ast::decl::EffectAnnotation::Nothing => crate::hir::HirCapability::Nothing,
            })
            .collect();

        HirFn {
            id,
            name: f.name.clone(),
            generics: f.generics.clone(),
            params,
            return_type: f.return_type.as_ref().map(|t| self.lower_type(t)),
            body,
            is_component,
            is_async: false,
            is_pub: f.is_pub,
            is_mobile_native: f.is_mobile_native,
            is_pure: f.is_pure,
            is_reactive: f.is_reactive,
            is_llm: f.is_llm,
            llm_model: f.llm_model.clone(),
            is_deprecated: f.is_deprecated,
            schedule_interval: None,
            durability: None,
            actor_state_fields: vec![],
            capabilities,
            postconditions: f
                .postconditions
                .iter()
                .map(|p| HirPostCondition {
                    condition: self.lower_expr(&p.condition),
                    fallback: p.fallback.clone(),
                })
                .collect(),
            ts_extern_module: f.ts_extern_module.clone(),
            span: f.span,
        }
    }

    pub(crate) fn lower_param(&mut self, p: &expr::Param) -> HirParam {
        let id = self.def_map.define(p.name.clone());
        HirParam {
            id,
            name: p.name.clone(),
            type_ann: p.type_ann.as_ref().map(|t| self.lower_type(t)),
            default: p.default.as_ref().map(|e| self.lower_expr(e)),
            span: p.span,
        }
    }

    pub(crate) fn lower_type(&self, t: &TypeExpr) -> HirType {
        match t {
            TypeExpr::Named { name, .. } => {
                if name == "Unit" {
                    HirType::Unit
                } else {
                    HirType::Named(name.clone())
                }
            }
            TypeExpr::Generic { name, args, .. } => HirType::Generic(
                name.clone(),
                args.iter().map(|a| self.lower_type(a)).collect(),
            ),
            TypeExpr::Function {
                params,
                return_type,
                ..
            } => HirType::Function(
                params.iter().map(|p| self.lower_type(p)).collect(),
                Box::new(self.lower_type(return_type)),
            ),
            TypeExpr::Tuple { elements, .. } => {
                HirType::Tuple(elements.iter().map(|e| self.lower_type(e)).collect())
            }
            TypeExpr::Unit { .. } => HirType::Unit,
            TypeExpr::Infer { .. } => HirType::Named("_".to_string()),
            TypeExpr::Decimal { .. } => HirType::Decimal,
        }
    }
    pub(crate) fn lower_typedef(&mut self, t: &TypeDefDecl) -> HirTypeDef {
        let id = self.def_map.define(t.name.clone());
        HirTypeDef {
            id,
            name: t.name.clone(),
            variants: t
                .variants
                .iter()
                .map(|v| HirVariant {
                    name: v.name.clone(),
                    fields: v
                        .fields
                        .iter()
                        .map(|f| (f.name.clone(), self.lower_type(&f.type_ann)))
                        .collect(),
                    span: v.span,
                })
                .collect(),
            fields: t
                .fields
                .iter()
                .map(|f| (f.name.clone(), self.lower_type(&f.type_ann)))
                .collect(),
            is_pub: t.is_pub,
            span: t.span,
        }
    }

    pub(crate) fn lower_table(&mut self, t: &TableDecl) -> HirTable {
        let id = self.def_map.define(t.name.clone());
        HirTable {
            id,
            name: t.name.clone(),
            fields: t
                .fields
                .iter()
                .map(|f| HirTableField {
                    name: f.name.clone(),
                    type_ann: self.lower_type(&f.type_ann),
                    span: f.span,
                })
                .collect(),
            is_pub: t.is_pub,
            is_deprecated: t.is_deprecated,
            span: t.span,
        }
    }

    pub(crate) fn lower_collection(&mut self, c: &CollectionDecl) -> HirCollection {
        let id = self.def_map.define(c.name.clone());
        HirCollection {
            id,
            name: c.name.clone(),
            fields: c
                .fields
                .iter()
                .map(|f| HirTableField {
                    name: f.name.clone(),
                    type_ann: self.lower_type(&f.type_ann),
                    span: f.span,
                })
                .collect(),
            is_pub: c.is_pub,
            has_spread: c.has_spread,
            span: c.span,
        }
    }

    pub(crate) fn lower_reactive_component(
        &mut self,
        r: &ReactiveComponentDecl,
    ) -> HirReactiveComponent {
        let id = self.def_map.define(r.name.clone());
        self.def_map.push_scope();
        let params = r.params.iter().map(|p| self.lower_param(p)).collect();
        let members: Vec<HirReactiveMember> = r
            .members
            .iter()
            .map(|m| match m {
                ReactiveMemberDecl::State(s) => HirReactiveMember::State(HirState {
                    id: self.def_map.define(s.name.clone()),
                    name: s.name.clone(),
                    ty: s.ty.as_ref().map(|t| self.lower_type(t)),
                    init: self.lower_expr(&s.init),
                    span: s.span,
                }),
                ReactiveMemberDecl::Derived(d) => HirReactiveMember::Derived(HirDerived {
                    id: self.def_map.define(d.name.clone()),
                    name: d.name.clone(),
                    ty: d.ty.as_ref().map(|t| self.lower_type(t)),
                    expr: self.lower_expr(&d.expr),
                    span: d.span,
                }),
                ReactiveMemberDecl::Effect(e) => HirReactiveMember::Effect(HirEffect {
                    body: self.lower_expr(&e.body),
                    span: e.span,
                }),
                ReactiveMemberDecl::OnMount(m) => HirReactiveMember::OnMount(HirOnMount {
                    body: self.lower_expr(&m.body),
                    span: m.span,
                }),
                ReactiveMemberDecl::OnCleanup(c) => HirReactiveMember::OnCleanup(HirOnCleanup {
                    body: self.lower_expr(&c.body),
                    span: c.span,
                }),
                ReactiveMemberDecl::Stmt(s) => HirReactiveMember::Stmt(self.lower_stmt(s)),
            })
            .collect();
        let view = r.view.as_ref().map(|v| self.lower_expr(v));
        self.def_map.pop_scope();

        HirReactiveComponent {
            id,
            name: r.name.clone(),
            params,
            members,
            view,
            styles: r.styles.clone(),
            span: r.span,
        }
    }

    pub(crate) fn lower_agent(&mut self, a: &AgentDecl) -> HirAgent {
        let id = self.def_map.define(a.name.clone());
        HirAgent {
            id,
            name: a.name.clone(),
            version: a.version.clone(),
            state_fields: a
                .state_fields
                .iter()
                .map(|f| HirTableField {
                    name: f.name.clone(),
                    type_ann: self.lower_type(&f.type_ann),
                    span: f.span,
                })
                .collect(),
            handlers: a
                .handlers
                .iter()
                .map(|h| {
                    self.def_map.push_scope();
                    let params = h.params.iter().map(|p| self.lower_param(p)).collect();
                    let body = h.body.iter().map(|s| self.lower_stmt(s)).collect();
                    self.def_map.pop_scope();
                    HirAgentHandler {
                        event_name: h.event_name.clone(),
                        params,
                        return_type: h.return_type.as_ref().map(|t| self.lower_type(t)),
                        body,
                        is_traced: h.is_traced,
                        span: h.span,
                    }
                })
                .collect(),
            migrations: a
                .migrations
                .iter()
                .map(|m| {
                    self.def_map.push_scope();
                    let body = m.body.iter().map(|s| self.lower_stmt(s)).collect();
                    self.def_map.pop_scope();
                    HirMigrationRule {
                        from_version: m.from_version.clone(),
                        body,
                        span: m.span,
                    }
                })
                .collect(),
            is_deprecated: a.is_deprecated,
            span: a.span,
        }
    }

    pub(crate) fn lower_environment(&mut self, e: &EnvironmentDecl) -> HirEnvironment {
        HirEnvironment {
            name: e.name.clone(),
            base_image: e.base_image.clone(),
            packages: e.packages.clone(),
            env_vars: e.env_vars.clone(),
            exposed_ports: e.exposed_ports.clone(),
            volumes: e.volumes.clone(),
            workdir: e.workdir.clone(),
            cmd: e.cmd.clone(),
            copy_instructions: e.copy_instructions.clone(),
            run_commands: e.run_commands.clone(),
            is_deprecated: e.is_deprecated,
            span: e.span,
        }
    }

    pub(crate) fn lower_route(&mut self, r: &HttpRouteDecl) -> HirRoute {
        let method = match r.method {
            HttpMethod::Get => HirHttpMethod::Get,
            HttpMethod::Post => HirHttpMethod::Post,
            HttpMethod::Put => HirHttpMethod::Put,
            HttpMethod::Delete => HirHttpMethod::Delete,
        };
        let route_contract = format!("{} {}", method.as_str(), r.path);
        self.def_map.push_scope();
        let body = r.body.iter().map(|s| self.lower_stmt(s)).collect();
        self.def_map.pop_scope();

        HirRoute {
            method,
            path: r.path.clone(),
            route_contract,
            return_type: r.return_type.as_ref().map(|t| self.lower_type(t)),
            body,
            span: r.span,
        }
    }

    pub(crate) fn lower_url_decl(&mut self, u: &UrlDecl) -> HirUrlDecl {
        let id = self.def_map.define(u.name.clone());
        HirUrlDecl {
            id,
            name: u.name.clone(),
            variants: u
                .variants
                .iter()
                .map(|v| HirUrlVariant {
                    name: v.name.clone(),
                    args: v
                        .args
                        .iter()
                        .map(|a| HirUrlArg {
                            name: a.name.clone(),
                            optional: a.optional,
                            ty: self.lower_type(&a.type_ann),
                            span: a.span,
                        })
                        .collect(),
                    span: v.span,
                })
                .collect(),
            is_pub: u.is_pub,
            span: u.span,
        }
    }

    /// Lower a `workflow` declaration to `HirFn` (durability set by caller).
    pub(crate) fn lower_workflow(&mut self, w: &crate::ast::decl::WorkflowDecl) -> HirFn {
        let id = self.def_map.define(w.name.clone());
        self.def_map.push_scope();
        let params = w.params.iter().map(|p| self.lower_param(p)).collect();
        let body = w.body.iter().map(|s| self.lower_stmt(s)).collect();
        self.def_map.pop_scope();
        HirFn {
            id,
            name: w.name.clone(),
            generics: vec![],
            params,
            return_type: w.return_type.as_ref().map(|t| self.lower_type(t)),
            body,
            is_component: false,
            is_async: false,
            is_pub: false,
            is_mobile_native: false,
            is_pure: false,
            is_reactive: false,
            is_llm: false,
            llm_model: None,
            is_deprecated: w.is_deprecated,
            schedule_interval: None,
            durability: None, // overwritten by caller
            actor_state_fields: vec![],
            capabilities: vec![],
            postconditions: vec![],
            ts_extern_module: None,
            span: w.span,
        }
    }

    /// Lower an `activity` declaration to `HirFn` (durability set by caller).
    pub(crate) fn lower_activity(&mut self, a: &crate::ast::decl::ActivityDecl) -> HirFn {
        let id = self.def_map.define(a.name.clone());
        self.def_map.push_scope();
        let params = a.params.iter().map(|p| self.lower_param(p)).collect();
        let body = a.body.iter().map(|s| self.lower_stmt(s)).collect();
        self.def_map.pop_scope();
        HirFn {
            id,
            name: a.name.clone(),
            generics: vec![],
            params,
            return_type: a.return_type.as_ref().map(|t| self.lower_type(t)),
            body,
            is_component: false,
            is_async: false,
            is_pub: false,
            is_mobile_native: false,
            is_pure: false,
            is_reactive: false,
            is_llm: false,
            llm_model: None,
            is_deprecated: a.is_deprecated,
            schedule_interval: None,
            durability: None, // overwritten by caller
            actor_state_fields: vec![],
            capabilities: vec![],
            postconditions: vec![],
            ts_extern_module: None,
            span: a.span,
        }
    }

    /// Lower an `actor` declaration to an actor-shell `HirFn` (durability set by caller).
    /// State fields are attached to the shell. Call [`lower_actor_handlers`] to obtain
    /// the per-handler `HirFn` entries that carry the executable bodies.
    pub(crate) fn lower_actor(&mut self, a: &crate::ast::decl::logic::ActorDecl) -> HirFn {
        use crate::hir::HirTableField;
        let id = self.def_map.define(a.name.clone());
        let actor_state_fields = a
            .state_fields
            .iter()
            .map(|f| HirTableField {
                name: f.name.clone(),
                type_ann: self.lower_type(&f.type_ann),
                span: f.span,
            })
            .collect();
        HirFn {
            id,
            name: a.name.clone(),
            generics: vec![],
            params: vec![],
            return_type: None,
            body: vec![],
            is_component: false,
            is_async: false,
            is_pub: false,
            is_mobile_native: false,
            is_pure: false,
            is_reactive: false,
            is_llm: false,
            llm_model: None,
            is_deprecated: a.is_deprecated,
            schedule_interval: None,
            durability: None, // overwritten by caller
            actor_state_fields,
            capabilities: vec![],
            postconditions: vec![],
            ts_extern_module: None,
            span: a.span,
        }
    }

    /// Lower each `on event(...)` handler inside an actor into a standalone `HirFn`.
    ///
    /// Each handler is named `"ActorName::event_name"` and carries the full param list,
    /// return type, and lowered body so typecheck / codegen / runtime planning can see
    /// the handler's executable semantics. Durability is set by the caller (same as shell).
    pub(crate) fn lower_actor_handlers(
        &mut self,
        a: &crate::ast::decl::logic::ActorDecl,
    ) -> Vec<HirFn> {
        a.handlers
            .iter()
            .map(|h| {
                let handler_name = format!("{}::{}", a.name, h.event_name);
                let id = self.def_map.define(handler_name.clone());
                self.def_map.push_scope();
                let params = h.params.iter().map(|p| self.lower_param(p)).collect();
                let body = h.body.iter().map(|s| self.lower_stmt(s)).collect();
                self.def_map.pop_scope();
                HirFn {
                    id,
                    name: handler_name,
                    generics: vec![],
                    params,
                    return_type: h.return_type.as_ref().map(|t| self.lower_type(t)),
                    body,
                    is_component: false,
                    is_async: false,
                    is_pub: false,
                    is_mobile_native: false,
                    is_pure: false,
                    is_reactive: false,
                    is_llm: false,
                    llm_model: None,
                    is_deprecated: a.is_deprecated,
                    schedule_interval: None,
                    durability: None, // overwritten by caller (same as shell)
                    actor_state_fields: vec![],
                    capabilities: vec![],
                    postconditions: vec![],
                    ts_extern_module: None,
                    span: h.span,
                }
            })
            .collect()
    }

    pub(crate) fn lower_state_machine(&mut self, s: &StateMachineDecl) -> HirStateMachineDecl {
        let id = self.def_map.define(s.name.clone());
        HirStateMachineDecl {
            id,
            name: s.name.clone(),
            states: s
                .states
                .iter()
                .map(|st| HirSmState {
                    name: st.name.clone(),
                    fields: st
                        .fields
                        .iter()
                        .map(|f| HirSmField {
                            name: f.name.clone(),
                            ty: f.type_ann.as_ref().map(|t| self.lower_type(t)),
                            span: f.span,
                        })
                        .collect(),
                    is_terminal: st.is_terminal,
                    span: st.span,
                })
                .collect(),
            transitions: s
                .transitions
                .iter()
                .map(|tr| HirSmTransition {
                    event_name: tr.event_name.clone(),
                    event_params: tr.event_params.clone(),
                    from: match &tr.from {
                        SmFromPattern::Named(n) => HirSmFrom::Named(n.clone()),
                        SmFromPattern::Any => HirSmFrom::Any,
                    },
                    to_state: tr.to_state.clone(),
                    span: tr.span,
                })
                .collect(),
            is_partial: s.is_partial,
            is_pub: s.is_pub,
            span: s.span,
        }
    }
}
