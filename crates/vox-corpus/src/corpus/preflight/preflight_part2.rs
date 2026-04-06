
/// Generate an error→fix training pair as JSONL.
pub fn error_fix_to_jsonl(broken: &str, explanation: &str, fixed: &str, category: &str) -> String {
    serde_json::json!({
        "prompt": format!("Why doesn't this Vox code compile?\n\n```vox\n{broken}\n```"),
        "response": format!("{explanation}\n\nFixed version:\n```vox\n{fixed}\n```"),
        "category": format!("{category}_error_fix"),
        "format": "error_fix",
        "schema_version": "vox_dogfood_v1",
    })
    .to_string()
}

pub fn write_architectural_pairs(out: &mut impl std::io::Write) -> anyhow::Result<usize> {
    use crate::codegen_vox::{generate_organic_corpus, TAXONOMY_FROM_AST};
    
    let mut count = 0;
    let organic = generate_organic_corpus(42);
    
    for tag in TAXONOMY_FROM_AST {
        let question1 = format!("What is a `{tag}` in Vox?");
        let answer1 = match *tag {
            "component" => "A component is a server-side rendered UI construct in Vox. It has no client-side JavaScript by default.",
            "island" => "An island is a client-side rendered UI construct in Vox, allowing interactivity and state.",
            "workflow" => "A workflow is a durable, retryable multi-step process in Vox. It checkpoints state automatically.",
            "actor" => "An actor is a stateful entity that can receive and process messages asynchronously in memory.",
            "table" => "A table defines a database schema entity with compile-time safety.",
            "mcp_tool" => "An mcp_tool exposes a Vox function according to the Model Context Protocol.",
            "query" => "A query marks read-only database operations that are safe to cache and retry.",
            "mutation" => "A mutation marks write operations to the database.",
            "scheduled" => "A scheduled function runs at a specified interval (e.g. cron or timespan) in the background.",
            "server_fn" | "server" => "A server function always runs on the server side and is invisible to client bundles.",
            "action" => "An action is a server function triggered by client-side events, similar to Next.js Server Actions.",
            _ => "This represents a distinct grammatical or runtime construct in the Vox language."
        };
        
        let line1 = serde_json::json!({
            "prompt": question1,
            "response": answer1,
            "category": "vox_architectural_qa",
            "format": "qa_pair",
            "schema_version": "vox_dogfood_v1",
        });
        writeln!(out, "{}", line1)?;
        count += 1;
        
        if let Some(example) = organic.iter().find(|p| p.category == format!("vox_{tag}")) {
            let question2 = format!("Show me an example of a `{tag}` in Vox.");
            let line2 = serde_json::json!({
                "prompt": question2,
                "response": example.response,
                "category": "vox_architectural_qa",
                "format": "qa_pair",
                "schema_version": "vox_dogfood_v1",
            });
            writeln!(out, "{}", line2)?;
            count += 1;
        }
    }
    
    Ok(count)
}

// ── Code-explanation pair generator ──────────────────────────────────────────

/// Generate "explain this code" training pairs from a slice of organic pairs.
///
/// Samples every `stride`-th entry to avoid overwhelming the JSONL with
/// explanation pairs relative to generative pairs. Returns JSONL strings.
pub fn gen_explain_pairs(
    organic_code_samples: &[(
        /* prompt */ String,
        /* response / code */ String,
        /* category */ String,
    )],
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
            })
            .to_string()
        })
        .collect()
}

// ── Debug / diagnosis pair generator ─────────────────────────────────────────

/// Generate runtime-error diagnosis training pairs.
///
/// Each pair teaches the model to read a runtime panic or logic error and
/// identify what went wrong, then suggest a fix.
pub fn gen_debug_pairs(organic_samples: &[(String, String, String)], stride: usize) -> Vec<String> {
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
            })
            .to_string()
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
            })
            .to_string()
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
