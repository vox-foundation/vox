---
title: "How-To: Islands and Pages"
description: "How to build and route full-stack web UIs using islands in Vox."
category: "how-to"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---

# How-To: Build UI with Islands and Pages

Vox relies on a server-first web architecture. Rather than building massive client-side bundles, Vox generates raw HTML routes and uses targeted interactive "islands" for dynamic functionality. 

*(Note: The legacy `@island` decorator has been removed in v0.3. Use `@island` and `http get` instead).*

## When to use `@island` vs `http get`

- Use **`http get`**: When you need to return server-side rendered data, pages that require no Javascript, or raw API responses like JSON.
- Use **`@island`**: When the user needs to click, type, drag, or interact with state dynamically. Islands compile into hydrated React components under the hood.

## Defining an Island with Props

Let's stick with the `Task` domain. Suppose you want a UI component to render a list of tasks.

```vox
// vox:skip
import react.use_state

@island
fn TaskList(tasks: list[Task]) -> Element {
    let (items, set_items) = use_state(tasks)

    <div class="task-list">
        <h1>"Your Tasks"</h1>
        <ul>
            {items.map(fn(task) {
                <li>{task.title}</li>
            })}
        </ul>
    </div>
}
```

### JSX Syntax within an Island

Within an `@island` body, the compiler supports standard JSX syntax. 
- You can embed variables and functions within braces `{}`.
- You can include inline conditionals and standard attributes. 
- Events like `onChange` or `onClick` are fully typed and bind directly to functions.

## Calling `@server` Functions from an Island

The power of Vox is that your frontend and backend are co-located in the same file. You can call an `@server` function directly from a client-side button click without writing manual `fetch()` bindings!

```tsx
// vox:skip
@server fn complete_task(id: Id[Task]) -> Result[Unit] {
    db.Task.update(id, { done: true })
    return Ok(())
}

@island
fn TaskRow(task: Task) -> Element {
    <div class="task-row">
        <input 
            type="checkbox" 
            checked={task.done} 
            onChange={fn(_e) complete_task(task.id)} 
        />
        <span>{task.title}</span>
    </div>
}
```

The Vox compiler automatically generates the TypeScript client, handles the asynchronous RPC call, and returns the result back to your interactive component.

## Passing Data from Server to UI 

To get your database state into the `TaskList`, you map an endpoint directly to the UI component via the `routes` block. The system will automatically resolve queries to fulfill the `tasks` prop of `TaskList`.

```vox
// vox:skip
@query
fn get_active_tasks() -> list[Task] {
    return db.Task.where({ done: false }).all()
}

routes {
    // The framework will fetch `get_active_tasks` and inject the data
    // into the `TaskList` component as props, then render to HTML.
    "/" -> TaskList(tasks: get_active_tasks())
}
```

## The Data/View `routes { }` Block

The `routes` block maps URL paths directly to server responses or UI.

```vox
// vox:skip
routes {
    "/"              -> HomeIsland     # Render an Island 
    "/tasks"         -> TaskList       # Render the TaskList
    "/dashboard"     -> Dashboard      # Render a complex page
}
```

## AI-Generated Islands

> [!TIP]
> Vox supports a special `@v0` decorator for pulling down interface prototypes.
> ```vox
> @v0 "yM1xXq6"
> fn PricingTable() -> Element
> ```
> The orchestrator will dynamically download the requested implementation into `target/generated/` at build time by calling Vercel's CLI. Use this pattern to integrate high-fidelity layouts without context switching.

---

**Related Topics**:
- [Tutorial: Building UI with Islands](../tutorials/tut-ui-integration.md)
- [Reference: Web Model](../reference/ref-web-model.md)
