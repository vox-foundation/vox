//! Regression test: `vox init` (and MCP `vox_project_init`) scaffold templates
//! must compile-check with **zero diagnostics** — not just zero errors.
//!
//! `vox doctor --project` (see `every_template_scaffolds_doctor_green` in
//! `cr_l7_new_deploy_doctor_e2e.rs`) only fails on error-severity diagnostics,
//! so it stayed green while every scaffolded `table` declared an explicit
//! `id: int` column and `lint_ast_declarations` (crates/vox-compiler/src/
//! typeck/ast_decl_lints.rs, code `lint.table_id_column`) flagged it as a
//! Warning on every single `vox init && vox check` — a broken-out-of-the-box
//! first impression that no existing test caught.
//!
//! This test runs the same frontend pipeline `vox check` uses
//! (`vox_compiler::pipeline::run_frontend_str`) directly against each
//! scaffolded `src/main.vox` and asserts there are no diagnostics at all.

use vox_compiler::pipeline::run_frontend_str;
use vox_project_scaffold::scaffold_vox_project_at;

fn check_diagnostics(kind: &str, template: Option<&str>) {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold_vox_project_at(tmp.path(), "diag-probe", kind, template)
        .expect("scaffold should succeed");
    let main_path = tmp.path().join("src/main.vox");
    let source = std::fs::read_to_string(&main_path).expect("read scaffolded main.vox");

    let result = run_frontend_str(&source, &main_path.display().to_string())
        .expect("scaffolded template should parse and lower cleanly");

    let messages: Vec<String> = result
        .diagnostics
        .iter()
        .map(|d| {
            format!(
                "{:?} [{}]: {}",
                d.severity,
                d.code.as_deref().unwrap_or("-"),
                d.message
            )
        })
        .collect();
    assert!(
        messages.is_empty(),
        "kind={kind:?} template={template:?} scaffold should compile-check with zero \
         diagnostics; got:\n{}",
        messages.join("\n")
    );
}

#[test]
fn default_application_scaffold_has_zero_diagnostics() {
    check_diagnostics("application", None);
}

#[test]
fn chatbot_template_scaffold_has_zero_diagnostics() {
    check_diagnostics("application", Some("chatbot"));
}

#[test]
fn web_template_scaffold_has_zero_diagnostics() {
    check_diagnostics("application", Some("web"));
}

#[test]
fn api_template_scaffold_has_zero_diagnostics() {
    check_diagnostics("application", Some("api"));
}

#[test]
fn mobile_pwa_template_scaffold_has_zero_diagnostics() {
    check_diagnostics("application", Some("mobile-pwa"));
}
