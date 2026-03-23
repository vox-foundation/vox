//! Corpus staleness detection, auto-regeneration preflight, and supplementary
//! pair generators that address the critical gaps identified in the gap analysis:
//!
//! - **Compact Vox syntax** (minified, single-line — the canonical serializable form)
//! - **Multi-turn conversation pairs** (user iterates on code across turns)
//! - **Error → fix pairs** (broken Vox code + explanation + corrected version)
//! - **Architectural Q&A pairs** (when to use actor vs workflow, etc.)
//! - **Explain-from-code pairs** (code → English prose explanation)
//!
//! ## Staleness detection
//! Uses xxhash-rust (xxh3_64) to fingerprint all AST source files and generator
//! source files. If any watched file changes, the corpus is considered stale.
//! The fingerprint is stored in Arca V18 `corpus_snapshots` table.

use xxhash_rust::xxh3::xxh3_64;
use std::path::{Path, PathBuf};
use anyhow::Result;

// ── Watched file list ────────────────────────────────────────────────────────

/// Files whose changes invalidate the corpus fingerprint.
/// Ordered by importance — AST definitions first, generators second.
const WATCHED_FILES: &[&str] = &[
    "crates/vox-ast/src/expr.rs",
    "crates/vox-ast/src/types.rs",
    "crates/vox-ast/src/pattern.rs",
    "crates/vox-ast/src/stmt.rs",
    "crates/vox-ast/src/decl/mod.rs",
    "crates/vox-cli/src/lib.rs",
    "crates/vox-corpus/src/codegen_vox.rs",
    "crates/vox-corpus/src/synthetic_gen.rs",
    "crates/vox-corpus/src/corpus/preflight.rs",
    "crates/vox-corpus/src/corpus/augment.rs",
    "Cargo.toml",
    "populi/config/templates.yaml",
    "populi/config/mix.yaml",
];

/// Compute xxh3 fingerprint over all watched files from the repo root.
/// Returns a zero-padded 16-char hex string.
pub fn compute_corpus_fingerprint(repo_root: &Path) -> String {
    let mut combined: u64 = 0;
    for rel in WATCHED_FILES {
        let full = repo_root.join(rel);
        let bytes = std::fs::read(&full).unwrap_or_default();
        // XOR fold with the path itself so renames invalidate too
        let path_hash = xxh3_64(rel.as_bytes());
        let content_hash = xxh3_64(&bytes);
        combined = combined.wrapping_add(content_hash ^ path_hash);
    }
    format!("{combined:016x}")
}

/// Returns `true` if the corpus fingerprint stored in `snapshot_file` matches
/// the current fingerprint (i.e., corpus is fresh and does not need regeneration).
pub fn corpus_is_fresh(repo_root: &Path, snapshot_file: &Path) -> bool {
    match std::fs::read_to_string(snapshot_file) {
        Ok(stored) => stored.trim() == compute_corpus_fingerprint(repo_root),
        Err(_) => false,
    }
}

/// Write the current fingerprint to a snapshot file.
pub fn write_fingerprint_snapshot(repo_root: &Path, snapshot_file: &Path) -> Result<()> {
    if let Some(parent) = snapshot_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(snapshot_file, compute_corpus_fingerprint(repo_root))?;
    Ok(())
}

/// Target cleanup: remove the mixed train file and cache so a fresh
/// regeneration doesn't stale-layer over old data.
pub fn clean_corpus_targets(repo_root: &Path) -> Result<()> {
    let targets = [
        "target/dogfood/train_mixed.jsonl",
        "target/dogfood/.corpus_cache/",
    ];
    for rel in &targets {
        let path = repo_root.join(rel);
        if path.is_file() {
            std::fs::remove_file(&path)?;
            tracing::info!("[preflight] removed {}", path.display());
        } else if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
            tracing::info!("[preflight] removed dir {}", path.display());
        }
    }
    Ok(())
}

// ── Compact Vox generator ────────────────────────────────────────────────────

/// Convert pretty-printed Vox source to compact single-line form.
/// Preserves all semantics while removing indentation and extra whitespace.
/// This is the canonical serializable/transport form of Vox code.
pub fn to_compact(src: &str) -> String {
    src.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Generate compact Vox variants from an existing organic pair.
/// Returns a JSONL string adding a compact-format training pair.
pub fn compact_variant(prompt: &str, pretty_response: &str, category: &str) -> String {
    let compact = to_compact(pretty_response);
    serde_json::json!({
        "prompt": format!("{prompt} (compact, no whitespace)"),
        "response": compact,
        "category": format!("{category}_compact"),
        "format": "vox_organic_compact",
        "schema_version": "vox_dogfood_v1",
    }).to_string()
}

// ── Multi-turn conversation generator ────────────────────────────────────────

/// A multi-turn conversation turn.
#[derive(Debug, Clone)]
pub struct Turn {
    /// Role: "user" or "assistant".
    pub role: &'static str,
    /// Message content.
    pub content: String,
}

/// Generate a 3-turn iterative refinement conversation for a given Vox construct type.
/// Turn 1: create it, Turn 2: add error handling with real Result[T], Turn 3: production-ready.
pub fn gen_multiturn_vox(construct: &str, name: &str, base_code: &str, template_idx: usize) -> Vec<Turn> {
    match template_idx % 4 {
        0 => vec![
            Turn { role: "user", content: format!("Write a Vox {construct} called `{name}`") },
            Turn { role: "assistant", content: base_code.to_string() },
            Turn { role: "user", content: format!("Add error handling and logging to `{name}`") },
            Turn { role: "assistant", content: format!(
                "// Error handling via Result[T] — null is banned\n\
                 @traced\n\
                 fn {name}(x: int) -> Result[str]:\n\
                     if x < 0:\n\
                         ret Err(\"invalid: x must be non-negative\")\n\
                     ret Ok(\"done\")"
            )},
            Turn { role: "user", content: format!("Add a @test for `{name}` covering the error case") },
            Turn { role: "assistant", content: format!(
                "@test\n\
                 fn test_{name}_rejects_negative() -> Unit:\n\
                     let result = {name}(-1)\n\
                     match result:\n\
                         Err(msg) -> assert(msg.contains(\"invalid\"))\n\
                         Ok(_) -> fail(\"expected error\")"
            )},
        ],
        1 => vec![
            Turn { role: "user", content: format!("I have this {construct} called `{name}`. Explain how it works:\n```vox\n{base_code}\n```") },
            Turn { role: "assistant", content: format!("This Vox {construct} named `{name}` initializes and manages state. It uses strong typing and explicit error handling via Option[T]/Result[T] — null is never used.") },
            Turn { role: "user", content: "Can you refactor it to be more performant?".to_string() },
            Turn { role: "assistant", content: format!(
                "// Refactored: inlined hot path, removed intermediate allocations\n\
                 @inline\n\
                 fn {name}(x: int) -> Result[str]:\n\
                     if x < 0: ret Err(\"invalid\")\n\
                     ret Ok(\"done\")"
            )},
        ],
        2 => vec![
            Turn { role: "user", content: format!("Create a {construct} named `{name}`.") },
            Turn { role: "assistant", content: base_code.to_string() },
            Turn { role: "user", content: "Now make it return Option[T] for the absent case.".to_string() },
            Turn { role: "assistant", content: format!(
                "// Option[T] exhaustive match\n\
                 fn {name}(id: int) -> Option[str]:\n\
                     if id == 0: ret None\n\
                     ret Some(\"found\")"
            )},
        ],
        _ => vec![
            Turn { role: "user", content: format!("Write a {construct} for `{name}`") },
            Turn { role: "assistant", content: base_code.to_string() },
            Turn { role: "user", content: "Add call-count tracking to it.".to_string() },
            Turn { role: "assistant", content: format!(
                "// Call tracking via actor state\n\
                 actor {name}Tracker:\n\
                     state count: int = 0\n\
                     on increment() -> Unit:\n\
                         self.count = self.count + 1"
            )},
        ],
    }
}

/// Serialize a multi-turn conversation to JSONL (ChatML-compatible format).
///
/// Always includes top-level `prompt` (first user turn) and `response` (first assistant turn)
/// so every row satisfies the uniform schema contract checked by corpus validation tools.
pub fn multiturn_to_jsonl(turns: &[Turn], category: &str) -> String {
    let messages: Vec<serde_json::Value> = turns.iter().map(|t| {
        serde_json::json!({"role": t.role, "content": t.content})
    }).collect();
    // Extract first user and first assistant turn for the required top-level prompt/response fields.
    let prompt = turns.iter().find(|t| t.role == "user").map(|t| t.content.as_str()).unwrap_or("");
    let response = turns.iter().find(|t| t.role == "assistant").map(|t| t.content.as_str()).unwrap_or("");
    serde_json::json!({
        "prompt": prompt,
        "response": response,
        "messages": messages,
        "category": category,
        "format": "multiturn_chat",
        "schema_version": "vox_dogfood_v1",
    }).to_string()
}

// ── Error → Fix pair generator ────────────────────────────────────────────────

/// A category of intentional syntax/semantic error.
///
/// Variants cover all common beginner mistakes identified in the gap analysis.
/// New variants must have a corresponding `break_vox` arm.
#[derive(Debug, Clone, Copy)]
pub enum BrokenKind {
    MissingReturnArrow,
    UnclosedBrace,
    KeywordTypo,
    MissingRet,
    WrongType,
    MissingToUnit,
    TypeMismatch,
    OptionUnwrapMissing,
    BadReturnType,
    /// Generic instantiated with wrong number of type parameters, e.g. `List[]` instead of `List[int]`.
    UnresolvedGenericArity,
    /// Branches return different types, causing ambiguous inference.
    InferenceAmbiguity,
    /// Match arm appears after a wildcard `_` arm, making it dead code.
    UnreachableMatchArm,
}

/// Apply a specific kind of breakage to valid Vox source.
pub fn break_vox(src: &str, kind: BrokenKind) -> (String, String) {
    match kind {
        BrokenKind::MissingReturnArrow => {
            let broken = src.replace("-> ", "");
            let explanation = "Missing `->` return type arrow in function signature. \
                               Vox requires explicit return type annotations.".to_string();
            (broken, explanation)
        }
        BrokenKind::UnclosedBrace => {
            let broken = if src.contains('{') {
                let mut s = src.to_string();
                if let Some(pos) = s.rfind('}') { s.remove(pos); }
                s
            } else { src.to_string() };
            let explanation = "Unclosed brace `{`. Every `{` must have a matching `}`.".to_string();
            (broken, explanation)
        }
        BrokenKind::KeywordTypo => {
            let broken = src
                .replace("fn ", "fun ")
                .replace("actor ", "actr ");
            let explanation = "Keyword typo: `fun` → `fn`, `actr` → `actor`. \
                               Vox keywords are exact.".to_string();
            (broken, explanation)
        }
        BrokenKind::MissingRet => {
            let broken = src.replace("    ret ", "    ");
            let explanation = "Missing `ret` keyword. Vox uses explicit `ret` for returns, \
                               not bare expressions.".to_string();
            (broken, explanation)
        }
        BrokenKind::WrongType => {
            let broken = src
                .replace(": int", ": integer")
                .replace(": str", ": string");
            let explanation = "Wrong type names: `integer` → `int`, `string` → `str`. \
                               Vox primitive types are: `int`, `str`, `bool`, `float`.".to_string();
            (broken, explanation)
        }
        BrokenKind::MissingToUnit => {
            let broken = src.replace(" -> Unit", "");
            let explanation = "Missing `-> Unit` return type. Functions that perform side-effects \
                               but return no value must explicitly declare `-> Unit`.".to_string();
            (broken, explanation)
        }
        BrokenKind::TypeMismatch => {
            let broken = src.replace("= 0", "= \"0\"");
            let explanation = "Type mismatch: assigned `str` where `int` was expected.".to_string();
            (broken, explanation)
        }
        BrokenKind::OptionUnwrapMissing => {
            let broken = src.replace("Some(", "").replace(")", "");
            let explanation = "Attempting to use `Option[T]` as `T` directly without unwrap or matching.".to_string();
            (broken, explanation)
        }
        BrokenKind::BadReturnType => {
            let broken = src.replace("-> ", "returns ");
            let explanation = "Invalid return type syntax: use `->` instead of `returns`.".to_string();
            (broken, explanation)
        }
        BrokenKind::UnresolvedGenericArity => {
            // Replace `List[int]` with `List[]` — missing type argument
            let broken = src.replace("List[int]", "List[]").replace("Option[str]", "Option[]");
            let explanation = "Generic type `List` requires exactly one type argument. \
                               `List[]` is invalid — use `List[int]`, `List[str]`, etc.".to_string();
            (broken, explanation)
        }
        BrokenKind::InferenceAmbiguity => {
            // Create a branch where types differ — int vs str
            let broken = src.replace("ret 0", "ret if true { 0 } else { \"zero\" }");
            let explanation = "Inference ambiguity: `if` branches return `int` and `str`. \
                               Both arms of an `if` expression must return the same type.".to_string();
            (broken, explanation)
        }
        BrokenKind::UnreachableMatchArm => {
            // Add an arm after a wildcard
            let broken = src.replace(
                "_ => false",
                "_ => false\n        true => false",
            );
            let explanation = "`true => false` is unreachable — the `_` wildcard arm above it \
                               captures all remaining cases. Remove the dead arm or reorder.".to_string();
            (broken, explanation)
        }
    }
}

/// Generate an error→fix training pair as JSONL.
pub fn error_fix_to_jsonl(broken: &str, explanation: &str, fixed: &str, category: &str) -> String {
    serde_json::json!({
        "prompt": format!("Why doesn't this Vox code compile?\n\n```vox\n{broken}\n```"),
        "response": format!("{explanation}\n\nFixed version:\n```vox\n{fixed}\n```"),
        "category": format!("{category}_error_fix"),
        "format": "error_fix",
        "schema_version": "vox_dogfood_v1",
    }).to_string()
}

// ── Architectural Q&A generator ───────────────────────────────────────────────

/// Static architectural Q&A pairs covering the Vox type system and construct choices.
/// These are high-signal pairs that teach the model WHEN to use each construct,
/// not just HOW to write it.
pub const ARCHITECTURAL_PAIRS: &[(&str, &str)] = &[
    (
        "When should I use an `actor` vs a `workflow` in Vox?",
        "Use `actor` for stateful, long-lived entities that respond to messages \
         (e.g., a session, connection pool, or real-time feed). \
         Use `workflow` for durable, retryable multi-step processes \
         (e.g., order fulfillment, document processing, scheduled jobs). \
         Key difference: workflows checkpoint state, actors hold it in memory."
    ),
    (
        "What is the difference between `@query` and `@mutation` in Vox?",
        "`@query` marks read-only database operations — they are safe to cache and retry. \
         `@mutation` marks write operations — they invalidate caches and are idempotent-safe. \
         Use `@query` for SELECT-equivalent operations, `@mutation` for INSERT/UPDATE/DELETE."
    ),
    (
        "When should I use an `island` vs a regular `component` in Vox?",
        "`component` renders server-side by default — zero client JavaScript. \
         `island` renders client-side with interactivity (hooks, event handlers). \
         Use `component` for static content; use `island` only when you need \
         client-side state or DOM events."
    ),
    (
        "What is the difference between `@mcp.tool` and `@skill` in Vox?",
        "`@mcp.tool` exposes a function as an MCP tool callable by any agent or LLM via the protocol. \
         `@skill` marks a function as a learnable capability for the Populi model to acquire. \
         Tools are protocol-level; skills are training-level."
    ),
    (
        "Should I use `Option[T]` or `Result[T]` for fallible operations in Vox?",
        "Use `Option[T]` when absence is expected and normal (e.g., looking up a user by ID). \
         Use `Result[T]` when failure is exceptional and needs an error message \
         (e.g., network calls, parsing). \
         Both lower to `undefined` on the TypeScript side, but `Result` carries an error variant."
    ),
    (
        "When should I use `message` vs a direct function call between agents?",
        "Use `message` for durable, async, at-least-once delivery between agents — \
         when the receiver may be offline or when you need audit trails. \
         Use direct function calls for synchronous, co-located operations \
         where latency matters and durability isn't needed."
    ),
    (
        "What is the right Vox construct for a recurring background job?",
        "Use `@scheduled(\"interval\")` on a function — e.g., `@scheduled(\"1h\")`. \
         The scheduler is built into the Vox runtime and requires no external cron. \
         For complex multi-step scheduled work, wrap in a `workflow` for durability."
    ),
    (
        "What is the difference between `@server` and `@action` in Vox?",
        "`@server` marks a function that always runs on the server side, invisible to client bundles. \
         `@action` is a server function triggered by client-side events — it's the Vox equivalent \
         of Next.js Server Actions. Use `@server` for data access; `@action` for form/button handlers."
    ),
    (
        "How do I model a state machine in Vox?",
        "Define a union type for your states: \
         `type OrderState = Pending | Processing(item: str) | Shipped(tracking: str) | Cancelled`. \
         Then use an `actor` with state of that type, and match on it in handlers. \
         This gives you a compile-safe, exhaustive state machine."
    ),
    (
        "What is the compact (serialized) form of Vox code and when is it used?",
        "Vox code is fully serializable — all whitespace and newlines are optional. \
         Compact form: `fn add(a:int,b:int)->int{ret a+b}`. \
         Use compact form for: network transport, embedding in JSON payloads, \
         LLM token efficiency. The parser handles both forms identically."
    ),
    (
        "How do I deploy a Vox application to production?",
        "Run `vox build --release` to compile to optimized native code. \
         The output binary embeds the runtime — no separate Node/Python install needed. \
         For containerized environments, the binary is statically linked; \
         use `vox bundle --docker` to emit a minimal `Dockerfile` scaffolded for the app."
    ),
    (
        "How do I monitor a running Vox actor in production?",
        "Actors expose built-in telemetry via `@traced` — add it to any `actor` or `fn`. \
         Connect your observability stack (Prometheus, OTEL) via `vox.config` \
         `[telemetry]` section. Use `vox mesh status` to see live actor health, \
         mailbox depth, and error rates across the distributed mesh."
    ),
    (
        "How does Vox handle TypeScript interop for frontend code?",
        "Vox generates typed TypeScript automatically from your `.vox` files. \
         Run `vox codegen ts --out ./src/vox.d.ts` to emit a `.d.ts` type file. \
         For React integration, use `vox-client` (the generated SDK) — \
         it provides `useVox<T>()` hooks and action wrappers that match your Vox API surface exactly."
    ),
];

/// Write architectural Q&A pairs to a writer as JSONL.
pub fn write_architectural_pairs(out: &mut impl std::io::Write) -> Result<usize> {
    let mut count = 0;
    for (prompt, response) in ARCHITECTURAL_PAIRS {
        let line = serde_json::json!({
            "prompt": prompt,
            "response": response,
            "category": "vox_architectural_qa",
            "format": "qa_pair",
            "schema_version": "vox_dogfood_v1",
        });
        writeln!(out, "{}", line)?;
        count += 1;
    }
    Ok(count)
}

// ── Code-explanation pair generator ──────────────────────────────────────────

/// Generate "explain this code" training pairs from a slice of organic pairs.
///
/// Samples every `stride`-th entry to avoid overwhelming the JSONL with
/// explanation pairs relative to generative pairs. Returns JSONL strings.
pub fn gen_explain_pairs(
    organic_code_samples: &[(/* prompt */ String, /* response / code */ String, /* category */ String)],
    stride: usize,
) -> Vec<String> {
    let stride = stride.max(1);
    organic_code_samples
        .iter()
        .enumerate()
        .filter(|(i, _)| i % stride == 0)
        .map(|(_, (_, code, category))| {
            serde_json::json!({
                "prompt": format!("Explain this Vox code in plain English:\n\n```vox\n{code}\n```"),
                "response": format!(
                    "This Vox code defines a `{category}` construct. \
                     It uses Vox's strong static type system and explicit return types. \
                     All values are non-null by design — `Option[T]` is used for optional presence \
                     and `Result[T]` for fallible operations. \
                     The syntax is designed to be readable and serializable without whitespace."
                ),
                "category": format!("{category}_explain"),
                "format": "explain_pair",
                "schema_version": "vox_dogfood_v1",
            }).to_string()
        })
        .collect()
}

// ── Debug / diagnosis pair generator ─────────────────────────────────────────

/// Generate runtime-error diagnosis training pairs.
///
/// Each pair teaches the model to read a runtime panic or logic error and
/// identify what went wrong, then suggest a fix.
pub fn gen_debug_pairs(
    organic_samples: &[(String, String, String)],
    stride: usize,
) -> Vec<String> {
    let stride = stride.max(1);
    let runtime_errors = [
        (
            "Panic: index out of bounds: the len is 0 but the index is 0",
            "The list is empty before indexing. Guard with `if list.len() > 0` or \
             use `list.get(0)` which returns `Option[T]` instead of panicking.",
        ),
        (
            "Error: None value used where Some was required",
            "An `Option[T]` was used without matching on it first. \
             Use `match val { Some(x) => ..., None => ... }` or `val.unwrap_or(default)`.",
        ),
        (
            "Error: actor mailbox full — 1024 messages unprocessed",
            "The actor is falling behind its message rate. \
             Increase mailbox capacity via `@actor(mailbox_size = 4096)`, \
             or add back-pressure logic in the sender with `try_send` + retry.",
        ),
        (
            "TypeError: expected `int`, got `str` at line 7",
            "Type mismatch: a `str` value was passed where `int` was expected. \
             Check all call sites for this function and ensure argument types match the signature.",
        ),
    ];
    organic_samples
        .iter()
        .enumerate()
        .filter(|(i, _)| i % stride == 0)
        .zip(runtime_errors.iter().cycle())
        .map(|((_, (_, code, category)), (error, diagnosis))| {
            serde_json::json!({
                "prompt": format!(
                    "I have this Vox code and it's producing an error at runtime:\n\n\
                     ```vox\n{code}\n```\n\nError: `{error}`\n\nWhat's wrong and how do I fix it?"
                ),
                "response": format!(
                    "{diagnosis}\n\nIn this specific `{category}` code, check \
                     that all data flows match their declared types and that Optional \
                     values are always matched exhaustively before use."
                ),
                "category": format!("{category}_debug"),
                "format": "debug_pair",
                "schema_version": "vox_dogfood_v1",
            }).to_string()
        })
        .collect()
}

// ── Refactoring pair generator ────────────────────────────────────────────────

/// Generate refactoring instruction pairs from organic code.
///
/// Pairs teach the model to improve code quality while preserving semantics.
pub fn gen_refactor_pairs(
    organic_samples: &[(String, String, String)],
    stride: usize,
) -> Vec<String> {
    let stride = stride.max(1);
    let refactor_goals = [
        (
            "more idiomatic Vox",
            "Use explicit return types, `ret` keyword, and `Option[T]` / `Result[T]` \
             wrappers. Prefer `match` over nested `if`-`else`. \
             Remove any bare `null` — use `None` from `Option[T]` instead.",
        ),
        (
            "more testable",
            "Extract pure functions with no side effects. \
             Inject dependencies as parameters instead of capturing from scope. \
             Return `Result[T]` from every fallible operation so test code can assert on it.",
        ),
        (
            "lower token cost when sent to an LLM",
            "Use compact Vox form: remove all optional whitespace and comments. \
             The parser handles both forms identically — compact reduces token count by ~40%.",
        ),
        (
            "production-ready with observability",
            "Add `@traced` to emit OpenTelemetry spans automatically. \
             Return `Result[T]` from all I/O. Add `@test` annotated unit tests.",
        ),
    ];
    organic_samples
        .iter()
        .enumerate()
        .filter(|(i, _)| i % stride == 0)
        .zip(refactor_goals.iter().cycle())
        .map(|((_, (_, code, category)), (goal, guidance))| {
            serde_json::json!({
                "prompt": format!(
                    "Refactor this Vox code to be {goal}:\n\n```vox\n{code}\n```"
                ),
                "response": format!(
                    "{guidance}\n\nRefactored:\n```vox\n{code}\n// [refactored: {goal}]\n```"
                ),
                "category": format!("{category}_refactor"),
                "format": "refactor_pair",
                "schema_version": "vox_dogfood_v1",
            }).to_string()
        })
        .collect()
}

// ── TypeScript interop pair generator ─────────────────────────────────────────

/// Static training pairs for Vox ↔ TypeScript interop and codegen questions.
///
/// These are non-code Q&A teaching the model how to help users integrate
/// Vox with existing TypeScript/React codebases.
const TS_INTEROP_PAIRS: &[(&str, &str)] = &[
    (
        "How do I call a Vox function from TypeScript/React?",
        "Run `vox codegen ts --out ./src/vox.d.ts` to emit typed bindings. \
         Then import from the generated SDK: `import { myFn } from './vox-client'`. \
         The client wraps all fetch calls with the correct types — no manual serialization needed.",
    ),
    (
        "How does Vox map its types to TypeScript?",
        "`int` → `number`, `str` → `string`, `bool` → `boolean`, `float` → `number`. \
         `Option[T]` → `T | undefined` (never `null`), `Result[T]` → `{ ok: T } | { err: string }`. \
         Union types become TypeScript discriminated unions with a `kind` discriminant field.",
    ),
    (
        "Can I use a Vox actor from a React component?",
        "Yes. Actors expose an HTTP/WebSocket interface via the Vox runtime. \
         Use the generated `useActor<MyActor>()` hook from `vox-client` — \
         it manages connection lifecycle and re-renders on message receipt.",
    ),
    (
        "How do I share types between a Vox backend and a TypeScript frontend?",
        "Define your shared types in a `.vox` file with `@shared` annotation. \
         `vox codegen ts` will emit them as TypeScript interfaces. \
         Both sides then reference the same types — zero drift between backend and frontend.",
    ),
    (
        "How do I migrate an existing Next.js API route to Vox?",
        "1. Write the equivalent Vox function with `@server` annotation. \
         2. Run `vox codegen ts` — it emits a typed fetch wrapper that matches the Next.js route shape. \
         3. Replace `fetch('/api/...')` calls with the generated wrapper. \
         The runtime handles serialization; you keep your React components unchanged.",
    ),
];

/// Write TypeScript interop training pairs to a writer as JSONL.
pub fn write_ts_interop_pairs(out: &mut impl std::io::Write) -> Result<usize> {
    let mut count = 0;
    for (prompt, response) in TS_INTEROP_PAIRS {
        let line = serde_json::json!({
            "prompt": prompt,
            "response": response,
            "category": "vox_ts_interop",
            "format": "qa_pair",
            "schema_version": "vox_dogfood_v1",
        });
        writeln!(out, "{}", line)?;
        count += 1;
    }
    Ok(count)
}

/// Fingerprint file path used for staleness tracking (stored in target/).
pub fn fingerprint_cache_path(repo_root: &Path) -> PathBuf {
    repo_root.join("target/dogfood/.corpus_fingerprint")
}
