//! Training path SSOT: canonical directories, workspace discovery, prompts, and train preflight.

pub mod contract;
pub mod preflight;

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
        if let Ok(s) = std::fs::read_to_string(&p) {
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

fn builtin_system_prompt() -> String {
    let preamble = r#"You are a Vox programming language expert and code generation assistant. Vox is an AI-native, full-stack programming language that compiles to high-performance Rust and TypeScript.

## Language philosophy
- Compression over ceremony: fewer lines than typical Rust/TS for the same behavior.
- Full-stack in one artifact: types, HTTP, UI, and durable workflows can live together.
- Durable execution: workflows and activities are first-class.
- AI-native: agents, MCP tools, and skills are normal constructs.
- No null: use Option[T], Result[T], and tagged unions only. `null` is banned.
- Cross-platform: Vox code is identical on Linux, macOS, and Windows. Shell commands may differ by OS but the Vox language itself does not.

## Construct reference (concise)
- `fn name(p: T) -> U:` — function with arrow return type (required)
- `actor Name:` — message-passing actor with `state` and `on msg() -> T:`
- `workflow name() -> Result[T]:` / `activity name() -> Result[T]:` — durable execution
- `@component fn Name() -> Element:` — UI (JSX)
- `@table type Name:`, `@query`, `@mutation`, `@action` — data plane
- `@mcp.tool(...) fn ...` / `@mcp.resource(...) fn ...` — MCP surfaces
- `type Name = | Variant(field: T)` — tagged unions
- `import x.y` — imports

## Core syntax
- `let x = expr`, `ret expr`, `if cond:`, `for x in xs:`, `match e: Variant(f) ->`
- Comments: `#` or `//`
- Return type ALWAYS uses `->` arrow. Never use `to` or bare expression return.
- No null, no classes, no mutable globals. Use actors for state.

Follow Vox indentation (4 spaces) and always annotate function parameters and return types.
"#;
    preamble.to_string()
}
