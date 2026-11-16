---
phase: 406-mens-multi-spoke-corpus-fidelity-compiler-verified-rft-dpo-s
plan: "01"
subsystem: mens-corpus
tags: [vox-corpus, extract_docs, extract_vox, dpo, spoke-validate, include-lanes]
requires:
  - phase: 405
    provides: structural dependency DAG decoupling and clean workspace baseline
provides:
  - "Breadcrumb-contextualized docs Q&A/code extraction, `response_mode: \"mixed\"` relational chunk tagging, and `vox_compiler` frontend checks"
  - "Multi-construct whole-module cohesive `.vox` synthesis pairs (`vox_module_cohesive`) alongside AST slices"
  - "Compiler-verified DPO preference pair mining filtering unparseable and retired syntax (`generate_dpo_skips_unparseable_and_retired_chosen`)"
  - "Explicit `include_lanes` coverage across all spoke mix YAML files and `spoke_validate` enforcement"
affects: [vox-corpus, vox-populi, mens-config]
coverage:
  - id: D-406-01
    description: "Contextualized doc prompts (`frontmatter.title` + heading breadcrumbs), `response_mode: \"mixed\"` on relational fences, `vox_compiler` block checks, non-Vox fence tracking, confined doc links, retired-syntax QA fence filtering, and `domain-profiles.yaml` alignment."
    requirement: MENS-CORPUS-406-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-corpus --lib extract_docs (28 passed; 0 failed incl. qa_prompt_tracks_frontmatter_title_breadcrumbs_and_mixed_mode, code_blocks_skip_retired_syntax_and_annotate_compiler_check, code_blocks_ignore_hash_comments_inside_other_fences, qa_sections_drop_retired_inline_vox_fence_and_reject_traversal_link)"
        status: pass
    human_judgment: false
  - id: D-406-02
    description: "Whole-module cohesive multi-construct `.vox` synthesis (`vox_module_cohesive`) and compiler-verified DPO preference pair generation with frontend failure checks on mutated/fallback candidates."
    requirement: MENS-CORPUS-406-02
    verification:
      - kind: unit
        ref: "cargo test -p vox-corpus --lib dpo (12 passed; 0 failed incl. generate_dpo_skips_unparseable_and_retired_chosen); extract_vox (10 passed; 0 failed pre-remediation, part_walk.rs untouched afterward)"
        status: pass
    human_judgment: false
  - id: D-406-03
    description: "Explicit `include_lanes` declarations across spoke mix configs and strict IO/YAML/non-blank `spoke_validate` enforcement."
    requirement: MENS-CORPUS-406-03
    verification:
      - kind: unit
        ref: "cargo test -p vox-populi --features mens --lib spoke_validate (10 passed; 0 failed incl. every_shipped_mix_declares_explicit_include_lanes, flags_mix_config_missing_include_lanes, flags_invalid_yaml_and_whitespace_only_include_lanes)"
        status: pass
    human_judgment: false
key-files:
  modified:
    - crates/vox-corpus/src/corpus/extract_docs.rs
    - crates/vox-corpus/src/corpus/extract_vox/part_walk.rs
    - crates/vox-corpus/src/corpus/dpo.rs
    - mens/config/domain-profiles.yaml
    - mens/config/mix-research-expert.yaml
    - mens/config/mix-research.yaml
    - mens/config/mix-populi-meta.yaml
    - mens/config/mix-rocks.yaml
    - mens/config/mix-agents.yaml
    - crates/vox-populi/src/mens/tensor/spoke_validate.rs
requirements-completed:
  - MENS-CORPUS-406-01
  - MENS-CORPUS-406-02
  - MENS-CORPUS-406-03
completed: 2026-11-12
---

# Phase 406 Plan 01: Docs Contextualization, Whole-Module .vox Synthesis, Compiler-Verified DPO & Multi-Spoke Mix Coverage Summary

## What Was Built & Verified
- Contextualized docs Q&A and code block extraction in `crates/vox-corpus/src/corpus/extract_docs.rs` (`format_qa_prompt`, `prepend_doc_context`, `frontend_check_passes`, `response_mode: "mixed"`, `base_rating: 4` for `status: current`, non-Vox fence tracking, `resolve_doc_confine_root` link confinement, and retired-syntax QA fence filtering).
- Cohesive multi-construct synthesis (`vox_module_cohesive`) in `crates/vox-corpus/src/corpus/extract_vox/part_walk.rs` and compiler-verified `chosen` + `rejected` (`fails_vox_frontend`) filtering in `crates/vox-corpus/src/corpus/dpo.rs`.
- Explicit `include_lanes` coverage across `mens/config/` (`domain-profiles.yaml`, `mix-research-expert.yaml`, `mix-research.yaml`, `mix-populi-meta.yaml`, `mix-rocks.yaml`, `mix-agents.yaml`) verified by strict IO/YAML/blank-lane diagnostics in `crates/vox-populi/src/mens/tensor/spoke_validate.rs`.

## Verification Results
- `cargo test -p vox-corpus --lib extract_docs`: `28 passed; 0 failed` (`1.24s`).
- `cargo test -p vox-corpus --lib dpo`: `12 passed; 0 failed` (`0.01s`).
- `cargo test -p vox-populi --features mens --lib spoke_validate`: `10 passed; 0 failed` (`0.01s`).
- `cargo test -p vox-corpus --lib extract_vox`: `10 passed; 0 failed` (pre-remediation run; `part_walk.rs` untouched afterward).
