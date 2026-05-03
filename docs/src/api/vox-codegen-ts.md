---
title: "vox-codegen-ts API"
description: "API reference for the vox-codegen-ts crate: HIR-to-TypeScript/TSX code generation pipeline."
category: "api-crate"
status: current
---

# vox-codegen-ts API

The `vox-codegen-ts` crate translates Vox HIR into TypeScript/TSX output files.

## Overview

The codegen pipeline lowers HIR through the Web IR layer before final TypeScript emit.
See [internal-web-ir-implementation-blueprint.md](../architecture/internal-web-ir-implementation-blueprint.md)
and [internal-web-ir-side-by-side-schema.md](../architecture/internal-web-ir-side-by-side-schema.md)
for the full Web IR specification.

For the strategic rationale, see [ADR 012](../adr/012-internal-web-ir-strategy.md) (`adr/012`).

## Output files

| File | Description |
|------|-------------|
| `<Component>.tsx` | React component for each `component` declaration |
| `routes.manifest.ts` | Route manifest for TanStack Start / Vite adapter |
| `vox-client.ts` | Type-safe client hooks for `@endpoint(kind: query/mutation)` |
| `types.ts` | TypeScript union types for `type` declarations |
| `server.ts` | Express/Axum route handlers for `@endpoint(kind: server)` |
