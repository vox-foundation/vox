# Language: Struct types (records) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** First-class struct types in Vox: `type Name { field: T, ... }`, struct literal construction `Name { field: value, ... }`, and field projection `s.field` — usable as parameters, return types, locals, and (subject to scalar-mapping rules) inside `@table` field types.

**Why now:** Blocks the vox-mental-tracker app's Phase 2 voice flow. The `VoicePage` save handler needs to receive `(kind, payload_json, confidence)` from one classification pass over a transcript. Without structs, callers must invoke 3 separate extractor endpoints that re-classify the same input — wasteful, redundant, and the rule lives in 3 places. Also unblocks any future endpoint that wants to return a structured result (export metadata, materialized rollups, voice/parser intermediates).

**Architecture:**
- **Parser:** `parse_typedef` already handles `type Name = | A | B(...)` (ADT). Extend it to branch on the next token after the name: `=` keeps existing ADT path; `{` enters a new struct body parser that fills `TypeDefDecl.fields` (already exists in the AST).
- **HIR:** `HirTypeDef` currently has only `variants` (comment: "structs handled elsewhere" — they aren't). Add `fields: Vec<HirField>`. Update `lower_typedef` to thread fields through.
- **Typeck:** When a `TypeDef` has non-empty `fields`, register it in the type environment as a struct: name → `Vec<(field_name, Ty)>`. Resolve `s.f` against the registered shape (already partially handled for `@table` rows; reuse the same field-projection machinery if possible). Typecheck struct literal expressions (`Foo { f: e, ... }`) by looking up `Foo`, requiring exactly the declared field set, and unifying each initializer expression's type with the declared field type.
- **AST/HIR for struct literals:** Audit whether an expression node already exists. `@table` rows are constructed via the `db.X.insert({ ... })` call which uses an anonymous record literal; check whether that maps to an existing `Expr::Record` or similar. If yes: extend it to optionally carry a type name (`Foo { ... }` vs `{ ... }`). If no: add `Expr::StructLit { name: Option<String>, fields: Vec<(String, Expr)> }`.
- **Codegen TS:** Struct type → TS type alias (`export type Foo = { f: T, ... }`). Struct literal → object literal (`{ f: e, ... }`). Field access → `.f`. (TS already understands all of this; codegen is mostly tag-and-emit.)
- **Codegen Rust:** Struct type → `#[derive(Clone, Debug, Serialize, Deserialize)] pub struct Foo { pub f: T, ... }`. Literal → `Foo { f: e, ... }`. Field access → `.f`. Update derives if any specific use case (e.g. PartialEq) is needed.

**Tech Stack:** Rust (parser/AST/HIR/typeck/codegen in `crates/vox-compiler`), Insta golden snapshots, golden vox examples under `examples/`.

**Out of scope (defer to follow-up plans):**
- Pattern matching on structs (separate plan: structural-match).
- Generic structs (`type Pair[A, B] { first: A, second: B }`) — `TypeDefDecl` already has `generics` but threading them through typeck for structs is its own piece.
- Methods on structs (`impl Foo { fn bar(self) ... }`) — separate plan once traits/impls land.
- Spread/rest in struct literals (`Foo { ..base, f: 9 }`).
- Anonymous structural typing (compatibility between two struct types with the same shape) — Vox is nominal.

---

## File Structure

**Created:**
- `crates/vox-compiler/tests/golden_struct_types.rs` (or extend an existing golden runner) — driven by new `examples/golden/struct_types.vox`.
- `examples/golden/struct_types.vox` — minimal struct decl + literal + field access + endpoint return.
- `docs/src/reference/lang-struct-types.md` — user-facing reference for the syntax.

**Modified:**
- `crates/vox-compiler/src/parser/descent/decl/mid.rs` — `parse_typedef` branches on `{` vs `=`.
- `crates/vox-compiler/src/ast/decl/typedef.rs` — keep `fields` member; doc-comment that it's now populated by the parser for struct syntax.
- `crates/vox-compiler/src/hir/nodes/decl.rs` — add `fields: Vec<HirField>` to `HirTypeDef`; introduce `HirField { name, ty, span }`.
- `crates/vox-compiler/src/hir/lower/decl.rs` — `lower_typedef` lowers fields.
- `crates/vox-compiler/src/typeck/**` — register struct definitions; resolve field access; typecheck struct literals.
- `crates/vox-compiler/src/ast/expr.rs` and HIR expr nodes — extend / add struct literal expression.
- `crates/vox-compiler/src/parser/descent/expr/**` — parse `Name { field: expr, ... }` as a primary expression.
- `crates/vox-compiler/src/codegen_ts/**` — emit type alias + object literal.
- `crates/vox-compiler/src/codegen_rust/**` — emit `pub struct` + literal.
- `apps/vox-mental-tracker/src/main.vox` — once landed, replace the planned 3-extractor stub with a single `parse_voice(t) to ParsedVoice` endpoint.

---

## Part A — Parser + AST

### Task A1: Decide on syntax via probe

- [ ] **Step 1: Lock the surface syntax.** Both `type Name { ... }` and `type Name = { ... }` are plausible. Pick **`type Name { f: T, ... }`** (no `=`) because:
  1. `@table type Name { ... }` already uses brace syntax, so symmetry is natural.
  2. `=` is reserved for the existing ADT/alias path; mixing them produces ambiguous LL(1) lookahead unless we branch on the next-after-name token.
  3. Decision: after `parse_ident_name()`, peek; if `LBrace`, parse as struct; if `Eq`, fall into existing ADT/alias branch; otherwise emit a clear parse error pointing at the missing delimiter.
- [ ] **Step 2: Document the decision** with a one-line comment at the top of `parse_typedef`.

### Task A2: Implement struct branch in `parse_typedef`

**Files:**
- Modify: `crates/vox-compiler/src/parser/descent/decl/mid.rs`
- Modify: `crates/vox-compiler/src/ast/decl/typedef.rs` (already has `fields`; just doc-update)

- [ ] **Step 1: Branch on `LBrace`** in `parse_typedef`. New code path mirrors `parse_table`'s field loop (read ident `:` type, optional comma, ident loop until `}`).
- [ ] **Step 2: Populate `TypeDefDecl.fields`** as `Vec<VariantField>` (reuse the existing field type — name + type_ann + span).
- [ ] **Step 3: Add a parser unit test** (in the existing parser test module): `type Foo { a: int, b: str }` parses to a `TypeDef` with two fields.

### Task A3: Struct literal expression

- [ ] **Step 1: Audit existing record-literal handling.** Search for how `db.X.insert({ ... })` is parsed/lowered. If `Expr::Record` (or similar anonymous record) exists, extend it; else add `Expr::StructLit`.
- [ ] **Step 2: Parser change.** When we see `Ident LBrace` at expression position, look ahead far enough to disambiguate from `if cond { ... }` and JSX. The simplest disambiguator: only accept struct literals when the brace is followed by `Ident Colon` (a field initializer) — otherwise fall back to the existing block/JSX path.
- [ ] **Step 3: Test.** Parse `let p = Foo { a: 1, b: "x" }`.

---

## Part B — HIR

### Task B1: Add `fields` to `HirTypeDef`

**Files:**
- Modify: `crates/vox-compiler/src/hir/nodes/decl.rs`
- Modify: `crates/vox-compiler/src/hir/lower/decl.rs`

- [ ] **Step 1:** Add `pub fields: Vec<HirField>` and a `HirField { pub name: String, pub ty: HirType, pub span: Span }` struct.
- [ ] **Step 2:** Update `lower_typedef` to lower AST `fields` into HIR `fields`.
- [ ] **Step 3:** Update any exhaustive matches over `HirTypeDef` fields (compile errors will surface them).

### Task B2: HIR for struct literal expression

- [ ] **Step 1:** Mirror the AST expression node into HIR (`HirExpr::StructLit { name, fields }` if you went that route).
- [ ] **Step 2:** Update HIR lowering to thread the new node through.
- [ ] **Step 3:** Update HIR walkers (`db_op_walk`, `dead_code`, `state_deps`, `async_flags`, `contracts`) to recurse into struct-literal field initializers.

---

## Part C — Typeck

### Task C1: Register struct types in the type environment

- [ ] **Step 1:** When walking `Decl::TypeDef` with non-empty `fields`, insert into the typeck env: `name → StructDef { fields: Vec<(String, Ty)> }`.
- [ ] **Step 2:** Make sure `Ty::Named(name)` lookups for declared structs resolve to the struct's field set when used in field-projection or literal-construction contexts.

### Task C2: Field access on struct values

- [ ] **Step 1:** Audit how `@table` row field access (`row.event_kind`) works today; reuse the same path. The receiver type's name resolves to either a table or a struct; both expose the same `fields` shape downstream.

### Task C3: Struct literal type-check

- [ ] **Step 1:** Look up the named type. If not a registered struct, error.
- [ ] **Step 2:** Require the initializer's field set equal the declared set (no extras, no missing). Emit clear errors naming the offending field(s).
- [ ] **Step 3:** For each initializer, unify its expression type with the declared field type. Surface mismatches with the field name in the error.

---

## Part D — Codegen

### Task D1: TypeScript codegen

**Files:**
- Modify: `crates/vox-compiler/src/codegen_ts/**`

- [ ] **Step 1:** Emit `export type Foo = { f: T, ... }` for each struct decl.
- [ ] **Step 2:** Emit struct literal as an object literal `{ f: e, ... }`.
- [ ] **Step 3:** Field access emits `.f` (likely already works — same path as record/table field access).
- [ ] **Step 4:** Add a golden snapshot covering type alias + literal + access.

### Task D2: Rust codegen

**Files:**
- Modify: `crates/vox-compiler/src/codegen_rust/**`

- [ ] **Step 1:** Emit `#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)] pub struct Foo { pub f: T, ... }`.
- [ ] **Step 2:** Emit struct literal as `Foo { f: e, ... }`.
- [ ] **Step 3:** Field access emits `.f`.
- [ ] **Step 4:** Add a golden snapshot.

---

## Part E — Integration

### Task E1: Golden example

- [ ] **Step 1:** `examples/golden/struct_types.vox` — declares `type Point { x: int, y: int }`, an endpoint that constructs and returns one, and a consumer that reads fields. Must `vox check` cleanly.

### Task E2: Wire into vox-mental-tracker

- [ ] **Step 1:** In `apps/vox-mental-tracker/src/main.vox`, replace the `voice_*` extractor stubs (planned in 2026-05-08-app-phase2-voice-e2e.md) with a single `type ParsedVoice { kind: str, payload_json: str, confidence: float }` and `@endpoint fn parse_voice(t: str) to ParsedVoice`.
- [ ] **Step 2:** Update `VoicePage` save handler to consume the struct directly.
- [ ] **Step 3:** Remove the per-extractor endpoints.

---

## Verification

- [ ] `cargo nextest run -p vox-compiler` passes.
- [ ] `vox check examples/golden/struct_types.vox` passes.
- [ ] `vox check apps/vox-mental-tracker/src/main.vox` (after E2 completes) passes.
- [ ] Manual: render `vox build` for a sample and confirm the emitted TS / Rust round-trips.
