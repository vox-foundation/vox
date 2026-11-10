# Phase 342: Rust Spoke: Corpus Extraction, 0.6B Training, rust_bench Gate, and Hub Sync - Context

**Gathered:** 2026-11-07
**Status:** Ready for planning
**Milestone:** v0.6 MENS Multi-Spoke & Full Ladder Gate Closure
**Source:** User-confirmed milestone decisions and `mens/config/domain-profiles.yaml`.

<domain>
## Phase Boundary

Phase 342 extracts the clean Rust corpus, trains the `Qwen3-0.6B` Rust spoke adapter, evaluates it against the sandboxed `rust_bench` suite and collateral gates, and publishes the passing weights to HuggingFace.

Key deliverables:
1. **Verified Rust Workspace Corpus Extraction**:
   - Parse and extract Rust authoring patterns across the Vox workspace and vetted direct dependencies per `mix-rust.yaml` categories (`rust_source`, `rust_pairs`, `rust_doc`).
   - Materialize target dataset `target/dogfood/train_rust.jsonl`.
   - Enforce strict negative deduplication and leakage exclusion against the 12 held-out items in `mens/data/rust_bench/items.v1.jsonl`.
2. **Qwen3-0.6B Rust Adapter Training**:
   - Train on Metal/CUDA using `grad_accum=8`, `rank=8`, `alpha=16`, `seq_len=1024`, `min_rating=3`.
   - Apply system prompt from `scripts/rust_system_prompt.txt`.
   - Wire reward hook or post-verification for rustc syntax checks where applicable.
3. **Rigorous Rust Spoke Gating**:
   - Gate on `mens/config/eval-gates-rust.yaml`:
     - `suite:rust_bench`: 12-item sandboxed compile-and-test execution running `cargo test --offline` in isolated temp directories.
     - Known-bad control validation (untrained base model must fail).
     - `eval-gates-ladder.yaml` collateral regression check (GSM8K, MATH-500).
4. **HuggingFace Multi-Spoke Hub Publishing**:
   - Commit passing adapter to `spokes/rust/` inside `brbrainerd/vox-mens-qwen3-0.6b`.
   - Update root `spoke_gates.json` marking `rust` as passed.
   - Regenerate 2-panel programmatic SVG strips (`overview.svg` and `scorecard.svg`).
</domain>

<decisions>
## Implementation Decisions

### D-01: Sandboxed Cargo Test Verification
- Evaluation cannot rely on regex matching code snippets.
- Each generation is placed in a minimal Cargo test crate and executed via `cargo test --offline` with a 30s timeout and network isolation.
- Passing requires both exit code 0 and non-empty test assertions.

### D-02: Authoring-Only Scope
- Per `domain-profiles.yaml`, the Rust spoke is specialized for authoring workspace patterns (`lane:vox_rust_authoring`, `*.rs`).
- Review tasks remain routed to the base dense model or external judge.
</decisions>

<verification>
## Verification Requirements
- `train_rust.jsonl` contains zero overlap with `mens/data/rust_bench/items.v1.jsonl`.
- `cargo test --offline` executes cleanly for winning adapter on the benchmark.
- Negative control fails the suite as expected.
- `spokes/rust/` artifact directory published with valid checksum and manifest.
</verification>
