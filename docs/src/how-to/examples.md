---
title: "Examples"
description: "Official documentation for Examples for the Vox language. Detailed technical reference, architecture guides, and implementation patterns "
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Examples

All examples are located in the [`examples/`](../../../examples) directory. Each demonstrates a key feature of the Vox language.

---

## Quick Reference

Golden (parse in CI) live at `examples/*.vox` — see [`examples/README.md`](../adr/README.md). Archive paths are under `examples/archive/…`.

| Example | Path | Features demonstrated |
|---------|------|------------------------|
| [Server functions](#server-functions) | `examples/archive/simple_server_fn.vox` | `@server`, `@component`, fetch wrapper |
| [Full-stack chatbot](#full-stack-chatbot) | `examples/chatbot.vox` | `@component`, actors, `style`, `routes`, `http` |
| [Chatbot + server fn](#chatbot-with-server-functions) | `examples/archive/chatbot_server_fn.vox` | `@server`, `@component`, ADTs |
| [Data layer](#data-layer) | `examples/data_layer.vox` | `@table`, `@index` (server `db.*` commented until typeck) |
| [Actors (legacy syntax)](#actors) | `examples/archive/legacy_syntax/actor.vox` | **Does not parse** — aspirational `message` / `fn main` |
| [Durable counter](#durable-counter) | `examples/durable_counter.vox` | `state_load`/`state_save`, `@component`, `routes` |
| [Workflows](#workflows) | `examples/workflow.vox` | `workflow`, `activity`, `with` |
| [Durable execution](#durable-execution) | `examples/archive/durable_execution.vox` | Retry/timeout/backoff |
| [MCP tools (legacy syntax)](#mcp-tools) | `examples/archive/legacy_syntax/mcp_tool.vox` | **Does not parse** — `@mcp.tool("…")` form |
| [AI agents](#ai-agents) | `examples/archive/agent.vox` | `@agent_def`, tools, memory |
| [Dashboard](#dashboard) | `examples/archive/dashboard.vox` | `@v0`, `@component`, `http`, `routes` |
| [Sharing](#sharing--skills) | `examples/archive/sharing.vox` | `@skill`, `workflow` |
| [Testing](#testing) | `examples/testing.vox` | `@test`, `assert` |
| [Minimal server fn](#minimal-server-function) | `examples/server_fn.vox` | Minimal `@server` |
| [Full-stack minimal](#full-stack-minimal) | `examples/full_stack_minimal.vox` | `routes`, `http`, `@server`, `@component` |

---

## Server Functions

**File**: `examples/archive/simple_server_fn.vox`

Demonstrates how `@server` generates both a backend HTTP route and a frontend fetch wrapper:

```vox
# Skip-Test
@server fn greet(name: str) to Greeting:
    ret Hello("Welcome, " + name + "!")

@component fn App() to Element:
    let result = use_state("")
    let handle_click = fn(_e):
        let greeting = greet("Alice")     # ← auto-generated fetch call
        set_result(greeting)
    ret <div><button onClick={handle_click}>Greet</button><p>{result}</p></div>
```

**Key insight**: `greet("Alice")` in the component compiles to a typed `fetch()` call — the compiler generates the HTTP client automatically.

---

## Full-Stack Chatbot

**File**: `examples/chatbot.vox`

A complete application in a single file: React UI + Axum backend + database + actors:

```vox
# Skip-Test
@component fn Chat() to Element:
    let (messages, set_messages) = use_state([])
    let (input, set_input) = use_state("")
    let send = fn (_e) (set_messages(messages.append({role: "user", text: input})),
                        spawn(ChatClient).send(input), set_input(""))
    <div class="chat_container">
        <div class="messages">
            for msg in messages:
                <div class="msg {msg.role}">{msg.text}</div>
        </div>
        <div class="input_area">
            <input class="chat_input" bind={input} placeholder="Type a message..." />
            <button class="send_btn" on_click={send}>Send</button>
        </div>
    </div>
```

This single file defines: UI components, styling, routes, database tables, queries, mutations, actions, and actors.

---

## Chatbot with Server Functions

**File**: `examples/chatbot_server_fn.vox`

Shows ADT pattern matching inside JSX for rendering different message types:

```vox
# Skip-Test
type Message =
    | User(text: str)
    | Assistant(text: str)

{messages.map(fn(msg) match msg:
    | User(text) -> <div className="user-message">{text}</div>
    | Assistant(text) -> <div className="assistant-message">{text}</div>
)}
```

---

## Data Layer

**File**: `examples/data_layer.vox`

Typed database tables with indexes and server functions:

```vox
# Skip-Test
@table type Task:
    title: str
    done: bool
    priority: int
    owner: str

@index Task.by_done on (done, priority)
@index Task.by_owner on (owner)

# Server `db.*` bodies are commented in-tree until typeck lands — see examples/data_layer.vox
```

---

## Actors

**File**: `examples/archive/legacy_syntax/actor.vox`

**Parser status:** does **not** parse (legacy / aspirational syntax). For a **parseable** actor sample with durable state, see [Durable counter](#durable-counter) (`examples/durable_counter.vox`).

The actor model with state, message handlers, and inter-actor communication:

```vox
# Skip-Test
actor Counter:
    state count: int = 0
    on increment(amount: int) to int:
        count = count + amount
        count
    on get_count() to int:
        count

fn main():
    let counter = spawn(Counter)
    let new_count = counter.send(increment(5))   # returns 5
    let _ = counter.send(increment(3))
    let total = counter.send(get_count())         # returns 8
```

---

## Durable Counter

**File**: `examples/durable_counter.vox`

An actor whose state persists across restarts via `state_load`/`state_save`:

```vox
# Skip-Test
actor PersistentCounter:
    on increment() to int:
        let current = state_load("counter")
        let next = current + 1
        state_save("counter", next)
        ret next
```

---

## Workflows

**File**: `examples/workflow.vox`

Durable workflows with activities and the `with` expression:

```vox
# Skip-Test
activity fetch_user_data(user_id: str) to Result[str]:
    ret Ok("User data for " + user_id)

workflow onboard_user(user_id: str, email: str) to Result[str]:
    let profile = fetch_user_data(user_id) with { retries: 3, timeout: "30s" }
    let _ = send_notification(email, "Welcome! " + profile) with { retries: 5, timeout: "60s" }
    ret Ok("Onboarding complete for " + user_id)
```

---

## Durable Execution

**File**: `examples/archive/durable_execution.vox`

Full workflow with varied retry policies:

```vox
# Skip-Test
workflow process_order(customer: str, order_data: str, amount: int) to Result[str]:
    let validated = validate_order(order_data) with { timeout: "5s" }
    let payment = charge_payment(amount, "card-123") with { retries: 3, timeout: "30s", initial_backoff: "500ms" }
    let confirmation = send_confirmation(customer, "order-001") with { retries: 2, activity_id: "confirm-order-001" }
    ret confirmation
```

---

## MCP Tools

**File**: `examples/mcp_tool.vox`

Expose functions as AI-discoverable tools via the Model Context Protocol:

```vox
# Skip-Test
@mcp.tool("create_note", "Create a new note with a title and content")
fn create_note(title: str, content: str) to str:
    ret "Created note: " + title

@mcp.resource("notes://recent", "List of recently created notes")
fn recent_notes() to list[str]:
    ret ["Recent note 1", "Recent note 2"]
```

---

## AI Agents

**File**: `examples/archive/agent.vox`

Define AI agents with memory and tool access:

```vox
# Skip-Test
@agent_def fn SupportBot(query: str, session: str) to str:
    let past = db.agent_memory.find(session)
    let response = "Based on " + past.context + " -> " + query
    db.agent_memory.insert(AgentMemory(session, query))
    ret response
```

---

## Dashboard

**File**: `examples/archive/dashboard.vox`

AI-generated UI + hand-coded components + routing:

```vox
# Skip-Test
@v0 "A metrics dashboard with cards showing KPIs and a line chart" fn Dashboard() to Element

@component fn ChatWidget() to Element:
    # ... hand-coded component

routes:
    "/" to Dashboard
    "/chat" to ChatWidget
```

---

## Sharing & Skills

**File**: `examples/sharing.vox`

Publishable skills and reusable components:

```vox
# Skip-Test
@skill fn DataSummarizer(text: str) to str:
    "Summary of " + text

workflow process_document(doc_id: str) to Result[bool]:
    let doc = db.documents.find(doc_id)
    let summary = DataSummarizer(doc.content)
    db.documents.update(doc_id, summary)
    ret Ok(true)
```

---

## Testing

**File**: `examples/testing.vox`

Unit tests with `@test` and `assert`:

```vox
# Skip-Test
@test fn test_addition() to Unit:
    let sum = 1 + 2
    assert(sum is 3)

@test fn test_str_cast() to Unit:
    let n = 42
    let s = str(n)
    assert(s is "42")
```

---

## Minimal Server Function

**File**: `examples/server_fn.vox`

The simplest possible server function:

```vox
# Skip-Test
type Greeting =
    | Hello(message: str)

@server fn greet(name: str) to Greeting:
    ret Hello("Welcome, " + name + "!")
```

---

## Full-stack minimal

**File**: `examples/full_stack_minimal.vox`

Smallest **golden** sample with `routes:`, `http`, `@server`, and `@component`. See [How to: First full-stack app](first-full-stack-app.md).
