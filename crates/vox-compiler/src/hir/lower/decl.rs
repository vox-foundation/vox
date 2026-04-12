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
            is_deprecated: f.is_deprecated,
            schedule_interval: None,
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
            is_pub: t.is_pub,
            span: t.span,
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

    pub(crate) fn lower_actor(&mut self, a: &ActorDecl) -> HirActor {
        let id = self.def_map.define(a.name.clone());
        HirActor {
            id,
            name: a.name.clone(),
            handlers: a
                .handlers
                .iter()
                .map(|h| {
                    self.def_map.push_scope();
                    let params = h.params.iter().map(|p| self.lower_param(p)).collect();
                    let body = h.body.iter().map(|s| self.lower_stmt(s)).collect();
                    self.def_map.pop_scope();
                    HirActorHandler {
                        event_name: h.event_name.clone(),
                        params,
                        return_type: h.return_type.as_ref().map(|t| self.lower_type(t)),
                        body,
                        span: h.span,
                    }
                })
                .collect(),
            span: a.span,
        }
    }

    pub(crate) fn lower_workflow(&mut self, w: &WorkflowDecl) -> HirWorkflow {
        let id = self.def_map.define(w.name.clone());
        self.def_map.push_scope();
        let params = w.params.iter().map(|p| self.lower_param(p)).collect();
        let body = w.body.iter().map(|s| self.lower_stmt(s)).collect();
        self.def_map.pop_scope();

        HirWorkflow {
            id,
            name: w.name.clone(),
            params,
            return_type: w.return_type.as_ref().map(|t| self.lower_type(t)),
            body,
            span: w.span,
        }
    }

    pub(crate) fn lower_activity(&mut self, a: &ActivityDecl) -> HirActivity {
        let id = self.def_map.define(a.name.clone());
        self.def_map.push_scope();
        let params = a.params.iter().map(|p| self.lower_param(p)).collect();
        let body = a.body.iter().map(|s| self.lower_stmt(s)).collect();
        self.def_map.pop_scope();

        HirActivity {
            id,
            name: a.name.clone(),
            params,
            return_type: a.return_type.as_ref().map(|t| self.lower_type(t)),
            body,
            span: a.span,
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
        let members = r
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
}
