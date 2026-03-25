# MCP tool registry (contract SSOT)

Machine-readable **MCP tool names and descriptions** live in the repository at:

**[`contracts/mcp/tool-registry.canonical.yaml`](../../../contracts/mcp/tool-registry.canonical.yaml)** (from repo root)

Rust code consumes this file via **`crates/vox-mcp-registry`** (`build.rs` emits `TOOL_REGISTRY`).  
`vox-mcp`, `vox-corpus`, and `vox-mcp-meta` re-export that table — do not hand-edit duplicate lists.

- Regenerate YAML from a legacy extract (one-time): `python scripts/extract_mcp_tool_registry.py write`
- Compliance: `vox ci command-compliance` checks YAML ↔ `handle_tool_call` wiring.

See also [`contracts/README.md`](../../../contracts/README.md) and [SSOT convergence roadmap](../architecture/ssot-convergence-roadmap.md).
