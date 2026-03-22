---
title: "Vox script automation primitives (MVP)"
category: architecture
last_updated: 2026-03-21
---

# Automation primitives

Script-mode codegen (feature **`script-execution`**) exposes:

| Surface | Semantics |
|---------|-----------|
| `print(str)` | Line to stdout (`println!`). |
| `std.args` | `Vec<String>` of argv after the script path. |
| `std.env.get(key: str)` | `Option[str]` via `std::env::var`. |
| `std.fs.read(path)` | `Result[str]` — UTF-8 text. |
| `std.fs.write(path, data)` | `Result[Unit]`. |
| `std.fs.read_bytes(path)` | `Result[str]` — bytes as string (lossy where needed at boundary). |
| `std.fs.exists(path)` | `bool`. |
| `std.fs.remove(path)` | `Result[Unit]` — file remove. |
| `std.fs.mkdir(path)` | `Result[Unit]` — `create_dir_all`. |
| `std.fs.list_dir(path)` | `Result[List[str]]]` — file names only (non-recursive). |
| `std.fs.glob(pattern)` | `Result[List[str]]]` — sorted paths matching a [`glob`](https://docs.rs/glob/) pattern. |
| `std.fs.remove_dir_all(path)` | `Result[Unit]` — recursive directory removal. |
| `std.fs.copy(src, dst)` | `Result[Unit]` — copy a file. |
| `std.path.join(a, b)` | `str` — platform path join. |
| `std.path.join_many(segments)` | `str` — join a `List[str]` with the platform separator (empty list → `"."`). |
| `std.path.basename` / `dirname` / `extension` | `str` — path helpers. |
| `std.process.run(cmd, args)` | `Result[int]` — success exit code; non-zero → `Error`. |
| `std.process.run_ex(cmd, args, cwd, env)` | `Result[int]` — like `run`, optional `cwd` (`""` = inherit) and `env` as `List[str]` of `KEY=value` pairs merged into the subprocess environment. |
| `std.process.run_capture(cmd, args)` | `Result[Record]` — `{ exit: int, stdout: str, stderr: str }`; spawn/read errors → `Error`; **non-zero exit is still `Ok`** (inspect `exit`). |
| `std.process.run_capture_ex(cmd, args, cwd, env)` | Same as `run_capture`, with optional `cwd` and `env` (same shape as `run_ex`). |
| `std.process.exit(code)` | Terminates the process (`std::process::exit`). |
| `std.json.read_str(json, key)` | `Result[str]` — parse a JSON object and read a string field (top-level). |
| `std.json.read_f64(json, key)` | `Result[float]` — parse a JSON object and read a numeric field (ints coerced). |
| `std.json.quote(s)` | `str` — JSON-encode a string value (quotes + escapes). |

Type-checker routing: `crates/vox-typeck/src/checker.rs` (`StdFsNs`, `StdPathNs`, `StdEnvNs`, `StdProcessNs`, `StdJsonNs`). Codegen: `crates/vox-codegen-rust/src/emit.rs` (`std.fs.*` / `std.process.*` / `std.json.*` builtins). Runtime: `crates/vox-runtime/src/builtins.rs` (`vox_list_dir`, `vox_process_run`, `vox_process_run_capture`, `vox_fs_glob`, …).

## Security

`std.process.run`, `run_capture`, `run_ex`, and `run_capture_ex` use the host `Command` API — **trusted dev** contexts only. Untrusted inputs should use the WASI / sandbox lanes documented for `vox script`, not arbitrary command strings.
