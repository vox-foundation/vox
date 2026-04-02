# Antigravity Overlay (Windows + PowerShell)

This file is Antigravity-specific. It narrows behavior for this repo without replacing `AGENTS.md`.

## Scope

- Treat `AGENTS.md` as the cross-tool base policy.
- Use this file only for Antigravity-specific behavior and shell discipline.
- Keep rules concise, concrete, and executable.

## Shell Environment

- Workspace environment is Windows; **PowerShell is canonical** here.
- Repo-wide doctrine (see [`AGENTS.md`](AGENTS.md)): on **any** OS, prefer **`pwsh`** for terminal/agent shell work when installed, so behavior aligns with `vox shell check` and `contracts/terminal/exec-policy.v1.yaml`.
- Prefer PowerShell-native commands for filesystem and process tasks.
- Use project tools directly (`vox`, `cargo`, `pnpm`, `uv`, `rg`, `git`) instead of shell wrappers.

## Command Shape Rules (Important)

- Emit one terminal command per step by default.
- Do not chain commands with `|`, `&&`, `;`, or `||` unless explicitly required.
- Do not wrap routine commands in `bash -lc` or nested shell invocations.
- If a task is multi-step, execute it as separate terminal calls.

Reason: command approval/allowlist matching in current IDE ecosystems is often brittle on compound commands, especially in PowerShell contexts.

Research synthesis (Cursor, Gemini, Codex, PowerShell, bypass classes, future Vox contract): [`docs/src/architecture/terminal-exec-policy-research-findings-2026.md`](docs/src/architecture/terminal-exec-policy-research-findings-2026.md).

## Tooling Preferences

- Search text: `rg`
- Filesystem listing and checks: `Get-ChildItem`, `Test-Path`, `Resolve-Path`
- File reads/writes from the IDE: use structured read/edit tools when available
- Package managers: `pnpm` for JS/TS, `uv` for Python

## Safety Posture

- Treat allowlists as convenience, not as a hard security boundary.
- Keep destructive operations explicitly denied in IDE policy where supported.
- When unsure, choose decomposition and explicitness over shell cleverness.
