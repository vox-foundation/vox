# Language: Match-arm statement bodies — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Allow `match` arms to contain statement-level constructs — `return`, `break`, multi-statement blocks — not just expressions.

**Why now:** Hit during vox-mental-tracker development. Today writing:
```
match db.X.all() {
    Ok(rows) => { ... }
    Error(_) => return "{\"error\":\"db\"}"   // parse error
}
```
forces a contortion: assign the result to a local then `return` after the `match`, even when one arm logically wants to bail early. Reduces clarity for the most common error-handling shape in the codebase.

**Architecture:**
- The match-arm body parser (`parse_match_arm` or equivalent) currently parses an expression. Extend it to optionally parse a block-of-statements when the body starts with `return`, `break`, `{`, or other statement-leaders.
- HIR change: `HirMatchArm.body` becomes `Vec<HirStmt>` ending in an optional tail expression — i.e. the same shape as a function body. Existing arms with a single expression desugar to a 1-element body with a tail expression.
- Codegen TS: arm body becomes a thunk `(() => { ...stmts; return tail; })()` only when statements are present; bare-expression arms keep their current emitted shape (zero-cost path).
- Codegen Rust: arm body becomes `{ stmts; tail }` block, which is already idiomatic.

**Tech Stack:** Rust (parser + HIR + codegen).

**Out of scope:**
- Pattern guards (`Ok(x) if x > 0 => ...`) — separate concern, may already work.
- Or-patterns (`Ok(_) | Err(_)`) — separate plan.

---

## File Structure

**Modified:**
- `crates/vox-compiler/src/parser/descent/expr/match.rs` (or wherever match arms parse) — accept stmt bodies.
- `crates/vox-compiler/src/ast/expr.rs` — `MatchArm.body` becomes `Block` (or `Vec<Stmt> + Option<Expr>`).
- `crates/vox-compiler/src/hir/nodes/expr.rs` — same shape change for `HirMatchArm`.
- `crates/vox-compiler/src/hir/lower/expr.rs` — pass stmts through.
- `crates/vox-compiler/src/codegen_ts/**` and `codegen_rust/**` — emit blocks/thunks.

**Created:**
- `examples/golden/match_arm_stmts.vox` covering `return`, multi-stmt block, and bare-expr arms in one match.

---

## Tasks

- [ ] **A1.** Audit current match-arm AST/parser. Confirm whether the AST already has a block shape that can be reused.
- [ ] **A2.** Extend the parser to read either a single expression OR a `{ ... }` block OR a leading `return ...` / `break ...` statement.
- [ ] **A3.** Migrate existing AST/HIR consumers to handle the new shape (most likely a single field type change).
- [ ] **A4.** Codegen: TS thunks where statements are present; Rust blocks always.
- [ ] **A5.** Golden example + snapshot.
- [ ] **A6.** Update the relevant section of `docs/src/reference/syntax-and-semantics.md` (or equivalent).

---

## Verification

- [ ] `cargo nextest run -p vox-compiler` passes.
- [ ] `vox check examples/golden/match_arm_stmts.vox` passes.
- [ ] All existing matches in the workspace still type-check (regression suite catches this).
