---
schema_version: 1
open_count: 1
waived_count: 0
fixed_count: 0
total_count: 1
last_updated: 2026-10-27T05:00:49.953Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 316 | unrun-verify | .planning/phases/316-before-after-comparison-bench-01/316-01-PLAN.md |  | command-compliance not run: vox-cli fails to compile at vox-orchestrator-mcp on this branch; installed vox is stale | open |  | 2026-10-27T05:00:49.953Z |  |

````json
[
  {
    "id": 1,
    "kind": "unrun-verify",
    "phase": "316",
    "file": ".planning/phases/316-before-after-comparison-bench-01/316-01-PLAN.md",
    "line": null,
    "description": "command-compliance not run: vox-cli fails to compile at vox-orchestrator-mcp on this branch; installed vox is stale",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-27T05:00:49.953Z",
    "resolved_at": null,
    "milestone": "v5.6"
  }
]
````
