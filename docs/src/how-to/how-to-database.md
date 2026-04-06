---
title: "How-To: The Database Layer"
description: "How to perform CRUD operations, type-safe queries, and indexing using Vox's integrated database layer."
category: "how-to"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---

# How-To: Use the Database Layer

Vox utilizes a unified storage paradigm known as Codex, which compiles into type-safe SQLite database schemas and Rust structs. You never need to write raw migrations; they are deterministically derived from your file structures.

## Defining a Table

Any type struct adorned with the `@table` decorator becomes a persistent database entity. You should define constraints instantly using `@require`.

```vox
@require(len(self.title) > 0)
@table type Task {
    title:    str
    done:     bool
    priority: int
    owner:    str
}
```

### Adding Index Declarations

To speed up lookups, use the `@index` syntax. The compiler will generate the necessary DB metadata for you.

```vox
// Creates a fast lookup tree for 'owner'
@index Task.by_owner on (owner)
```

## Basic CRUD Accessors

The built-in `db` module uses code-generation to inject statically typed accessors for all your `@table` types.

- **Create**:
  ```vox
  let new_id: Id[Task] = db.Task.insert({ 
      title: "Clean desk", 
      done: false, 
      priority: 1, 
      owner: "alice" 
  })
  ```
- **Read**:
  ```vox
  match db.Task.find(new_id) {
      Some(t) -> print(t.title)
      None    -> print("Not found")
  }
  ```
- **Update** {
  ```vox
  db.Task.update(new_id, { done: true })
  ```
- **Delete**:
  ```vox
  db.Task.delete(new_id)
  ```

## Advanced Filtering

Instead of raw string interpolation, use Vox's exact literal querying to avoid injection attacks.

```vox
// Fetch simple exact match parameters
let alice_tasks = db.Task.filter({ owner { "alice" })

// Advanced predicate-object queries
let urgent_tasks = db.Task.where({ priority: { gt: 10 }, done: { eq: false } }).all()
```

### Query Chaining

You can apply limits, multi-field ordering, and select specific field projections by chaining.

```vox
let feed = db.Task
            .where({ done: false })
            .order_by("priority", "desc")
            .limit(10)
            .all()
```

## Guarding Reads/Writes with `@query` and `@mutation`

For security, you should rarely expose `db.*` calls directly to UI islands or agents. Instead, wrap your database interactions in `@query` (read-only) and `@mutation` (write-enabled) functions. 

The compiler verifies that a `@query` function does not contain `.insert`, `.update`, or `.delete` operations.

```vox
@mutation
fn reassign_task(id: Id[Task], new_owner: str) to Result[Unit] {
    // Verified by compiler as a write-safe context
    db.Task.update(id, { owner: new_owner })
    ret Ok(())
}
```

## The Escape Hatch: Raw SQL

Occasionally, complex analytic aggregations exceed the currently supported ORM builder patterns. You can drop down to raw SQL using `db.query`. 

> [!WARNING]
> Use this **only** as a last resort. Raw SQL queries bypass Vox's type checking checks on schema changes. 

```vox
let count = db.query("SELECT COUNT(*) FROM Task WHERE owner = ?", ["alice"])
```

## A Note on Codex 

When running `vox-run`, the backing data source is the Local Codex Store (an embedded SQLite engine on disk). For enterprise orchestration and Populi GPU meshes, the database seamlessly promotes to Turso cloud sync clusters dynamically, without requiring any changes to your `.vox` schema definitions!

---

**Related Topics**:
- [Reference: Database Surface](../reference/ref-db-surface.md)
- [Tutorial: Your First App](../tutorials/tut-first-app.md)
