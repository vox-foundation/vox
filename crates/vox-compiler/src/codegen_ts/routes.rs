use crate::codegen_ts::hir_emit::{emit_hir_expr, emit_hir_pattern};
use crate::hir::{HirExpr, HirHttpMethod, HirModule, HirStmt};
use std::collections::HashSet;

fn emit_hir_route_expr(expr: &HirExpr) -> String {
    let empty = HashSet::new();
    match expr {
        HirExpr::MethodCall(object, method, args, _) => {
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
    }
}

/// Generate Express.js route handlers from Vox HTTP routes and server functions (HIR-first).
pub fn generate_routes(hir: &HirModule) -> String {
    let routes = &hir.routes;
    let server_fns: Vec<_> = hir
        .server_fns
        .iter()
        .chain(hir.query_fns.iter())
        .chain(hir.mutation_fns.iter())
        .collect();

    if routes.is_empty() && server_fns.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("import express, { Request, Response } from \"express\";\n");
    out.push_str("import cors from \"cors\";\n\n");
    out.push_str("const app = express();\n");
    out.push_str("app.use(cors());\n");
    out.push_str("app.use(express.json());\n\n");

    // Only emit mock actor when there are HTTP routes that might use actors
    if !routes.is_empty() {
        out.push_str("// Mock LLM actor (replace with real API key for production)\n");
        out.push_str("class ClaudeActor {\n");
        out.push_str("  async send(message: string): Promise<string> {\n");
        out.push_str("    const apiKey = process.env.ANTHROPIC_API_KEY;\n");
        out.push_str("    if (apiKey) {\n");
        out.push_str(
            "      const response = await fetch(\"https://api.anthropic.com/v1/messages\", {\n",
        );
        out.push_str("        method: \"POST\",\n");
        out.push_str("        headers: {\n");
        out.push_str("          \"Content-Type\": \"application/json\",\n");
        out.push_str("          \"x-api-key\": apiKey,\n");
        out.push_str("          \"anthropic-version\": \"2023-06-01\",\n");
        out.push_str("        },\n");
        out.push_str("        body: JSON.stringify({\n");
        out.push_str("          model: \"claude-sonnet-4-20250514\",\n");
        out.push_str("          max_tokens: 256,\n");
        out.push_str("          messages: [{ role: \"user\", content: message }],\n");
        out.push_str("        }),\n");
        out.push_str("      });\n");
        out.push_str("      const data = await response.json() as any;\n");
        out.push_str("      return data.content?.[0]?.text || \"No response from Claude\";\n");
        out.push_str("    }\n");
        out.push_str("    // Mock response when no API key is set\n");
        out.push_str("    return `Vox AI Echo: ${message}`;\n");
        out.push_str("  }\n");
        out.push_str("}\n\n");
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

    for sf in &server_fns {
        let route_path = &sf.route_path;
        out.push_str(&format!(
            "app.post(\"{route_path}\", async (req: Request, res: Response) => {{\n"
        ));
        out.push_str("  try {\n");
        for param in &sf.params {
            out.push_str(&format!(
                "    const {} = req.body.{};\n",
                param.name, param.name
            ));
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
