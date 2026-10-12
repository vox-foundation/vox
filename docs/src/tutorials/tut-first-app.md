---
title: "Tutorial: Building a Collaborative Task List"
description: "Build a full-stack Task app end to end with Vox."
category: "Tutorials"
status: "current"
sort_order: 2
training_eligible: true

schema_type: "HowTo"
---
# Tutorial: Building a Collaborative Task List

Learn how to build a full-stack task list app with Vox. This tutorial covers data modeling, server-side logic, and UI integration using a single `.vox` file.

## 1. Project Initialization

Scaffold a new Vox application (an application is the default package kind) and enter it:

```bash
vox init vox-task-list
cd vox-task-list
```

`vox init` writes `Vox.toml` and a starter `src/main.vox`. Replace the starter contents with the declarations below, one section at a time.

## 2. Define the Data Model

Open `src/main.vox`. We'll start by defining what a "Task" is. Using the `table` keyword, we create a persistent database table.

```vox
table Task {
    title: str
    done: bool
}
```

## 3. Implement Server Logic

Next, we add `mutation` and `query` functions to interact with the database.

```vox
query get_tasks() to int {
    return len(db.Task.all())
}

mutation create_task(title: str) to Result[str] {
    db.Task.insert({ title: title, done: false })?
    return Ok("created")
}
```

`vox build` turns `get_tasks` into a `GET /api/query/get_tasks` endpoint and `create_task` into a write endpoint, each with a typed function of the same name in the generated `vox-client.ts`.

## 4. Build the UI

Now, we'll create the frontend using a `component` declaration. Vox components use a JSX-like syntax and compile to plain React/TSX components consumed by the external frontend.

```vox
component TaskList() {
    view: column() {
        heading(level=1) { "Tasks" }
        text() { "Add tasks with create_task; count them with get_tasks." }
    }
}
```

## 5. Wiring It Together

Finally, we map a route to our `TaskList` component.

```vox
routes {
    "/" to TaskList
}
```

## 6. Build and Run

Check the file, compile it, and start the development server:

```bash
vox check src/main.vox
vox build src/main.vox -o dist
vox run src/main.vox
```

Visit `http://localhost:3000` to see the `TaskList` page.

---

**Next Steps**:
- [Actor Basics](tut-actor-basics.md) — Declare actors with message handlers.
- [Durable Workflows](tut-workflow-durability.md) — Automate task reminders.
