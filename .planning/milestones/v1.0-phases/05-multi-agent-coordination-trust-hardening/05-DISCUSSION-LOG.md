# Phase 5: Multi-Agent Coordination & Trust Hardening - Discussion Log

> Audit trail only. Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-28
**Phase:** 05-multi-agent-coordination-trust-hardening
**Areas discussed:** TRUST-01 dispatch wiring, Two-tier formal-intent semantics, ResourceLockManager gaps, Cross-node vs single-process scope

---

## Scout findings presented up front

Both ADR-025 and ADR-029's core types (`ResourceLockManager`, `ToolReceiptLedger`) are already
fully implemented and unit-tested, but neither has a real caller in the live dispatch/task path.
The user selected all four areas to discuss given this, plus both housekeeping items.

## TRUST-01 dispatch wiring — which tools get a receipt

| Option | Selected |
|--------|----------|
| All tools, uniformly | ✓ |
| Mutating/write tools only | |

## TRUST-01 — fail mode on issue_intent error / fabricated verdict

| Option | Selected |
|--------|----------|
| Fail-open, log only | ✓ |
| Fail-closed on unknown tool | |

## Two-tier formal-intent — verdict surface

| Option | Selected |
|--------|----------|
| New read-only MCP tool | ✓ |
| CLI subcommand instead | |
| Neither yet — library only | |

## ResourceLockManager — real caller

| Option | Selected |
|--------|----------|
| Hopper task dispatch | ✓ |
| Something else | |

## ResourceLockManager — eviction sweep design

| Option | Selected |
|--------|----------|
| Lazy sweep on every acquire | ✓ |
| Background timer task | |

## Cross-node vs single-process scope

| Option | Selected |
|--------|----------|
| Single-process only | ✓ |
| Needs cross-node too | |

## Hopper task dispatch + locks — resource_id source

| Option | Selected |
|--------|----------|
| Explicit field on the task | ✓ |
| Derived from task content | |

## Housekeeping

| Option | Selected |
|--------|----------|
| Ratify ADR-025 and ADR-029 | ✓ |
| Fix tool_receipt.rs's crypto-policy violation | ✓ |

## Chat-GUI surfacing (user directive, mid-planning)

User: changes and orchestration must surface at the chat level, testable with our instruments —
not the orchestrator built in isolation, but surfaced up to the chat GUI so chat changes can be
worked with and visualized. Captured as D-10/D-11.

## Claude's Discretion
- Exact MCP tool name/schema for the verify surface (working name `vox_verify_task_claims`).
- Exact shape of the `resource_id` field addition to the task/intake spec.
- Whether the eviction sweep also runs on `release`, not just acquire/is_locked.

## Deferred Ideas
- Cross-node/mesh-backed resource locking.
- Automatic consequences for a fabricated tool-claim verdict.
- Background-timer-based lock sweeping.
