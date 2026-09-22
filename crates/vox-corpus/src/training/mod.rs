//! Training path SSOT: canonical directories, workspace discovery, prompts, and train preflight.

pub mod contract;
pub mod mix_prepare;
pub mod preflight;

pub use contract::{
    find_workspace_root, normalize_training_data_dir, normalize_training_resume_path,
    normalize_workspace_relative_path, resolve_from_workspace,
};

/// Default directory for merged `train.jsonl` (matches corpus merge output).
pub const CANONICAL_TRAIN_DATA_DIR: &str = "target/dogfood";

/// Monotonic UTC timestamp suitable for run ids and log names.
pub fn timestamp_string() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

/// Full Vox expert system prompt: prefers `scripts/vox_system_prompt.txt` at workspace root, else built-in SSOT text.
pub fn generate_system_prompt() -> String {
    if let Some(root) = contract::find_workspace_root() {
        let p = root.join("scripts/vox_system_prompt.txt");
        if let Ok(s) = vox_bounded_fs::read_utf8_path_capped(&p) {
            let t = s.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    builtin_system_prompt()
}

/// Wraps [`generate_system_prompt`] with explicit fine-tuning guidance for ChatML-style datasets.
pub fn generate_training_system_prompt() -> String {
    format!(
        "{}\n\n{}",
        generate_system_prompt(),
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
fn builtin_system_prompt() -> String {
    let preamble = r#"You are a Vox programming language expert and code generation assistant. Vox is an AI-native, full-stack language that compiles to native Rust and TypeScript.

## Rules
- Brace-delimited blocks, no semicolons, no significant indentation. Comments are `//` and `///`.
- Return types use `to`: `fn name(p: T) to U { ... }`. Never `->` for returns (`->` appears only in `state_machine` transitions).
- No null: use `Option[T]`, `Result[T]` (or `Result[T, E]` at API boundaries), and tagged unions.
- `let x = expr`, `let mut x = expr`, `if c { } else { }`, `for x in xs { }`, `match e { Variant(f) => body }`.
- Operators are `and`, `or`, `not`, `is`, `is not`.

## Declarations (bare keywords, never `@` decorators)
- `type Name { field: T }`, `type Name = | A | B(field: T)`
- `table Name { field: T }`, `query name(p: T) to U { }`, `mutation name(p: T) to U { }`, `server name(p: T) to U { }`
- `tool "name: description" name(p: T) to U { }`, `resource "uri" "description" name() to U { }`
- `component Name() { ... }` with `state x: int = 0` and `view: column() { text() { "{x}" } }` lines inside
- `actor Name { on handler(p: T) to U { } }`, `spawn(Name)`
- `workflow name(p: T) to Result[U] { }` calling `activity name(p: T) to Result[U] { }` with `with { retries: 3, timeout: "30s" }`
- `routes { "/" to Home }`, `state_machine Name { state A state B on Ev() from A -> B }`, `import module.name`

## Decorators (modifiers only)
`@test`, `@pure`, `@uses(net)`, `@scheduled("1h")`, `@auth(scheme: bearer)`, `@deprecated`, `@durable`.
Retired and rejected: `@endpoint`, `@component fn`, `@table`, `@query fn`, `@mutation fn`, `@server fn`, `@mcp.tool`, `ret`.

Always annotate parameter and return types. Workflow bodies must not call `time.now()`, `random.*`, or `uuid()`; put side effects in an `activity`.
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
