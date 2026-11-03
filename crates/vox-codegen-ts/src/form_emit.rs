//! `@form` declaration → React form component emit (Task C3).
//!
//! Each `form Name { … }` decl emits a self-contained React component into
//! `forms.tsx` with:
//! - `React.useState` for every visible field
//! - client-side validation (required, max_len, min_len)
//! - async submit handler that calls the bound `@mutation` (or `@server`) function
//! - optional router-agnostic redirect on success (`history.pushState` +
//!   `popstate`, so any router that observes `popstate` navigates)
//! - accessible `<label>` / `<input>` pairs with ARIA error associations
//! - a banner error `<div>` for server-side failures

use vox_compiler::hir::HirType;
use vox_compiler::hir::nodes::form::{HirFieldConstraint, HirForm, HirFormField};

/// Emit a single `@form` as a React function component string.
pub fn emit_form(form: &HirForm) -> String {
    let mut out = String::new();
    let name = &form.name;
    let visible: Vec<&HirFormField> = form.fields.iter().filter(|f| !f.hidden).collect();

    out.push_str(&format!("export function {name}() {{\n"));

    // State variables
    for f in &visible {
        let init = field_initial_value(f);
        out.push_str(&format!(
            "  const [{fname}, set_{fname}] = React.useState({init});\n",
            fname = f.name
        ));
    }
    out.push_str("  const [errors, setErrors] = React.useState<Record<string, string>>({});\n");
    out.push_str("  const [submitting, setSubmitting] = React.useState(false);\n");
    out.push_str("  const [bannerError, setBannerError] = React.useState<string | null>(null);\n");
    // Redirect (if any) uses the history API directly — see the submit handler.
    // No router hook, so the form renders under any router (incl. the Vox-emitted
    // dependency-free one) without a provider.

    // Validation function
    out.push_str("  function validate(): Record<string, string> {\n");
    out.push_str("    const e: Record<string, string> = {};\n");
    for f in &visible {
        if f.required {
            let label = f.label.as_deref().unwrap_or(&f.name);
            // Numeric fields hold a `number` (`NaN` when empty); string fields use
            // the `=== ""` empty check.
            let empty_check = if hir_type_to_input_type(&f.ty) == "number" {
                format!("Number.isNaN({n})", n = f.name)
            } else {
                format!("{n} === \"\"", n = f.name)
            };
            out.push_str(&format!(
                "    if ({n} === undefined || {n} === null || {empty_check}) e.{n} = \"{label} is required\";\n",
                n = f.name
            ));
        }
        for c in &f.constraints {
            match c {
                HirFieldConstraint::MaxLen(n) => {
                    let label = f.label.as_deref().unwrap_or(&f.name);
                    out.push_str(&format!(
                        "    if (typeof {fname} === \"string\" && {fname}.length > {n}) e.{fname} = \"{label} too long (max {n})\";\n",
                        fname = f.name
                    ));
                }
                HirFieldConstraint::MinLen(n) => {
                    let label = f.label.as_deref().unwrap_or(&f.name);
                    out.push_str(&format!(
                        "    if (typeof {fname} === \"string\" && {fname}.length < {n}) e.{fname} = \"{label} too short (min {n})\";\n",
                        fname = f.name
                    ));
                }
                HirFieldConstraint::Range(HirExpr::IntLit(lo, _), HirExpr::IntLit(hi, _)) => {
                    let label = f.label.as_deref().unwrap_or(&f.name);
                    out.push_str(&format!(
                        "    if (typeof {fname} === \"number\" && !Number.isNaN({fname}) && ({fname} < {lo} || {fname} > {hi})) e.{fname} = \"{label} must be between {lo} and {hi}\";\n",
                        fname = f.name
                    ));
                }
                HirFieldConstraint::MutuallyExclusiveGroup(grp) => {
                    for peer in &visible {
                        if peer.name != f.name
                            && peer.constraints.iter().any(|pc| {
                                matches!(pc, HirFieldConstraint::MutuallyExclusiveGroup(g) if g == grp)
                            })
                        {
                            out.push_str(&format!(
                                "    if ({fname} && {peer_name}) e.{fname} = \"Mutually exclusive with {peer_name} (group: {grp})\";\n",
                                fname = f.name,
                                peer_name = peer.name
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out.push_str("    return e;\n  }\n");

    // Submit handler
    let submit_fn = form.on_submit.as_deref().unwrap_or("_noSubmit");
    let fields: Vec<String> = visible
        .iter()
        .map(|f| {
            // Numeric state initializes to NaN so an untouched optional field
            // doesn't submit 0; coerce NaN → undefined at the call boundary.
            if hir_type_to_input_type(&f.ty) == "number" {
                format!("{n}: Number.isNaN({n}) ? undefined : {n}", n = f.name)
            } else {
                f.name.clone()
            }
        })
        .collect();
    let submit_call_args = if fields.is_empty() {
        "{}".to_string()
    } else {
        format!("{{ {} }}", fields.join(", "))
    };
    let err_msg = form
        .error_message
        .as_ref()
        .map(|m| format!("\"{}\"", m.replace('"', "\\\"")))
        .unwrap_or_else(|| "String(err)".into());

    out.push_str(&format!(
        "  const onSubmit = async (ev: React.FormEvent) => {{\n\
         \x20   ev.preventDefault();\n\
         \x20   const errs = validate();\n\
         \x20   setErrors(errs);\n\
         \x20   if (Object.keys(errs).length > 0) return;\n\
         \x20   setSubmitting(true);\n\
         \x20   setBannerError(null);\n\
         \x20   try {{\n\
         \x20     await {submit_fn}({submit_call_args});\n"
    ));
    if let Some(r) = &form.success_redirect {
        // Router-agnostic SPA navigation: pushState + popstate notifies TanStack Router /
        // React Router listeners without triggering a full-document reload.
        out.push_str(&format!(
            "      window.history.pushState(null, \"\", \"{r}\");\n      window.dispatchEvent(new PopStateEvent(\"popstate\"));\n"
        ));
    }
    out.push_str(&format!(
        "    }} catch (err) {{\n\
         \x20     setBannerError({err_msg});\n\
         \x20   }} finally {{\n\
         \x20     setSubmitting(false);\n\
         \x20   }}\n\
         \x20 }};\n"
    ));

    // Render
    out.push_str("  return (\n    <form onSubmit={onSubmit} className=\"vox-form\">\n");
    out.push_str("      {bannerError && <div role=\"alert\" className=\"vox-form-error-banner\">{bannerError}</div>}\n");

    for f in &visible {
        let label = f.label.as_deref().unwrap_or(&f.name);
        let req_marker = if f.required { " *" } else { "" };
        if let Some(variants) = enum_variants(f) {
            let options: String = variants
                .iter()
                .map(|v| format!("<option value=\"{v}\">{v}</option>"))
                .collect();
            out.push_str(&format!(
                "      <label className=\"vox-form-field\">\n\
                 \x20       <span>{label}{req_marker}</span>\n\
                 \x20       <select value={{{fname} ?? \"\"}} onChange={{e => set_{fname}(e.target.value)}} aria-invalid={{!!errors.{fname}}}>\n\
                 \x20         <option value=\"\"></option>{options}\n\
                 \x20       </select>\n\
                 \x20       {{errors.{fname} && <span id=\"{fname}-error\" role=\"alert\" className=\"vox-form-error\">{{errors.{fname}}}</span>}}\n\
                 \x20     </label>\n",
                fname = f.name));
            continue;
        }
        let file_accept = f.constraints.iter().find_map(|c| match c {
            HirFieldConstraint::FileKind(k) => Some(k.as_str()),
            _ => None,
        });
        let input_type = if file_accept.is_some() {
            "file"
        } else {
            hir_type_to_input_type(&f.ty)
        };
        let value_prop = match input_type {
            "checkbox" => "checked",
            "number" => "valueAsNumber",
            _ => "value",
        };
        let mut extra_attrs = String::new();
        let mut unit_badge = String::new();
        let mut cost_badge = String::new();
        for c in &f.constraints {
            match c {
                HirFieldConstraint::MaxLen(n) => {
                    extra_attrs.push_str(&format!(" maxLength={{{n}}}"))
                }
                HirFieldConstraint::Range(HirExpr::IntLit(lo, _), HirExpr::IntLit(hi, _)) => {
                    extra_attrs.push_str(&format!(" min={{{lo}}} max={{{hi}}}"));
                }
                HirFieldConstraint::Step(s) => extra_attrs.push_str(&format!(" step={{{s}}}")),
                HirFieldConstraint::FileKind(k) => {
                    extra_attrs.push_str(&format!(" accept=\"{k}\""))
                }
                HirFieldConstraint::Unit(u) => {
                    unit_badge = format!(" <span className=\"vox-form-unit\">{u}</span>");
                }
                HirFieldConstraint::CostCredits(credits) => {
                    cost_badge = format!(
                        " <span data-testid=\"vox-form-cost-credits\" className=\"vox-form-cost\">{credits} cr</span>"
                    );
                }
                _ => {}
            }
        }

        let bind_attr = if input_type == "checkbox" {
            format!("checked={{{fname}}}", fname = f.name)
        } else if input_type == "number" {
            format!(
                "value={{Number.isNaN({fname}) ? \"\" : {fname}}}",
                fname = f.name
            )
        } else if input_type == "file" {
            String::new()
        } else {
            format!("value={{{fname} ?? \"\"}}", fname = f.name)
        };

        out.push_str(&format!(
            "      <label className=\"vox-form-field\">\n\
             \x20       <span>{label}{req_marker}{unit_badge}{cost_badge}</span>\n\
             \x20       <input type=\"{input_type}\" {bind_attr} onChange={{e => set_{fname}(e.target.{value_prop})}}{extra_attrs} aria-invalid={{!!errors.{fname}}} aria-describedby=\"{fname}-error\" />\n\
             \x20       {{errors.{fname} && <span id=\"{fname}-error\" role=\"alert\" className=\"vox-form-error\">{{errors.{fname}}}</span>}}\n\
             \x20     </label>\n",
            fname = f.name
        ));
    }

    out.push_str("      <button type=\"submit\" disabled={submitting}>{submitting ? \"Saving\u{2026}\" : \"Submit\"}</button>\n    </form>\n  );\n}\n");
    out
}

/// Emit a React `<ToolForm_<name> />` island for a `.vox` `@tool` declaration (`POLYTOOL-04`)
/// with preset profile selector, live CLI `argv` preview (`data-testid="tool-form-argv-preview"`),
/// and streaming execution state.
pub fn emit_tool_form(tool: &vox_compiler::hir::HirMcpTool, form: Option<&HirForm>) -> String {
    let tool_name = &tool.func.name;
    let cli_subcmd = tool_name.replace('_', "-");
    let mut out = String::new();
    out.push_str(&format!("export function ToolForm_{tool_name}() {{\n"));
    for p in &tool.func.params {
        out.push_str(&format!(
            "  const [{pname}, set_{pname}] = React.useState(\"\");\n",
            pname = p.name
        ));
    }
    out.push_str("  const [preset, setPreset] = React.useState(\"default\");\n");
    out.push_str("  const [running, setRunning] = React.useState(false);\n");

    let argv_parts: Vec<String> = tool
        .func
        .params
        .iter()
        .map(|p| {
            let flag = p.name.replace('_', "-");
            format!(
                "({pname} ? \" --{flag} \" + String({pname}) : \"\")",
                pname = p.name
            )
        })
        .collect();
    let argv_expr = if argv_parts.is_empty() {
        format!("\"{cli_subcmd}\"")
    } else {
        format!("\"{cli_subcmd}\" + {}", argv_parts.join(" + "))
    };
    out.push_str(&format!("  const argvPreview = {argv_expr};\n"));

    let mut presets_jsx = String::new();
    if let Some(f) = form {
        for field in &f.fields {
            for c in &field.constraints {
                if let HirFieldConstraint::Preset(preset_name, expr) = c {
                    let val_lit = match expr {
                        HirExpr::IntLit(v, _) => v.to_string(),
                        HirExpr::FloatLit(v, _) => v.to_string(),
                        HirExpr::StringLit(s, _) => format!("\"{s}\""),
                        _ => "\"\"".to_string(),
                    };
                    presets_jsx.push_str(&format!(
                        "        <button type=\"button\" data-testid=\"tool-form-preset-{preset_name}\" onClick={{() => {{ setPreset(\"{preset_name}\"); set_{fname}(String({val_lit})); }}}}>{preset_name}</button>\n",
                        fname = field.name
                    ));
                }
            }
        }
    }

    out.push_str("  return (\n    <div className=\"vox-tool-form\">\n");
    if !presets_jsx.is_empty() {
        out.push_str(
            "      <div className=\"vox-tool-presets\" data-testid=\"tool-form-presets\">\n",
        );
        out.push_str(&presets_jsx);
        out.push_str("      </div>\n");
    }
    out.push_str("      <code data-testid=\"tool-form-argv-preview\" className=\"vox-tool-argv\">{argvPreview}</code>\n");
    if let Some(f) = form {
        out.push_str(&format!("      <{fname} />\n", fname = f.name));
    }
    out.push_str("    </div>\n  );\n}\n");
    out
}

use vox_compiler::hir::HirExpr;

fn enum_variants(f: &HirFormField) -> Option<Vec<String>> {
    f.constraints.iter().find_map(|c| match c {
        HirFieldConstraint::Enum(exprs) => Some(
            exprs
                .iter()
                .filter_map(|e| match e {
                    HirExpr::StringLit(s, _) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    })
}

fn hir_type_to_input_type(ty: &HirType) -> &'static str {
    match ty {
        HirType::Named(t) if t == "int" || t == "float" || t == "decimal" => "number",
        HirType::Named(t) if t == "bool" => "checkbox",
        HirType::Named(t) if t == "timestamp" => "datetime-local",
        HirType::Named(t) if t == "email" => "email",
        HirType::Named(t) if t == "url" => "url",
        HirType::Named(t) if t == "phone" => "tel",
        _ => "text",
    }
}

fn field_initial_value(f: &HirFormField) -> &'static str {
    match &f.ty {
        // Initialize numeric fields to NaN (mirrors empty <input type="number">
        // e.target.valueAsNumber) so required check catches untouched inputs
        // and optional inputs don't silently submit 0.
        HirType::Named(t) if t == "int" || t == "float" || t == "decimal" => "NaN",
        HirType::Named(t) if t == "bool" => "false",
        _ => "\"\"",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_compiler::ast::span::Span; // re-exported path used across vox-codegen-ts; NOT vox_ast (design §5b.1)
    use vox_compiler::hir::HirType;
    use vox_compiler::hir::nodes::form::{HirForm, HirFormField};

    fn field(name: &str, ty: HirType) -> HirFormField {
        HirFormField {
            name: name.into(),
            ty,
            label: None,
            required: false,
            hidden: false,
            default: None,
            constraints: vec![],
            span: Span::new(0, 0),
        }
    }
    fn form_with(fields: Vec<HirFormField>) -> HirForm {
        HirForm {
            name: "T".into(),
            fields,
            on_submit: None,
            success_redirect: None,
            error_message: None,
            span: Span::new(0, 0),
        }
    }

    #[test]
    fn branded_scalars_render_typed_inputs() {
        let out = emit_form(&form_with(vec![
            field("e", HirType::Named("email".into())),
            field("u", HirType::Named("url".into())),
            field("p", HirType::Named("phone".into())),
        ]));
        assert!(out.contains("type=\"email\""), "email:\n{out}");
        assert!(out.contains("type=\"url\""), "url:\n{out}");
        assert!(out.contains("type=\"tel\""), "tel:\n{out}");
    }

    #[test]
    fn enum_constraint_renders_select() {
        use vox_compiler::hir::HirExpr;
        use vox_compiler::hir::nodes::form::HirFieldConstraint;
        let mut f = field("role", HirType::Named("Role".into()));
        f.constraints = vec![HirFieldConstraint::Enum(vec![
            HirExpr::StringLit("admin".into(), Span::new(0, 0)),
            HirExpr::StringLit("user".into(), Span::new(0, 0)),
        ])];
        let out = emit_form(&form_with(vec![f]));
        assert!(out.contains("<select"), "select:\n{out}");
        assert!(out.contains(">admin<"), "admin opt:\n{out}");
        assert!(out.contains(">user<"), "user opt:\n{out}");
    }

    #[test]
    fn parameter_facets_and_tool_form_island_emit_argv_and_presets() {
        use vox_compiler::hir::HirExpr;
        use vox_compiler::hir::nodes::form::HirFieldConstraint;

        let mut crf = field("crf", HirType::Named("int".into()));
        crf.constraints = vec![
            HirFieldConstraint::Range(
                HirExpr::IntLit(0, Span::new(0, 0)),
                HirExpr::IntLit(51, Span::new(0, 0)),
            ),
            HirFieldConstraint::Step(1.0),
            HirFieldConstraint::Unit("CRF".into()),
            HirFieldConstraint::CostCredits(5),
            HirFieldConstraint::Preset("fast".into(), HirExpr::IntLit(28, Span::new(0, 0))),
        ];
        let mut mode_a = field("cbr", HirType::Named("str".into()));
        mode_a.constraints = vec![HirFieldConstraint::MutuallyExclusiveGroup("rate".into())];
        let mut mode_b = field("vbr", HirType::Named("str".into()));
        mode_b.constraints = vec![HirFieldConstraint::MutuallyExclusiveGroup("rate".into())];

        let form = form_with(vec![crf, mode_a, mode_b]);
        let form_tsx = emit_form(&form);
        assert!(form_tsx.contains("min={0} max={51}"));
        assert!(form_tsx.contains("step={1}"));
        assert!(form_tsx.contains("vox-form-unit\">CRF<"));
        assert!(form_tsx.contains("vox-form-cost-credits"));
        assert!(form_tsx.contains("Mutually exclusive with vbr (group: rate)"));

        let src = r#"
tool "Compress video"
compress_video(crf: int, cbr: str) to str {
    return cbr
}
"#;
        let tokens = vox_compiler::lexer::lex(src);
        let ast = vox_compiler::parser::parse(tokens).unwrap();
        let hir = vox_compiler::hir::lower_module(&ast);
        let tool_tsx = emit_tool_form(&hir.mcp_tools[0], Some(&form));
        assert!(tool_tsx.contains("export function ToolForm_compress_video()"));
        assert!(tool_tsx.contains("data-testid=\"tool-form-argv-preview\""));
        assert!(tool_tsx.contains("data-testid=\"tool-form-preset-fast\""));
    }
}
