---
phase: 406
phase_name: "mens-multi-spoke-corpus-fidelity-compiler-verified-rft-dpo-s"
project: "Vox"
generated: "2026-11-13"
counts:
  decisions: 6
  lessons: 1
  patterns: 3
  surprises: 2
missing_artifacts: []
---

# Phase 406 Learnings: mens-multi-spoke-corpus-fidelity-compiler-verified-rft-dpo-s

## Decisions

- **Breadcrumb context and `response_mode: "mixed"` on Docs Q&A and code pairs**
  - **Context:** Standalone extracted documentation Q&A and code examples lacked document context and triggered `"prose_only"` metadata or silent replay drops.
  - **Outcome:** Include `frontmatter.title` and heading breadcrumbs in `extract_qa_sections` and `extract_code_blocks` prompts (`format_qa_prompt`, `prepend_doc_context`), validate extracted `vox` blocks with `vox_compiler`, emit `response_mode: "mixed"` when relational fences are injected, and align `base_rating: 4` / `domain-profiles.yaml` for domain training replay.
  - **Source:** `406-01-PLAN.md`, `406-01-SUMMARY.md`

- **Cohesive multi-construct `.vox` synthesis and compiler-verified DPO**
  - **Context:** Individual AST slices needed whole-module synthesis context, and DPO `chosen` pairs could admit retired syntax or unparseable snippets.
  - **Outcome:** Emit whole-module `vox_module_cohesive` synthesis pairs alongside AST slices (`part_walk.rs`), and gate DPO `chosen` candidates through `vox_compiler::pipeline::run_frontend_str` (`fails_vox_frontend`, `generate_dpo_skips_unparseable_and_retired_chosen`) plus retired `fn ... ) -> ` syntax filtering.
  - **Source:** `406-01-PLAN.md`, `406-01-SUMMARY.md`

- **Explicit `include_lanes` across all spoke mix manifests**
  - **Context:** Spoke mix YAML configs needed explicit lane selection and strict validation guarantees.
  - **Outcome:** Declare explicit non-empty `include_lanes:` lists in `mix-research-expert.yaml`, `mix-research.yaml`, `mix-populi-meta.yaml`, `mix-rocks.yaml`, and `mix-agents.yaml`, enforced by IO, YAML, and whitespace checks in `spoke_validate.rs`.
  - **Source:** `406-01-PLAN.md`, `406-01-SUMMARY.md`

- **Token-normalized gradient accumulation and packed sequence attention (`Device::Cpu` verified)**
  - **Context:** Micro-batch loss averaging and unpacked training could drop boundary fidelity or disturb running GPU jobs during verification.
  - **Outcome:** Replace per-micro-step loss averaging with `token_norm_window_scale`, epoch-end `flush_pending_grads`, weighted cross-document boundary zeroing (`build_packed_mask_and_doc_ids`), shared `PackedAttnContext` with contiguous `doc_id` validation, and `forward_packed` / `forward_checkpointed_packed` verified against unpacked passes on `Device::Cpu`.
  - **Source:** `406-02-PLAN.md`, `406-02-SUMMARY.md`

- **`rsLoRA` scaling and `--lora-exclude-lm-head` guard**
  - **Context:** `qlora-rs` applies `alpha / rank` scaling internally and `lm_head` needed a freeze path that preserves gradient propagation through tied embeddings.
  - **Outcome:** Compute `effective_lora_alpha` (`alpha * sqrt(rank)`) with `PopuliAdapterManifestV3::effective_alpha_f64` for non-square precision, and freeze `lm_head` via `QuantizedLinear::from_weight_frozen` with `gpu_nf4: None` so input grads propagate across all `seq_len`.
  - **Source:** `406-02-PLAN.md`, `406-02-SUMMARY.md`

- **Workspace-relative preflight paths in `run_train.rs`**
  - **Context:** Training preflight checks risked machine-local absolute paths and eager full-file loads.
  - **Outcome:** Resolve `train_path` and `training_contract.yaml` strictly relative to the workspace in `check_corpus_threshold` with streaming `BufReader` early-stop line counting.
  - **Source:** `406-02-PLAN.md`, `406-02-SUMMARY.md`

## Lessons

- **Structured code review closed parser, traversal, gradient-flush, and path-hygiene gaps**
  - `406-REVIEW.md` surfaced 13 findings (1 Critical, 3 High, 6 Medium, 3 Low) across `extract_docs`, `dpo`, `spoke_validate`, `forward.rs`, `model.rs`, `qlora-rs-1.0.5`, and `run_train.rs` — including hash-comment handling inside non-Vox fences (`code_blocks_ignore_hash_comments_inside_other_fences`), doc-link traversal confinement (`resolve_doc_confine_root` / `qa_sections_drop_retired_inline_vox_fence_and_reject_traversal_link`), epoch-end remainder gradient flushing (`flush_pending_grads`), and removing machine-local paths in `run_train.rs`.
  - **Source:** `406-01-SUMMARY.md`, `406-02-SUMMARY.md`, `406-REVIEW.md`

## Patterns

- **Bitwise packed-forward verification on `Device::Cpu`**
  - Testing `forward_packed` and `forward_checkpointed_packed` bitwise against separate unpacked forward passes on `Device::Cpu` proves cross-document causal masking and local RoPE position resets deterministically in `0.00s` without Metal/CUDA hardware in CI.
  - **Source:** `406-02-PLAN.md`, `406-02-SUMMARY.md`

- **Compiler-oracle corpus filtering**
  - Gating both `extract_docs` (`frontend_check_passes`) and `dpo` (`generate_dpo_skips_unparseable_and_retired_chosen`) through `vox_compiler::pipeline::run_frontend_str` prevents retired `fn ... ) -> ` syntax and unparseable examples from entering training and preference datasets.
  - **Source:** `406-01-SUMMARY.md`, `406-VERIFICATION.md`

- **Adapter math adaptation and gradient-preserving frozen heads**
  - Pre-multiplying `alpha * sqrt(rank)` bridges `rsLoRA` scaling with `qlora-rs`'s `alpha / rank` divisor, while `QuantizedLinear::from_weight_frozen` with `gpu_nf4: None` freezes `lm_head` without blocking backward input gradients.
  - **Source:** `406-02-PLAN.md`, `406-02-SUMMARY.md`

## Surprises

- **Post-review regression coverage expanded three corpus test suites**
  - Addressing review findings added 5 regression tests across 3 suites (`extract_docs`: `25/25` UAT -> `28/28` post-fix; `dpo`: `11/11` UAT -> `12/12` post-fix; `spoke_validate`: `9/9` UAT -> `10/10` post-fix).
  - **Source:** `406-01-SUMMARY.md`, `406-UAT.md`, `406-VERIFICATION.md`

- **Non-square `rsLoRA` ranks and remainder micro-batches required explicit handling**
  - Non-square `rsLoRA` ranks required float precision via `PopuliAdapterManifestV3::effective_alpha_f64`, and token-normalized gradient accumulation needed explicit epoch-end `flush_pending_grads` so remainder micro-batches are not silently dropped.
  - **Source:** `406-02-SUMMARY.md`
