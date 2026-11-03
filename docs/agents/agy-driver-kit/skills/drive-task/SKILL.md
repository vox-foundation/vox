---
name: drive-task
description: Execute exactly one task of a GSD plan headlessly for the Claude Code driver (agy -p). Use when the prompt is /drive-task <plan path> <task number>.
---

# /drive-task <plan file> <task number>

You are running headless (`agy -p`). Nobody can answer questions. Claude Code is driving you: afterwards it
re-runs every check itself, reviews the diff, then commits. **Your job is the task's steps, with proof.**

1. Read the plan's header (goal, **Global Constraints**, file structure) and the whole of the named task — either a
   `<task>` block whose `<name>` starts `Task N:`, or a `### Task N:` section (its **Files**, **Interfaces** and
   checkbox steps). Only that task; later tasks are someone else's run. Read `AGENTS.md` sections the task cites.
   Code blocks in steps are to be transcribed exactly; a `<…>` marker must be replaced with a real value as the step
   says before anything is run. A step labelled "Commit (Claude Code)" is not yours: skip it.
2. **Preconditions.** For every file in the task's `<files>` and every symbol/line the task cites, run
   `rg -n "<symbol>" <file>` (or `ls <path>` for a file to create's parent dir) and paste the result.
   On any mismatch, end with `DRIVE: STOPPED step 0: <what did not match>`.
3. **Execute the steps in order.** Test-first steps: write the test, run it, paste the failing output,
   then implement. Run each `<verify>` command and paste its raw output (tail -40 for long output).
4. **Format** each `.rs` file you changed with `rustfmt --edition 2024 <file>`.
5. Run `git status --short` and paste it. Do not stage or commit.
6. The **last line** of your final message is exactly one of:
   ```
   DRIVE: READY
   DRIVE: STOPPED step <n>: <one-line reason>
   ```
   `DRIVE: READY` only if every `<verify>` and `<done>` criterion was met **in this conversation**.
   A false READY costs a rerun, not a shortcut.

Never edit files outside the task's `<files>` / **Files** list. Never touch `.agents/`, `.planning/`, `.env*`, `Cargo.lock`,
`contracts/ci/crate-edges.allow.v1.json`, `contracts/ci/fan-in-snapshot.v1.json`, `docs/src/architecture/layers.toml`.
