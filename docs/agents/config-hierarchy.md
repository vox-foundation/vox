# Vox Config Hierarchy — Single Source of Truth

Configuration in the Vox ecosystem flows through a strict precedence chain. Understanding
this chain is essential for correct behavior across all three layers (Extension, Orchestrator, CLI).

## Precedence Chain (highest → lowest)

```
1. CLI flags      (--model, --budget-limit, etc.)
2. ENV vars       (VOX_MODEL, VOX_BUDGET_USD, OPENROUTER_API_KEY, ...)
3. Vox.toml       (workspace-level, committed to repo)
4. ~/.vox/        (global user config, machine-local)
5. Compiled defaults  (primarily `Default` impls in `crates/vox-config/src/config/vox_config.rs`)
```

VS Code settings (`vox.*`) are **UX overrides** for items that have no workspace equivalent
(e.g., `vox.vcsShowSnapshotBar`, `vox.mcpBinaryPath`). They are not part of the
precedence chain for toolchain config.

## Canonical Ownership Map (`vox-config` + domain config modules)

Prefer source pointers over copied struct snapshots, since fields evolve frequently:

- `crates/vox-config/src/config/vox_config.rs`: `VoxConfig` type + `Default` values.
- `crates/vox-config/src/config/impl_ops.rs`: merge/load behavior and env/file overlay operations.
- `crates/vox-config/src/env_parse.rs`: shared env parsers for typed config values.
- `crates/vox-config/src/inference.rs`: inference profile and provider endpoint/env resolution.
- `crates/vox-config/src/rollout.rs`: rollout/kill-switch env helpers.
- `crates/vox-config/src/paths.rs`: canonical data/config path resolution.
- `crates/vox-db/src/config.rs`: DB connection config/precedence (separate domain owner; not a `VoxConfig` field mirror).
- `crates/vox-clavis/src/lib.rs`: secret canonical names/aliases (separate from non-secret tuning config).

`VoxConfig::load()` remains the entrypoint for workspace/user config resolution; secrets and DB wire config are intentionally owned by Clavis / `vox-db`.

## Vox.toml (Workspace Level)

```toml
[vox]
model = "anthropic/claude-sonnet-4"
daily_budget_usd = 10.0

[train]
data_dir = "target/dogfood"
epochs = 3

[db]
# url = "libsql://..." # optional remote DB
```

## Global User Config (`~/.vox/config.toml`)

Same schema as Vox.toml. Contains machine-local secrets and preferences not committed to git.

## ENV Variables

| Variable | Maps to |
|---|---|
| `OPENROUTER_API_KEY` | `VoxConfig.openrouter_key` |
| `OPENAI_API_KEY` | `VoxConfig.openai_key` |
| `GEMINI_API_KEY` | `VoxConfig.gemini_key` |
| `VOX_MODEL` | `VoxConfig.model` |
| `VOX_BUDGET_USD` | `VoxConfig.daily_budget_usd` |
| `VOX_DATA_DIR` | `VoxConfig.data_dir` |
| `VOX_DB_URL` | `VoxConfig.db_url` |
| `VOX_MCP_BINARY` | `VoxConfig.mcp_binary` |

## Accessing Config

**From CLI:**
```bash
vox config get model
vox config set daily_budget_usd 20.0
```

**From Orchestrator (MCP):**
```json
{ "tool": "vox_config_get", "params": { "key": "model" } }
```

**From VS Code extension:**
```typescript
const config = await mcp.call<VoxConfigResponse>('vox_config_get', { key: 'model' });
// Never use: vscode.workspace.getConfiguration('vox').get('model')
// for shared settings — use MCP instead.
```

## What VS Code Settings Own (UX-only)

These are not part of the VoxConfig SSOT. They are purely editor preferences:

| VS Code Setting | Purpose |
|---|---|
| `vox.mcpBinaryPath` | Override path to `vox-mcp` binary |
| `vox.vcsShowSnapshotBar` | Toggle VCS snapshot sidebar panel |
| `vox.statusBarVisible` | Show/hide the status bar item |
| `vox.inlineGhostText` | Enable/disable tab ghost text |
| `vox.outputChannelVerbosity` | Extension log level |
