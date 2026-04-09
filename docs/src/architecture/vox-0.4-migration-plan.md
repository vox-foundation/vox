---
title: "Vox 0.4 Grand Migration Plan"
description: "Comprehensive research-to-practice implementation plan: 300+ atomic tasks translating 9 deep research clusters into a greenfield Vox 0.4 standard."
category: "architecture"
status: "roadmap"
research_source: "gemini_deep_research"
research_date: "2026-04-08"
training_eligible: true
last_updated: 2026-04-09
---

# Vox 0.4 Grand Migration Plan

## How to Read This Document

**For an implementing agent:** This document is structured as a priority-ordered sequence of atomic task cards. Execute tasks within each wave sequentially. Each task names the exact file, function, or struct to modify, with code examples where the change is non-trivial. Verify each task against the milestone gate at the end of its wave before proceeding.

**Supporting documents (read in this order):**
1. `research-synthesis-grand-strategy-seed-2026.md` — strategic framework connecting 9 research tracks
2. `vox_agentic_loop_and_mens_plan.md` — existing 254-task blueprint (subsumed; this plan extends and corrects it)
3. The 9 research clusters under `docs/src/architecture/research-*-2026.md` — primary evidence base
4. `expl-architecture.md` — current compiler pipeline overview
5. `expl-ml-pipeline.md` — current MENS training pipeline overview

**Key corrections this plan makes to the existing 254-task blueprint:**
- Blueprint tasks 211-217 use reward weights `0.6/0.3/0.1` — replaced here with gated multiplication (§P2-W1)
- Blueprint tasks 195-197 use GBNF/FSA constrained gen — replaced here with EBNF/Earley/PDA (§P1-W3)
- Blueprint task 214 uses mean-baseline GRPO — replaced here with median-centered MC-GRPO (§P2-W1)
- Blueprint task ~91 refines word-count adequacy — replaced here with LLM-as-judge (§P3-W2)

**Codebase key findings that reduce or increase scope:**
- `Diagnostic` struct already has `expected_type`, `found_type`, `fixes`, `line_col`, `code`, `category` — Phase 1 changes are smaller than initially estimated
- `match_exhaust.rs` already computes missing variants as `Vec<&str>` — just needs structured field instead of string formatting
- Lexer already has `@forall`, `@fuzz`, `@require`, `@ensure`, `@invariant` tokens — parser wiring is partially done
- HIR already has `foralls: Vec<HirForall>` — PBT infrastructure is scaffolded
- `vox-eval` is entirely regex-based (Gap G-15) — replacement with real parser is high-impact
- `speech_constraints.rs` is prompt-hint only, no actual logits masking — constrained-gen is fully greenfield
- Trust system uses fixed EWMA alpha=0.10 with greedy routing — replacement is a coordinated multi-file change

---

## Priority Ordering Rationale

Phases are ordered by **impact-to-effort ratio**, not by pillar:

| Priority | Phase | Rationale |
| :---- | :---- | :---- |
| P0 | Compiler error payloads + `vox-eval` parser upgrade | Highest leverage: 63% autonomous fix rate (research ref 27). Smallest effort: `Diagnostic` already 80% structured. |
| P1 | Grammar export + constrained inference | Eliminates entire class of syntax hallucinations. Required before GRPO training can be meaningful. |
| P2 | MENS reward overhaul + corpus gating | Prevents catastrophic training failures. Current reward function is provably pathological. |
| P3 | Trust/routing + plan adequacy | Prevents WTA collapse and evaluation hacking. Moderate effort, high long-term value. |
| P4 | Language syntax reduction + IR-first | Reduces K-complexity → fewer hallucinations. Some items are long-term. |
| P5 | Testing infrastructure | Essential for oracle problem but depends on P0-P1 compiler work. |
| P6 | Cost defense + mesh economics | Important for production but not blocking other phases. |
| P7 | Data organization + CI gates | Operational quality; depends on all above. |

---

# P0 — COMPILER DIAGNOSTICS & EVAL UPGRADE

**Research source:** TS-Hallucination cluster (compiler feedback as oracle, §3.1–§3.2)
**Estimated benefit:** +20–30% autonomous repair rate for LLM-generated Vox code
**Risk:** LOW — extending existing, well-tested structures
**Effort:** 2–4 days

## P0-W1: Structured Diagnostic Fields

### P0-001: Add `missing_cases` field to `Diagnostic`
**File:** `crates/vox-compiler/src/typeck/diagnostics.rs` (line 74, `Diagnostic` struct)
**Action:** Add field `pub missing_cases: Vec<String>` after line 91 (`fixes` field). Initialize to `vec![]` in all existing constructors (`error()`, `warning()`, `hir_invariant()`, `lowering()`).
**Verification:** `cargo check -p vox-compiler` passes.

### P0-002: Add `ast_node_kind` field to `Diagnostic`
**File:** `crates/vox-compiler/src/typeck/diagnostics.rs`
**Action:** Add field `pub ast_node_kind: Option<String>` after `missing_cases`. Initialize to `None` in all constructors.
**Verification:** `cargo check -p vox-compiler` passes.

### P0-003: Populate `missing_cases` in match exhaustiveness checker
**File:** `crates/vox-compiler/src/typeck/checker/match_exhaust.rs` (lines 71-81)
**Current code:**
```rust
if !missing.is_empty() {
    diags.push(Diagnostic::error(
        format!(
            "Non-exhaustive match on type '{}'. Missing variant(s): {}",
            type_name,
            missing.join(", ")
        ),
        span,
        source,
    ));
}
```
**Change to:**
```rust
if !missing.is_empty() {
    let mut d = Diagnostic::error(
        format!(
            "Non-exhaustive match on type '{}'. Missing variant(s): {}",
            type_name,
            missing.join(", ")
        ),
        span,
        source,
    );
    d.missing_cases = missing.iter().map(|s| s.to_string()).collect();
    d.ast_node_kind = Some("MatchExpr".to_string());
    diags.push(d);
}
```
**Verification:** Existing match exhaustiveness tests still pass; new field is populated.

### P0-004: Add `missing_cases` to JSON serialization output
**File:** `crates/vox-compiler/src/typeck/diagnostics.rs`
**Action:** Ensure the `#[serde(default, skip_serializing_if = "Vec::is_empty")]` attribute is on `missing_cases` (follows existing pattern for `fixes`).
**Verification:** `vox check --json` on a file with non-exhaustive match includes `"missing_cases": ["VariantName"]` in output.

### P0-005: Integration test for structured match error payload
**File:** `crates/vox-integration-tests/tests/pipeline/` (new file `match_exhaust_payload.rs`)
**Action:** Write test that:
1. Defines ADT `type Color = | Red | Green | Blue | Yellow`
2. Writes `match c { Red -> 1, Green -> 2, Blue -> 3 }` (missing `Yellow`)
3. Asserts diagnostic has `missing_cases == ["Yellow"]`
4. Asserts diagnostic has `ast_node_kind == Some("MatchExpr")`
**Verification:** `cargo test -p vox-integration-tests match_exhaust_payload`

### P0-006: Enrich `Diagnostic` with stable error codes
**File:** `crates/vox-compiler/src/typeck/diagnostics.rs`
**Current state:** `code` field exists but is only set for reactive component diagnostics (e.g., `"typecheck.reactive.state"`).
**Action:** Define a `const` block of stable error codes:
- `E0101` = type mismatch
- `E0201` = unknown identifier
- `E0301` = non-exhaustive match
- `E0401` = argument count mismatch
- `E0501` = HIR invariant violation
Set `d.code = Some("E0301".into())` in `match_exhaust.rs` and equivalents elsewhere.
**Verification:** JSON output includes stable `"code"` field on all error diagnostics.

### P0-007: `--output-format json` flag for `vox check`
**File:** `crates/vox-cli/src/commands/check.rs`
**Current state:** Already supports `--json` via `VOX_CLI_GLOBAL_JSON` env var (line 11).
**Action:** This is already implemented. Document it in `docs/src/reference/cli.md` under `vox check`. No code change needed.
**Verification:** `vox check src/main.vox --json` produces JSON array of diagnostics.

## P0-W2: Replace Regex-Based `vox-eval` with Parser

### P0-008: Add `ast_eval` function using real parser
**File:** `crates/vox-eval/src/lib.rs`
**Action:** Add new public function:
```rust
pub fn ast_eval(code: &str) -> AstEvalReport {
    let tokens = vox_compiler::lexer::lex(code);
    match vox_compiler::parser::parse(tokens) {
        Ok(module) => {
            let constructs = count_hir_constructs(&module);
            AstEvalReport {
                parse_success: true,
                node_count: constructs.total,
                construct_histogram: constructs.histogram,
                has_tests: constructs.has_tests,
                error_span: None,
            }
        }
        Err(errors) => AstEvalReport {
            parse_success: false,
            node_count: 0,
            construct_histogram: HashMap::new(),
            has_tests: false,
            error_span: errors.first().map(|e| e.span),
        },
    }
}
```
**Dependency:** Add `vox-compiler` to `vox-eval/Cargo.toml` dependencies.
**Verification:** `cargo test -p vox-eval ast_eval`

### P0-009: Define `AstEvalReport` struct
**File:** `crates/vox-eval/src/lib.rs`
```rust
#[derive(Debug, Clone, Serialize)]
pub struct AstEvalReport {
    pub parse_success: bool,
    pub node_count: usize,
    pub construct_histogram: HashMap<String, usize>,
    pub has_tests: bool,
    pub error_span: Option<vox_compiler::ast::span::Span>,
}
impl AstEvalReport {
    pub fn coverage_score(&self) -> f64 {
        if !self.parse_success { return 0.0; }
        (self.construct_histogram.len() as f64 / 8.0).min(1.0)
    }
}
```

### P0-010: Implement `count_hir_constructs` helper
**File:** `crates/vox-eval/src/lib.rs`
**Action:** Walk `Module` AST and count each declaration type (functions, actors, workflows, tables, tests, etc.). Return a struct with `total: usize`, `histogram: HashMap<String, usize>`, `has_tests: bool`.

### P0-011: Deprecate `detect_constructs` and `construct_coverage_score`
**File:** `crates/vox-eval/src/lib.rs` (lines 193-207)
**Action:** Add `#[deprecated(since = "0.4.0", note = "Use ast_eval() for parser-backed evaluation")]` to both functions.
**Verification:** `cargo check -p vox-eval` emits deprecation warnings for callers.

### P0-012: Update `vox corpus eval` to use `ast_eval`
**File:** Grep for `construct_coverage_score` calls across the workspace. Update each to use `ast_eval(code).coverage_score()`.
**Verification:** `vox corpus eval` produces identical or better parse_rate numbers.

### P0-013: Update `vox eval --mode ast` CLI integration
**File:** `crates/vox-cli/src/` (grep for `vox-eval` usage)
**Action:** Add `--mode ast` flag that routes through `ast_eval` instead of regex `detect_constructs`.

### P0-014: Tests for `ast_eval`
**File:** `crates/vox-eval/src/lib.rs` (test module)
- Test: valid Vox function → `parse_success=true`, `construct_histogram["fn"] >= 1`
- Test: invalid snippet (missing `}`) → `parse_success=false`, `error_span.is_some()`
- Test: file with `@test` → `has_tests=true`
- Test: empty string → `parse_success=false`

**P0 Milestone Gate:** `vox check --json` produces structured diagnostics with `missing_cases`, `ast_node_kind`, and stable error codes. `ast_eval` replaces regex detection in eval pipeline.

---

# P1 — GRAMMAR EXPORT & CONSTRAINED INFERENCE

**Research source:** Grammar Constraints cluster (§1–§6), Grand Strategy Seed §B1
**Estimated benefit:** 100% syntax validity on constrained generations (vs. ~85% unconstrained)
**Risk:** MEDIUM — constrained-gen is greenfield; grammar export extends existing parser knowledge
**Effort:** 3–6 weeks

## P1-W1: Grammar Export (EBNF-First)

### P1-001: Create `crates/vox-grammar-export/Cargo.toml`
**Action:** New crate with dependencies on `vox-compiler` (for parser types).

### P1-002: Create `crates/vox-grammar-export/src/lib.rs`
**Action:** Define `GrammarFormat` enum (`Ebnf`, `Gbnf`, `Lark`, `JsonSchema`) and `GrammarExportResult`.

### P1-003: Catalog all production rules from the parser
**File:** `crates/vox-compiler/src/parser/descent/mod.rs` and sub-modules
**Action:** Read every `parse_*` function in the descent parser. For each, extract:
- Rule name (e.g., `fn_decl`, `match_expr`, `type_def`)
- Alternatives (each branch in the function)
- Terminal tokens consumed
Document in `docs/src/architecture/vox-grammar-production-rules.md`.

### P1-004: Implement EBNF emitter
**File:** `crates/vox-grammar-export/src/ebnf.rs`
**Action:** For each production rule from P1-003, emit EBNF syntax. The emitter walks the catalog, not the parser code directly.

### P1-005: Implement GBNF emitter (lossy secondary)
**File:** `crates/vox-grammar-export/src/gbnf.rs`
**Action:** Convert EBNF → GBNF with recursion-depth cap (configurable, default 8). Emit warning comment for every rule where recursion was truncated.
**Research rationale:** §2.1 proves FSAs cannot handle recursive CFGs natively. The cap is explicit damage limitation.

### P1-006: Implement Lark emitter
**File:** `crates/vox-grammar-export/src/lark.rs`
**Action:** Emit Lark-compatible grammar for `llguidance` bridge integration.

### P1-007: Implement grammar versioning
**File:** `crates/vox-grammar-export/src/versioning.rs`
**Action:** Compute a hash of all production rules. Embed as semver suffix. `vox ci grammar-drift` fails if hash changes without version bump.

### P1-008: CLI command `vox grammar export`
**File:** `crates/vox-cli/src/commands/` (new `grammar.rs`)
**Action:** `vox grammar export --format ebnf|gbnf|lark|json-schema --output <file>`

### P1-009: MCP tool `vox_grammar_export`
**File:** `crates/vox-mcp/src/tools/` (new entry in tool registry)
**Action:** Expose grammar export as MCP tool for agent self-use.

### P1-010: Replace `llm_prompt.rs` with derived grammar cheatsheet
**File:** `crates/vox-compiler/src/llm_prompt.rs` (currently 59 lines, hand-written)
**Action:** Replace the hardcoded string with a call to `vox_grammar_export::emit_cheatsheet()` that generates a compact, token-efficient grammar summary from the EBNF export. Target: <200 tokens for the grammar prompt (current is ~350 tokens).
**Research rationale:** K-Complexity research proves every unnecessary token increases hallucination surface.

### P1-011–P1-015: Tests for grammar export
- P1-011: Emitted EBNF parses without error in a reference EBNF validator
- P1-012: 10 known-valid Vox programs accepted by the GBNF
- P1-013: 5 known-invalid programs rejected by the GBNF
- P1-014: Grammar version hash changes when a new keyword is added to lexer
- P1-015: Lark output is accepted by a Lark parser (if available)

## P1-W2: Constrained Inference Engine

### P1-016: Create `crates/vox-constrained-gen/Cargo.toml`
**Action:** New crate. Dependencies: `vox-grammar-export`, `vox-compiler` (for `Span`, `Token`).

### P1-017: Define `ConstrainedSampler` trait
**File:** `crates/vox-constrained-gen/src/lib.rs`
```rust
pub trait ConstrainedSampler: Send + Sync {
    fn mask_logits(&mut self, logits: &[f32]) -> Vec<f32>;
    fn feed_token(&mut self, token_id: u32);
    fn is_complete(&self) -> bool;
    fn is_deadlocked(&self) -> bool;
    fn reset(&mut self);
}
```

### P1-018: Implement Earley parser backend
**File:** `crates/vox-constrained-gen/src/earley.rs`
**Action:** Earley parser that consumes the EBNF grammar export. On each token, advances the Earley items and computes the set of valid next tokens. Returns a bitmask over the vocabulary.
**Research rationale:** §1.1 shows Earley parsers handle CFG recursion natively with ~50µs/token overhead.

### P1-019: Implement context-independent token cache (PDA strategy)
**File:** `crates/vox-constrained-gen/src/pda.rs`
**Action:** Classify ~99% of vocabulary tokens as "context-independent" (always valid or always invalid regardless of stack state). Pre-compute bitmask for these. Only evaluate ~1% "context-dependent" tokens at runtime.
**Research rationale:** §1.1 (XGrammar-2) shows this achieves <40µs/token.

### P1-020: Implement deadlock watchdog
**File:** `crates/vox-constrained-gen/src/deadlock.rs`
**Action:** Configurable timeout (default 5s per generation). On deadlock (empty valid token set), emit `VoxValidationError` and signal retry.
**Research rationale:** §4.1–§4.3 proves deadlocks are systemic, not edge cases. CVE-2026-2069 shows GBNF crashes on nested recursion.

### P1-021: Define `VoxValidationError`
**File:** `crates/vox-compiler/src/parser/error.rs`
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxValidationError {
    pub code: String,
    pub span: Option<Span>,
    pub message: String,
    pub suggested_correction: Option<String>,
    pub partial_output: Option<String>,
}
```

### P1-022: Implement "Stream of Revision" backtrack mechanism
**File:** `crates/vox-constrained-gen/src/revision.rs`
**Action:** Inject a special `<REVISE>` token into the vocabulary. When the LLM emits it, the constrained sampler enters edit mode: the next tokens are cursor operations (delete-back-N, replace) that modify the generated history within the same forward pass.
**Research rationale:** §6.2 proves this prevents the LLM from cornering itself in invalid states.

### P1-023: Integration into `vox populi serve`
**File:** `crates/vox-populi/src/` (serving layer)
**Action:** Add `?grammar=vox` query parameter or `X-Vox-Grammar: true` header. When set, wrap the generation loop with the `ConstrainedSampler`.

### P1-024: Wire into MCP code generation
**File:** `crates/vox-mcp/src/speech_constraints.rs`
**Action:** Replace the no-op `ConstrainedDecodePolicy` (lines 84-109) with actual `ConstrainedSampler` integration. The `from_env()` resolver now checks for the PDA backend and enables real logit masking when available.

### P1-025–P1-030: Tests for constrained inference
- P1-025: Constrained sampler produces only grammar-accepted tokens (10 random prompts)
- P1-026: Deadlock watchdog triggers within 6s on adversarial prompt
- P1-027: 50 consecutive constrained generations achieve 100% parse rate
- P1-028: Revision token correctly backtracks 3 tokens in a test case
- P1-029: Context-independent cache achieves >95% vocabulary coverage
- P1-030: Performance benchmark: <100µs/token overhead on RTX 4080

**P1 Milestone Gate:** `vox grammar export --format ebnf` produces valid EBNF. 100 consecutive constrained-inference generations achieve 100% parse rate.

---

# P2 — MENS TRAINING PIPELINE OVERHAUL

**Research source:** GRPO Reward Shaping cluster (all 7 pages), Continual Learning cluster (all 8 pages)
**Estimated benefit:** Eliminate reward hacking, prevent mode collapse, +15% OOD performance
**Risk:** HIGH — changes to reward function and training loop are irreversible without checkpoint rollback
**Effort:** 4–8 weeks

## P2-W1: Reward Function & GRPO Core

### P2-001: Create `crates/vox-tensor/src/grpo.rs`
**Action:** New module for GRPO training loop. This is blueprint task 209, but with corrected reward function.

### P2-002: Implement gated reward function
**File:** `crates/vox-tensor/src/grpo.rs`
```rust
pub struct RewardWeights {
    pub test_weight: f64,      // default 0.7
    pub conciseness_weight: f64, // default 0.3
    pub max_expected_len: usize, // default 2000 chars
}

pub fn compute_reward(
    candidate: &str,
    test_results: &TestResults,
    weights: &RewardWeights,
) -> f64 {
    let r_syntax = if parse_vox(candidate).is_ok() { 1.0 } else { 0.0 };
    let r_test = test_results.pass_rate();
    let r_conciseness = 1.0 - (candidate.len() as f64 / weights.max_expected_len as f64).min(1.0);
    // GATED: syntax is a multiplier, not additive
    r_syntax * (weights.test_weight * r_test + weights.conciseness_weight * r_conciseness)
}
```
**Critical change from blueprint:** Replaces `0.6*syntax + 0.3*test + 0.1*coverage`. Syntax=0 → total reward=0. No AST density metric.
**Research rationale:** §AST reward hacking proves density metric causes Goodhart's Law exploitation. §Reward weights proves 0.6 syntax weight creates pathological local optima.

### P2-003: Implement median-centered advantage computation (MC-GRPO)
**File:** `crates/vox-tensor/src/grpo.rs`
```rust
pub fn compute_advantages(rewards: &[f64]) -> Vec<f64> {
    let mut sorted = rewards.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len()/2 - 1] + sorted[sorted.len()/2]) / 2.0
    } else {
        sorted[sorted.len()/2]
    };
    let std = (rewards.iter().map(|r| (r - median).powi(2)).sum::<f64>() / rewards.len() as f64).sqrt().max(1e-8);
    rewards.iter().map(|r| (r - median) / std).collect()
}
```
**Critical change from blueprint:** Replaces mean with median baseline.
**Research rationale:** §VRAM small-batch proves k=8 mean baseline causes advantage sign flipping. MC-GRPO eliminates this.

### P2-004: Implement GRPO policy gradient update with asymmetric clipping (DAPO)
**File:** `crates/vox-tensor/src/grpo.rs`
**Action:** Implement PPO-clip style update with asymmetric clip bounds and NO KL penalty.
**Research rationale:** §VRAM efficiency proves KL penalty removal saves ~4GB VRAM (reference model offloaded).

### P2-005: Implement `generate_k_candidates`
**File:** `crates/vox-tensor/src/grpo.rs`
**Action:** Generate k=8 candidate completions for a prompt at temperature 0.8 using the active policy model.

### P2-006: Implement `score_candidate` using real parser
**File:** `crates/vox-tensor/src/grpo.rs`
**Action:** For each candidate, compute `RewardSignal { parse_score, test_score, conciseness_score, composite }` using the gated reward function from P2-002.

### P2-007: GRPO training loop orchestrator
**File:** `crates/vox-tensor/src/grpo.rs`
**Action:** Main training loop:
```
for each prompt in training_set:
    candidates = generate_k_candidates(prompt, model, k=8, temp=0.8)
    rewards = [score_candidate(c) for c in candidates]
    advantages = compute_advantages(rewards)  // MC-GRPO median baseline
    policy_gradient_update(model, candidates, advantages)  // DAPO asymmetric clip
    persist_telemetry(rewards, advantages)
```

### P2-008: `GrpoConfig` struct
**File:** `crates/vox-tensor/src/grpo.rs`
```rust
pub struct GrpoConfig {
    pub k_samples: usize,        // default 8
    pub temperature: f64,         // default 0.8
    pub reward_weights: RewardWeights,
    pub policy_lr: f64,          // default 1e-5
    pub clip_epsilon: f64,       // default 0.2
    pub clip_upper: f64,         // default 0.28 (asymmetric, DAPO)
    pub max_steps: usize,        // default 500
    pub min_corpus_pairs: usize, // default 1000 (hard gate)
}
```

### P2-009: Hard corpus gate
**File:** `crates/vox-cli/src/training/` (mens train dispatch)
**Action:** Before starting GRPO, count validated corpus pairs. If < `config.min_corpus_pairs` (default 1000), refuse to start and print:
```
Error: Corpus has {n} validated pairs (minimum: 1000).
Use `vox mens serve --rag` for in-context learning until corpus reaches threshold.
See: docs/src/architecture/research-cl-qlora-minimum-corpus-2026.md
```
**Research rationale:** CL minimum corpus research proves <500 pairs guarantees catastrophic overfitting. Current corpus is 340 pairs (Gap G-11).

### P2-010: CLI flag `vox mens train --mode grpo`
**File:** `crates/vox-cli/src/commands/`
**Action:** Add `--mode grpo` flag alongside existing `--backend qlora`.

### P2-011–P2-015: GRPO tests
- P2-011: `compute_reward` with syntax=0 → total=0 regardless of test score
- P2-012: `compute_advantages` with median: no sign flips when one outlier reward=0.9 in group of 0.2s
- P2-013: `compute_advantages` with mean (old): verify sign flipping does occur (regression test for the bug)
- P2-014: Corpus gate refuses training with 340 pairs
- P2-015: GRPO loop completes 10 steps without panic (CPU mode, small test model)

## P2-W2: Negative Sample Integration

### P2-016: Ingest parse failures as hard negatives
**File:** `crates/vox-tensor/src/grpo.rs`
**Action:** Failed parses get reward=0 and enter the GRPO group directly. The median-centered advantage estimator naturally assigns negative advantages.
**Critical change from blueprint:** Blueprint task 221 separates failures into SFT phase. Research proves this 15.81% worse on OOD.

### P2-017: Wire MCP validation failures to corpus
**File:** `crates/vox-mcp/src/tools/compiler_tools.rs`
**Action:** When `validate_file` or `vox_generate_code` produces errors, call `auto_ingest_negative(code, errors)` to add the failed snippet as a negative training example.

### P2-018: Implement Anna Karenina sampling
**File:** `crates/vox-tensor/src/sampling.rs` (new)
**Action:** Batch constructor that ensures minimum 30% negative examples per GRPO group. Draws negatives from the model's own recent rollout failures.
**Research rationale:** §Positive-only optimization proves this maintains +35% policy entropy.

### P2-019: Test: Anna Karenina sampler composition
- Verify batch of 8 always has at least 2 negatives (ceil(8 * 0.3) = 3)
- Verify negatives are drawn from model's own failures, not random data

## P2-W3: Corpus Quality & Catastrophic Forgetting

### P2-020: Tag corpus pairs with `origin`
**File:** `crates/vox-corpus/src/` (pair schema)
**Action:** Add `origin: Origin` enum (`Human`, `Synthetic`, `Agent`) to `TrainingPair`.

### P2-021: Enforce human ratio in training batches
**File:** `crates/vox-tensor/src/grpo.rs` (batch construction)
**Action:** Minimum 15% `origin=Human` in every training batch. Log warning if ratio drops below 20%.

### P2-022: Implement experience replay buffer
**File:** `crates/vox-tensor/src/replay.rs` (new)
**Action:** `ReplayBuffer` that mixes 10% base pre-training data samples into each QLoRA fine-tuning batch. Use density-estimation-based `mix-cd` strategy to prioritize "collateral damage" samples.

### P2-023: Implement collateral damage rate monitoring
**File:** `crates/vox-eval/src/` (new `collateral.rs`)
**Action:** `eval_collateral_damage(model, held_out_benchmark) -> f64`. Run before and after each training run. Fail promotion if degradation >5%.

### P2-024: AI slop curator gate for Schola/Scientia
**File:** `crates/vox-schola/src/` (new `curator.rs`)
**Action:** `curator_validate(prose: &str) -> CuratorVerdict`. Calls a frontier API model to score typicality bias. Rejects prose with typicality >0.7.

### P2-025–P2-030: Corpus quality tests
- P2-025: `origin=Human` ratio enforcement blocks batch with 0% human
- P2-026: Replay buffer includes pre-training samples in every batch
- P2-027: Collateral damage function returns 0.0 for identical model
- P2-028: Curator rejects "Here is a comprehensive guide to..." (typical slop)
- P2-029: Curator accepts terse, factual documentation
- P2-030: Corpus metadata tracks `origin` distribution per split

**P2 Milestone Gate:** GRPO dry-run with gated reward: mean reward >0.4, zero advantage sign flips in 100 steps. `vox mens train` refuses to start with <1000 corpus pairs.

---

# P3 — ORCHESTRATION: TRUST, ROUTING & CONTEXT

**Research source:** Trust Reliability cluster, Plan Adequacy cluster, Context Handoff cluster
**Estimated benefit:** Eliminate WTA routing collapse, prevent evaluation hacking, prevent context bleed
**Risk:** MEDIUM — multi-file coordinated change across orchestrator, db, and MCP
**Effort:** 3–5 weeks

## P3-W1: Trust System Overhaul

### P3-001: Add `variance` field to `AgentTrustScore`
**File:** `crates/vox-orchestrator/src/attention/routing.rs` (line 8, struct)
**Action:** Add `pub variance: f64` field. Initialize to 0.25 (high uncertainty) in `new()`.

### P3-002: Add `process_noise` and `measurement_noise` to trust config
**File:** `crates/vox-orchestrator/src/` (OrchestratorConfig)
**Action:** Add `trust_process_noise: f64` (default 0.01) and `trust_measurement_noise: f64` (default 0.1).

### P3-003: Replace EWMA update with Kalman filter
**File:** `crates/vox-orchestrator/src/attention/routing.rs` (lines 35-52, `record_outcome`)
**Current code:**
```rust
self.trust_score = alpha * outcome + (1.0 - alpha) * self.trust_score;
```
**Replace with:**
```rust
let predicted_variance = self.variance + process_noise;
let kalman_gain = predicted_variance / (predicted_variance + measurement_noise);
self.trust_score += kalman_gain * (outcome - self.trust_score);
self.variance = (1.0 - kalman_gain) * predicted_variance;
```
**Research rationale:** §EWMA tracking failure proves fixed alpha is variance-blind and causes detection lag for performance degradation.

### P3-004: Replace greedy routing with UCB exploration
**File:** `crates/vox-orchestrator/src/services/routing.rs` (lines 177-182)
**Current code:**
```rust
if let Some((&best_agent, _)) = scores
    .iter()
    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
```
**Replace with:**
```rust
// UCB: score + exploration_bonus * sqrt(variance)
let ucb_scores: Vec<_> = scores.iter().map(|(id, score)| {
    let exploration = if let Some(trust_map) = attention_trust_scores {
        if let Some(ts) = trust_map.get(id) {
            config.ucb_exploration_weight * ts.variance.sqrt()
        } else { config.ucb_exploration_weight * 0.5 }  // high bonus for unknown agents
    } else { 0.0 };
    (id, score + exploration)
}).collect();
if let Some((&best_agent, _)) = ucb_scores
    .iter()
    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
```
**Research rationale:** §WTA routing collapse proves greedy selection starves low-sample agents and creates topological fragility.

### P3-005: Implement Empirical Bayes priors
**File:** `crates/vox-db/src/trust_telemetry.rs`
**Action:** Add `compute_empirical_prior() -> (f64, f64)` that reads all `trust_observations` and computes global α, β via Method of Moments. New agents initialize with this prior instead of fixed 0.3.

### P3-006: Add `ucb_exploration_weight` to `OrchestratorConfig`
**Action:** Default 1.0. Controls exploration vs. exploitation tradeoff.

### P3-007–P3-010: Trust system tests
- P3-007: Kalman filter converges faster than EWMA on sudden performance drop (5 observations)
- P3-008: UCB exploration: no single agent holds >60% task share over 100 random tasks
- P3-009: Empirical Bayes prior differs from 0.5 when system has historical data
- P3-010: Variance decreases with more observations (calibration test)

## P3-W2: Plan Adequacy Overhaul

### P3-011: Remove word-count complexity heuristic
**File:** `crates/vox-orchestrator/src/planning/plan_adequacy.rs` (lines 48-59)
**Action:** Replace `estimate_goal_word_complexity` with a call to Socrates LLM-as-judge. The judge receives the goal text and returns a structured complexity assessment.
**Research rationale:** §Complexity cap 9 proves Miller's Law is misapplied to LLMs.

### P3-012: Remove keyword vagueness blacklist
**File:** `crates/vox-orchestrator/src/planning/plan_adequacy.rs` (lines 81-95, `vague_phrases`)
**Action:** Replace with LLM-as-judge rubric that scores semantic coverage, not keyword presence.
**Research rationale:** §Regex validation proves keyword blocks miss semantic ambiguity and are trivially evaded.

### P3-013: Add precondition assertion requirement
**File:** `crates/vox-orchestrator/src/planning/`
**Action:** Each plan step that mutates state must declare at least one precondition. Fail adequacy if missing.

### P3-014: Socrates rubric for plan evaluation
**File:** `crates/vox-orchestrator/src/planning/plan_adequacy.rs`
**Action:** Define a structured rubric with 5 dimensions:
1. Coverage: Does the plan address all requirements in the goal?
2. Dependencies: Are chronological state dependencies explicitly declared?
3. Destructive actions: Are any implicit destructive operations identified?
4. Verification: Does the plan include verification steps?
5. Concreteness: Are actions specific enough to execute without interpretation?
Call Socrates with the rubric and the plan text. Score each dimension 0-2.

## P3-W3: Context Handoff

### P3-015: Define `ContextEnvelope` struct
**File:** `crates/vox-orchestrator/src/handoff/` (new module)
```rust
pub struct ContextEnvelope {
    pub task_id: String,
    pub thread_id: String,
    pub obo_token: String,           // On-Behalf-Of cryptographic token
    pub scoped_task: TaskDefinition, // Never raw transcript
    pub artifact_uris: Vec<String>,  // URIs to large data
    pub parent_agent_id: Option<AgentId>,
}
```

### P3-016: Implement OBO token generation
**File:** `crates/vox-orchestrator/src/handoff/`
**Action:** Generate ed25519 signed token binding task_id + thread_id + user scope. Verify on receipt.

### P3-017: Strip raw transcripts from handoff
**File:** `crates/vox-orchestrator/src/` (agent handoff code path)
**Action:** When Agent A delegates to Agent B, construct `ContextEnvelope` from the task definition only. Never include `conversation_history` or raw tool outputs.

### P3-018: Implement CRAG retrieval gateway
**File:** `crates/vox-orchestrator/src/retrieval/` (new module)
**Action:** Replace hardcoded "always retrieve" with a lightweight evaluator that classifies queries as:
- `TrustMemory` → use agent's local context
- `VectorRetrieval` → query vector store
- `WebSearch` → query web
- `Skip` → query is self-contained

### P3-019: Implement async memory distillation worker
**File:** `crates/vox-orchestrator/src/memory/`
**Action:** Background tokio task that periodically extracts semantic key-value pairs from conversation turns and persists to vector store. Prevents silent rolling truncation.

### P3-020–P3-022: Context handoff tests
- P3-020: Agent B cannot access Agent A's raw conversation history
- P3-021: OBO token verification fails with tampered payload
- P3-022: CRAG gateway returns `Skip` for simple arithmetic query

**P3 Milestone Gate:** Agent routing uses UCB exploration; no single agent holds >60% task share. Context handoff test: Agent B has zero visibility into Agent A's raw transcript.

---

# P4 — LANGUAGE SYNTAX K-COMPLEXITY REDUCTION

**Research source:** K-Complexity cluster, TS-Hallucination frontier cluster
**Estimated benefit:** ≥15% token reduction per construct → proportional hallucination reduction
**Risk:** MEDIUM — parser changes affect entire downstream pipeline
**Effort:** 3–6 weeks

## P4-W1: Boilerplate Reduction

### P4-001: K-complexity audit document
**File:** `docs/src/architecture/k-complexity-audit-0.4.md` (new)
**Action:** For each Vox construct (fn, type, actor, workflow, match, etc.), count required tokens vs. equivalent in Gleam, Zig, and Rust.

### P4-002: Implement `?` operator for Result unwrapping
**File:** `crates/vox-compiler/src/lexer/token.rs` (line 159, `Question` token already exists)
**File:** `crates/vox-compiler/src/parser/descent/` (expression parsing)
**Action:** Parse `expr?` as syntactic sugar for `match expr { Ok(v) -> v, Err(e) -> ret Err(e) }`. The `Question` token already exists in the lexer.
**Benefit:** Eliminates 3-line match boilerplate per error handling site. In the canonical app, this saves ~45 tokens across 15 error handling points.

### P4-003: Implement return type inference
**File:** `crates/vox-compiler/src/typeck/checker/`
**Action:** When function signature omits `to Type`, infer return type from the last expression in the body. Already common in Rust, Gleam, and Zig.
**Benefit:** Eliminates `to Type` annotation on simple functions. Saves ~2 tokens per function × ~20 functions in canonical app = ~40 tokens.

### P4-004: Implement `_` discard pattern in let bindings
**Action:** `let _ = side_effect()` — already parseable if `Ident("_")` is treated as discard.

## P4-W2: IR-First Architecture (Long-term)

### P4-005: Define Vox IR JSON schema
**File:** `contracts/vox-ir/vox-ir.v1.schema.json` (new)
**Action:** JSON Schema that mirrors `HirModule` structure. Each field maps to a semantic HIR concept.

### P4-006: Implement `vox emit-ir` CLI
**File:** `crates/vox-cli/src/commands/` (new `emit_ir.rs`)
**Action:** Parse → HIR → serialize `HirModule` as JSON → validate against schema → output.

### P4-007: Implement `vox compile-ir` CLI
**Action:** Deserialize JSON → reconstruct `HirModule` → run codegen_rust / codegen_ts.
**Research rationale:** Frontier research §1 recommends decoupling "LLM generates IR" from "human reads textual Vox."

---

# P5 — TESTING INFRASTRUCTURE

**Research source:** Compiler Testing cluster (all waves)
**Effort:** 2–4 weeks

### P5-001: `test` block syntax in parser
**File:** `crates/vox-compiler/src/parser/descent/mod.rs`
**Note:** `@test` decorator already exists (lexer token `AtTest`, line 111). The new `test "description" { }` syntax is Zig-style string-named. Parse as a new `Decl::NamedTest(name, body)`.

### P5-002: Compile-time stripping of test blocks
**Action:** `codegen_rust` and `codegen_ts` skip `NamedTest` declarations unless `--include-tests` flag is set.

### P5-003: `vox test` CLI subcommand
**Action:** Run all `@test` and `test` blocks in a file. Report pass/fail with timing.

### P5-004: LSP CodeLens for test blocks
**File:** `crates/vox-lsp/src/`
**Action:** Emit `textDocument/codeLens` with "▶ Run Test" above each test block.

### P5-005: Snapshot testing infrastructure
**Action:** `vox test --update-snapshots` records HIR output as `.snap` files. CI diffs.

### P5-006: `@forall` property-based testing
**Note:** Lexer token `AtForall` already exists (line 138). HIR has `foralls: Vec<HirForall>`. Wire the parser to produce `HirForall` nodes and implement a `proptest`-inspired runtime strategy layer.

### P5-007: `@spec` annotation for oracle generation
**Note:** `@require` and `@ensure` tokens already exist (lines 132-134). Wire parser to produce `HirRequire`/`HirEnsure` nodes. Lower to `debug_assert!` in codegen.

### P5-008: Parser roundtrip property test
**Action:** Add `proptest` to `vox-compiler` dev-dependencies. Implement `parse(unparse(ast)) == ast` for a subset of AST node types.

---

# P6 — COST DEFENSE & MESH ECONOMICS

**Research source:** Multi-Agent Mesh Economics cluster
**Effort:** 2–3 weeks

### P6-001–P6-005: 5-Layer circuit breakers
**File:** `crates/vox-scaling-policy/src/` (extend existing crate)
- P6-001: Hard per-task timeout (default 300s)
- P6-002: Recovery anti-loops (max 3 re-attempts per task/day)
- P6-003: Daily cost aggregate kill switch
- P6-004: Model pinning (prevent silent fallback to expensive frontier)
- P6-005: Monthly pacing with 80% spend early warning

### P6-006: Cascade routing matrix
**File:** `crates/vox-orchestrator/src/services/routing.rs`
**Action:** Add `ModelTier` enum and route based on Orient phase complexity score.

### P6-007: Hardware amortization breakeven routing
**Action:** Track per-token cost local vs. API. Auto-route to local above 9.1M daily output tokens.

---

# P7 — CI GATES & DATA ORGANIZATION

### P7-001: `vox ci grammar-drift`
### P7-002: `vox ci mens-corpus-health`
### P7-003: `vox ci grpo-reward-baseline`
### P7-004: `vox ci collateral-damage`
### P7-005: `vox ci constrained-gen-smoke`
### P7-006: `vox ci k-complexity-budget`
### P7-007: Corpus storage migration to Codex/Arca
### P7-008: Research-to-code traceability contract

---

# READING ORDER & AGENT INSTRUCTION SET

## For an implementing agent executing this migration:

### Step 1: Understand the strategic context
Read these in order:
1. **This document** — `docs/src/architecture/vox-0.4-migration-plan.md` — task-level instructions
2. **`docs/src/architecture/research-synthesis-grand-strategy-seed-2026.md`** — the "why" behind every change
3. **`docs/src/architecture/vox_agentic_loop_and_mens_plan.md`** — the existing 254-task blueprint that this plan subsumes and corrects

### Step 2: Understand the current codebase
4. **`docs/src/explanation/expl-architecture.md`** — compiler pipeline overview
5. **`docs/src/explanation/expl-ml-pipeline.md`** — MENS training pipeline overview
6. **`crates/vox-compiler/src/lib.rs`** — compiler module inventory
7. **`crates/vox-compiler/src/typeck/diagnostics.rs`** — the `Diagnostic` struct you'll be extending

### Step 3: Read the research clusters (only when needed per phase)
- **P0:** `research-ts-hallucination-cognitive-science-2026.md` (§Compiler Feedback as Oracle)
- **P1:** `research-grammar-constrained-decoding-2026.md` (full document)
- **P2:** `research-grpo-reward-shaping-2026.md` (overview), then binary-parse-rate, vram-small-batch, ast-reward-hacking, reward-weights, positive-only, gaps-and-adjustments
- **P2 (CL):** `research-continual-learning-flywheel-2026.md` (overview), then mad-mode-collapse, qlora-catastrophic-forgetting, qlora-minimum-corpus, slop-typicality-bias
- **P3:** `research-trust-reliability-signals-2026.md`, `research-plan-adequacy-heuristics-2026.md`, `research-context-handoff-continuity-2026.md`
- **P4:** `research-ts-hallucination-k-complexity-2026.md`, `research-ts-hallucination-frontier-2026.md`
- **P5:** `research-pbt-oracles-compiled-lang-2026.md`, `automated-testing-research-2026.md`
- **P6:** `research-multi-agent-mesh-economics-2026.md`

### Step 4: Execute phases in priority order (P0 → P7)
- Complete all tasks within a wave before moving to the next wave
- Run the milestone gate test at the end of each phase
- If a milestone gate fails, fix before proceeding
- Reference specific research pages when you need to understand *why* a change is required

### Step 5: After each phase, update these documents
- Mark completed tasks in this plan
- Update `research-index.md` if new research surfaces
- Update `SUMMARY.md` if new architecture docs are created
- Run `vox ci` gates to verify nothing regressed
