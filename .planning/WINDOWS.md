---
schema_version: 1
open_count: 0
waived_count: 0
fixed_count: 2
total_count: 2
last_updated: 2026-11-13T18:10:00.000Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 316 | deviation | crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs |  | Ready carries BaseLabel; shard paths that escape the snapshot are incomplete | fixed | Verified & expanded in Phase 417 (`HYGIENE-TEST-01`: `Detected::Ready` `BaseLabel` + `../`, `sub/`, and absolute shard path rejection) | 2026-10-29T01:21:34.526Z | 2026-11-13T18:10:00.000Z |
| 2 | 317 | deviation | crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs |  | Resolver and contract tests need --features mens and --features mens-train or the harness runs zero tests | fixed | Fixed in Phase 417 (`HYGIENE-TEST-02`: `spoke_base_resolver` unit tests run with default features; `training_presets_yaml_contract` runs under `--features mens` without `mens-train`) | 2026-10-29T03:53:57.909Z | 2026-11-13T18:10:00.000Z |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "316",
    "file": "crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs",
    "line": null,
    "description": "Ready carries BaseLabel; shard paths that escape the snapshot are incomplete",
    "status": "fixed",
    "reason": "Verified & expanded in Phase 417 (HYGIENE-TEST-01)",
    "recorded_at": "2026-10-29T01:21:34.526Z",
    "resolved_at": "2026-11-13T18:10:00.000Z",
    "milestone": "v5.6"
  },
  {
    "id": 2,
    "kind": "deviation",
    "phase": "317",
    "file": "crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs",
    "line": null,
    "description": "Resolver and contract tests need --features mens and --features mens-train or the harness runs zero tests",
    "status": "fixed",
    "reason": "Fixed in Phase 417 (HYGIENE-TEST-02)",
    "recorded_at": "2026-10-29T03:53:57.909Z",
    "resolved_at": "2026-11-13T18:10:00.000Z",
    "milestone": "v5.6"
  }
]
````
