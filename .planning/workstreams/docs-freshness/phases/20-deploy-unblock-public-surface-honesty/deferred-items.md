# Phase 20 — Deferred Items

Out-of-scope discoveries logged by executors. Not fixed in the plan that found them.

| Found in | File | Issue |
|---|---|---|
| 20-03 | `docs/src/contributors/documentation-governance.md` ("Frontmatter starter template") | The template claims "docs lint passes on first run" but pairs `status: "roadmap"` with `training_eligible: true` and no `training_rationale:`; `vox-doc-pipeline` rejects that combination (`missing-training-rationale`). Add a `training_rationale:` line to the template or switch its example status to `current`. |
| 20-03 | Plan 20-03 Task 1 verify | `grep -h '^status:'` over the deep-research spec also matches an example frontmatter block in the spec body (line ~89, `status: "current"`), so the command prints four lines. Frontmatter-only check (`rg -m1 '^status:'` per file) gives the intended three. Verify-command defect, not a content defect. |
