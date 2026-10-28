---
name: nushell-structured-shell
description: Execute structured Nushell pipelines returning typed JSON tables/records instead of brittle POSIX text parsing (awk/sed/cut). Use when querying filesystem metadata, process tables, structured logs, git histories, or JSON/TOML/YAML configs in Vox terminal sessions.
metadata:
  vox-id: nushell-structured-shell
  vox-category: terminal
  vox-shell: nushell
  vox-structured-output: "true"
  vox-tools: vox_run_shell,term_run
---

# Nushell Structured Shell (`nushell-structured-shell`)

Prefer structured Nushell (`nu`) pipelines over fragile POSIX `awk`/`sed`/`cut` text-scraping whenever inspecting files, processes, structured data, or system state in Vox.

## 3-Layer Execution Pyramid

1. **Layer 1: Native Vox / Stdlib (`vox_interp` / `std.*`)**
   - For pure computation, repo automation, cross-platform file transforms, and script workflows, write `.vox` files and run via `vox run` (interpreted or compiled).
2. **Layer 2: Structured Shell (`!nu <pipeline>` / `/nu <pipeline>`)**
   - For system exploration, structured pipeline querying (JSON, YAML, CSV, SQLite, ps, ls, netstat), and table filtering where typed fields save agent token parsing.
3. **Layer 3: Host Execution (`pwsh` / POSIX shell)**
   - For direct invocation of external platform binaries, native toolchains (`cargo`, `git`, `npm`), or tasks where unbuffered raw PTY terminal interactive IO is required.

## POSIX-to-Nushell Idiom Translation Table

| POSIX Idiom | Nushell Equivalent | Rationale / Explanation |
| :--- | :--- | :--- |
| `cmd1 && cmd2` | `cmd1; cmd2` or `if (cmd1) { cmd2 }` | Semicolon chains execution; conditional blocks evaluate success status |
| `cmd1 \|\| cmd2` | `cmd1 or cmd2` or `try { cmd1 } catch { cmd2 }` | Nushell boolean or / error handling block |
| `cmd 2>&1` | `cmd out+err>` or `do -i { cmd } \| complete` | Merges stdout and stderr into single stream or structured record |
| `cmd \| awk '{print $1}'` | `cmd \| get column_name` or `lines \| split column ' ' \| get column1` | Typed table column extraction preserves data fidelity |
| `grep pattern file` | `open file \| find "pattern"` or `lines \| find "pattern"` | Built-in text and stream filtering |
| `cat data.json \| jq '.foo'` | `open data.json \| get foo` | First-class JSON, TOML, YAML, CSV deserialization |
| `ps aux \| grep node` | `ps \| where name =~ "node"` | Filters structured process table rows by field |
| `ls -lh \| sort -k5 -n` | `ls \| sort-by size` | Sorts structured file metadata rows by size attribute |
| `$HOME`, `$PATH`, `$USER` | `$env.HOME`, `$env.PATH`, `$env.USER` | Environment variables live in the `$env` namespace |

## Vox Terminal Invocation

- In the Vox Console, TUI, or agent bridges:
  - `!nu ls | where size > 10kb | select name size modified`
  - `/nu sys host | select name os_version`
- Output is automatically captured structured as JSON records or tables when piped.
- If `nu` is not installed on the system, Vox provides an automatic POSIX diagnostic fallback.
