---
phase: 406
phase_name: "mens-multi-spoke-corpus-fidelity-compiler-verified-rft-dpo-s"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 6
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts: []
---

# Phase 406 Learnings: mens-multi-spoke-corpus-fidelity-compiler-verified-rft-dpo-s

## Decisions

### Contextualized docs Q&A and code block extraction in crates/vox-corpus/s...
Contextualized docs Q&A and code block extraction in `crates/vox-corpus/src/corpus/extract_docs.rs` (`format_qa_prompt`, `prepend_doc_context`, `frontend_check_passes`, `response_mode: "mixed"`, `base_rating: 4` for `status: current`, non-Vox fence tracking, `resolve_doc_confine_root` link confinement, and retired-syntax QA fence filtering).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 406-01-SUMMARY.md

---

### Cohesive multi-construct synthesis (vox_module_cohesive) in crates/vox-c...
Cohesive multi-construct synthesis (`vox_module_cohesive`) in `crates/vox-corpus/src/corpus/extract_vox/part_walk.rs` and compiler-verified `chosen` + `rejected` (`fails_vox_frontend`) filtering in `crates/vox-corpus/src/corpus/dpo.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 406-01-SUMMARY.md

---

### Explicit include_lanes coverage across mens/config/ (domain-profiles.yam...
Explicit `include_lanes` coverage across `mens/config/` (`domain-profiles.yaml`, `mix-research-expert.yaml`, `mix-research.yaml`, `mix-populi-meta.yaml`, `mix-rocks.yaml`, `mix-agents.yaml`) verified by strict IO/YAML/blank-lane diagnostics in `crates/vox-populi/src/mens/tensor/spoke_validate.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 406-01-SUMMARY.md

---

### Relative train_path and training_contract.yaml resolution with streaming...
Relative `train_path` and `training_contract.yaml` resolution with streaming `BufReader` early stop in `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 406-02-SUMMARY.md

---

### lora_exclude_lm_head (QuantizedLinear::from_weight_frozen in patches/qlo...
`lora_exclude_lm_head` (`QuantizedLinear::from_weight_frozen` in `patches/qlora-rs-1.0.5/src/qlora.rs`, `model.rs`, `candle_qlora_train/mod.rs`) and `use_rslora` (`PopuliAdapterManifestV3::effective_alpha_f64` in `adapter_schema_v3.rs`, `merge.rs`, `merge_qlora.rs`, `finalize.rs`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 406-02-SUMMARY.md

---

### Token-normalized gradient accumulation (token_norm_window_scale), epoch-...
Token-normalized gradient accumulation (`token_norm_window_scale`), epoch-end `flush_pending_grads` (`patches/qlora-rs-1.0.5/src/training.rs` + `training_loop/mod.rs`), heterogeneous weighted cross-document masking (`build_packed_mask_and_doc_ids` in `training_loop/forward.rs`), and checkpointed/eager `PackedAttnContext` (`forward_packed` / `forward_checkpointed_packed` in `model.rs`) verified on `Device::Cpu`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 406-02-SUMMARY.md

## Lessons

### total: 6 passed: 6 issues: 0 pending: 0 skipped: 0
total: 6 passed: 6 issues: 0 pending: 0 skipped: 0

**Context:** Observed during user acceptance testing.
**Source:** 406-UAT.md

---

### [none]
[none]

**Context:** Observed during user acceptance testing.
**Source:** 406-UAT.md

## Patterns

## Surprises

