//! Guardrail: every `examples/golden/**/*.vox` file must parse and lower with no `legacy_ast_nodes`.
//!
//! Goldens are rewritten to match the core recursive-descent grammar (see parser `parse_decl`).

use std::path::{Path, PathBuf};

use vox_compiler::hir::lower_module;
use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;
use vox_compiler::runtime_projection::{
    RUNTIME_PROJECTION_SCHEMA_VERSION, canonical_runtime_projection_bytes, project_runtime_from_hir,
};
use vox_compiler::syntax_k::{
    RepresentabilityPayload, SyntaxKInput, canonical_emitted_files_bytes, canonical_web_ir_bytes,
    enrich_syntax_k_support_metrics, measure_syntax_k_event, sha3_hex,
};
use vox_compiler::web_ir::emit_tsx::emit_component_view_tsx;
use vox_compiler::web_ir::lower::lower_hir_to_web_ir_with_summary;
use vox_compiler::web_ir::validate::{is_advisory_diagnostic, validate_web_ir_with_metrics};

fn syntax_k_output_root() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR")
        && !dir.trim().is_empty()
    {
        return PathBuf::from(dir).join("benchmarks/syntax-k/golden");
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/benchmarks/syntax-k/golden")
        .to_path_buf()
}

fn fixture_id_from(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown_fixture")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn assert_golden_file(path: &Path) {
    let src =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let tokens = lex(&src);
    let module = parse(tokens).unwrap_or_else(|errs| {
        panic!("parse {} failed: {errs:?}", path.display());
    });
    let hir = lower_module(&module);
    assert!(
        hir.legacy_ast_nodes.is_empty(),
        "{}: expected no legacy_ast_nodes after lowering, got {:?}",
        path.display(),
        hir.legacy_ast_nodes
    );

    let fixture_id = fixture_id_from(path);
    let (web_ir, lower_summary) = lower_hir_to_web_ir_with_summary(&hir);
    let (diags, validate_metrics) = validate_web_ir_with_metrics(&web_ir);
    let blocking_diags: Vec<_> = diags.iter().filter(|d| !is_advisory_diagnostic(d)).collect();
    assert!(
        blocking_diags.is_empty(),
        "{fixture_id}: web_ir validate diagnostics: {blocking_diags:?}"
    );

    let web_ir_bytes = canonical_web_ir_bytes(&web_ir)
        .unwrap_or_else(|e| panic!("{fixture_id}: canonical_web_ir_bytes failed: {e}"));
    let source_hash = sha3_hex(src.as_bytes());
    let web_ir_hash = sha3_hex(&web_ir_bytes);
    let hir_ok = hir.legacy_ast_nodes.is_empty();
    let rp = project_runtime_from_hir(&hir);
    let rp_bytes = canonical_runtime_projection_bytes(&rp)
        .unwrap_or_else(|e| panic!("{fixture_id}: runtime projection bytes: {e}"));
    let rp_summary = serde_json::json!({
        "schema_version": RUNTIME_PROJECTION_SCHEMA_VERSION,
        "sha3_hex": sha3_hex(&rp_bytes),
        "db_planning_policy_count": rp.db_planning_policies.len(),
        "has_host_capability_probe": rp.host_capability_probe.is_some(),
        "has_module_task_capability_hints": rp.module_task_capability_hints.is_some(),
    });
    let llm_surface = serde_json::json!({
        "interop_nodes": web_ir.interop_nodes.len(),
        "web_ir_lowering_diagnostics": web_ir.diagnostic_nodes.len(),
    });
    let support_metrics = enrich_syntax_k_support_metrics(
        serde_json::json!({
            "web_ir_lower_summary": {
                "client_route_trees": lower_summary.client_route_trees,
                "http_loader_contracts": lower_summary.http_loader_contracts,
                "server_fn_contracts": lower_summary.server_fn_contracts,
                "query_fn_contracts": lower_summary.query_fn_contracts,
                "mutation_contracts": lower_summary.mutation_contracts,
                "components": lower_summary.components,
                "classic_component_views_lowered": lower_summary.classic_component_views_lowered,
                "classic_components_deferred": lower_summary.classic_components_deferred,
                "style_rules_lowered": lower_summary.style_rules_lowered,
                "dom_expr_fallbacks": lower_summary.dom_expr_fallbacks,
                "lowering_diagnostics": lower_summary.lowering_diagnostics,
                "scheduled_jobs_lowered": lower_summary.scheduled_jobs_lowered
            },
            "web_ir_validate_metrics": {
                "view_roots_walked": validate_metrics.view_roots_walked,
                "dom_nodes_traversed": validate_metrics.dom_nodes_traversed,
                "route_contract_ids_checked": validate_metrics.route_contract_ids_checked,
                "behavior_nodes_checked": validate_metrics.behavior_nodes_checked,
                "style_nodes_checked": validate_metrics.style_nodes_checked,
                "scheduled_jobs_checked": validate_metrics.scheduled_jobs_checked
            },
        }),
        RepresentabilityPayload {
            parse_ok: true,
            hir_ok,
            web_ir_validate_ok: true,
            emit_preview_ok: None,
        },
        Some(llm_surface),
        Some(rp_summary),
    );

    let webir_event = measure_syntax_k_event(SyntaxKInput {
        fixture_id: &fixture_id,
        target_kind: "webir_json",
        bytes: &web_ir_bytes,
        source_hash: Some(&source_hash),
        web_ir_hash: Some(&web_ir_hash),
        baseline_bytes: None,
        support_metrics: Some(support_metrics),
    })
    .unwrap_or_else(|e| panic!("{fixture_id}: measure_syntax_k_event(webir_json) failed: {e}"));

    let mut emitted_files = Vec::<(String, String)>::new();
    for (component_name, _) in &web_ir.view_roots {
        if let Some(tsx) = emit_component_view_tsx(&web_ir, component_name) {
            emitted_files.push((format!("{component_name}.tsx"), tsx));
        }
    }
    let emitted_bytes = canonical_emitted_files_bytes(&emitted_files);
    let emit_support = enrich_syntax_k_support_metrics(
        serde_json::json!({
            "emitted_file_count": emitted_files.len(),
        }),
        RepresentabilityPayload {
            parse_ok: true,
            hir_ok,
            web_ir_validate_ok: true,
            emit_preview_ok: Some(!emitted_files.is_empty()),
        },
        None,
        None,
    );
    let emit_event = measure_syntax_k_event(SyntaxKInput {
        fixture_id: &fixture_id,
        target_kind: "emit_tsx_preview",
        bytes: &emitted_bytes,
        source_hash: Some(&source_hash),
        web_ir_hash: Some(&web_ir_hash),
        baseline_bytes: None,
        support_metrics: Some(emit_support),
    })
    .unwrap_or_else(|e| {
        panic!("{fixture_id}: measure_syntax_k_event(emit_tsx_preview) failed: {e}")
    });

    let artifact = serde_json::json!({
        "schema_version": 1,
        "fixture_id": fixture_id,
        "events": [webir_event, emit_event],
    });
    let out_dir = syntax_k_output_root();
    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| panic!("create_dir_all {} failed: {e}", out_dir.display()));
    let out_path = out_dir.join(format!("{}.json", fixture_id_from(path)));
    let payload = serde_json::to_vec_pretty(&artifact)
        .unwrap_or_else(|e| panic!("serialize artifact {} failed: {e}", out_path.display()));
    std::fs::write(&out_path, payload)
        .unwrap_or_else(|e| panic!("write {} failed: {e}", out_path.display()));
}

fn collect_golden_vox_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let read = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_golden_vox_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("vox") {
            out.push(path);
        }
    }
}

#[test]
fn all_golden_vox_examples_parse_and_lower() {
    let golden_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/golden");
    assert!(golden_dir.is_dir(), "missing {}", golden_dir.display());

    let mut paths = Vec::new();
    collect_golden_vox_files(&golden_dir, &mut paths);
    paths.sort();
    assert!(
        !paths.is_empty(),
        "no .vox files under {}",
        golden_dir.display()
    );

    for path in paths {
        assert_golden_file(&path);
    }
}
