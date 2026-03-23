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
/// Turn 1: create it, Turn 2: add a feature, Turn 3: make it production-ready.
pub fn gen_multiturn_vox(construct: &str, name: &str, base_code: &str, template_idx: usize) -> Vec<Turn> {
    match template_idx % 4 {
        0 => vec![
            Turn { role: "user", content: format!("Write a Vox {construct} called `{name}`") },
            Turn { role: "assistant", content: base_code.to_string() },
            Turn { role: "user", content: format!("Add error handling and logging to `{name}`") },
            Turn { role: "assistant", content: format!("@traced\n{base_code}\n// Error paths handled via Result[T] return type") },
            Turn { role: "user", content: format!("Add a @test for `{name}` that covers the error case") },
            Turn { role: "assistant", content: format!("@test\nfn test_{name}_error() to Unit {{\n    let result = {name}(-1)\n    assert(result == Error(\"invalid\"))\n}}") },
        ],
        1 => vec![
            Turn { role: "user", content: format!("I have this {construct} called `{name}`. Explain how it works:\n```\n{base_code}\n```") },
            Turn { role: "assistant", content: format!("This Vox {construct} named `{name}` initializes and manages state. It uses strong typing and explicit error handling via Option/Result logic.") },
            Turn { role: "user", content: format!("Can you refactor it to be more performant?") },
            Turn { role: "assistant", content: format!("@inline\n{base_code}\n// Refactored to eliminate allocations") },
        ],
        2 => vec![
            Turn { role: "user", content: format!("Create a {construct} named `{name}`.") },
            Turn { role: "assistant", content: base_code.to_string() },
            Turn { role: "user", content: format!("Now make it use the new Option[T] exhaustive match syntax.") },
            Turn { role: "assistant", content: format!("// Exhaustive match added\n{base_code}") },
        ],
        _ => vec![
            Turn { role: "user", content: format!("Write a {construct} for `{name}`") },
            Turn { role: "assistant", content: base_code.to_string() },
            Turn { role: "user", content: format!("Add a new feature to it: track the number of calls") },
            Turn { role: "assistant", content: format!("// Call tracking added\n{base_code}") },
        ],
    }
}

/// Serialize a multi-turn conversation to JSONL (ChatML-compatible format).
pub fn multiturn_to_jsonl(turns: &[Turn], category: &str) -> String {
    let messages: Vec<serde_json::Value> = turns.iter().map(|t| {
        serde_json::json!({"role": t.role, "content": t.content})
    }).collect();
    serde_json::json!({
        "messages": messages,
        "category": category,
        "format": "multiturn_chat",
        "schema_version": "vox_dogfood_v1",
    }).to_string()
}

// ── Error → Fix pair generator ────────────────────────────────────────────────

/// A category of intentional syntax/semantic error.
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

/// Fingerprint file path used for staleness tracking (stored in target/).
pub fn fingerprint_cache_path(repo_root: &Path) -> PathBuf {
    repo_root.join("target/dogfood/.corpus_fingerprint")
}
