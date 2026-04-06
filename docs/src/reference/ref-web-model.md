---
title: "Web Model Reference"
description: "Reference for building APIs and interactive frontends with the Vox web model."
category: "reference"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---

# Reference: Web Model

Vox embraces a server-first web architecture. In Vox v0.3+, the v0.2 `@island` decorator (colon-syntax) has been modernized to the v0.3 brace-syntax system alongside raw programmatic HTTP routing. 

## Interactive Islands

Client-side interactive user interfaces are modeled using hydrated React components known as islands. 

- `@island fn ComponentName(props: ModelType) -> Element { }`  
  *Compiles into a TypeScript/React TSX artifact injected via hydration into static HTML generated server-side.* 

### Using Functional State Hooks (`react.use_state`)

Because Islands are fully bridged React outputs, you can instantiate frontend React state mapping hooks seamlessly. 
```vox
// Skip-Test
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
- You do not need to call `ret <div/>`; trailing expressions resolve correctly. 

## Inline HTTP Layout Mappings 

Vox enables inline API mapping without full standalone Axum scaffolding using raw web directives. 

- `http get "/path" -> ResultType { }`  
  *Triggers a standard asynchronous GET routing returning raw string, UI templates, or JSON output payloads depending on structural data boundaries.*
- `http post "/path" (body: BodyType) -> ResultType { }`  
  *Determines direct incoming payload structures explicitly mapped inside Vox structural ADT data types.*

## Route Table Registrations 

All paths and mappings flow back into the monolithic static web table defined near the core namespace termination. 

```vox
// Skip-Test
// Registers physical route distributions for frontend UI hydration
routes {
    "/"              -> HomeIsland         
    "/dashboard"     -> AccountDashboardIsland
    "/terms"         -> StaticTermsContent
}
```

## Compilation and Hydration (Behind the scenes)

When generating code, the `@island` component operates as follows: 
1. Vox generates standard server-side HTML representations containing unique ID markers matching `data-vox-island="ComponentName"`.
2. A separate module bundle named `island-mount.js` is automatically resolved and built during compilation. 
3. When the user loads the page, `island-mount.js` detects the presence of the DOM attributes and runs automatic progressive hydration locally over that explicit piece of DOM tree.
