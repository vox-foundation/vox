---
title: "Standard library surfaces"
description: "Current canonical reference for the narrow Vox std surfaces that are implemented in the typechecker and runtime/codegen paths."
category: "reference"
last_updated: 2026-04-02
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
| `std.fs` | `read`, `write`, `read_bytes`, `exists`, `is_file`, `is_dir`, `canonicalize`, `mkdir`, `list_dir`, `glob`, `copy`, `remove`, `remove_dir_all` |
| `std.path` | `join`, `join_many`, `basename`, `dirname`, `extension` |
| `std.env` | `get` |
| `std.process` | `which`, `run`, `run_ex`, `run_capture`, `run_capture_ex`, `exit` |
| `std.json` | `read_str`, `read_f64`, `quote` |
| `std.http` | `get_text`, `post_json` |

## Notes

- `std.log.*` is intentionally narrow and message-only.
- `std.time` / `std.crypto` provide the same hash/time/id helpers that are also available through direct `std.*` calls.
- `std.http.post_json(url, body)` expects `body` to be a JSON string and returns response text on success.
- `std.http.*` on WASI script targets returns `Error(...)` because outbound HTTP is a native/container lane in the current runtime model.
- Workflow `with { ... }` options are not part of the `std` namespace; see [Actors & Workflows](../explanation/expl-actors-workflows.md).

## Host shell vs `std` (PowerShell vs Vox runtime)

- **Interactive terminals** (contributors, coding agents, local automation): prefer **PowerShell 7 (`pwsh`)** when installed on any OS, consistent with [`AGENTS.md`](../../../AGENTS.md), **`vox shell check`**, and [`contracts/terminal/exec-policy.v1.yaml`](../../../contracts/terminal/exec-policy.v1.yaml). That environment drives **how** you invoke `vox`, `cargo`, `git`, etc.
- **`std.fs` / `std.process` / `std.path`** describe the **Vox language** surface compiled to Rust (and other targets). They are **host-shaped** at the FFI boundary (OS paths, subprocess argv) but are **not** PowerShell cmdlets — Vox source does not embed PS syntax.
- Prefer **`std.*`** inside `.vox` programs for portable file and process operations; use PowerShell only in outer harness / agent steps around the toolchain.

Lane map (no shell emulator in `.vox`): [Vox shell operations boundaries](../architecture/vox-shell-operations-boundaries.md).

## Lessons from PowerShell-shaped ergonomics (mapped to `std`)

Shell environments emphasize **explicit path kinds**, **normalization before compare**, and **structured effects**. Vox expresses those as **typed `std.*` APIs** — not cmdlet names in `.vox` source. The table below is a **mental model** for contributors coming from PowerShell, not a language embedding.

| PowerShell idea | Typical cmdlet(s) | Vox surface | Notes |
|-----------------|-------------------|-------------|--------|
| Path algebra | `Join-Path`, `Split-Path` | `std.path.join`, `join_many`, `basename`, `dirname`, `extension` | Always build paths with these instead of concatenating `/` or `\\` strings. |
| Path exists | `Test-Path` | `std.fs.exists` | `true` for files **or** directories (like `Test-Path` without `-PathType`). |
| Leaf vs container | `Test-Path -PathType Leaf`, `-PathType Container` | `std.fs.is_file`, `std.fs.is_dir` | Use when logic must branch on entry kind (compare `Get-Item`’s `.PSIsContainer`). |
| Canonical / absolute path | `Resolve-Path` (no wildcards) | `std.fs.canonicalize` | `Result[str]`; resolves `.`, `..`, and symlinks per OS (`std::fs::canonicalize`). Fails if the path does not exist. |
| Read / write text | `Get-Content -Raw`, `Set-Content` | `std.fs.read`, `write` | `read` is UTF-8 text; `read_bytes` is the lossy-bytes-as-string escape hatch. |
| Enumerate children | `Get-ChildItem` (names) | `std.fs.list_dir` | Returns **names only**, non-recursive — closer to `RustReadDir` than to rich `FileInfo` objects. |
| Pattern match paths | `Get-ChildItem` with wildcards / `-Filter` | `std.fs.glob` | Sorted path strings; use for bulk discovery (see [`glob`](https://docs.rs/glob/) semantics). |
| Make tree / remove | `New-Item -ItemType Directory -Force`, `Remove-Item -Recurse` | `std.fs.mkdir`, `remove_dir_all`, `remove`, `copy` | `mkdir` is `create_dir_all`; `remove` is a single file. |
| Resolve executable on PATH | `(Get-Command name).Source` (concept) | `std.process.which` | `Option[str]` absolute path; pair with `std.process.run*` for deterministic spawn. |

**Lessons borrowed from PowerShell (design goals for `std.fs`):**

1. **Explicit path kind** — PS draws a clear line between “something is there” (`Test-Path`), “it’s a file”, and “it’s a directory”. Vox mirrors that with `exists` plus `is_file` / `is_dir` so scripts do not rely on heuristics or failed `read` calls.
2. **Normalize before compare** — Agents often compare paths as strings; PS teaches **canonical resolution first**. Use `std.fs.canonicalize` when comparing two locations that might differ by relative segments or symlinks (when the path exists).
3. **`list_dir` is intentionally flat** — `Get-ChildItem` returns rich objects; Vox today returns **names** only to keep the type surface small. For “full name” paths, combine with `std.path.join(dir, name)` or use `std.fs.glob` when a pattern is enough.
4. **Errors as `Result`, not exceptions** — Vox I/O returns `Result[...]` with a message string on failure, similar in spirit to `-ErrorAction Stop` with try/catch, but uniform across the language.

**Possible future extensions** (not in the std surface yet): file length / mtime, symlink metadata, optional recursive tree walks, or a small `record` for directory entries (name + kind). Those would move closer to `Get-ChildItem` / `Get-Item` richness without pulling PowerShell syntax into `.vox`.
