---
title: "First full-stack Vox app"
description: "Official documentation for First full-stack Vox app for the Vox language. Detailed technical reference, architecture guides, and implemen"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# First full-stack Vox app

Start from the smallest **golden** full-stack sample: [`examples/full_stack_minimal.vox`](../../../examples/reactive_counter.vox).

## What it demonstrates

- **`@component`** + React hooks (`use_state`)
- **`routes:`** for TanStack Router codegen
- **`http get`** for an Axum route
- **`@server fn`** for typed server functions and `api.ts`

## Commands

```bash
vox check examples/full_stack_minimal.vox
vox build examples/full_stack_minimal.vox -o dist
```

## TanStack Start (optional)

Align **`vox build`** with **`Vox.toml` `[web] tanstack_start = true`** or **`VOX_WEB_TANSTACK_START=1`** so TypeScript emits **`VoxTanStackRouter.tsx`** instead of nested SPA `App.tsx`. See [TanStack SSR with Axum](tanstack-ssr-with-axum.md) and [vox-web-stack SSOT](../reference/vox-web-stack.md).

## Next steps

- Larger UI sample: [`examples/chatbot.vox`](../../../examples/reactive_counter.vox)
- Style rules: [`examples/STYLE.md`](../../../examples/STYLE.md)
- Web spine: [ADR 010](../adr/010-tanstack-web-spine.md)
