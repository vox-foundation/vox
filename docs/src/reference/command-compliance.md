---
title: "Command compliance"
description: "Official documentation for Command compliance for the Vox language. Detailed technical reference, architecture guides, and implementation"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Command compliance

**`vox ci command-compliance`** validates the machine-readable registry **`contracts/cli/command-registry.yaml`** (JSON Schema: **`contracts/cli/command-registry.schema.json`**) against:

| Check | Source |
|-------|--------|
| Top-level `vox` subcommands exist in `Cli` | `crates/vox-cli/src/lib.rs` |
| Doc needles for `ref_cli_required` operations | `docs/src/ref-cli.md` if present, else canonical `docs/src/reference/cli.md` — checks **always** run (no skip). `vox ci …` and `vox codex` subcommands are validated **only inside** their `### \`vox ci …\`` / `### \`vox codex\`` sections (not whole-file substring matches) |
| Top-level reachability table rows | `docs/src/reference/cli.md` under **CLI command reachability** (legacy `cli-reachability.md` merged there; rows skipped for `completions`, `fabrica`, `mens`, `ars`, `recensio`, and when `reachability_required: false`) |
| Compiler daemon RPC method names | `crates/vox-cli/src/compilerd.rs` |
| DeI daemon RPC method ids | `crates/vox-cli/src/dei_daemon.rs` |
| MCP tool names vs `handle_tool_call` arms | `crates/vox-mcp/src/tools/mod.rs` — `TOOL_REGISTRY` names from the value array (`[` … `]` bracket scan); handler arms parsed inside `match name { … }` up to the first line that matches `^\s*_\s*=>` (indent-tolerant), collecting every `"(vox_…)"` literal on each arm line (aliases are **not** duplicated in `match`: they live in `crates/vox-mcp/src/tools/tool_aliases.rs` as `TOOL_WIRE_ALIASES`, normalized before `match`) |
| Script duals | `command-surface-duals.md` or `scripts/README.md` must mention each `script_duals` canonical CLI and script stem |

**CI:** `.github/workflows/ci.yml` runs this gate after **`vox ci check-docs-ssot`** (after **`vox ci line-endings`** and other early guards; see [workflow enumeration](../ci/workflow-enumeration.md)).

**Definition of done** for a new shipped CLI operation: registry row + docs + **`command-compliance`** green (see [`cli-design-rules.md`](cli.md)).
