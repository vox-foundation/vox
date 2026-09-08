---
title: "Interpreter isolation"
description: "Capability grammar, execution budgets, and exit codes for vox run --mode interp."
category: "Language Reference"
status: "current"
training_eligible: true
---

# Interpreter isolation

Receiver-imposed capabilities and resource ceilings for the tree-walking HIR interpreter (`vox run --mode interp`). This is the page `vox script` already tells users to read.

`vox run` without `--caps` grants everything; `--caps` is opt-in locally and mandatory on the mesh.

`vox run` on a script-shaped file (`fn main()`, no service surfaces) uses this interpreter by default. Three escape hatches still select the native lane: `--mode script`, `Vox.toml [web] run_mode = "script"`, `VOX_WEB_RUN_MODE=script`.

## `--caps` grammar

`--caps` is **repeatable** (`clap::ArgAction::Append`). Each flag is one token; do not comma-join tokens on the CLI (a Windows `TEMP` of `C:\Users\Smith, John\...` is then expressible).

| token | grants |
|---|---|
| `fs:ro=<dir>` | reads under `<dir>` (canonicalised at parse; must exist) |
| `fs:rw=<dir>` | reads and writes under `<dir>` |
| `net:none` / `net:allow` (`http:` accepted as alias) | `std.http.*` |
| `process:none` / `process:allow` | `process.*` — see the warning below |
| `env:none` / `env:ro` / `env:rw` | `env.get` / `env.args` need `ro`; `env.set` needs `rw` |
| `secrets:none` / `secrets:allow` | `secrets.*` |
| `time:real` / `time:frozen=<ms>` (non-negative) | `time.now_ms()` wall-clock or fixed |
| `random:seed=<u64>` / `random:deny` | every randomness-consuming builtin |
| `agentos:none` / `agentos:allow` | `agentos.*` |
| `deterministic` | shorthand: `time:frozen=0` + `random:seed=0` + `process:none` + `net:none` + `env:none` |

Unmentioned namespaces are **denied**. `io:` is rejected as a token — `io.open` / `io.save` take their authority from `fs:`.

**`process:allow` means "runs any binary on this host as the daemon user."** `process.exec` replaces the interpreter process; `process.spawn_background` outlives it. No interpreter property survives either.

The legacy `// vox:caps <word>...` directive is **unscoped**: it never provided per-directory roots, so `fs` there is the whole filesystem. `--caps` **overrides** the directive; the comment is documentation, never trusted.

## Limits

| flag | default | what it bounds |
|---|---|---|
| `--max-steps` | 10_000_000 | evaluated HIR nodes |
| `--max-depth` | 1024 | closure-application nesting (`apply_closure`, not every expression) |
| `--max-memory` | unset (`usize::MAX`) | Rust heap allocations in this process after the flag is armed |

`--max-steps` bounds evaluated HIR nodes, not CPU time; a local run has no wall-clock bound.

`--max-memory` counts Rust heap allocations in this process, armed after CLI parse. It does **not** count child processes, `mmap` by C dependencies, thread stacks, or bytes written to disk.

Disk and file caps (`max_disk_bytes`, `max_files`) apply on the mesh only. A local `developer_default()` run is uncapped on disk.

## Exit codes

| code | meaning |
|---|---|
| `0` | success |
| `1` | fault (parse, type, I/O, invalid `--caps` token) |
| `77` | capability denied (`vox: capability denied: <ns>.<method>`) |
| `78` | execution budget exceeded (`--max-steps` / `--max-depth`) |
| `79` | memory limit exceeded |
| `101` | interpreter bug |

## Filesystem TOCTOU residual

Roots are canonicalised at parse time. The check returns the resolved path and the syscall uses that path, so check and operation name the same file. This is not `openat`-based like WASI preopens: directory components are resolved twice, and a script that also holds `process:allow` can race the final component. Relative paths resolve against the process CWD; over the mesh that is the job directory, always an rw root.
