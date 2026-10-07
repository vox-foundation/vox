//! Merged integration tests: one binary instead of one per file (each binary is a separate link).
//! Test names keep their file as a module prefix: it::<file>::<test>.
mod ai_fixture_ts_diagnostic;
mod back_button_test;
mod component_registry_sync;
mod const_emit_test;
mod deep_link_test;
mod deprecated_emit_test;
mod emit_unsupported_expr_test;
mod frontend_backend;
mod frontend_coverage_ledger;
mod if_expr_emit_test;
mod jsx_conditional_emit;
mod list_hof_emit_test;
mod main_boot_hir_roundtrip;
mod on_stream_e2e;
mod on_stream_emit;
mod on_stream_webir;
mod property_tests;
mod push_test;
mod rate_limit_emit_test;
mod regex_string_literal_emit;
mod spawn_workflow_emit_test;
mod tauri_endpoint_client_parity_test;
mod traced_fn_emit;
mod webhook_emit_test;
