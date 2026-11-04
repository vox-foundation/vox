---
title: "Jev / System One for Vox harness & orchestrator — research 2026-10-28"
description: "Jev/System One for Vox: redundancy vs select/free_floor/hub-spoke, better harness surfaces, Oct 2026 research."
category: "Architecture SSOTs"
status: "research"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Jev / System One for Vox harness & orchestrator — research 2026-10-28

<!-- kb:summary -->
Research on integrating Jev System One style harness judgments into Vox agent
orchestration. Covers typed judgment surfaces, probability routing, and how
harness outputs should bind to existing vox-orchestrator model selection without
vendor lock-in. Status is research; not a shipped SSOT contract.
<!-- /kb:summary -->

## Status

This page is a **research** note, not a shipped SSOT. Prefer feeding calibrated
judgments into existing `build_selection_request` / repair / stop hooks rather
than a new crate or a PreferFree rewrite of `FREE_FLOOR`.

## Operator constraints (summary)

- No public Jev weights or self-host binary in-tree; treat any API as optional egress.
- Do not distill or imitate proprietary Jev behavior into MENS training corpus without legal review.
- Keep model IDs and clutch policy under existing orchestrator SSOT; do not expose raw harness vendor IDs on the hot path by default.

## Related code (existing)

- `crates/vox-orchestrator/src/models/select.rs`
- `crates/vox-orchestrator/src/confidence_fusion.rs`
- `crates/vox-orchestrator/src/models/autonomic.rs`

## Follow-ups

1. Prototype a thin optional client behind the model-agnostic facade only.
2. Bakeoff design: latency/cost/quality axes already used by model registry scoring.
3. Do not expand crate-edges for a `vox-system-one` package until product commits.
