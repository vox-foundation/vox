---
title: "Architecture Decision Records (ADR)"
description: "Official documentation for Architecture Decision Records (ADR) for the Vox language. Detailed technical reference, architecture guides, a"
category: "reference"
last_updated: 2026-04-01
training_eligible: true
---
# Architecture Decision Records (ADR)

This directory contains ADRs for the Vox project.

| ADR | Title |
|-----|--------|
| [001](001-burn-backend-selection.md) | Burn backend selection |
| [002](002-diataxis-doc-architecture.md) | Diátaxis doc architecture |
| [003](003-native-training-over-python.md) | Native training over Python |
| [004](004-codex-arca-turso-ssot.md) | **Codex over Arca over Turso (storage SSOT)** |
| [005](005-socrates-anti-hallucination-ssot.md) | **Socrates anti-hallucination (confidence SSOT)** |
| [006](006-mens-full-graph-qlora-qlora-rs.md) | **Mens full-graph Candle QLoRA (qlora-rs)** |
| [007](007-qlora-rs-multi-layer-training-api.md) | **qlora-rs 1.0.5 multi-layer training API gate** |
| [008](008-populi-transport.md) | **Mens control plane (HTTP; TLS at edge)** |
| [009](009-populi-hosted-baas.md) | **Hosted mens / BaaS (future trust model)** |
| [010](010-tanstack-web-spine.md) | **TanStack web spine (Router → Start, SSR topology)** |
| [011](011-scientia-publication-ssot.md) | **Scientia publication manifest SSOT** |
| [012](012-internal-web-ir-strategy.md) | **Internal web IR strategy for Vox frontend emission** |
| [013](013-openclaw-ws-native-strategy.md) | **OpenClaw WS-first native interop** |
| [014](014-async-openai-selective-adoption-spike.md) | **`async-openai` selective adoption (spike / no-go)** |
| [015](015-vox-docker-oci-portability-ssot.md) | **Vox Docker/OCI portability SSOT** |
| [016](016-oratio-streaming-whisper-and-constrained-decode.md) | **Oratio streaming Whisper + constrained decode** |
| [017](017-populi-lease-remote-execution.md) | **Populi lease-based authoritative remote execution (design intent)** |
| [018](018-populi-gpu-truth-layering.md) | **Populi GPU truth layering (verified vs policy labels)** |
| [019](019-durable-workflow-journal-contract-v1.md) | **Durable workflow journal contract v1 (interpreted runtime)** |
| [020](020-populi-mesh-scaling-transport-default.md) | **Populi mesh scaling transport default** |
| [021](021-generated-workflow-durability-parity.md) | **Generated workflow durability parity** |
| [022](022-orchestrator-bootstrap-and-daemon-boundaries.md) | **Orchestrator bootstrap factory + daemon boundaries** |
| [023](023-optional-telemetry-remote-upload.md) | **Optional telemetry remote upload (explicit CLI, Clavis, local spool)** |

See also: [Internal Web IR implementation blueprint](../architecture/internal-web-ir-implementation-blueprint.md), [WebIR operations catalog](../architecture/internal-web-ir-implementation-blueprint.md#operations-catalog-op-0001op-0320), [WebIR supplemental execution map](../architecture/internal-web-ir-implementation-blueprint.md#supplemental-execution-map-op-s050-op-s103-op-s149-op-s182), [Acceptance gates G1–G6](../architecture/internal-web-ir-implementation-blueprint.md#acceptance-gates-specific-filetest-thresholds), [Internal Web IR side-by-side schema](../architecture/internal-web-ir-side-by-side-schema.md), [WebIR appendix — tooling registry](../architecture/internal-web-ir-side-by-side-schema.md#appendix--tooling-registry-and-offline-gates-op-s049-op-s101-op-s102-op-s181), [WebIR K-complexity quantification](../architecture/internal-web-ir-side-by-side-schema.md#k-complexity-quantification), [WebIR K-metric appendix](../architecture/internal-web-ir-side-by-side-schema.md#k-metric-appendix-reproducible), [Codex vNext schema](../architecture/codex-vnext-schema.md), [Codex BaaS](../architecture/codex-baas.md).
