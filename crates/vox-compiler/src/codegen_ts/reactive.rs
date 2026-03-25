use crate::codegen_ts::hir_emit::{
    emit_block_stmts, emit_hir_expr, extract_state_deps, map_hir_type_to_ts,
};
use crate::hir::*;
use crate::react_bridge::react_exports::{USE_EFFECT, USE_MEMO, USE_STATE};
use std::collections::HashSet;

fn react_import_line(members: &[HirReactiveMember]) -> String {
    let mut need_state = false;
    let mut need_effect = false;
    let mut need_memo = false;
    for m in members {
        match m {
            HirReactiveMember::State(_) => need_state = true,
            HirReactiveMember::Derived(_) => need_memo = true,
            HirReactiveMember::Effect(_)
            | HirReactiveMember::OnMount(_)
            | HirReactiveMember::OnCleanup(_) => need_effect = true,
        }
    }
    let mut hooks = Vec::new();
    if need_state {
        hooks.push(USE_STATE);
    }
    if need_effect {
        hooks.push(USE_EFFECT);
    }
    if need_memo {
        hooks.push(USE_MEMO);
    }
    if hooks.is_empty() {
        return "import React from \"react\";\n\n".to_string();
    }
    format!(
        "import React, {{ {} }} from \"react\";\n\n",
        hooks.join(", ")
    )
}

pub fn generate_reactive_component(rc: &HirReactiveComponent) -> (String, String) {
    let name = &rc.name;
    let filename = format!("{name}.tsx");
    let mut out = String::new();

    let mut state_names = HashSet::new();
    for member in &rc.members {
        if let HirReactiveMember::State(s) = member {
            state_names.insert(s.name.clone());
        }
    }

    out.push_str(&react_import_line(&rc.members));

    if !rc.params.is_empty() {
        out.push_str(&format!("export interface {name}Props {{\n"));
        for param in &rc.params {
            let ts_type = param
                .type_ann
                .as_ref()
                .map_or("any".to_string(), map_hir_type_to_ts);
            out.push_str(&format!("  {}: {};\n", param.name, ts_type));
        }
        out.push_str("}\n\n");
    }

    if rc.params.is_empty() {
        out.push_str(&format!(
            "export function {name}(): React.ReactElement {{\n"
        ));
    } else {
        let param_names: Vec<String> = rc.params.iter().map(|p| p.name.clone()).collect();
        out.push_str(&format!(
            "export function {name}({{ {} }}: {name}Props): React.ReactElement {{\n",
            param_names.join(", ")
        ));
    }

    for member in &rc.members {
        match member {
            HirReactiveMember::State(s) => {
                let init = emit_hir_expr(&s.init, &state_names);
                out.push_str(&format!(
                    "  const [{}, set_{}] = useState({});\n",
                    s.name, s.name, init
                ));
            }
            HirReactiveMember::Derived(d) => {
                let expr = emit_hir_expr(&d.expr, &state_names);
                let deps = extract_state_deps(&d.expr, &state_names);
                let dep_str = deps.join(", ");
                out.push_str(&format!(
                    "  const {} = useMemo(() => {}, [{}]);\n",
                    d.name, expr, dep_str
                ));
            }
            HirReactiveMember::Effect(e) => {
                let stmts_str = emit_block_stmts(&e.body, &state_names, 2);
                let deps = extract_state_deps(&e.body, &state_names);
                let dep_str = deps.join(", ");
                out.push_str(&format!(
                    "  useEffect(() => {{\n{}  }}, [{}]);\n",
                    stmts_str, dep_str
                ));
            }
            HirReactiveMember::OnMount(m) => {
                let stmts_str = emit_block_stmts(&m.body, &state_names, 2);
                out.push_str(&format!("  useEffect(() => {{\n{}  }}, []);\n", stmts_str));
            }
            HirReactiveMember::OnCleanup(c) => {
                let stmts_str = emit_block_stmts(&c.body, &state_names, 2);
                out.push_str(&format!(
                    "  useEffect(() => () => {{\n{}  }}, []);\n",
                    stmts_str
                ));
            }
        }
    }

    if let Some(view) = &rc.view {
        let view_js = emit_hir_expr(view, &state_names);
        out.push_str(&format!("  return (\n    {}\n  );\n", view_js));
    }

    out.push_str("}\n");
    (filename, out)
}
