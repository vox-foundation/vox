---
title: "Web Model Reference"
description: "Reference for building APIs and interactive frontends with the Vox web model."
category: "reference"
status: "current"
last_updated: "2026-04-08"
training_eligible: true

schema_type: "TechArticle"
---

# Reference: Web Model

Vox embraces a server-first web architecture. In Vox v0.3+, the v0.2 `@island` decorator (colon-syntax) has been modernized to the v0.3 brace-syntax system alongside raw programmatic HTTP routing. 

## Interactive Islands

Client-side interactive user interfaces are modeled using hydrated React components known as islands. 

- `@island ComponentName { props: ModelType }`  
  *Compiles into a TypeScript/React TSX artifact injected via hydration into static HTML generated server-side.* 

### Using Functional State Hooks (`react.use_state`)

Because Islands are fully bridged React outputs, you can instantiate frontend React state mapping hooks seamlessly. 
```vox
// vox:skip
import react.use_state

@island
fn ToggleBtn() -> Element {
    let (on, set_on) = use_state(false)
    <button onClick={fn() set_on(!on)}>
        {if on { "Active" } else { "Inactive" }}
    </button>
}
```

### Inner JSX Rules

Inside the body of any function that returns `Element`, you can directly emit standard JSX elements. Note that: 
- Variables are evaluated implicitly within `{braces}`.
- Handlers (`onClick`, `onChange`) capture inline lambda functions implicitly. 
- You do not need to call `return <div/>`; trailing expressions resolve correctly. 

## Inline HTTP Layout Mappings 

Vox enables inline API mapping without full standalone Axum scaffolding using raw web directives. 

- `http get "/path" -> ResultType { }`  
  *Triggers a standard asynchronous GET routing returning raw string, UI templates, or JSON output payloads depending on structural data boundaries.*
- `http post "/path" (body: BodyType) -> ResultType { }`  
  *Determines direct incoming payload structures explicitly mapped inside Vox structural ADT data types.*

## `routes { }` (canonical syntax, 2026)

Vox emits a **`routes.manifest.ts`** (`VoxRoute[]`) for adapters; the **normative surface in `.vox`** is:

- **Paths:** string literals with **`to`** before the component name: `"/" to Home`.
- **Loaders / pending:** `with loader: myQuery` and/or `with pending: Spinner` (tuple form `with (loader: a, pending: b)` supported).
- **Nesting:** child routes inside `{ ... }` after the parent entry (path strings only inside nested blocks).
- **Global screens:** `not_found: NotFoundPage` and `error: ErrorPage` in the `routes { }` body.

**Deferred (not in the parser yet):** `"/path" as layout Shell { }`, `under LayoutName`, redirect-only entries, wildcard segments, and populating `RouteEntry.redirect` / `is_wildcard` from source — see [`react-interop-implementation-plan-2026.md`](../architecture/react-interop-implementation-plan-2026.md) and [`tanstack-start-codegen-spec.md`](../architecture/tanstack-start-codegen-spec.md) (historical examples may overshoot grammar).

## Route table (legacy arrow sketch)

Older prose used arrow forms; prefer **`to`** and manifests per [`vox-web-stack.md`](./vox-web-stack.md).

```vox
// vox:skip
routes {
    "/" to Home
    "/dashboard" to AccountDashboard
}
```

## Compilation and Hydration (Behind the scenes)

When generating code, the `@island` component operates as follows: 
1. Vox generates standard server-side HTML representations containing unique ID markers matching `data-vox-island="ComponentName"`.
2. A separate module bundle named `island-mount.js` is automatically resolved and built during compilation. 
3. When the user loads the page, `island-mount.js` detects the presence of the DOM attributes and runs automatic progressive hydration locally over that explicit piece of DOM tree.
