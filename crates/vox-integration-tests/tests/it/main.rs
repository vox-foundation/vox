//! Merged integration tests: one binary instead of one per file (each binary is a separate link).
//! Test names keep their file as a module prefix: it::<file>::<test>.
mod actor_gc_sandbox_test;
mod docker_healthcheck_contract_test;
mod docs_syntax_verify_test;
mod golden_typecheck_gate;
mod mcp_project_init_test;
mod memory_retrieval_envelope_smoke;
mod mens_system_prompt_syntax_test;
mod metamorphic_axiom_test;
mod orchestrator_bootstrap_surface_parity_test;
mod repo_shared_ops_lifecycle_test;
mod repository_ssot_test;
mod scalar_mapping_ssot_test;
mod skill_mcp_permissions_test;
mod skill_mcp_sandbox_test;
