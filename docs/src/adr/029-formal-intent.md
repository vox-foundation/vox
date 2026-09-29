---
title: "ADR-029: Formal Intent and Tool Receipt Auditing"
description: "Architecture Decision Record for formal intent and cryptographic tool receipts in the Vox ecosystem."
category: "Architecture Decisions (ADRs)"
status: "current"
---
# ADR 029: Formal Intent and Tool Receipt Auditing

> Renumbered from ADR-024 on 2026-05-02 to resolve a duplicate-number collision with [`024-dashboard-axum-spa`](024-dashboard-axum-spa.md).

## Status
Accepted (2026-09-29) — proposed 2026-04-23, ratified after Phase 5 implemented it.

## Context
AI agents in the Vox ecosystem perform high-stakes operations (file edits, VCS commits, database writes). Hallucinations and autonomous loops can lead to corrupted codebases and budget exhaustion. We need a way to verify that a tool call was explicitly intended by the orchestrator and actually executed as reported.

## Decision
We implement a two-tier verification system:
1. **Formal Intent**: Agents must claim a "receipt" for every tool call they wish to report as successful.
2. **Cryptographic Tool Receipts**: The orchestrator issues HMAC-signed receipts for every tool execution it brokers. Agents include these receipt IDs in their task completion claims.

## Consequences
- Agents cannot hallucinate tool outputs that were never executed.
- Socrates (hallucination defense) can explicitly check for "fabricated" claims.
- Auditing logs gain a cryptographic trail for every side-effect in the repository.

## Implementation (Phase 5)

- **Receipts on every brokered call** (D-01): `handle_tool_call_with_receipt` in `crates/vox-orchestrator-mcp/src/dispatch.rs` issues a receipt through `ToolReceiptLedger` (`crates/vox-orchestrator/src/tool_receipt.rs`) for each tool call that passes the dispatch gates. A call rejected by a gate (scope, budget) gets no receipt.
- **Crypto through `vox-crypto`** (D-04): `receipt_mac` computes the receipt MAC with `vox_crypto::facades::keyed_hash`; a test pins the MAC bytes so the construction cannot drift silently. The session key comes from `tool_ledger_key` in the orchestrator config when set, otherwise it is generated per process.
- **Claim verification tool** (D-03): `vox_verify_task_claims` (`crates/vox-orchestrator-mcp/src/receipt_tools.rs`) checks receipt ids an agent claims against the ledger via `validate_agent_claims`. It is registered in the tool catalog as `verify.task.claims` and is not exposed to the HTTP read role.
- **Chat surface** (D-10, D-11): the chat turn emits a receipt event and a claims event (`receipt_turn_event` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`); `ChatTurnEventRow.tsx` renders them as `tool_receipt` and `receipt_claims` chips, covered by `crates/vox-gui/ui/e2e/chat-trust-chips.spec.ts`.

### Known limitations

- Fail-open, and not an authorization gate (D-02): a missing or unverifiable receipt does not block a tool call or a task completion.
- Detection only (D-03): a fabricated claim is reported as such; no automatic consequence (task failure, agent penalty) follows.
- The ledger contents are in memory and per process: receipts do not survive a restart and are not visible to another process. `tool_ledger_key` only makes the MAC key stable across restarts; it does not persist the receipts themselves.
