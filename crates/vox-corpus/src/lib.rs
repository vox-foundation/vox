//! Corpus crate for the Vox workspace: aggregates training data, MCP meta corpora,
//! synthetic-search generators, tool-workflow corpora, and codegen-Vox samples.
//! Used by `vox-ml-cli` and downstream training pipelines.

pub mod arca_replay;
pub mod ast_mutator;
pub mod codegen_vox;
pub mod corpus;
pub mod dataset_snapshot;
pub mod external_review_replay;
pub mod flywheel;
pub mod humaneval_runner;
pub mod research_gen;
pub mod rust_to_vox;
#[cfg(test)]
mod semcov_wave30_tests;
pub mod synthetic_gen;
pub mod synthetic_search_gen;
pub mod tool_workflow_corpus;
pub mod training;

pub use training::{
    AgentHarnessSpec, ChosenRejectedPairV2, McpToolSpec, MensAdapterRegistryEntryV1,
    MensCheckpointManifestV1, MensSpokeScorecardV1, OracleReceipt, PreflightSafetyVerdict,
    RecordProvenance, SpokeCurriculumWeights, SpokeEvalTask, SpokeKind, SpokeRoundtripCase,
    SpokeRoundtripReceipt, SpokeScorecardEntry, TaskPlanEnvelope, TrainingBatchManifestV1,
    TrainingRecordV2, build_adapter_registry_entry, canonical_five_spoke_golden_suite,
    evaluate_multi_spoke_suite, evaluate_spoke_case, prepare_training_batch,
    run_deterministic_qlora_dpo_step, sample_curriculum_records, strip_code_fences,
    validate_snapshot_preflight_gate, verify_agent_harness_roundtrip, verify_mcp_roundtrip,
    verify_rust_roundtrip, verify_rust_workspace_roundtrip, verify_shell_roundtrip,
    verify_structured_shell_roundtrip, verify_voxlang_roundtrip,
};
#[cfg(feature = "database")]
pub use training::{load_checkpoint_from_db, persist_checkpoint_to_db};
