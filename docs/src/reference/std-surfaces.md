---
title: "Standard library surfaces"
description: "Current canonical reference for the narrow Vox std surfaces that are implemented in the typechecker and runtime/codegen paths."
category: "reference"
last_updated: 2026-03-28
training_eligible: true
---

# Standard library surfaces

This page is the current reference for the narrow `std` surface used by bell-curve Vox apps and operator scripts.

## Direct `std` helpers

| Surface | Current shape |
|---------|----------------|
| `std.uuid()` | monotonic string id |
| `std.now_ms()` | current unix time in milliseconds |
| `std.hash_fast(str)` | fast deterministic hash |
| `std.hash_secure(str)` | cryptographic hash |
| `std.args` | process argument list |

## Namespaced modules

| Namespace | Representative helpers |
|-----------|------------------------|
| `std.crypto` | `hash_fast`, `hash_secure`, `uuid` |
| `std.time` | `now_ms` |
| `std.log` | `debug`, `info`, `warn`, `error` |
| `std.fs` | `read`, `write`, `exists`, `mkdir`, `list_dir`, `glob`, `copy`, `remove_dir_all` |
| `std.path` | `join`, `join_many`, `basename`, `dirname`, `extension` |
| `std.env` | `get` |
| `std.process` | `run`, `run_ex`, `run_capture`, `run_capture_ex`, `exit` |
| `std.json` | `read_str`, `read_f64`, `quote` |

## Notes

- `std.log.*` is intentionally narrow and message-only.
- `std.time` / `std.crypto` provide the same hash/time/id helpers that are also available through direct `std.*` calls.
- Workflow `with { ... }` options are not part of the `std` namespace; see [Actors & Workflows](../explanation/expl-actors-workflows.md).
