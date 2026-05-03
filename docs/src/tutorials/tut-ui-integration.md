---
title: "Tutorial: UI Integration via React Interop"
description: "Build user interfaces in Vox by emitting plain React components and calling them from an external React frontend."
category: "tutorials"
status: "current"
last_updated: "2026-05-03"
training_eligible: true
schema_type: "TechArticle"
---

# Tutorial: UI Integration via React Interop

> **Note (2026-05-03).** The `@island` decorator was retired. Vox now compiles `component`
> declarations to plain React/TSX components and `@endpoint` declarations to a generated
> `vox-client.ts`. An external React, TanStack, or mobile app imports the components or
> calls the endpoints over RPC. There is no island-mount harness.
>
> The full bidirectional React interop story (server-only and fullstack build modes,
> Phase 5 React adapter) lives in
> [`architecture/external-frontend-interop-plan-2026.md`](../architecture/external-frontend-interop-plan-2026.md).

## 1. The `component` declaration

Define interactive UI with `component`. The codegen emits a plain React component to the
generated `app/` directory.

```vox
component Counter(initial: int) {
    let count = initial
    view: <div class="counter">
        <p>"Count: " {count}</p>
    </div>
}
```

## 2. Wiring routes

Map a path to a `component` through the global `routes { }` block.

```vox
component HomePage() {
    view: <Counter initial=0 />
}

routes { "/" to HomePage }
```

## 3. Calling endpoints from the frontend

Declare server logic with `@endpoint`. The codegen emits a typed RPC stub into
`vox-client.ts` that the React frontend imports directly.

```vox
@endpoint(kind: query)
fn get_count() to int { return 42 }
```

```ts
import { getCount } from "./vox-client";
const n = await getCount();
```

## See also

- [Reference: Decorators](../reference/ref-decorators.md)
- [Reference: Vox Web Stack](../reference/vox-web-stack.md)
- [Architecture: External Frontend Interop Plan](../architecture/external-frontend-interop-plan-2026.md)
