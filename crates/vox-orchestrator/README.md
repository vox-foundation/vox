# vox-orchestrator

Multi-agent orchestration, resource-affinity scheduling, Socrates evidence gating, and context handoff engine for Vox (`v0.6.0` / `v6.8`).

## Modular Orchestrator Stack

The orchestrator architecture spans four layered crates (`docs/src/architecture/layers.toml`):

1. **`vox-orchestrator-types` (`L0`)**:
   - Foundational identifiers (`AgentId`, `TaskId`, `SessionId`), `TaskPriority`, `PricingSource`, and `ClutchProfile`.
2. **`vox-orchestrator-models` (`L2`)**:
   - Dynamic model registry, OpenRouter (`~family-latest`) + first-class local MENS routing, `ExternalBenchmarkPrior`, and Pareto/McNemar multi-objective model scoring.
3. **`vox-orchestrator` (`L2`)**:
   - `agent_loop`: Autonomous execution loop with tool dispatch and budget tracking.
   - `hopper`: SQLite-backed resource lock and task inbox (`hopper_inbox` `resource_id` locks per [ADR-025](../../docs/src/adr/index.md)).
   - `memory`: Working, episodic, and semantic agent memory management.
   - `socrates`: 5-stage epistemic evidence gate verifying claims before completion.
   - `context_handoff` & `checkpoints`: Structured inter-agent context transfers and resumable state snapshots.
4. **`vox-mcp-context` → `vox-mcp-dispatch` → `vox-orchestrator-mcp` (`L3`)**:
   - 376-tool Model Context Protocol (MCP) server with cryptographic HMAC-SHA256 tool execution receipts per [ADR-029](../../docs/src/adr/index.md).

## Key Modules (`crates/vox-orchestrator/src/`)

| Module | Purpose |
|--------|---------|
| `agent_loop/` | Turn execution, retry policies, and model-agnostic LLM invocation (`vox_actor_runtime::llm`) |
| `hopper/` | SQLite-coordinated resource locking and priority task queue |
| `socrates/` | 5-stage evidence verification gate and anti-hallucination checks |
| `memory/` | Agent memory tiers and retrieval integration |
| `context_handoff.rs` | Structured handoff envelopes between parent and subagent sessions |
| `checkpoints.rs` | Durable task checkpoints and rollback support |
| `affinity.rs` | File and resource ownership routing |

## Canonical References

- AgentOS / ACI SSOT: [`docs/src/architecture/agentos-ssot-2026.md`](../../docs/src/architecture/agentos-ssot-2026.md)
- Where Things Live: [`docs/src/architecture/where-things-live.md`](../../docs/src/architecture/where-things-live.md)
