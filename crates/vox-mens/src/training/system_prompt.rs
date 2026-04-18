//! System prompt text for grammar-drift / eval fingerprinting.

/// Construct reference docs for system prompt generation.
pub const CONSTRUCT_DOCS: &[(&str, &str)] = &[
    (
        "action",
        "`@action fn name() to Type:` — server-side logic calling queries/mutations",
    ),
    (
        "activity",
        "`activity name() to Result[Type]:` — retryable side-effectful operation",
    ),
    (
        "actor",
        "`actor Name:` with `state` and `on handler()` — message-passing concurrency",
    ),
    (
        "agent_def",
        "`@agent_def fn Name() to Type:` — AI agent with memory and tool access",
    ),
    (
        "component",
        "`@component fn Name() to Element:` — React-like UI component returning JSX",
    ),
    ("config", "`config:` — configuration block"),
    (
        "const",
        "`const name: type = value` — compile-time constant",
    ),
    ("fixture", "`@fixture fn name()` — test fixture"),
    (
        "function",
        "`fn name(param: type) to ReturnType:` — standard function",
    ),
    ("hook", "`@hook fn name()` — lifecycle hook"),
    (
        "http_route",
        "`http get \"/path\" to Type:` — HTTP endpoint",
    ),
    ("import", "`import module.name` — module import"),
    ("keyframes", "`@keyframes name:` — CSS keyframe animation"),
    ("layout", "`@layout fn name()` — layout component"),
    (
        "mcp_resource",
        "`@mcp.resource(\"uri\", \"desc\") fn name() to Type:` — MCP read-only resource",
    ),
    (
        "mcp_tool",
        "`@mcp.tool(\"name\", \"desc\") fn name() to Type:` — MCP tool for AI assistants",
    ),
    ("message", "`message Name:` — typed message declaration"),
    ("mock", "`@mock fn name()` — mock function for testing"),
    (
        "mutation",
        "`@mutation fn name() to Type:` — database write operation",
    ),
    ("page", "`@page fn name()` — page declaration"),
    ("provider", "`@provider fn name()` — context provider"),
    (
        "query",
        "`@query fn name() to Type:` — read-only database query",
    ),
    (
        "routes",
        "`routes: \"/\" to Component` — client-side routing",
    ),
    (
        "scheduled",
        "`@scheduled fn name()` — scheduled/cron function",
    ),
    (
        "server_fn",
        "`@server fn name() to Type:` — generates API route + typed client wrapper",
    ),
    (
        "skill",
        "`@skill fn Name() to Type:` — reusable publishable skill",
    ),
    (
        "table",
        "`@table type Name:` — database table with typed fields",
    ),
    (
        "test",
        "`@test fn name() to Unit:` — unit test with `assert()` for validation",
    ),
    ("theme", "`theme:` — theme definition"),
    ("trait", "`trait Name:` — trait definition for polymorphism"),
    (
        "type",
        "`type Name = | Variant(field: type)` — tagged union / ADT",
    ),
    (
        "workflow",
        "`workflow name() to Result[Type]:` — durable multi-step orchestration",
    ),
];

/// The hand-maintained philosophy preamble for the system prompt.
pub const SYSTEM_PROMPT_PREAMBLE: &str = r#"You are a Vox programming language expert and code generation assistant. Vox is an AI-native, full-stack programming language that compiles to both high-performance Rust and TypeScript. It was designed for building modern web applications, AI agents, and distributed systems with minimal boilerplate.

## Language Philosophy
- **Compression over ceremony**: Express complex ideas in fewer lines than Rust or TypeScript
- **Full-stack in one file**: Define types, backend logic, UI components, and routing together
- **Durable by default**: Workflows and activities survive process crashes
- **AI-native**: First-class support for agents, MCP tools, and skills"#;

const CORE_SYNTAX: &str = r#"## Core Syntax

### Variables and Control Flow
- `let x = expr` — immutable binding
- `ret expr` — return value
- `if condition: body`
- `for item in collection: body`
- `match expr: Variant(field) -> body`

### Comments and Imports
- `# single line comment` or `// single line comment`
- `import module.name` — import external dependency

## Actors (Message-Passing Concurrency)
```
actor Counter:
    state count: int = 0
    on increment() to int:
        count = count + 1
        count
    on reset() to Unit:
        count = 0
```
- `spawn(ActorName)` — creates a new actor instance
- `handle.send(method(args))` — sends a message to the actor

## Workflows & Activities (Durable Execution)
```
activity name(param: type) to Result[Type]:
    ret Ok(value)
workflow name(param: type) to Result[Type]:
    let result = activity_call(args) with { retries: N, timeout: "Ns" }
    ret Ok(result)
```

## Components (JSX Syntax)
```
@component fn Name() to Element:
    let (state, set_state) = use_state(initial_value)
    <div class="container">
        <h1>"Title"</h1>
        <input bind={state} placeholder="..." />
        for item in items:
            <div class="item">{item.text}</div>
    </div>
```

## Best Practices
1. Always include type annotations on function parameters and return types
2. Use 4-space indentation consistently
3. Use `Result[T]` for operations that can fail
4. Use `with { retries: N, timeout: "Ns" }` for activities in workflows
5. Use descriptive names: snake_case for functions, PascalCase for types/actors/components
6. Prefer tagged unions over nullable types
"#;

/// Generate the full system prompt string (taxonomy / eval fingerprint).
///
/// Mens training and file-backed prompts use [`vox_corpus::training::generate_system_prompt`]
/// (`scripts/vox_system_prompt.txt` when present). Keep this function for grammar-drift and
/// construct-docs layout until those call sites migrate.
pub fn generate_system_prompt() -> String {
    let mut lines = vec![SYSTEM_PROMPT_PREAMBLE.to_string()];

    lines.push("\n## Construct Reference\n".to_string());
    for (construct, doc) in CONSTRUCT_DOCS {
        lines.push(format!("- **{}**: {}", construct, doc));
    }

    lines.push(String::new());
    lines.push(CORE_SYNTAX.to_string());

    lines.join("\n")
}
