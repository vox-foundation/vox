//! Formatter state and declaration / top-level printing (OP-0205).

use crate::ast::decl::*;
use crate::ast::expr::Param;
use crate::ast::stmt::Stmt;
use crate::ast::types::TypeExpr;

pub(crate) struct Printer {
    pub(crate) out: String,
    pub(crate) indent_level: usize,
}

impl Printer {
    pub(crate) fn new() -> Self {
        Self {
            out: String::new(),
            indent_level: 0,
        }
    }

    pub(crate) fn finish(self) -> String {
        self.out
    }

    pub(crate) fn indent(&mut self) {
        self.indent_level += 4;
    }

    pub(crate) fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(4);
    }

    pub(crate) fn write_indent(&mut self) {
        self.out.push_str(&" ".repeat(self.indent_level));
    }

    pub(crate) fn print_module(&mut self, module: &Module) {
        let mut last_was_import = false;
        for (i, decl) in module.declarations.iter().enumerate() {
            let is_import = matches!(decl, Decl::Import(_) | Decl::PyImport(_));

            if i > 0 {
                if is_import && last_was_import {
                    self.out.push('\n'); // Single newline between imports
                } else {
                    self.out.push_str("\n\n"); // Two newlines between top-level decl sections
                }
            }

            self.print_decl(decl);
            last_was_import = is_import;
        }
    }

    pub(crate) fn print_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Import(i) => self.print_import(i),
            Decl::PyImport(p) => {
                self.write_indent();
                self.out.push_str("@py.import ");
                self.out.push_str(&p.module);
                if p.alias != p.module.split('.').next_back().unwrap_or(p.module.as_str()) {
                    self.out.push_str(" as ");
                    self.out.push_str(&p.alias);
                }
            }
            Decl::Function(f) => self.print_fn(f, ""),
            Decl::Component(c) => self.print_fn(&c.func, "@component "),
            Decl::TypeDef(t) => self.print_typedef(t),
            Decl::Const(c) => {
                self.write_indent();
                if c.is_pub {
                    self.out.push_str("pub ");
                }
                self.out.push_str("const ");
                self.out.push_str(&c.name);
                if let Some(ref ty) = c.type_ann {
                    self.out.push_str(": ");
                    self.print_type(ty);
                }
                self.out.push_str(" = ");
                self.print_expr(&c.value);
            }
            Decl::HttpRoute(h) => {
                self.write_indent();
                self.out.push_str("http ");
                self.out.push_str(match h.method {
                    HttpMethod::Get => "get",
                    HttpMethod::Post => "post",
                    HttpMethod::Put => "put",
                    HttpMethod::Delete => "delete",
                });
                self.out.push(' ');
                self.out.push_str(&format!("\"{}\"", h.path));
                self.print_fn_body(&h.params, &h.return_type, &h.body);
            }
            Decl::Table(t) => {
                self.write_indent();
                if t.is_pub {
                    self.out.push_str("pub ");
                }
                self.out.push_str("@table type ");
                self.out.push_str(&t.name);
                self.out.push_str(
                    " {
",
                );
                self.indent();
                for field in &t.fields {
                    self.write_indent();
                    self.out.push_str(&field.name);
                    self.out.push_str(": ");
                    self.print_type(&field.type_ann);
                    self.out.push('\n');
                }
                self.dedent();
                self.write_indent();
                self.out.push('}');
            }
            Decl::ServerFn(s) => self.print_fn(&s.func, "@server "),
            Decl::Query(q) => self.print_fn(&q.func, "@query "),
            Decl::Mutation(m) => self.print_fn(&m.func, "@mutation "),

            Decl::Skill(s) => self.print_fn(&s.func, "@skill "),
            Decl::AgentDef(a) => self.print_fn(&a.func, "@agent_def "),
            Decl::Scheduled(s) => {
                self.write_indent();
                self.out
                    .push_str(&format!("@scheduled(\"{}\") ", s.interval));
                self.out.push_str("fn ");
                self.out.push_str(&s.func.name);
                self.print_fn_body(&s.func.params, &s.func.return_type, &s.func.body);
            }
            Decl::Test(t) => self.print_fn(&t.func, "@test "),
            Decl::Hook(h) => self.print_fn(&h.func, "@hook "),
            Decl::McpTool(m) => {
                self.write_indent();
                self.out
                    .push_str(&format!("@mcp.tool(\"{}\") ", m.description));
                self.out.push_str("fn ");
                self.out.push_str(&m.func.name);
                self.print_fn_body(&m.func.params, &m.func.return_type, &m.func.body);
            }
            Decl::McpResource(m) => {
                self.write_indent();
                let ue = m.uri.replace('\\', "\\\\").replace('"', "\\\"");
                let de = m.description.replace('\\', "\\\\").replace('"', "\\\"");
                self.out
                    .push_str(&format!("@mcp.resource(\"{ue}\", \"{de}\") "));
                self.out.push_str("fn ");
                self.out.push_str(&m.func.name);
                self.print_fn_body(&m.func.params, &m.func.return_type, &m.func.body);
            }
            Decl::Routes(r) => {
                self.write_indent();
                self.out.push_str(
                    "routes {
",
                );
                self.indent();
                for entry in &r.entries {
                    self.write_indent();
                    self.out.push_str(&format!("\"{}\"", entry.path));
                    self.out.push_str(" to ");
                    self.out.push_str(&entry.component_name);
                    self.out.push('\n');
                }
                self.dedent();
                self.write_indent();
                self.out.push('}');
            }
            Decl::Config(c) => {
                self.write_indent();
                self.out.push_str("@config ");
                self.out.push_str(&c.name);
                self.out.push_str(":\n");
                self.indent();
                for field in &c.fields {
                    self.write_indent();
                    self.out.push_str(&field.name);
                    self.out.push_str(": ");
                    self.print_type(&field.type_ann);
                    self.out.push('\n');
                }
                self.dedent();
            }
            Decl::Workflow(w) => {
                self.write_indent();
                self.out.push_str("workflow ");
                self.out.push_str(&w.name);
                self.print_fn_body(&w.params, &w.return_type, &w.body);
            }
            Decl::Activity(a) => {
                self.write_indent();
                self.out.push_str("activity ");
                self.out.push_str(&a.name);
                self.print_fn_body(&a.params, &a.return_type, &a.body);
            }
            Decl::Actor(a) => {
                self.write_indent();
                self.out.push_str("actor ");
                self.out.push_str(&a.name);
                self.out.push_str(
                    " {
",
                );
                self.indent();
                for handler in &a.handlers {
                    self.write_indent();
                    self.out.push_str("on ");
                    self.out.push_str(&handler.event_name);
                    self.print_fn_body(&handler.params, &handler.return_type, &handler.body);
                    self.out.push('\n');
                }
                self.dedent();
                self.write_indent();
                self.out.push('}');
            }
            Decl::Environment(e) => self.print_environment(e),
            Decl::Island(isle) => {
                self.write_indent();
                self.out.push_str("@island ");
                self.out.push_str(&isle.name);
                self.out.push_str(
                    " {
",
                );
                if !isle.props.is_empty() {
                    self.indent();
                    for p in &isle.props {
                        self.write_indent();
                        self.out.push_str(&p.name);
                        if p.is_optional {
                            self.out.push('?');
                        }
                        self.out.push_str(": ");
                        self.print_type(&p.ty);
                        self.out.push('\n');
                    }
                    self.dedent();
                }
                self.write_indent();
                self.out.push('}');
            }
            _ => {}
        }
    }

    fn print_import(&mut self, i: &ImportDecl) {
        self.write_indent();
        self.out.push_str("import ");
        for (idx, path) in i.paths.iter().enumerate() {
            if idx > 0 {
                self.out.push_str(", ");
            }
            match &path.kind {
                ImportPathKind::SymbolPath { segments } => {
                    self.out.push_str(&segments.join("."));
                }
                ImportPathKind::RustCrate(spec) => {
                    self.out.push_str("rust:");
                    self.out.push_str(&spec.crate_name);
                    let mut meta = Vec::new();
                    if let Some(v) = &spec.version {
                        meta.push(format!("version: \"{v}\""));
                    }
                    if let Some(v) = &spec.path {
                        meta.push(format!("path: \"{v}\""));
                    }
                    if let Some(v) = &spec.git {
                        meta.push(format!("git: \"{v}\""));
                    }
                    if let Some(v) = &spec.rev {
                        meta.push(format!("rev: \"{v}\""));
                    }
                    if !meta.is_empty() {
                        self.out.push('(');
                        self.out.push_str(&meta.join(", "));
                        self.out.push(')');
                    }
                }
            }
            if let Some(alias) = &path.alias {
                self.out.push_str(" as ");
                self.out.push_str(alias);
            }
        }
    }

    fn print_typedef(&mut self, t: &TypeDefDecl) {
        self.write_indent();
        if t.is_pub {
            self.out.push_str("pub ");
        }
        self.out.push_str("type ");
        self.out.push_str(&t.name);

        if let Some(ref alias) = t.type_alias {
            self.out.push_str(" = ");
            self.print_type(alias);
        } else if !t.variants.is_empty() {
            self.out.push_str(" = \n");
            self.indent();
            for var in &t.variants {
                self.write_indent();
                self.out.push_str("| ");
                self.out.push_str(&var.name);
                if !var.fields.is_empty() {
                    self.out.push_str(" { ");
                    for (i, field) in var.fields.iter().enumerate() {
                        if i > 0 {
                            self.out.push_str(", ");
                        }
                        self.out.push_str(&field.name);
                        self.out.push_str(": ");
                        self.print_type(&field.type_ann);
                    }
                    self.out.push_str(" }");
                }
                self.out.push('\n');
            }
            self.dedent();
        } else {
            self.out.push_str(" {\n");
            self.indent();
            for field in &t.fields {
                self.write_indent();
                self.out.push_str(&field.name);
                self.out.push_str(": ");
                self.print_type(&field.type_ann);
                self.out.push('\n');
            }
            self.dedent();
            self.write_indent();
            self.out.push('}');
        }
    }

    fn print_fn(&mut self, f: &FnDecl, prefix: &str) {
        self.write_indent();
        self.out.push_str(prefix);
        if f.is_pub {
            self.out.push_str("pub ");
        }
        if f.is_async {
            self.out.push_str("async ");
        }
        self.out.push_str("fn ");
        self.out.push_str(&f.name);
        self.print_fn_body(&f.params, &f.return_type, &f.body);
    }

    pub(crate) fn print_fn_body(
        &mut self,
        params: &[Param],
        ret: &Option<TypeExpr>,
        body: &[Stmt],
    ) {
        self.out.push('(');
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            self.out.push_str(&p.name);
            if let Some(ty) = p.type_ann.as_ref() {
                self.out.push_str(": ");
                self.print_type(ty);
            }
        }
        self.out.push(')');

        if let Some(r) = ret.as_ref() {
            self.out.push_str(" to ");
            self.print_type(r);
        }

        if body.is_empty() {
            self.out.push_str(
                " {
",
            );
            self.indent();
            self.write_indent();
            self.out.push_str("pass\n");
            self.dedent();
            self.write_indent();
            self.out.push('}');
        } else {
            self.out.push_str(
                " {
",
            );
            self.indent();
            for s in body {
                self.print_stmt(s);
            }
            self.dedent();
            self.write_indent();
            self.out.push('}');
        }
    }

    fn print_environment(&mut self, env: &EnvironmentDecl) {
        self.write_indent();
        self.out.push_str("@environment ");
        self.out.push_str(&env.name);
        self.out.push_str(
            " {
",
        );
        self.indent();

        if let Some(ref base) = env.base_image {
            self.write_indent();
            self.out.push_str("base: ");
            self.out.push_str(&format!("\"{}\"", base));
            self.out.push('\n');
        }

        if let Some(ref wd) = env.workdir {
            self.write_indent();
            self.out.push_str("workdir: ");
            self.out.push_str(&format!("\"{}\"", wd));
            self.out.push('\n');
        }

        if !env.packages.is_empty() {
            self.write_indent();
            self.out.push_str("packages: [");
            for (i, pkg) in env.packages.iter().enumerate() {
                if i > 0 {
                    self.out.push_str(", ");
                }
                self.out.push_str(&format!("\"{}\"", pkg));
            }
            self.out.push_str("]\n");
        }

        if !env.env_vars.is_empty() {
            self.write_indent();
            self.out.push_str("env:\n");
            self.indent();
            for (k, v) in &env.env_vars {
                self.write_indent();
                self.out.push_str(k);
                self.out.push_str(": ");
                self.out.push_str(&format!("\"{}\"", v));
                self.out.push('\n');
            }
            self.dedent();
        }

        if !env.exposed_ports.is_empty() {
            self.write_indent();
            self.out.push_str("expose: [");
            for (i, p) in env.exposed_ports.iter().enumerate() {
                if i > 0 {
                    self.out.push_str(", ");
                }
                self.out.push_str(&p.to_string());
            }
            self.out.push_str("]\n");
        }

        if !env.volumes.is_empty() {
            self.write_indent();
            self.out.push_str("volumes: [");
            for (i, v) in env.volumes.iter().enumerate() {
                if i > 0 {
                    self.out.push_str(", ");
                }
                self.out.push_str(&format!("\"{}\"", v));
            }
            self.out.push_str("]\n");
        }

        if !env.copy_instructions.is_empty() {
            self.write_indent();
            self.out.push_str("copy:\n");
            self.indent();
            for (src, dest) in &env.copy_instructions {
                self.write_indent();
                self.out.push_str(&format!("\"{}\" to \"{}\"\n", src, dest));
            }
            self.dedent();
        }

        if !env.run_commands.is_empty() {
            self.write_indent();
            self.out.push_str("run: [");
            for (i, cmd) in env.run_commands.iter().enumerate() {
                if i > 0 {
                    self.out.push_str(", ");
                }
                self.out.push_str(&format!("\"{}\"", cmd));
            }
            self.out.push_str("]\n");
        }

        if !env.cmd.is_empty() {
            self.write_indent();
            self.out.push_str("cmd: [");
            for (i, c) in env.cmd.iter().enumerate() {
                if i > 0 {
                    self.out.push_str(", ");
                }
                self.out.push_str(&format!("\"{}\"", c));
            }
            self.out.push_str("]\n");
        }

        self.dedent();
        self.write_indent();
        self.out.push('}');
    }
}
