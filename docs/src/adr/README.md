---
title: "Architecture Decision Records (ADR)"
description: "Official documentation for Architecture Decision Records (ADR) for the Vox language. Detailed technical reference, architecture guides, a"
category: "reference"
last_updated: 2026-03-26
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
| [008](008-mens-transport.md) | **Mens control plane (HTTP; TLS at edge)** |
| [009](009-mens-hosted-baas.md) | **Hosted mens / BaaS (future trust model)** |
| [010](010-tanstack-web-spine.md) | **TanStack web spine (Router → Start, SSR topology)** |
| [011](011-scientia-publication-ssot.md) | **Scientia publication manifest SSOT** |
| [012](012-internal-web-ir-strategy.md) | **Internal web IR strategy for Vox frontend emission** |

See also: [Internal Web IR implementation blueprint](../architecture/internal-web-ir-implementation-blueprint.md), [Internal Web IR side-by-side schema](../architecture/internal-web-ir-side-by-side-schema.md), [Codex vNext schema](../architecture/codex-vnext-schema.md), [Codex BaaS](../architecture/codex-baas.md).
