//! Express `server.ts` generation from HIR HTTP routes and `@server` / `@query` / `@mutation` fns.
//!
//! ## Adapter seam (OP-0161..OP-0176)
//!
//! - **Input:** [`ExpressRouteEmitCtx`] wraps [`HirModule`] for validation + deterministic emission order.
//! - **Contracts:** [`crate::web_ir::lower::lower_hir_to_web_ir`] is the structural SSOT for route IDs and
//!   client/tree tooling; this module remains **body-driven** Express glue (actor `send`, `spawn`, etc.).
//! - **TanStack Start / SPA:** unchanged here — [`super::emitter::CodegenOptions::tanstack_start`] only
//!   affects client route-tree files (OP-0168).
//!
//! Use [`validate_express_route_emit_input`] before enabling `VOX_EMIT_EXPRESS_SERVER` when you need
//! fail-fast checks (tests, CI).
//!
//! ## Route contract mapper (OP-S033)
//!
//! - Each [`HirRoute`](crate::hir::HirRoute) carries a stable `route_contract` string (`"{METHOD} {path}"`)
//!   from HIR lowering—surfaced in validation errors and logs alongside this module’s Express wire-up.
//! - Client SPA / TanStack tree **IDs** and loader contracts are produced by
//!   [`crate::web_ir::lower::lower_hir_to_web_ir`]; this file does **not** remap Web IR `RouteContract`
//!   back into HTTP handlers—keep duplicate-path checks here (`validate_express_route_emit_input`)
//!   orthogonal to client route family validation inside [`validate_web_ir`](crate::web_ir::validate::validate_web_ir).
//!
//! **Route contract + diff policy (OP-S061 / S089 / S117 / S139 / S159 / S191):** deterministic sort orders
//! in this module must stay aligned with duplicate-detection in [`validate_express_route_emit_input`] and Web IR
//! route id policy — changing sort keys requires dual updates in `validate_web_ir` route stage.

use crate::codegen_ts::hir_emit::{emit_hir_expr, emit_hir_pattern};
use crate::hir::{HirExpr, HirHttpMethod, HirModule, HirRoute, HirEndpointFn, HirEndpointKind, HirStmt};
use std::collections::HashSet;

/// Mock `ClaudeActor` embedded in generated `server.ts` when HTTP routes exist (OP-0172 SSOT).
const EXPRESS_TYPESCRIPT_CLAUDE_ACTOR_CLASS: &str = r#"class ClaudeActor {
  async send(message: string): Promise<string> {
    const apiKey = process.env.ANTHROPIC_API_KEY;
    if (apiKey) {
      const response = await fetch("https://api.anthropic.com/v1/messages", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-api-key": apiKey,
          "anthropic-version": "2023-06-01",
        },
        body: JSON.stringify({
          model: "claude-sonnet-4-20250514",
          max_tokens: 256,
          messages: [{ role: "user", content: message }],
        }),
      });
      const data = await response.json() as any;
      return data.content?.[0]?.text || "No response from Claude";
    }
    // Mock response when no API key is set
    return `Vox AI Echo: ${message}`;
  }
}

"#;

fn http_method_ord(m: HirHttpMethod) -> u8 {
    match m {
        HirHttpMethod::Get => 0,
        HirHttpMethod::Post => 1,
        HirHttpMethod::Put => 2,
        HirHttpMethod::Delete => 3,
    }
}

/// Fail-fast checks for duplicate / unresolvably-ambiguous Express registrations and
/// empty paths (OP-0170, extended in Phase C of the Svelte-mineable features plan with
/// segment-aware overlap detection via [`super::route_pattern`]).
///
/// **Conflict policy:**
/// - Empty path → error.
/// - Same method, [`super::route_pattern::Overlap::Ambiguous`] (e.g. `/users/me` declared
///   twice, or `/:a/:b` vs `/:x/:y`) → error.
/// - Same method, [`super::route_pattern::Overlap::Shadowed`] (e.g. `/users/me` and
///   `/users/:id`) → allowed; the more specific route wins by source order. No diagnostic
///   surfaced from this fast-path validator (a future change can route a structured
///   `routes.overlap.shadowed` info diagnostic through the typeck channel).
pub fn validate_express_route_emit_input(hir: &HirModule) -> Result<(), String> {
    use super::route_pattern::{Overlap, RoutePattern};
    use crate::codegen_shared::lower_module_routes;

    let routes = lower_module_routes(hir);
    let mut parsed: Vec<(usize, RoutePattern)> = Vec::with_capacity(routes.len());

    for (idx, r) in routes.iter().enumerate() {
        let path = r.path.trim();
        if path.is_empty() {
            return Err(format!(
                "HTTP {} route has empty path (contract {})",
                r.method.as_uppercase_str(),
                r.contract_key
            ));
        }

        let pattern = RoutePattern::parse(path);

        for (prev_idx, prev_pattern) in &parsed {
            if routes[*prev_idx].method != r.method {
                continue;
            }
            if matches!(pattern.overlap_with(prev_pattern), Overlap::Ambiguous) {
                return Err(format!(
                    "ambiguous Express handlers for {} {} vs {} (no specificity tiebreaker — \
                     declare the more specific route or rename one of the params)",
                    r.method.as_uppercase_str(),
                    r.path,
                    routes[*prev_idx].path
                ));
            }
        }

        parsed.push((idx, pattern));
    }

    Ok(())
}

/// Wrapper for HIR-first Express emission (OP-0161).
pub struct ExpressRouteEmitCtx<'a> {
    hir: &'a HirModule,
}

impl<'a> ExpressRouteEmitCtx<'a> {
    #[must_use]
    pub fn new(hir: &'a HirModule) -> Self {
        Self { hir }
    }

    #[must_use]
    pub fn hir(&self) -> &'a HirModule {
        self.hir
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_express_route_emit_input(self.hir)
    }
}

fn sorted_http_routes(hir: &HirModule) -> Vec<&HirRoute> {
    let mut v: Vec<_> = hir.routes.iter().collect();
    v.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| http_method_ord(a.method).cmp(&http_method_ord(b.method)))
    });
    v
}

fn sorted_endpoint_fns(hir: &HirModule) -> Vec<&HirEndpointFn> {
    let mut v: Vec<_> = hir.endpoint_fns.iter().collect();
    v.sort_by(|a, b| {
        a.route_path
            .cmp(&b.route_path)
            .then_with(|| a.name.cmp(&b.name))
    });
    v
}

fn emit_hir_route_expr(expr: &HirExpr) -> String {
    let empty = HashSet::new();
    match expr {
        HirExpr::MethodCall(object, method, args, _, _) => {
            let obj = emit_hir_route_expr(object);
            let args_str: Vec<String> = args
                .iter()
                .map(|a| emit_expr_from_hir_arg(&a.value))
                .collect();
            if method == "send" {
                format!("await {obj}.send({})", args_str.join(", "))
            } else {
                format!("{obj}.{method}({})", args_str.join(", "))
            }
        }
        HirExpr::Spawn(target, _) => {
            format!("new {}Actor()", emit_hir_expr(target, &empty))
        }
        HirExpr::FieldAccess(object, field, _) => {
            let obj = emit_hir_route_expr(object);
            format!("{obj}.{field}")
        }
        HirExpr::Call(callee, args, _, _) => {
            let callee_str = emit_hir_route_expr(callee);
            let args_str: Vec<String> = args
                .iter()
                .map(|a| emit_expr_from_hir_arg(&a.value))
                .collect();
            format!("{callee_str}({})", args_str.join(", "))
        }
        _ => emit_hir_expr(expr, &empty),
    }
}

fn emit_expr_from_hir_arg(expr: &HirExpr) -> String {
    emit_hir_expr(expr, &HashSet::new())
}

fn emit_hir_route_stmt(stmt: &HirStmt) -> String {
    match stmt {
        HirStmt::Let {
            pattern,
            value,
            mutable,
            ..
        } => {
            let keyword = if *mutable { "let" } else { "const" };
            let pat = emit_hir_pattern(pattern);
            let val = emit_hir_route_expr(value);
            format!("{keyword} {pat} = {val};\n")
        }
        HirStmt::Return {
            value: Some(expr), ..
        } => {
            let result = emit_hir_route_expr(expr);
            format!("const result = {result};\n    res.json({{ text: result }});\n")
        }
        HirStmt::Return { value: None, .. } => "res.sendStatus(204);\n".to_string(),
        HirStmt::Assign { target, value, .. } => {
            format!(
                "{} = {};\n",
                emit_hir_route_expr(target),
                emit_hir_route_expr(value)
            )
        }
        HirStmt::Expr { expr, .. } => {
            format!("{};\n", emit_hir_route_expr(expr))
        }
        HirStmt::While {
            condition, body, ..
        } => {
            let cond = emit_hir_route_expr(condition);
            let mut out = format!("while ({cond}) {{\n");
            for s in body {
                out.push_str(&format!("  {}", emit_hir_route_stmt(s)));
            }
            out.push_str("    }\n");
            out
        }
        HirStmt::Loop { body, .. } => {
            let mut out = "while (true) {\n".to_string();
            for s in body {
                out.push_str(&format!("  {}", emit_hir_route_stmt(s)));
            }
            out.push_str("    }\n");
            out
        }
        HirStmt::Break { .. } => "break;\n".to_string(),
        HirStmt::Continue { .. } => "continue;\n".to_string(),
    }
}

/// Generate Express.js route handlers from Vox HTTP routes and server functions (HIR-first).
///
/// Route and server-fn blocks are emitted in **stable sorted order** (path, then method / name) (OP-0166).
pub fn generate_routes(hir: &HirModule) -> String {
    generate_routes_from_ctx(&ExpressRouteEmitCtx::new(hir))
}

/// Like [`generate_routes`] but accepts a pre-built [`ExpressRouteEmitCtx`].
#[must_use]
pub fn generate_routes_from_ctx(ctx: &ExpressRouteEmitCtx<'_>) -> String {
    let hir = ctx.hir();
    let routes = sorted_http_routes(hir);
    let endpoint_fns = sorted_endpoint_fns(hir);

    if routes.is_empty() && endpoint_fns.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("import express, { Request, Response } from \"express\";\n");
    out.push_str("import cors from \"cors\";\n\n");
    out.push_str("const app = express();\n");
    out.push_str("app.use(cors());\n");
    out.push_str("app.use(express.json());\n\n");

    if !routes.is_empty() {
        out.push_str("// Mock LLM actor (replace with real API key for production)\n");
        out.push_str(EXPRESS_TYPESCRIPT_CLAUDE_ACTOR_CLASS);
    }

    for route in routes {
        let method = match route.method {
            HirHttpMethod::Get => "get",
            HirHttpMethod::Post => "post",
            HirHttpMethod::Put => "put",
            HirHttpMethod::Delete => "delete",
        };
        let path = &route.path;

        out.push_str(&format!(
            "app.{method}(\"{path}\", async (req: Request, res: Response) => {{\n"
        ));
        out.push_str("  try {\n");
        out.push_str("    const request = req;\n");

        for stmt in &route.body {
            out.push_str(&format!("    {}", emit_hir_route_stmt(stmt)));
        }

        out.push_str("  } catch (err) {\n");
        out.push_str("    res.status(500).json({ error: String(err) });\n");
        out.push_str("  }\n");
        out.push_str("});\n\n");
    }

    for sf in &endpoint_fns {
        let route_path = &sf.route_path;
        let is_query = sf.kind == HirEndpointKind::Query;
        let method = if is_query { "get" } else { "post" };
        out.push_str(&format!(
            "app.{method}(\"{route_path}\", async (req: Request, res: Response) => {{\n"
        ));
        out.push_str("  try {\n");
        if is_query {
            out.push_str("    const q = req.query;\n");
            for param in &sf.params {
                out.push_str(&format!(
                    "    const {p} = q.{p} !== undefined ? JSON.parse(String(q.{p})) : null;\n",
                    p = param.name
                ));
            }
        } else {
            for param in &sf.params {
                out.push_str(&format!(
                    "    const {} = req.body.{};\n",
                    param.name, param.name
                ));
            }
        }
        for stmt in &sf.body {
            out.push_str(&format!("    {}", emit_hir_route_stmt(stmt)));
        }
        out.push_str("  } catch (err) {\n");
        out.push_str("    res.status(500).json({ error: String(err) });\n");
        out.push_str("  }\n");
        out.push_str("});\n\n");
    }

    out.push_str("const PORT = process.env.PORT || 3001;\n");
    out.push_str("app.listen(PORT, () => {\n");
    out.push_str("  console.log(`Vox server running on port ${PORT}`);\n");
    out.push_str("});\n");

    out
}
