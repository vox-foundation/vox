---
title: "How-To: The Database Layer"
description: "How to perform CRUD operations, type-safe queries, and indexing using Vox's integrated database layer."
category: "How-To Guides"
status: "current"
training_eligible: true

schema_type: "HowTo"
---

# How-To: Use the Database Layer

Vox utilizes a unified storage paradigm known as Codex, which compiles into type-safe SQLite database schemas and Rust structs. You never need to write raw migrations; they are deterministically derived from your file structures.

## Defining a Table

Any type struct declared with the `table` keyword becomes a persistent database entity.

```vox
table Note {
    title: str
    content: str
}
```

### Indexing for Performance

To speed up lookups on large datasets, use the `index` keyword. Vox determines the optimal storage engine (B-Tree or Hash) and generates the SQL automatically.

```vox
table Team {
    name: str
}

table User {
    email: str
    team_id: Id[Team]
}

// Index: speeds up email lookups
index User.unique_email on (email)

// Composite index: speeds up filtered team lookups
index User.by_team on (team_id, email)
```

> [!TIP]
> Always index foreign keys (like `Id[T]`) if you plan to filter or join on them frequently.

## Basic CRUD Accessors

The built-in `db` module uses code-generation to inject statically typed accessors for all your `table` types.

- **Create**:
  ```vox
  table Task {
      title: str
      done: bool
      priority: int
      owner: str
  }

  mutation create_task() to Result[int] {
      let new_id = db.Task.insert({
          title: "Clean desk",
          done: false,
          priority: 1,
          owner: "alice"
      })?
      return Ok(new_id)
  }
  ```
- **Read**:
  ```vox
  query read_task(task_id: int) to Result[str] {
      let found = db.Task.find(task_id)?
      return match found {
          Some(t) => Ok(t.title)
          None    => Ok("Not found")
      }
  }
  ```
- **Update**:
  ```vox
  mutation mark_task_done(task_id: int) to Result[Unit] {
      db.Task.update(task_id, {
          title: "Clean desk",
          done: true,
          priority: 1,
          owner: "alice"
      })?
      return Ok(Unit)
  }
  ```
- **Delete**:
  ```vox
  mutation remove_task(task_id: int) to Result[Unit] {
      db.Task.delete(task_id)?
      return Ok(Unit)
  }
  ```

## Advanced Filtering

Instead of raw string interpolation, use Vox's exact literal querying to avoid injection attacks.

// Fetch simple exact match parameters
```vox
query tasks_for_alice() to Result[int] {
    let alice_tasks = db.Task.filter({ owner: "alice" })?
    return Ok(len(alice_tasks))
}
```

// Advanced predicate-object queries
```vox
query urgent_open_tasks() to Result[int] {
    let urgent_tasks = db.Task.where({ priority: 10, done: false })?
    return Ok(len(urgent_tasks))
}
```

### Query Chaining

You can apply limits, multi-field ordering, and select specific field projections by chaining.

```vox
query top_priority_feed() to Result[int] {
    let feed = db.Task.where({ done: false }).order_by("priority", "desc").limit(10)?
    return Ok(len(feed))
}
```

## Guarding Reads/Writes with `query` and `mutation`

For security, you should rarely expose `db.*` calls directly to UI islands or agents. Instead, wrap your database interactions in `query` (read-only) and `mutation` (write-enabled) functions.

The compiler verifies that a `query` function does not contain `.insert`, `.update`, or `.delete` operations.

### Transactional Integrity with `mutation`

Every function declared with `mutation` is automatically wrapped in a database transaction. If the function returns an `Error` or panics, the transaction is rolled back.

```vox
table Account {
    balance: int
}

mutation transfer_funds(from_id: int, to_id: int, amount: int) to Result[Unit] {
    db.Account.update(from_id, { balance: 100 - amount })?
    db.Account.update(to_id, { balance: 100 + amount })?
    return Ok(Unit)
}
```

Under the hood, this uses `Codex::transaction` to ensure ACID compliance across the local SQLite or distributed Turso mesh.


## Counting and Aggregating Rows

Instead of dropping down to raw SQL (which the HIR validator rejects to prevent injection and schema drift), use typed collection helpers like `.count()` or `.all()`:

```vox
query total_task_count() to Result[int] {
    let total = db.Task.count()?
    return Ok(total)
}
```

## A Note on Codex 

When running `vox-run`, the backing data source is the Local Codex Store (an embedded SQLite engine on disk). For enterprise orchestration and Populi GPU meshes, the database seamlessly promotes to Turso cloud sync clusters dynamically, without requiring any changes to your `.vox` schema definitions!

---

**Related Topics**:
- [Reference: Database Surface](../reference/ref-db-surface.md)
- [Tutorial: Your First App](../tutorials/tut-first-app.md)
