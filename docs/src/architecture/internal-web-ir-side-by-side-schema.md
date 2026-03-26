---
title: "Internal Web IR Side-by-Side Schema"
description: "Parser-grounded current-vs-target WebIR mapping for one full-stack Vox app across source, IR, TSX/TS, CSS, HTML, and Rust/API."
category: "architecture"
last_updated: 2026-03-26
training_eligible: true
---

# Internal Web IR Side-by-Side Schema

## Scope

This document is intentionally strict:

- every `.vox` syntax example is accepted by the current parser
- every "current output" claim is grounded in test assertions or implementation files
- every "target WebIR" claim is explicitly marked as either implemented now or planned

Canonical parser and output truth sources:

- `crates/vox-compiler/src/parser/descent/decl/head.rs`
- `crates/vox-compiler/src/parser/descent/decl/tail.rs`
- `crates/vox-compiler/src/parser/descent/expr/pratt_jsx.rs`
- `crates/vox-compiler/src/parser/descent/expr/style.rs`
- `crates/vox-compiler/tests/reactive_smoke.rs`
- `crates/vox-compiler/tests/web_ir_lower_emit.rs`
- `crates/vox-integration-tests/tests/pipeline.rs`
- `crates/vox-cli/tests/full_stack_minimal_build.rs`
- `crates/vox-cli/src/frontend.rs`
- `crates/vox-cli/src/templates/islands.rs`

## Parser-Verified Syntax Matrix

| Surface | Parser-accepted form (today) | Source anchor |
| --- | --- | --- |
| Reactive component (Path C) | `component Name(params) { state ... derived ... mount: ... view: <div /> }` | `crates/vox-compiler/src/parser/descent/decl/tail.rs` |
| Reactive via decorator | `@component Name(params) { ... }` (same reactive body) | `crates/vox-compiler/src/parser/descent/decl/head.rs` |
| Legacy component fn | `@component fn Name(...) to Element { ... }` | `crates/vox-compiler/src/parser/descent/decl/head.rs` |
| Island declaration | `@island Name { prop: Type prop2?: Type }` | `crates/vox-compiler/src/parser/descent/decl/head.rs` |
| Routes declaration | `routes { "/" to Home "/about" to About }` | `crates/vox-compiler/src/parser/descent/decl/tail.rs` |
| Server fn declaration | `@server fn echo(x: str) to str { ret x }` | `crates/vox-compiler/src/parser/descent/decl/head.rs` |
| JSX attributes | `class=`, `on:click=`, `on_click=`, `data-*=` forms | `crates/vox-compiler/src/parser/descent/expr/pratt_jsx.rs` |
| Component style block | `style { .class { prop: "value" } }` (string literal values) | `crates/vox-compiler/src/parser/descent/expr/style.rs` |

### Parser boundaries (non-speculative)

- `routes { ... }` is implemented; `routes:` is not the parser shape in current descent code.
- `style { ... }` parsing is wired through `parse_style_blocks()` on the `@component fn` path.
- `@island` props are parsed in a brace block with explicit `?` optional marker.

## Current Output Evidence Map (tests + code)

| Output layer | Verified current behavior | Evidence |
| --- | --- | --- |
| TSX islands mount | island tags emit `data-vox-island="Name"` and `data-prop-*` attrs | `crates/vox-compiler/tests/reactive_smoke.rs`, `crates/vox-compiler/src/codegen_ts/hir_emit.rs` |
| TS islands metadata | `vox-islands-meta.ts` contains island names | `crates/vox-compiler/tests/reactive_smoke.rs`, `crates/vox-compiler/src/codegen_ts/emitter.rs` |
| CSS output | style block emits `Component.css` and TSX imports it | `crates/vox-integration-tests/tests/pipeline.rs`, `crates/vox-compiler/src/codegen_ts/emitter.rs` |
| HTML shell islands script | frontend injects `/islands/island-mount.js` script | `crates/vox-cli/src/frontend.rs` |
| Islands hydration contract | hydrator reads `data-prop-*` as element attribute string values | `crates/vox-cli/src/templates/islands.rs` |
| Rust/API output | build emits `api.ts`; rust codegen emits `src/main.rs` + `src/lib.rs` | `crates/vox-cli/tests/full_stack_minimal_build.rs`, `crates/vox-compiler/src/codegen_rust/emit/mod.rs` |

## Worked Full-Stack App (Current vs Target)

### 1) `.vox` source today (parser-valid, island + CSS + routes + HTTP + server)

```vox
import react.use_state

@island DataChart {
    title: str
    data: str
    width?: int
}

@component fn Dashboard() to Element {
    let (title, _set_title) = use_state("Ops")
    let payload = "[1,2,3]"
    <div class="dashboard">
        <h1>{title}</h1>
        <DataChart title={title} data={payload} />
    </div>
}

style {
    .dashboard {
        display: "grid"
        gap: "12px"
    }
}

routes {
    "/" to Dashboard
}

http get "/api/ping" to str {
    ret "ok"
}

@server fn echo(x: str) to str {
    ret x
}
```

Why this shape is canonical:

- it uses only parser-supported forms listed in the matrix
- it includes every requested layer: JSX/HTML, CSS, routes, HTTP, server fn, island boundary

### 2) `.vox` low-k translation today (parser-valid Path C form)

```vox
@island DataChart {
    title: str
    data: str
}

component Dashboard(title: str) {
    state payload: str = "[1,2,3]"
    view: (
        <div class="dashboard">
            <h1>{title}</h1>
            <DataChart title={title} data={payload} />
        </div>
    )
}

routes {
    "/" to Dashboard
}
```

This is a real parser-accepted lower-k surface for component logic today (`component ... { state/view }`), not a future grammar proposal.

### 3) Internal representation side-by-side

#### Current pipeline (implemented)

```text
parse -> AST:
  Decl::Island(IslandDecl)
  Decl::Component(ComponentDecl) or Decl::ReactiveComponent(ReactiveComponentDecl)
  Decl::Routes(RoutesDecl)
  Decl::ServerFn(ServerFnDecl)
  Decl::Route(RouteDecl) [http ...]

lower -> HIR:
  HirIsland(pub IslandDecl)
  HirComponent(pub ComponentDecl)
  HirReactiveComponent { members, view }
  HirRoutes(pub RoutesDecl)
  HirServerFn { route_path, ... }
  HirRoute { method, path, ... }
```

Anchors:

- `crates/vox-compiler/src/ast/decl/ui.rs`
- `crates/vox-compiler/src/hir/nodes/decl.rs`

#### Target WebIR (implemented now: V0_1)

`WebIrModule` and core lowering/validation/preview emit are already present:

- schema: `crates/vox-compiler/src/web_ir/mod.rs`
- lower: `crates/vox-compiler/src/web_ir/lower.rs`
- validate: `crates/vox-compiler/src/web_ir/validate.rs`
- preview emit: `crates/vox-compiler/src/web_ir/emit_tsx.rs`

Current lowered shape (today):

```text
WebIrModule {
  dom_nodes,            // includes Element/Text/Expr and IslandMount
  view_roots,           // reactive component root pointers
  behavior_nodes,       // StateDecl/DerivedDecl/EffectDecl from reactive members
  route_nodes,          // RouteTree from routes declarations
  style_nodes,          // currently not lowered from style blocks
  interop_nodes,        // present in schema, not a main lowering source yet
  version: V0_1
}
```

Target completed shape (planned in ADR 012 + blueprint):

- extend lowering to include style contracts and route/server/mutation contracts in `RouteNode`
- make `validate_web_ir` enforce optionality and contract checks, not only structural DOM checks
- switch main `codegen_ts` printers to consume WebIR as canonical semantic source

### 4) Generated TSX/TS side-by-side

#### Current TSX/TS output (verified)

- island mount attrs appear:
  - `data-vox-island="DataChart"`
  - `data-prop-title=...`
- metadata file exists:
  - `vox-islands-meta.ts` with island names
- routes emit TanStack router output (`App.tsx` / `VoxTanStackRouter.tsx` depending mode)

Evidence:

- `crates/vox-compiler/tests/reactive_smoke.rs`
- `crates/vox-integration-tests/tests/pipeline.rs`

#### Target TSX/TS output after WebIR cutover (planned)

No claim of full cutover yet. The implemented, test-covered WebIR TSX preview guarantees:

- `lower_hir_to_web_ir` + `validate_web_ir` + `emit_component_view_tsx` roundtrip for reactive views
- class/style attr mapping and JSX structure parity checks for covered fixtures

Evidence:

- `crates/vox-compiler/tests/web_ir_lower_emit.rs`

### 5) Generated CSS side-by-side

#### Current CSS output (verified)

- style blocks emit `Component.css`
- generated TSX imports that CSS (`import "./Component.css"`)

Evidence:

- `crates/vox-integration-tests/tests/pipeline.rs`
- `crates/vox-compiler/src/codegen_ts/emitter.rs`

#### Target CSS output after WebIR style lowering (planned)

- `StyleNode` is in schema now
- style lowering and style validation are planned migration tasks before printer cutover
- until then, CSS emission remains in `codegen_ts/emitter.rs`

### 6) Generated HTML / island runtime side-by-side

#### Current HTML and island runtime output (verified)

- built app HTML gets `<script type="module" src="/islands/island-mount.js"></script>`
- `island-mount.tsx` scans `[data-vox-island]`, extracts `data-prop-*`, and mounts React components

Evidence:

- `crates/vox-cli/src/frontend.rs`
- `crates/vox-cli/src/templates/islands.rs`

#### Target completed WebIR output (planned compatibility)

- keep `data-vox-island` + `data-prop-*` contract in phase 1/2 migration
- any typed hydration payload upgrade must be explicit and versioned (no silent break)

### 7) Generated Rust/API side-by-side

#### Current Rust/API output (verified)

- `vox build` full-stack minimal writes `api.ts` for frontend server-fn/http access
- rust codegen writes `src/main.rs` and `src/lib.rs` from HIR routes/server functions/tables

Evidence:

- `crates/vox-cli/tests/full_stack_minimal_build.rs`
- `crates/vox-compiler/src/codegen_rust/emit/mod.rs`
- `crates/vox-integration-tests/tests/pipeline.rs`

#### Target completed WebIR output (planned scope)

- WebIR is frontend IR; Rust emission remains HIR/back-end lowering owned
- completed WebIR should unify frontend contracts, then map to existing backend contracts without changing Rust ownership boundaries

## Critique -> Improvement -> File Actions

| Current issue (verified) | Why it hurts | Target improvement | Primary files |
| --- | --- | --- | --- |
| JSX/island semantics split across `jsx.rs` and `hir_emit.rs` | duplicated logic drift risk | single semantic lower in `web_ir/lower.rs` | `crates/vox-compiler/src/codegen_ts/jsx.rs`, `crates/vox-compiler/src/codegen_ts/hir_emit.rs`, `crates/vox-compiler/src/web_ir/lower.rs` |
| Hydration props decoded as strings | runtime type erosion | versioned typed hydration contract, preserving V1 compatibility | `crates/vox-cli/src/templates/islands.rs`, `crates/vox-compiler/src/web_ir/mod.rs` |
| `validate_web_ir` is structural-only today | misses optionality/contract failures | enforce optionality, route/server/mutation constraints before emit | `crates/vox-compiler/src/web_ir/validate.rs`, `crates/vox-compiler/src/web_ir/mod.rs` |
| Style semantics not lowered into WebIR yet | split ownership between IR and emitter | lower style blocks to `StyleNode` and print from WebIR | `crates/vox-compiler/src/web_ir/lower.rs`, `crates/vox-compiler/src/codegen_ts/emitter.rs` |

## Research Anchors Applied

| Design choice | Practical reason | Source |
| --- | --- | --- |
| keep a compiler-owned normalized IR before final emit | simplifies ownership and reduces duplicate transforms | [SWC architecture](https://raw.githubusercontent.com/swc-project/swc/main/ARCHITECTURE.md), [ESTree](https://raw.githubusercontent.com/estree/estree/master/es5.md) |
| keep React interop boundary stable during migration | preserve ecosystem compatibility while internal IR changes | [React Compiler](https://react.dev/learn/react-compiler) |
| explicit nullability policy in IR | avoid implicit undefined/null behavior at emit boundary | [TypeScript strictNullChecks](https://www.typescriptlang.org/tsconfig/strictNullChecks.html) |
| typed style representation over raw string-only internals | better static checks and transforms | [CSS Typed OM](https://developer.mozilla.org/en-US/docs/Web/API/CSS_Typed_OM_API), [Lightning CSS transforms](https://lightningcss.dev/transforms.html) |
