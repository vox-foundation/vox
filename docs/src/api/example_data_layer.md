---
title: "Example: Data layer example — typed record store with Convex-inspired API."
description: "Official documentation for Example: Data layer example — typed record store with Convex-inspired API. for the Vox language."
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Example: Data layer example — typed record store with Convex-inspired API.

```vox
// Data layer example — typed record store with Convex-inspired API.
// Tables are defined with @table, indexes with @index.

@table type Task:
    title: str
    done: bool
    priority: int
    owner: str

@index Task.by_done on (done, priority)
@index Task.by_owner on (owner)

// Server function using the db object (codegen coming soon).
@server fn list_tasks() to List[Task]:
    ret db.query(Task).collect()

@server fn add_task(title: str, owner: str) to Id[Task]:
    ret db.insert(Task, { title: title, done: false, priority: 0, owner: owner })

@server fn complete_task(id: Id[Task]):
    db.patch(id, { done: true })
```
