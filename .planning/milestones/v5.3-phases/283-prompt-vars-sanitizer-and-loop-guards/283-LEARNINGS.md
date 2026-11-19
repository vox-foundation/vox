---
phase: 283
phase_name: "prompt-vars-sanitizer-and-loop-guards"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
  - "STATE.md"
---

# Phase 283 Learnings: prompt-vars-sanitizer-and-loop-guards

## Decisions

### Interpolate $variables on system_prompt (not just the user prompt), form...
Interpolate `$variables` on `system_prompt` (not just the user prompt), format `$current_datetime` as full ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SSZ`), and teach `detect_workspace_git_branch` to follow `gitdir:` when `.git` is a regular worktree file.

**Rationale:** Primary phase objective established in plan.
**Source:** 283-01-PLAN.md

## Lessons

## Patterns

## Surprises

