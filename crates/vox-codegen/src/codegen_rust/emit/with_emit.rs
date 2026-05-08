//! `emit_expr` helper for `HirExpr::With` (activity options).

use vox_compiler::hir::HirExpr;

pub(super) fn emit_with<F>(emit_expr: &F, operand: &HirExpr, options: &HirExpr) -> String
where
    F: Fn(&HirExpr) -> String,
{
    let mut opts_builder = String::from("vox_runtime::ActivityOptions::new()");

    if let HirExpr::ObjectLit(fields, _) = options {
        for (key, value) in fields {
            match key.as_str() {
                "retries" => {
                    opts_builder.push_str(&format!(".with_retries({} as u32)", emit_expr(value)));
                }
                "timeout" => match value {
                    HirExpr::StringLit(s, _) => {
                        opts_builder.push_str(&format!(
                                ".with_timeout(vox_runtime::ActivityOptions::parse_duration(\"{}\").expect(\"vox codegen: activity timeout duration\"))",
                                s
                            ));
                    }
                    _ => {
                        opts_builder
                            .push_str(&format!(".with_timeout_secs({} as u64)", emit_expr(value)));
                    }
                },
                "initial_backoff" => {
                    if let HirExpr::StringLit(s, _) = value {
                        opts_builder.push_str(&format!(
                            ".with_initial_backoff(vox_runtime::ActivityOptions::parse_duration(\"{}\").expect(\"vox codegen: initial_backoff duration\"))",
                            s
                        ));
                    }
                }
                "max_backoff" => {
                    if let HirExpr::StringLit(s, _) = value {
                        opts_builder.push_str(&format!(
                            ".with_max_backoff(vox_runtime::ActivityOptions::parse_duration(\"{}\").expect(\"vox codegen: max_backoff duration\"))",
                            s
                        ));
                    }
                }
                "backoff_multiplier" => {
                    opts_builder.push_str(&format!(
                        ".with_backoff_multiplier({} as f64)",
                        emit_expr(value)
                    ));
                }
                "activity_id" | "id" => {
                    if let HirExpr::StringLit(s, _) = value {
                        opts_builder.push_str(&format!(".with_activity_id(\"{}\".to_string())", s));
                    }
                }
                _ => {}
            }
        }
    }

    let operand_str = emit_expr(operand);
    let activity_name = resolve_activity_name(operand);
    format!(
        "vox_runtime::execute_activity_result(\"{name}\", &{opts}, || async {{ {call} }}).await",
        name = activity_name,
        opts = opts_builder,
        call = operand_str,
    )
}

fn resolve_activity_name(operand: &HirExpr) -> String {
    match operand {
        HirExpr::Call(callee, _, _, _) => match callee.as_ref() {
            HirExpr::Ident(name, _) => name.clone(),
            _ => "activity".to_string(),
        },
        HirExpr::MethodCall(_, method, _, _, _) => method.clone(),
        HirExpr::Ident(name, _) => name.clone(),
        _ => "activity".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::emit_with;
    use vox_compiler::ast::span::Span;
    use vox_compiler::hir::HirExpr;

    #[test]
    fn emits_typed_activity_result_path_without_panics() {
        let span = Span::new(0, 0);
        let operand = HirExpr::Ident("work".to_string(), span);
        let options = HirExpr::ObjectLit(vec![], span);
        let emitted = emit_with(&|_| "work()".to_string(), &operand, &options);
        assert!(
            emitted.contains("execute_activity_result"),
            "expected typed result helper in {emitted}"
        );
        assert!(
            !emitted.contains("panic!("),
            "with emission should not panic-map failures: {emitted}"
        );
        assert!(
            emitted.contains("execute_activity_result(\"work\""),
            "with emission should carry stable activity identity: {emitted}"
        );
    }

    #[test]
    fn supports_id_alias_for_activity_id_option() {
        let span = Span::new(0, 0);
        let operand = HirExpr::Ident("work".to_string(), span);
        let options = HirExpr::ObjectLit(
            vec![(
                "id".to_string(),
                HirExpr::StringLit("alias-id".to_string(), span),
            )],
            span,
        );
        let emitted = emit_with(&|_| "work()".to_string(), &operand, &options);
        assert!(
            emitted.contains(".with_activity_id(\"alias-id\".to_string())"),
            "`id` alias should map to ActivityOptions::with_activity_id: {emitted}"
        );
    }
}
