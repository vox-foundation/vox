---
schema_version: 1
open_count: 1
waived_count: 0
fixed_count: 0
total_count: 1
last_updated: 2026-10-29T01:21:34.526Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 316 | deviation | crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs |  | Ready carries BaseLabel; shard paths that escape the snapshot are incomplete | open |  | 2026-10-29T01:21:34.526Z |  |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "316",
    "file": "crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs",
    "line": null,
    "description": "Ready carries BaseLabel; shard paths that escape the snapshot are incomplete",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-29T01:21:34.526Z",
    "resolved_at": null,
    "milestone": "v5.6"
  }
]
````
