## Summary

<!-- What changed and why (complete sentences). -->

## Gamify (if applicable)

- [ ] New event `type` has **`base_reward`** and **`process_event_rewards`** companion/quest/counter behavior _or_ is documented as policy-only in [`docs/src/archive/research-2026-q1/agent-event-kind-ludus-matrix.md`](../docs/src/archive/research-2026-q1/agent-event-kind-ludus-matrix.md) (archived — add new event types to commit message or a living replacement doc until a current matrix is established).
- [ ] If the signal is a mistake/nudge, **`teaching_hook`** in `event_router` is updated (or explicitly N/A).
- [ ] MCP tools: [`contracts/mcp/tool-registry.canonical.yaml`](../contracts/mcp/tool-registry.canonical.yaml) + `vox-mcp` dispatch/schemas if adding or renaming tools.
- [ ] Env vars: [`docs/src/reference/env-vars.md`](../docs/src/reference/env-vars.md) Gamify section if new `VOX_LUDUS_*` knobs were added.

## Testing

<!-- e.g. `cargo test -p vox-gamify`, `cargo test -p vox-mcp`, etc. -->

## Data Storage / DB

- [ ] If `crates/vox-db/src/schema/manifest.rs` was touched, include `BASELINE_VERSION: <new_value>` in this PR body (matching the Rust constant) to auto-update the baseline contract post-merge.
