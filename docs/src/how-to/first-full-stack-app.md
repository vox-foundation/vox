---
title: "How to: First full-stack Vox app"
category: how-to
last_updated: 2026-03-22
---

# First full-stack Vox app

Start from the smallest **golden** full-stack sample: [`examples/full_stack_minimal.vox`](../../../examples/full_stack_minimal.vox).

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

Align **`vox build`** with **`Vox.toml` `[web] tanstack_start = true`** or **`VOX_WEB_TANSTACK_START=1`** so TypeScript emits **`VoxTanStackRouter.tsx`** instead of nested SPA `App.tsx`. See [TanStack SSR with Axum](tanstack-ssr-with-axum.md) and [vox-web-stack SSOT](../architecture/vox-web-stack-ssot.md).

## Next steps

- Larger UI sample: [`examples/chatbot.vox`](../../../examples/chatbot.vox)
- Style rules: [`examples/STYLE.md`](../../../examples/STYLE.md)
- Web spine: [ADR 010](../adr/010-tanstack-web-spine.md)
