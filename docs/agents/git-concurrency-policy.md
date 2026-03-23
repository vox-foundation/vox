# Vox Git Concurrency & Integrity Policy

> [!IMPORTANT]
> The Vox workspace is often developed by multiple AI agents concurrently. This policy outlines rules and practices to strictly prevent agents from destroying or mangling each other's uncommitted work.

## 1. The Core Hazard
When multiple agents run in the same cloned repository (as is standard), they share the same Git index and working tree. 
In the past, agents encountering a "dirty working tree" would reflexively use `git stash`, `git restore .`, or `git clean -fd` to clear the workspace for their own task. 
**This results in the immediate, silent deletion of work being performed by concurrent agents.**

For example:
- Agent A is writing `CodeStore` updates and has uncommitted files.
- Agent B encounters a compilation error, tries to pull from main, and runs `git stash; git pull; git stash pop`.
- Due to merge conflicts on pop, Agent B might run `git restore .` out of confusion, permanently deleting Agent A's work.

## 2. Banned Commands (Zero Exceptions)

The following commands are **permanently banned** across all agents, human users, and automated workflows acting within the repository:
- `git stash` (and variants: `pop`, `drop`, `clear`)
- `git reset --hard`
- `git clean -fd`
- `git restore .` or `git checkout -- .`

*There is no exception for "I'll stash and pop right back." If a task requires a clean state, you must either commit your current work or switch branches.*

## 3. The Mandated Workflow: Branch Isolation

### A. Working on Tasks
1. **Never work directly on `main` for complex, multi-step changes.**
2. When starting a sizable task, checkout a unique branch: `git checkout -b agent/<your-task-slug>`.
3. If an agent task spans multiple prompt turns, **commit your WIP frequently**:
   ```bash
   git add path/to/changed/files
   git commit -m "wip: halfway through the cache refactor"
   ```

### B. Pulling Upstream Changes
If you need to sync with new changes from the remote:
1. Commit any currently uncommitted work as a `wip:` commit.
2. `git pull --rebase origin main` (or `git fetch` followed by `git rebase`).
3. If there are conflicts, resolve them manually or defer. **Do not run `git reset --hard` to escape conflicts.** Use `git rebase --abort` instead.

### C. Completing the Work
1. When finished, ensure all tests compile and pass.
2. If working on a branch, optionally interactive rebase (`git rebase -i HEAD~N`) to squash your `wip:` commits into a single logical commit.
3. Merge or open a PR depending on your operational parameters. 

## 4. Recovering Lost Work & Error Handling 

If you suspect you (or another agent) accidentally deleted or mangled files through an unauthorized command:
1. Use `git reflog` to identify previous HEAD states. Note that uncommitted, lost working-directory changes are rarely recoverable unless stashed.
2. If `git stash` was used recently, you can view the stash list with `git stash list` and inspect contents with `git stash show -p stash@{0}`. 
3. **Instead of blindly popping**, manually port the relevant changes from the stash diff into the codebase. 

## 5. Temporal Safeties

Expensive operations such as exhaustive searching, codebase-wide formatting, or full project rebuilds must not be executed blindly in a loop.
- **Always verify if the state has changed** in the last N seconds before re-running expensive ops.
- **Never auto-run build/test loops** without an exit condition.

## 6. Tooling Constraints
When editing files, **always prefer native file editing tools** (e.g. `multi_replace_file_content`, `replace_file_content`) available to the agent via the system prompt rather than resorting to arbitrary execution of `sed`, `awk`, or `echo >` commands. Advanced multi-file changes should leverage these granular AST-aware/line-aware tools to prevent collateral damage.
