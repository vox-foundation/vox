---
title: "Tutorial: Building a Collaborative Task List"
description: "Build a full-stack Task app end to end with Vox."
category: "tutorials"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---
# Tutorial: Building a Collaborative Task List

Learn how to build a full-stack, collaborative task list app with Vox. This tutorial covers data modeling, server-side logic, and UI integration using a single `.vox` file.

## 1. Project Initialization

Create a new directory and initialize a Vox application:

```bash
mkdir vox-task-list && cd vox-task-list
vox init --kind application
```

## 2. Define the Data Model

Open `src/main.vox`. We'll start by defining what a "Task" is. Using the `@table` decorator, we create a persistent database table. We use `@require` to ensure titles are not empty.

```vox
# Skip-Test: ui-only
@require(len(self.title) > 0)
@table type Task {
    title:    str
    done:     bool
    priority: int
    owner:    str
}
```

## 3. Implement Server Logic

Next, we add `@mutation` and `@query` functions to interact with the database.

```vox
# Skip-Test: ui-only
@mutation
fn add_task(title: str, owner: str) to Id[Task] {
    ret db.Task.insert({ title: title, done: false, priority: 1, owner: owner })
}

@query
fn open_tasks() to List[Task] {
    ret db.Task.where({ done: false }).order_by("priority", "desc").limit(10)
}

@mutation
fn complete_task(id: Id[Task]) to Unit {
    db.Task.update(id, { done: true })
}
```

## 4. Build the UI

Now, we'll create the frontend using the `@island` decorator. Vox islands use a JSX-like syntax that compiles to high-performance hydrated React components.

```tsx
# Skip-Test: ui-only
import react.use_state

@island
fn TaskList(tasks: List[Task]) to Element {
    let (items, _set_items) = use_state(tasks)
    <div class="task-list">
        <h1>"Vox Tasks"</h1>
        {items.map(fn(task) {
            <label>
                <input type="checkbox" checked={task.done}
                       onChange={fn(_e) complete_task(task.id)} />
                {task.title}
            </label>
        })}
    </div>
}
```

## 5. Wiring It Together

Finally, we map a route to our `TaskList` component.

```vox
# Skip-Test: ui-only
routes {
    "/" to TaskList
}
```

## 6. Build and Run

Compile your app and start the development server:

```bash
vox check src/main.vox
vox build src/main.vox
vox run src/main.vox
```

Visit `http://localhost:3000` to see your collaborative task list in action!

---

**Next Steps**:
- [Actor Basics](tut-actor-basics.md) — Add real-time collaboration with shared state.
- [Durable Workflows](tut-workflow-durability.md) — Automate task reminders.
