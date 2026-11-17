//! Training path SSOT: canonical directories, workspace discovery, prompts, and train preflight.

pub mod adapter_registry;
pub mod contract;
pub mod curriculum;
pub mod mix_prepare;
pub mod preflight;
pub mod spoke_roundtrip;

pub use contract::{
    OracleReceipt, OrganicSpokeKind, RecordProvenance, SpokeKind, TrainingRecordV2,
    find_workspace_root, normalize_training_data_dir, normalize_training_resume_path,
    normalize_workspace_relative_path, resolve_from_workspace,
};
pub use curriculum::{
    ChosenRejectedPairV2, DEFAULT_MIN_SPOKE_WEIGHT_FLOOR, MensAdapterRegistryEntryV1,
    MensCheckpointManifestV1, PreflightSafetyVerdict, SpokeCurriculumWeights,
    TrainingBatchManifestV1, build_adapter_registry_entry, compute_curriculum_weights,
    compute_curriculum_weights_with_velocity_boost, compute_eight_spoke_curriculum_weights,
    prepare_training_batch, run_deterministic_qlora_dpo_step, sample_curriculum_records,
    validate_snapshot_preflight_gate,
};
#[cfg(feature = "database")]
pub use curriculum::{load_checkpoint_from_db, persist_checkpoint_to_db};
pub use spoke_roundtrip::{
    AgentHarnessSpec, McpToolSpec, MensSpokeScorecardV1, OrganicSearchSpokeCase,
    OrganicSearchSpokeReceipt, OrganicSearchSpokeScorecardV1, OrganicSpokeScorecardEntry,
    PopuliMeshDispatchEnvelope, SpokeEvalTask, SpokeRoundtripCase, SpokeRoundtripReceipt,
    SpokeScorecardEntry, TaskPlanEnvelope, canonical_five_spoke_benchmark_suite_25,
    canonical_five_spoke_golden_suite, canonical_organic_search_to_compilation_suite_15,
    canonical_organic_search_to_compilation_suite_24, evaluate_multi_spoke_suite,
    evaluate_organic_search_spoke_case, evaluate_organic_search_spoke_suite, evaluate_spoke_case,
    strip_code_fences, strip_filename_hints, verify_agent_harness_roundtrip, verify_mcp_roundtrip,
    verify_organic_benchmark_decontamination, verify_organic_scorecard_non_decreasing_trend,
    verify_populi_mesh_roundtrip, verify_rag_context_grounding_and_distractor_resilience,
    verify_rust_roundtrip, verify_rust_snippet_with_rustc, verify_rust_workspace_roundtrip,
    verify_shell_roundtrip, verify_skill_creation_roundtrip, verify_speech_to_code_roundtrip,
    verify_structured_shell_roundtrip, verify_voxlang_roundtrip,
};
#[cfg(feature = "database")]
pub use spoke_roundtrip::{
    list_organic_scorecard_history_from_db, load_organic_scorecard_from_db,
    persist_organic_scorecard_to_db, record_organic_scorecard_telemetry_to_db,
};

/// Default directory for merged `train.jsonl` (matches corpus merge output).
pub const CANONICAL_TRAIN_DATA_DIR: &str = "target/dogfood";

/// Monotonic UTC timestamp suitable for run ids and log names.
pub fn timestamp_string() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

/// Full Vox expert system prompt: prefers `scripts/vox_system_prompt.txt` or `mens/config/system_prompt.txt` at workspace root, else built-in SSOT text.
pub fn generate_system_prompt() -> String {
    if let Some(root) = contract::find_workspace_root() {
        for rel in [
            "scripts/vox_system_prompt.txt",
            "mens/config/system_prompt.txt",
        ] {
            let p = root.join(rel);
            if let Ok(s) = vox_bounded_fs::read_utf8_path_capped(&p) {
                let t = s.trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            }
        }
    }
    builtin_system_prompt()
}

/// Wraps [`builtin_system_prompt`] with explicit fine-tuning guidance for ChatML-style datasets.
///
/// Uses [`builtin_system_prompt`] (~350 tokens) rather than [`generate_system_prompt`]
/// (~2,200 tokens from `mens/config/system_prompt.txt`) so the per-row prompt does not
/// consume 95% of `seq_len` or collapse `supervised_ratio` below the 10% gate floor.
pub fn generate_training_system_prompt() -> String {
    format!(
        "{}\n\n{}",
        builtin_system_prompt(),
        "## Fine-tuning mode\n\
         You are being trained to emit **valid Vox** that passes parse and typecheck.\n\
         Prefer complete programs, explicit types on `fn` signatures, and `Result[T]` for fallible work.\n\
         Never emit `null`; use `Option`, `Result`, or tagged unions.\n"
    )
}

/// Concise built-in prompt. Kept short on purpose: it is prepended to every
/// training row, so its tokens come out of `seq_len`. The long-form reference is
/// `mens/config/system_prompt.txt` (guarded by `mens_system_prompt_syntax_test`);
/// this text must stay consistent with it.
pub fn builtin_system_prompt() -> String {
    let preamble = r#"You are a Vox language expert. Vox compiles to Rust and TypeScript.

## Rules
- Brace blocks, no semicolons, `//` comments. Return types use `to`: `fn name(p: T) to U { ... }` (no `->` except `state_machine`).
- No null: `Option[T]`, `Result[T]`, `Result[T, E]`, tagged unions.
- `let x = e`, `let mut x = e`, `if c { } else { }`, `while c { }`, `for x in xs { }`, `for i in range(0, n) { }` (no `0..n`), `match e { V(f) => body }`.
- Ops: `and`, `or`, `not`, `is`, `is not`, `/`, `%` (no `&`, `|`, `<<`, `>>`, or `x in xs`).
- Lists: `list[T]`, `xs.len()`, `xs = xs.push(v)`, `xs.remove_at(i)`, `xs.reversed()`, `xs.contains(v)` (no `xs[a..b]` or `(A, B)` types; use `type P = | P(k: str, n: int)`). `xs[i]` is `Option[T]`: `xs[i].unwrap()` or `match xs[i] { Some(v) => { } None => { } }`.
- Strings: 1-char `str` (no `char`), `s.len()`, `s.chars()`, `s.slice(i, i + 1)`, `c.ord()`, `chr(n)`, `str(n)`, `s.to_int()`, `float(n)`. No `std.math.*` or `std.Set`.

## Declarations (bare keywords)
- `type Name { f: T }`, `type Name = | A | B(f: T)`, `table Name { f: T }`
- `query name(p: T) to U { }`, `mutation name(p: T) to U { }`, `server name(p: T) to U { }` (use `db.Name.*` + params)
- `tool "name: desc" name(p: T) to U { }`, `resource "uri" "desc" name() to U { }`
- `component Name() { state x: int = 0  view: column() { text() { "{x}" } } }`, `routes { "/" to Home }`
- `actor Name { on h(p: T) to U { } }`, `spawn(Name)`
- `workflow name(p: T) to Result[U] { }` calling `activity name(p: T) to Result[U] { }` with `with { retries: 3, timeout: "30s" }`
- `state_machine Name { state A state B on Ev() from A -> B }`, `import mod.name`, `@test fn name() to Unit { assert(c) }` (define helpers)

## Decorators
`@test`, `@pure`, `@uses(net)`, `@scheduled("1h")`, `@auth(scheme: bearer)`, `@deprecated`, `@durable`.
Rejected: `@endpoint`, `@component fn`, `@table`, `@query fn`, `@mutation fn`, `@server fn`, `@mcp.tool`, `ret`.
"#;
    preamble.to_string()
}

/// Heuristic for curriculum learning difficulty (1-10).
pub fn construct_difficulty(category: &str, record_type: &str) -> u8 {
    match record_type {
        "cli" => 3,
        "tool_call" | "tool_trace" => 5,
        "workflow" | "chatml_trace" | "multi_turn_session" => 10,
        "actor" => 8,
        "skill" => 7,
        "a2a" | "a2a_trace" => 6,
        _ => match category {
            "boilerplate" => 2,
            "basic_syntax" => 3,
            "complex_logic" => 9,
            _ => 5,
        },
    }
}

#[cfg(test)]
#[test]
fn builtin_system_prompt_uses_current_vox_syntax() {
    let b = builtin_system_prompt();
    assert!(
        b.contains("fn name(p: T) to U {"),
        "must show `to` return syntax"
    );
    assert!(
        !b.contains("Never use `to`"),
        "must not forbid `to` returns"
    );
    assert!(!b.contains(") -> "), "must not show `->` as a return arrow");
    assert!(
        !b.contains("Name:`"),
        "must not show colon-block declarations"
    );
    // Prepended to every training row; keep it well under a 1024-token window.
    assert!(b.len() < 2600, "builtin prompt grew to {} chars", b.len());
}

#[cfg(test)]
#[test]
fn training_system_prompt_uses_concise_builtin_for_supervised_ratio() {
    let t = generate_training_system_prompt();
    let b = builtin_system_prompt();
    assert!(
        t.starts_with(&b),
        "training system prompt must wrap concise builtin_system_prompt(), not the 8.6KB reference file"
    );
    assert!(
        t.len() < 2200,
        "training system prompt grew to {} chars (bloats prompt tokens and collapses supervised_ratio < 10%)",
        t.len()
    );
}
