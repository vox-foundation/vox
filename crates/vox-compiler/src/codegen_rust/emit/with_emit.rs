//! `emit_expr` helper for `HirExpr::With` (activity options).

use crate::hir::HirExpr;

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
                "activity_id" => {
                    if let HirExpr::StringLit(s, _) = value {
                        opts_builder.push_str(&format!(".with_activity_id(\"{}\".to_string())", s));
                    }
                }
                _ => {}
            }
        }
    }

    let operand_str = emit_expr(operand);
    format!(
        "match vox_runtime::execute_activity(\"activity\", &{opts}, || async {{ {call} }}).await {{ \
            vox_runtime::ActivityResult::Ok(v) => v, \
            vox_runtime::ActivityResult::Failed(e) => panic!(\"Activity failed: {{}}\", e), \
            vox_runtime::ActivityResult::Cancelled => panic!(\"Activity cancelled\"), \
        }}",
        opts = opts_builder,
        call = operand_str,
    )
}
