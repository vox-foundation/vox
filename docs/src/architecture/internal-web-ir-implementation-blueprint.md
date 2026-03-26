---
title: "Internal Web IR Implementation Blueprint"
description: "Detailed execution blueprint for implementing WebIR in Vox with weighted task quotas and critical-path checklist."
category: "architecture"
last_updated: 2026-03-26
training_eligible: true
---

# Internal Web IR Implementation Blueprint

## Goal

Provide a concrete, execution-ready implementation plan for introducing `WebIR` into Vox while preserving React ecosystem interoperability and island compatibility.

> **Progress:** The normative `WebIrModule` schema, `lower_hir_to_web_ir`, `validate_web_ir`, and `emit_component_view_tsx` now live under [`crates/vox-compiler/src/web_ir/`](../../../crates/vox-compiler/src/web_ir/mod.rs) (see [ADR 012](../adr/012-internal-web-ir-strategy.md)). Checklist items below remain the long-range migration map; many CP-* rows are partially satisfied by this layer without implying full emitter cutover.

This blueprint is designed for future LLM-assisted implementation and includes:

- Layer A: explicit critical-path tasks (150 tasks)
- Layer B: weighted work-package quotas (target 500-900 weighted tasks)
- Token/effort budgets based on complexity and risk

## Scope and non-goals

- In scope: compiler pipeline changes from AST/HIR to WebIR and WebIR to target emitters, parity testing, migration strategy, documentation, and rollout gates.
- In scope: keeping current islands mount contract stable through compatibility phases.
- Out of scope (near-term): replacing React runtime wholesale or breaking third-party React interop contracts.

## Baseline code touchpoints

- `crates/vox-compiler/src/hir/nodes/decl.rs`
- `crates/vox-compiler/src/hir/nodes/stmt_expr.rs`
- `crates/vox-compiler/src/codegen_ts/jsx.rs`
- `crates/vox-compiler/src/codegen_ts/hir_emit.rs`
- `crates/vox-compiler/src/codegen_ts/emitter.rs`
- `crates/vox-cli/src/templates/islands.rs`
- `crates/vox-cli/src/frontend.rs`

Canonical side-by-side representation mapping:

- [internal-web-ir-side-by-side-schema.md](internal-web-ir-side-by-side-schema.md)

## Parser-grounded gap analysis (current -> target)

| Area | Current verified state | Gap to close | Primary files |
| --- | --- | --- | --- |
| JSX and island lowering ownership | split between `codegen_ts/jsx.rs` and `codegen_ts/hir_emit.rs`; island rewrite exists in both paths | consolidate semantic ownership in `web_ir/lower.rs` and keep emitters thin | `crates/vox-compiler/src/codegen_ts/jsx.rs`, `crates/vox-compiler/src/codegen_ts/hir_emit.rs`, `crates/vox-compiler/src/web_ir/lower.rs` |
| WebIR validation depth | `validate_web_ir` currently checks structural DOM references and arena bounds | add optionality, route/server/mutation, and style contract validation prior to emit | `crates/vox-compiler/src/web_ir/validate.rs`, `crates/vox-compiler/src/web_ir/mod.rs` |
| Style representation | style emission lives in TS emitter (`Component.css` generation) | lower style blocks into `StyleNode` then emit from WebIR printer path | `crates/vox-compiler/src/codegen_ts/emitter.rs`, `crates/vox-compiler/src/web_ir/lower.rs` |
| Route/data contract convergence | routes and server outputs are generated from HIR-oriented emit modules | represent route/data/server contracts in `RouteNode` and bridge to emitters | `crates/vox-compiler/src/codegen_ts/routes.rs`, `crates/vox-compiler/src/web_ir/lower.rs`, `crates/vox-compiler/src/codegen_ts/emitter.rs` |
| Islands runtime typing | hydration reads `data-prop-*` values from DOM attributes (string channel) | preserve V1 contract first; introduce explicit versioned V2 typing when ready | `crates/vox-cli/src/templates/islands.rs`, `crates/vox-cli/src/frontend.rs`, `crates/vox-compiler/src/web_ir/mod.rs` |

## Test gate matrix (file-level)

| Gate | Required evidence | Current anchors |
| --- | --- | --- |
| Parser syntax gate | parser-accepted forms for component/routes/island/style/server | `crates/vox-compiler/src/parser/descent/decl/head.rs`, `crates/vox-compiler/src/parser/descent/decl/tail.rs`, `crates/vox-compiler/src/parser/descent/expr/style.rs` |
| Current output parity gate | TSX/TS/CSS/asserted output substrings for baseline fixtures | `crates/vox-compiler/tests/reactive_smoke.rs`, `crates/vox-integration-tests/tests/pipeline.rs` |
| WebIR structural gate | `lower_hir_to_web_ir` + `validate_web_ir` + preview emit pass | `crates/vox-compiler/tests/web_ir_lower_emit.rs` |
| Build artifact gate | full-stack build emits expected frontend artifacts | `crates/vox-cli/tests/full_stack_minimal_build.rs` |
| Islands runtime gate | mount script injection and hydration behavior unchanged | `crates/vox-cli/src/frontend.rs`, `crates/vox-cli/src/templates/islands.rs` |

## Legacy direct-emit registry (authoritative for migration)

| File | Current role | Migration disposition | Target owner |
| --- | --- | --- | --- |
| `crates/vox-compiler/src/codegen_ts/emitter.rs` | output orchestrator and file assembly | `legacy-wrap` | WebIR lower/validate/emit adapters |
| `crates/vox-compiler/src/codegen_ts/hir_emit.rs` | HIR expr/stmt to TS/JSX strings | `legacy-replace` | `crates/vox-compiler/src/web_ir/emit_tsx.rs` + future target emitters |
| `crates/vox-compiler/src/codegen_ts/jsx.rs` | AST JSX render path | `legacy-replace` | `crates/vox-compiler/src/web_ir/lower.rs` + emitters |
| `crates/vox-compiler/src/codegen_ts/component.rs` | `@component` generation from AST-retained path | `legacy-shrink` | WebIR lowering adapters + thin wrapper |
| `crates/vox-compiler/src/codegen_ts/reactive.rs` | reactive component generation | `legacy-shrink` | WebIR view roots + emitter |
| `crates/vox-compiler/src/codegen_ts/routes.rs` | route-specific TS generation | `legacy-replace` | `RouteNode` contracts + target printer |
| `crates/vox-compiler/src/codegen_ts/tanstack_programmatic_routes.rs` | TanStack route tree strings | `legacy-shrink` | target formatter over `RouteNode` |
| `crates/vox-compiler/src/codegen_ts/tanstack_query_emit.rs` | query helper emit | `legacy-wrap` | contract-driven helper generation |
| `crates/vox-compiler/src/codegen_ts/tanstack_start.rs` | constants/literals for Start mode | `retain-support` | keep as target support surface |
| `crates/vox-compiler/src/codegen_ts/activity.rs` | activity wrappers | `legacy-shrink` | consume WebIR/contract nodes |
| `crates/vox-compiler/src/codegen_ts/schema.rs` | schema TS emit path | `legacy-wrap` | route/data/DB contracts over WebIR |
| `crates/vox-compiler/src/codegen_ts/adt.rs` | ADT/type generation | `retain-support` | remains mostly independent |
| `crates/vox-compiler/src/codegen_ts/island_emit.rs` | island-name and data-attr helpers | `legacy-shrink` | compatibility adapter until V2 mount contract |

## File-level edit guide (where, what, how, why)

### Stage A - stabilize source contracts (no behavior break)

1. `crates/vox-compiler/src/parser/descent/decl/head.rs`
   - What: keep `@island` grammar stable; add diagnostics only if needed.
   - Why: language churn is out of scope during representation migration.
2. `crates/vox-compiler/src/hir/lower/mod.rs`
   - What: preserve `Decl::Island -> HirIsland` compatibility.
   - Why: WebIR migration should not break existing HIR consumers in same tranche.

### Stage B - expand WebIR lower/validate

1. `crates/vox-compiler/src/web_ir/lower.rs`
   - What: absorb rewrite semantics currently split in `jsx.rs` and `hir_emit.rs`.
   - How: ensure tag/island classification, attr mapping, ignored-child semantics are canonical here.
   - Why: remove dual semantic ownership.
2. `crates/vox-compiler/src/web_ir/validate.rs`
   - What: add strict checks for optionality, route ids/contracts, island prop representation.
   - Why: validation before emission is the key safety boundary.
3. `crates/vox-compiler/src/web_ir/mod.rs`
   - What: evolve node shapes only under versioned policy (`WebIrVersion`).
   - Why: prevent silent schema drift.

### Stage C - bridge emitters with wrappers

1. `crates/vox-compiler/src/codegen_ts/emitter.rs`
   - What: keep `generate` API stable, but call WebIR lower/validate/emit internally.
   - Why: avoids rippling API changes across CLI/tests.
2. `crates/vox-compiler/src/codegen_ts/component.rs`
   - What: transition to wrapper that resolves component metadata then delegates view output to WebIR emitter.
   - Why: gradual migration of AST-retained component path.
3. `crates/vox-compiler/src/codegen_ts/reactive.rs`
   - What: delegate view rendering to WebIR emit path.
   - Why: unify with component path and island semantics.

### Stage D - de-duplicate legacy internals

1. `crates/vox-compiler/src/codegen_ts/hir_emit.rs`
   - What: retire island/JSX rendering ownership; retain only compatibility helpers during transition.
2. `crates/vox-compiler/src/codegen_ts/jsx.rs`
   - What: retire direct island mount rendering path.
3. `crates/vox-compiler/src/codegen_ts/routes.rs`
   - What: route tree and contract output should consume WebIR `RouteNode`.

### Stage E - islands runtime compatibility and V2 gate

1. `crates/vox-cli/src/templates/islands.rs`
   - What: preserve current `data-vox-island`/`data-prop-*` semantics while WebIR migration lands.
2. `crates/vox-cli/src/frontend.rs`
   - What: preserve script injection and asset wiring behavior.
3. V2 gate (future)
   - What: if changing hydration payload typing, introduce explicit versioned adapter (`IslandMountV2`) and parity fixtures.
   - Why: runtime compatibility is a hard gate.

## Complexity model

- `C1` trivial: weight `1.0`, token multiplier `1.0`
- `C2` moderate: weight `2.0`, token multiplier `1.8`
- `C3` complex: weight `3.5`, token multiplier `3.2`
- `C4` deep/refactor: weight `5.0`, token multiplier `5.0`

Work package score:

`weighted_tasks = task_count * complexity_weight * risk_multiplier`

Where risk multiplier is in `[1.0, 1.8]`.

## Layer A: explicit critical-path checklist (150 tasks)

### Phase 0 - contracts, governance, and measurement (CP-001..CP-015)

- [ ] CP-001 Define `WebIR` term as canonical in architecture docs.
- [ ] CP-002 Define `WebIrVersion` policy and compatibility rules.
- [ ] CP-003 Freeze island mount attribute contract fixtures.
- [ ] CP-004 Baseline duplicate emit path inventory (`jsx.rs`, `hir_emit.rs`).
- [ ] CP-005 Baseline framework-shaped syntax exposure metrics in `.vox`.
- [ ] CP-006 Baseline nullability ambiguity points at TS emit boundary.
- [ ] CP-007 Baseline route/data emission parity examples.
- [ ] CP-008 Baseline style emission parity examples.
- [ ] CP-009 Add migration status flagging policy to docs.
- [ ] CP-010 Define WebIR acceptance gate checklist.
- [ ] CP-011 Define rollback criteria for each migration phase.
- [ ] CP-012 Define deprecation policy for legacy `@component fn` hooks.
- [ ] CP-013 Add source-of-truth file list for WebIR ownership.
- [ ] CP-014 Define lint/test ownership for WebIR modules.
- [ ] CP-015 Define release-note template for WebIR milestones.

### Phase 1 - WebIR type system and module layout (CP-016..CP-040)

- [ ] CP-016 Add `codegen_web_ir` module root.
- [ ] CP-017 Add `web_ir/mod.rs` with public exports.
- [ ] CP-018 Define `WebIrModule` root struct.
- [ ] CP-019 Define `DomNode` enum.
- [ ] CP-020 Define `BehaviorNode` enum.
- [ ] CP-021 Define `StyleNode` enum.
- [ ] CP-022 Define `RouteNode` enum.
- [ ] CP-023 Define `InteropNode` enum.
- [ ] CP-024 Define `WebIrDiagnostic` struct.
- [ ] CP-025 Define `SourceSpanId` + span table model.
- [ ] CP-026 Define `FieldOptionality` enum (`Required`, `Optional`, `Defaulted`).
- [ ] CP-027 Define `IslandMountNode` with compatibility fields.
- [ ] CP-028 Define `RouteContract` payload shape.
- [ ] CP-029 Define `ServerFnContract` payload shape.
- [ ] CP-030 Define `MutationContract` payload shape.
- [ ] CP-031 Define `StyleDeclarationValue` typed union.
- [ ] CP-032 Define selector AST surface for CSS rules.
- [ ] CP-033 Define `ExternalModuleRef` interop node.
- [ ] CP-034 Define `EscapeHatchExpr` policy wrapper node.
- [ ] CP-035 Add serialization/deserialization traits for debug dumps.
- [ ] CP-036 Add stable debug printer for WebIR snapshots.
- [ ] CP-037 Add constructor helpers for test fixtures.
- [ ] CP-038 Add invariants doc comments to all node types.
- [ ] CP-039 Add semantic versioning comments in WebIR root.
- [ ] CP-040 Add smoke compile test for WebIR type compilation.

### Phase 2 - lowering from HIR/AST into WebIR (CP-041..CP-065)

- [ ] CP-041 Add `lower_to_web_ir` entry point.
- [ ] CP-042 Map `HirReactiveComponent` to `BehaviorNode` state declarations.
- [ ] CP-043 Map derived members to `BehaviorNode::DerivedDecl`.
- [ ] CP-044 Map effects to `BehaviorNode::EffectDecl`.
- [ ] CP-045 Lower HIR JSX elements to `DomNode::Element`.
- [ ] CP-046 Lower HIR text/content nodes to `DomNode::Text`.
- [ ] CP-047 Lower HIR fragment constructs to `DomNode::Fragment`.
- [ ] CP-048 Lower HIR loops to `DomNode::Loop`.
- [ ] CP-049 Lower HIR conditionals to `DomNode::Conditional`.
- [ ] CP-050 Lower event attributes to `BehaviorNode::EventHandler`.
- [ ] CP-051 Lower known style blocks to `StyleNode::Rule`.
- [ ] CP-052 Lower route declarations to `RouteNode::RouteTree`.
- [ ] CP-053 Lower server function declarations to `RouteNode::ServerFnContract`.
- [ ] CP-054 Lower mutation declarations to `RouteNode::MutationContract`.
- [ ] CP-055 Lower island tags to `DomNode::IslandMount`.
- [ ] CP-056 Preserve island `data-prop-*` mapping semantics in node fields.
- [ ] CP-057 Add adapter for AST-retained `HirComponent`.
- [ ] CP-058 Add shim lowering for legacy `@component fn` path.
- [ ] CP-059 Attach source spans to all lowered nodes.
- [ ] CP-060 Emit lowering diagnostics for unsupported edge expressions.
- [ ] CP-061 Add lowering unit tests for each node family.
- [ ] CP-062 Add golden fixture for mixed reactive + island source.
- [ ] CP-063 Add lowering benchmark harness.
- [ ] CP-064 Add lowering trace logs behind debug flag.
- [ ] CP-065 Gate lowering feature behind compiler option.

### Phase 3 - validation and safety passes (CP-066..CP-085)

- [ ] CP-066 Add `validate_web_ir` entry point.
- [ ] CP-067 Validate required fields are always present.
- [ ] CP-068 Validate optionality annotations are explicit.
- [ ] CP-069 Validate no unresolved `Defaulted` at print boundary.
- [ ] CP-070 Validate route contracts have unique ids.
- [ ] CP-071 Validate server function signatures are serializable.
- [ ] CP-072 Validate mutation contracts use supported payload forms.
- [ ] CP-073 Validate island mount props are representable.
- [ ] CP-074 Validate style selectors are parseable and scoped.
- [ ] CP-075 Validate declaration units by typed value category.
- [ ] CP-076 Validate escape hatches against policy allowlist.
- [ ] CP-077 Add validator diagnostics categories.
- [ ] CP-078 Add validator snapshot tests.
- [ ] CP-079 Add strict mode that fails on warnings.
- [ ] CP-080 Add compatibility mode for legacy fixtures.
- [ ] CP-081 Add CLI switch for validator verbosity.
- [ ] CP-082 Add metrics counter for validation error classes.
- [ ] CP-083 Add nullability ambiguity metric export.
- [ ] CP-084 Add route contract ambiguity metric export.
- [ ] CP-085 Add style compatibility metric export.

### Phase 4 - WebIR to React/TanStack emitter (CP-086..CP-110)

- [ ] CP-086 Add `emit_react_from_web_ir` entry point.
- [ ] CP-087 Emit React component wrappers from `DomNode` roots.
- [ ] CP-088 Emit props interfaces from WebIR contracts.
- [ ] CP-089 Emit state hook bridge from behavior nodes.
- [ ] CP-090 Emit derived bridge expressions from behavior nodes.
- [ ] CP-091 Emit effect bridge expressions from behavior nodes.
- [ ] CP-092 Emit event handlers with explicit closure policies.
- [ ] CP-093 Emit route tree from `RouteNode::RouteTree`.
- [ ] CP-094 Emit loader wrappers from `LoaderContract`.
- [ ] CP-095 Emit server fn wrappers from `ServerFnContract`.
- [ ] CP-096 Emit mutation wrappers from `MutationContract`.
- [ ] CP-097 Emit island mount placeholders from `IslandMountNode`.
- [ ] CP-098 Preserve `data-vox-island` contract during migration.
- [ ] CP-099 Preserve `data-prop-*` key transform semantics.
- [ ] CP-100 Emit typed interop stubs for external components.
- [ ] CP-101 Emit escape hatch blocks with warning comments.
- [ ] CP-102 Emit sourcemap metadata for generated TSX.
- [ ] CP-103 Add parity tests against legacy emitter outputs.
- [ ] CP-104 Add route generation parity tests.
- [ ] CP-105 Add server fn generation parity tests.
- [ ] CP-106 Add island generation parity tests.
- [ ] CP-107 Add component generation parity tests.
- [ ] CP-108 Add emission benchmark harness.
- [ ] CP-109 Add fail-fast switch for parity regressions.
- [ ] CP-110 Add feature flag to select WebIR emitter path.

### Phase 5 - style IR and CSS emission (CP-111..CP-125)

- [ ] CP-111 Add `emit_css_from_web_ir` entry point.
- [ ] CP-112 Emit scoped rules from `StyleNode::Rule`.
- [ ] CP-113 Emit nested selector forms with stable ordering.
- [ ] CP-114 Emit at-rules with validation gate.
- [ ] CP-115 Emit token references with fallback behavior.
- [ ] CP-116 Emit declaration values from typed value unions.
- [ ] CP-117 Validate unit conversions before CSS print.
- [ ] CP-118 Add style-source map integration.
- [ ] CP-119 Add CSS parity tests against existing outputs.
- [ ] CP-120 Add style-lint compatibility checks.
- [ ] CP-121 Add container query support test fixtures.
- [ ] CP-122 Add `:has()` and nesting support fixtures.
- [ ] CP-123 Add style conflict diagnostics by selector collision.
- [ ] CP-124 Add style emission perf benchmark.
- [ ] CP-125 Add style regression triage protocol.

### Phase 6 - databasing and route-data contract integration (CP-126..CP-138)

- [ ] CP-126 Define mapping from DB query plans to `LoaderContract`.
- [ ] CP-127 Define mapping from mutation plans to `MutationContract`.
- [ ] CP-128 Add explicit serialization schema for loader payloads.
- [ ] CP-129 Add explicit serialization schema for mutation payloads.
- [ ] CP-130 Enforce non-nullability policy at route-data boundaries.
- [ ] CP-131 Add compatibility tests for existing generated client fetches.
- [ ] CP-132 Add compatibility tests for server fn API prefixes.
- [ ] CP-133 Add typed failure-channel contracts for route loaders.
- [ ] CP-134 Add typed failure-channel contracts for mutations.
- [ ] CP-135 Add parity tests for database-driven pages.
- [ ] CP-136 Add perf tests for route-data emit path.
- [ ] CP-137 Add diagnostics for schema drift between DB and WebIR.
- [ ] CP-138 Add docs for route-data + DB integration policy.

### Phase 7 - migration, rollout, and deprecation (CP-139..CP-150)

- [ ] CP-139 Add staged rollout flag (`VOX_WEB_IR_STAGE`).
- [ ] CP-140 Enable dual-run mode (legacy + WebIR output compare).
- [ ] CP-141 Add diff reporter for generated artifact mismatches.
- [ ] CP-142 Add warning docs for legacy syntax deprecations.
- [ ] CP-143 Add CLI command to audit WebIR readiness of project.
- [ ] CP-144 Add migration guide from legacy `@component fn`.
- [ ] CP-145 Add migration guide for islands compatibility.
- [ ] CP-146 Promote WebIR path to default in preview channel.
- [ ] CP-147 Define cutover gate requiring parity pass rate threshold.
- [ ] CP-148 Define rollback gate and incident protocol.
- [ ] CP-149 Promote WebIR path to default stable.
- [ ] CP-150 Archive legacy emitter-only code paths after freeze period.

---

## Layer B: weighted work-package quotas (target 500-900 weighted tasks)

### Allocation table

| Package | Focus | Raw tasks | Dominant class | Risk multiplier | Weighted tasks | Token budget |
| --- | --- | ---: | --- | ---: | ---: | ---: |
| WP-01 | contracts and baselines | 24 | C2 | 1.1 | 42 | 6k |
| WP-02 | WebIR type definitions | 30 | C3 | 1.1 | 58 | 8k |
| WP-03 | HIR -> WebIR lowering core | 36 | C4 | 1.2 | 74 | 12k |
| WP-04 | AST-retained compatibility shims | 18 | C3 | 1.1 | 36 | 5k |
| WP-05 | validation engine | 24 | C4 | 1.1 | 52 | 8k |
| WP-06 | React emitter rewrite | 30 | C4 | 1.1 | 66 | 10k |
| WP-07 | route/data contract emitter | 22 | C3 | 1.1 | 48 | 7k |
| WP-08 | islands compatibility layer | 18 | C3 | 1.1 | 40 | 6k |
| WP-09 | style IR + CSS emitter | 20 | C3 | 1.1 | 44 | 7k |
| WP-10 | DB contract mapping | 18 | C3 | 1.1 | 38 | 6k |
| WP-11 | parity fixture generation | 20 | C2 | 1.1 | 34 | 5k |
| WP-12 | differential test harness | 16 | C3 | 1.1 | 32 | 5k |
| WP-13 | perf and memory benchmarks | 14 | C3 | 1.0 | 28 | 4k |
| WP-14 | diagnostics and tooling UX | 14 | C2 | 1.0 | 24 | 3k |
| WP-15 | migration and docs | 20 | C2 | 1.0 | 40 | 5k |
| WP-16 | rollout + release engineering | 16 | C3 | 1.0 | 32 | 5k |

**Total weighted tasks**: **688 weighted units**

Notes:

- Weighted total is intentionally kept inside the 500-900 target range for near-term planning.
- Raw task volume remains high, while weighted units focus implementation effort on higher-risk refactors.

### Normalized tranche model (for release planning)

- Tranche A (foundation): 220 weighted units
- Tranche B (core migration): 300 weighted units
- Tranche C (cutover and cleanup): 168 weighted units

---

## Sequencing constraints

1. Do not begin emitter cutover before validation pass is stable.
2. Do not deprecate legacy path before parity thresholds are met.
3. Do not alter island mount contract before explicit V2 plan is accepted.
4. Do not enable default WebIR output without dual-run diff telemetry.

## Acceptance gates

- Gate G1: WebIR lower + validate pass on all golden compiler fixtures.
- Gate G2: React/TanStack output parity >= 95% on canonical fixture corpus.
- Gate G3: Island compatibility parity == 100% for contract fixtures.
- Gate G4: Nullability ambiguity metric reaches zero unresolved required fields.
- Gate G5: WebIR mode passes CI and targeted perf budgets.

## LLM execution guidance

- Prefer package-level batching: complete WP-01 through WP-04 before touching rollout packages.
- Use deterministic fixture updates and include before/after diff explanations.
- Keep one package in active refactor mode at a time; run validation/perf at package boundaries.
- Use token budgets as soft ceilings to avoid over-refactoring in a single pass.

## Related docs

- [ADR 012 — Internal web IR strategy](../adr/012-internal-web-ir-strategy.md)
- [Vox full-stack web SSOT](../reference/vox-web-stack.md)
- [Compiler architecture](../explanation/expl-architecture.md)
- [Compiler lowering phases](../explanation/expl-compiler-lowering.md)
