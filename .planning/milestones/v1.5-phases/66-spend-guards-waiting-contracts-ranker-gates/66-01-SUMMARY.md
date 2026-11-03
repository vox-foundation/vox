# Plan 66-01 Summary: Effect-Enforced Spend/Write/ExternalApi Guards & <WaitingNotice> View Contracts

## Execution Summary
- **Phase**: 66-spend-guards-waiting-contracts-ranker-gates
- **Plan**: 01
- **Status**: Complete & Verified

## Accomplishments
1. **Algebraic Effect System Extended (`RANK-01`)**:
   - Added `Spend`, `Write`, and `ExternalApi` to:
     - `EffectAnnotation` (`crates/vox-ast/src/decl/effect.rs`)
     - `HirEffectKind` (`crates/vox-compiler/src/hir/nodes/effect.rs`)
     - `HirCapability` (`crates/vox-compiler/src/hir/nodes/decl.rs`)
   - Implemented lowering in `crates/vox-compiler/src/hir/lower/decl.rs` and capability projection in `crates/vox-compiler/src/required_capabilities.rs`.
   - Enforced compile-time `E_QUERY_FORBIDDEN_EFFECT` in `crates/vox-compiler/src/typeck/effect_check.rs`:
     - `query` endpoints are statically verified as read-only and budget-neutral.
     - Declaring or transitively calling functions that require `Spend`, `Write`, or `ExternalApi` emits `E_QUERY_FORBIDDEN_EFFECT`.
   - Verified with unit tests in `crates/vox-compiler/src/typeck/effect_check.rs` (21/21 tests passed).

2. **First-Class `<WaitingNotice>` View Contract (`RANK-02` / `A1-04`)**:
   - Implemented `WaitingNoticeSpec` and `emit_waiting_notice_tsx` in `crates/vox-codegen/src/web_ir/async_state.rs`.
   - Replaces blank screens during rate limits, cooldowns, or external backoff with structured `<aside data-vox-waiting-notice="true" data-testid="vox-waiting-notice">`.
   - Statically validates that `reason`, `resume_at`, and `action_href` are non-empty strings.
   - Verified with unit tests in `crates/vox-codegen/src/web_ir/async_state.rs` (4/4 tests passed).

## Verification Evidence
```bash
env RUSTC_WRAPPER="" timeout 900s cargo test -p vox-compiler --lib effect_check::tests
# test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured

env RUSTC_WRAPPER="" timeout 900s cargo test -p vox-codegen --lib web_ir::async_state::tests
# test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured
```
