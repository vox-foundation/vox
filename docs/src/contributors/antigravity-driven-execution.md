---
title: "Driving Antigravity (agy) from Claude Code"
description: "How Claude Code drives Gemini Flash through the agy CLI to implement plan tasks headlessly: the guard hook, rules, driver loop, and the field-tested bugs to prevent before they happen."
category: "Contributors"
status: "current"
---

# Driving Antigravity (`agy`) from Claude Code

Claude Code plans, reviews and commits; Gemini Flash (`agy -p`, headless) does the transcription-heavy implementation of
one plan task per run. This page is the operating manual, distilled from the Phase 5 and model-routing runs of
2026-09-28 (about 13 driven tasks). Reference copies of every file in the kit are in
[`docs/agents/agy-driver-kit/`](../../agents/agy-driver-kit/).

## Why this split works, and where it does not

Flash is fast, cheap and faithful: it copies code blocks exactly and follows a numbered procedure. It does **not**
second-guess. So:

- **A wrong plan ships verbatim.** Two of the first three model-routing tasks in plan revision 1 could not pass as
  written (a fixture over the safety cap; a "guard" test that passed without the guard). Review the plan with the
  three-track pass (correctness, tests pre-mortem, simplicity/safety) *before* driving it.
- **Mutation proofs are the highest-value thing to put in a task.** A "break the guard, watch this test fail, restore"
  step turned a plan-author bug into a clean `DRIVE: STOPPED` instead of a silently vacuous test.
- **Stops are the system working.** Rule 5 ("prefer a stop to a silently shipped mistake") produced four correct stops.
  Budget five to ten minutes per stop for the fix.

## The kit (local, gitignored: `.agents/`)

`.agents/` is gitignored in this repo, so the live kit is per-checkout. Recreate it with:

```bash
mkdir -p .agents/rules .agents/skills/drive-task .agents/scripts .agents/runs
cp docs/agents/agy-driver-kit/hooks.json .agents/hooks.json
cp docs/agents/agy-driver-kit/rules/00-core.md .agents/rules/00-core.md
cp docs/agents/agy-driver-kit/skills/drive-task/SKILL.md .agents/skills/drive-task/SKILL.md
cp docs/agents/agy-driver-kit/scripts/agent-guard.mjs .agents/scripts/agent-guard.mjs
node .agents/scripts/agent-guard.mjs --self-test
```

| File | Role |
|---|---|
| `.agents/hooks.json` | PreToolUse hook: `node scripts/agent-guard.mjs` (runs with cwd = `.agents/`). The CLI enforces a hook `deny` even with `--dangerously-skip-permissions`. |
| `.agents/scripts/agent-guard.mjs` | Mechanical guard. Denies index/history git commands, `cargo fmt`/`fmt.vox`, dependency changes, `pre-push`, network, inline code, `.env`, and writes to `.agents/`, `.planning/`, `Cargo.lock`, the crate-edge and fan-in ledgers and `layers.toml`. 45 self-test cases. |
| `.agents/rules/00-core.md` | Always-on contract (scope, order, proof, stop conditions, git, foreground-only, big contract files). |
| `.agents/skills/drive-task/SKILL.md` | `/drive-task <plan> <task>`: preconditions, steps, `DRIVE: READY` / `DRIVE: STOPPED …`. |
| `scripts/drive.mjs` | Driver: runs `agy -p`, saves the JSON log, reports changed files, out-of-scope files and any file with more than 400 changed lines. Takes a phase id (`05-03`) or a repo-relative plan path. |
| `scripts/agy-watchdog.sh` | Emits `STALL:` when the agent's own log shows no model call for 12 minutes. |
| `scripts/drift-fix.sh` | Runs `vox ci ssot-drift`, executes each `--write` fix it names, repeats. |

## The loop (per task)

1. **Probe once per session.** `agy -p "run: git add --dry-run README.md ."` with `--model gemini-3.8-flash-low`: the guard must deny it and the agent must recite rule 5. About 12 seconds.
2. **Drive.** `node .agents/scripts/drive.mjs <plan|path> <task> --note "<extra guidance>"` in the background, and arm the watchdog as a Monitor.
3. **Never trust the summary.** Re-run every `<verify>` command yourself, read the whole diff, and view any screenshot. Check `git diff --numstat` against the size the task should produce.
4. **Fix or re-drive.** A stop caused by the plan is fixed in the plan first; a small slip is fixed by hand and noted.
5. **Commit yourself, by pathspec** (`git add -- <files>` then `git commit -- <files>`). The agent is denied all index commands because the working tree is shared with other sessions.
6. **Gates at plan end**, not per task: clippy on touched crates, then the fast `vox ci pre-push` tier (about 11 minutes), then regenerate whatever inventory it names.

## Prevent these before they happen

| # | What happened | Prevention |
|---|---|---|
| 1 | The agent started `playwright test` in the background, ended its turn "waiting", and the run sat for 57 minutes (headless has no wake-up). | Rule: foreground only, every build/test prefixed with `timeout`, exit 124 means STOP. Driver `--print-timeout 60m`. Watchdog on model-call gaps. |
| 2 | The guard's `^`-anchored rules were bypassed by the very `timeout 1500s …` prefix rule 1 tells the agent to use (`timeout 30s git add -A` would run). | The guard strips wrappers (`timeout [-k N] DUR`, `env X=`, `nice`, `command`, `exec`, `nohup`, `time`) before matching. **Every instruction you add is a new input shape the guard must be tested against.** |
| 3 | The agent re-serialized a 15,000-line generated YAML (3,745 lines changed, rows reordered, a header dropped) while every test passed. | Rule 11: one targeted edit in big contract files, generators are run not imitated. The driver lists any file over 400 changed lines. Look at `git diff --numstat`, always. |
| 4 | The plan's fixture could not fail (a test that passed without the guard it was meant to prove). | Review the plan before driving. Make mutation proofs a step. A RED log that says `passed` is a broken step (rule 2). |
| 5 | The RED log was recorded after implementing. | Rule 2: write tests, run, save the failing output, and only then touch implementation. |
| 6 | Plan tasks contain "commit" steps written for Claude executors. | The skill and a `--note` say commits are Claude's; for "commit X before changing Y" plans use a **split run** (part A = only step N, Claude commits, part B = the rest). Part A of the receipt-MAC pin took 5 minutes. |
| 7 | `ssot-drift` stops at the first stale generated file, one 3-minute cargo run per file. | `drift-fix.sh` loop (inline it if your shell allowlist blocks `source`). List every generator output in the task's Files, or say "generated outputs are in scope". |
| 8 | AGENTS.md is auto-loaded as a rule and truncated from 60 KB to 24 KB, so late sections are invisible to the agent. | Put anything the agent must know in `.agents/rules/`, not only in AGENTS.md. |
| 9 | Another session's `agy` run was on the same machine; a "newest log" heuristic watched the wrong log. | Pick the log the agent's pid holds open (`lsof -p`). macOS `pgrep` has no `-E`. |
| 10 | Pre-commit hooks rebuild `vox-cli` (the fmt-fix hook), so a commit takes 4+ minutes. | Chain commit and the next drive in one background command; make the watchdog wait for the agent to appear. |
| 11 | zsh does not word-split `$F`; Claude-side shell allowlists block `zsh`, `source`, `cmp`, `perl -pi`, `node -e`. | `${=F}`, `diff -q`, `sed -i ''`, and script files instead of inline code. |
| 12 | The agent guessed a fixture value when the plan had a `<…>` marker. | Plans must not leave placeholders; the skill says a `<…>` must be replaced with a real value before anything runs, and the planner fills data-dependent ones (contract slugs) before driving. |

## Choosing a model

`gemini-3.8-flash-high` for Rust refactors and anything with generated-file fallout; `-medium` for single-file
TypeScript tests (5 to 20 minutes, similar quality); `-low` only for the guard probe. Token use is 300k to 1.5M per
task, dominated by reading; tell the agent to `rg -n` rows in big files instead of opening them.

## Plan format that drives well

Use the `superpowers:writing-plans` standard: exact paths, a **Files** list including every generated output, an
**Interfaces** block (what earlier tasks produce, exact signatures), failing test code in full, an adversarial case per
task, a mutation step for every guard or filter, a `Commit (Claude Code)` block, and STOP conditions for existing tests
that break. Give data-dependent values as literals, not markers. State the sequence and shared files.

## Progress reporting while driving

Keep a ledger (agent minutes, tokens, result, review notes) and report a moving-average ETA every 15 minutes:
`(sum of agent minutes + review and commit minutes) / tasks done × tasks remaining`. Throw out hang and fix-up outliers
when quoting a range, but say they happened.
