//! Merged integration tests: one binary instead of one per file (each binary is a separate link).
//! Test names keep their file as a module prefix: it::<file>::<test>.
mod agents_md_grammar_section_test;
mod ast_decl_lints_pure_test;
mod audit_rule_collision;
mod bug_a_match_arms_repro;
mod bug_d_imports_repro;
mod durable_promise;
mod fmt_idempotent;
mod forbidden_corpus_test;
mod form_parse_test;
mod format_round_trip;
mod golden_props_test;
mod golden_vox_examples_test;
mod language_surface_coverage_schema_test;
mod llm_fixtures_test;
mod on_stream_lower;
mod placement_public_api;
mod result_match_eval_repro;
mod return_type_inference_test;
mod scaffold_idempotent_test;
mod shell_projection_smoke_test;
mod speech_grammar_artifact_test;
mod with_expression_typecheck_test;
