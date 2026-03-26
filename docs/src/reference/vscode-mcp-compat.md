# VS Code extension ↔ `vox-mcp` compatibility

## Single sources of truth

| Artifact | Role |
|----------|------|
| [`contracts/mcp/tool-registry.canonical.yaml`](../../../contracts/mcp/tool-registry.canonical.yaml) | Canonical MCP tool **names** (builds `vox-mcp-registry`) |
| [`vox-vscode/scripts/check-mcp-tool-parity.mjs`](../../../vox-vscode/scripts/check-mcp-tool-parity.mjs) | CI guard: every `call('…')` and `callTool({ name: … })` in extension sources resolves to the registry; aliases are parsed from [`tool_aliases.rs`](../../../crates/vox-mcp/src/tools/tool_aliases.rs) at check time |
| [`vox-vscode/scripts/generate-mcp-tool-registry.mjs`](../../../vox-vscode/scripts/generate-mcp-tool-registry.mjs) | Generates `mcpToolRegistry.generated.ts` (canonical name list + `MCP_EXTENSION_EXPECTED_TOOLS` for runtime warnings) on `npm run compile` |
| Runtime `list_tools` | **Actual** advertised tools (includes skill-merged tools); `CapabilityRegistry` stores a fingerprint |
| [`vox-vscode/src/protocol/hostToWebviewMessages.ts`](../../../vox-vscode/src/protocol/hostToWebviewMessages.ts) | zod schema for **host → webview** posts (`SidebarProvider.postMessage` validates before `postMessage`) |
| [`vox-vscode/scripts/smoke-host-messages.mjs`](../../../vox-vscode/scripts/smoke-host-messages.mjs) | Runs after `tsc` to ensure the host schema still accepts representative payloads |

## Wire aliases (match `vox-mcp` `TOOL_WIRE_ALIASES`)

- `vox_budget_history` → `vox_cost_history`
- `vox_model_list` → `vox_list_models`
- `vox_map_vscode_session` → `vox_map_agent_session`
- (etc. — keep parity script in sync with [`crates/vox-mcp/src/tools/tool_aliases.rs`](../../../crates/vox-mcp/src/tools/tool_aliases.rs))

## Extension settings

| Setting | Purpose |
|---------|---------|
| `vox.mcp.serverPath` | CLI binary for stdio (`vox mcp`) |
| `vox.mcp.debugPayloads` | Log tool args/results (truncated) to the **Vox** output channel |
| `vox.mcp.warnOnMissingTools` | Log when `list_tools` lacks names in generated `MCP_EXTENSION_EXPECTED_TOOLS` |

## Release checklist

1. Bump `vox-vscode` `package.json` version with the MCP/server bundle you test against.
2. `cd vox-vscode && npm run check:mcp-parity && npm run compile && npm run lint`
3. Manual smoke: connect MCP, open **Vox Workspace**, confirm status strip shows `execution_mode` and tool count.

## Compatibility matrix (manual)

| Extension version | Notes |
|-------------------|--------|
| 0.2.x | Expects `ToolResult` JSON envelope unwrapping, `vox_compiler::ast_inspect`, runtime capability strip |

Document the **pinned** `vox` / `vox-mcp` crate version per release in your rollout notes when cutting editor builds.

## Visual / webview regression

Automated Playwright against the embedded webview is **not** in-repo yet. Before release, manually verify **Vox Workspace** in **Default Dark**, **Light+**, and **High Contrast** themes: dashboard strip, Agent Flow (task graph + lifecycle buttons), and Pipeline tab. File an issue if you want `@vscode/test-web` coverage added to CI.
