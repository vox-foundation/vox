---
trigger: always_on
description: Core execution contract for headless agents driven by Claude Code in the vox repo
---

# Core contract (read every turn)

You execute ONE task from a GSD plan file under `.planning/phases/`. Claude Code names the plan and the task
number. That task is the whole job.

1. **Scope.** Touch only the files the task lists (its `<files>` block, plus a test file it names). Do not
   refactor, rename, reformat or "improve" anything else. A needed change outside the list is a STOP.
2. **Order.** Do the task's steps in order. A test-first step that says the test must fail must be run and
   must fail before you implement: write the tests, run them, save the failing output to the named file, and only
   then touch implementation code. A RED log that shows `passed` is a broken step, not a formality.
3. **Small edits.** Edit only the lines the task names. Never rewrite a whole existing file.
4. **Proof, not claims.** Never say a test passes or a command worked without running it in this
   conversation and pasting its raw output (tail is fine for long cargo output).
5. **Stop conditions — end with `DRIVE: STOPPED ...`, do not improvise:**
   - a path, function, type, field or line the task names does not exist or does not match;
   - the same failure after 3 attempts;
   - the task would need a new crate dependency, a crate-edge exception, a fan-in/layers.toml change,
     a Cargo.lock change, a network call, or a secret;
   - two instructions conflict, or the task's approach looks wrong (say why in one line — Claude Code
     prefers a stop to a silently shipped mistake).
6. **Git.** Never run `git add`, `commit`, `stash`, `checkout`, `restore`, `reset`, `clean`, `push`. Other
   sessions share this working tree and index. Claude Code verifies and commits your work.
7. **Formatting.** Never `cargo fmt` or `scripts/fmt.vox`; run `rustfmt --edition 2024 <each .rs file you
   changed>`.
8. **Cargo.** Plain `cargo` only (it is a build-broker shim). Scope builds/tests with `-p <crate>`. Never
   `cargo test --workspace`.
9. **Headless = foreground only.** Nobody will wake you when a background command finishes, so never
   run a command in the background and never end a turn "waiting". Prefix every build/test command with
   `timeout` (`timeout 900s cargo test ...`, `timeout 300s pnpm --dir crates/vox-gui/ui exec playwright
   test ...`). Exit 124 means it hung: STOP with the command name, do not retry it the same way.
10. **Never delete** files unless the task says to delete exactly that path.
11. **Big contract files (`contracts/**`, 1k+ lines).** Add a row with one targeted edit at its sorted
    position; never re-serialize, re-sort or rewrite the file (a rewrite dropped `x-vox-version` and
    reordered 650 rows once). Find rows with `rg -n`; never open a whole generated file. Files a
    generator writes (`operations-sync --write`) are produced by running the generator, never by hand.
    Check `git diff --numstat -- contracts/` afterwards: a one-row change is tens of lines, not thousands.
12. **Secrets.** Never open or print `.env*` (except `.env.example`).
