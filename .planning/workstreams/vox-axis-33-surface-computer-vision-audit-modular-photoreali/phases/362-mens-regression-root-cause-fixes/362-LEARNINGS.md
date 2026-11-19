---
phase: 362
phase_name: "mens-regression-root-cause-fixes"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 3
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
---

# Phase 362 Learnings: mens-regression-root-cause-fixes

## Decisions

### generate_training_system_prompt() wraps builtin_system_prompt() (~350 to...
generate_training_system_prompt() wraps builtin_system_prompt() (~350 tokens) instead of generate_system_prompt() (~2,200 tokens from mens/config/system_prompt.txt) so supervised_ratio stays ~20-25% (>10% gate floor) and prompts are not middle-trimmed.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 362-01-SUMMARY.md

---

### 120 compiler-verified Vox <think> reasoning traces (40 literal_debug, 40...
120 compiler-verified Vox <think> reasoning traces (40 literal_debug, 40 state_trace, 40 invariant_check) are auto-materialized into mens/data/mix_sources/vox_reasoning_traces.jsonl and mixed into train_mixed.jsonl to counter error_correction auto-fix hallucination and 2048-token unclosed <think> loops.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 362-01-SUMMARY.md

---

### extract_cued_answer scans cues right-to-left, rejects English prose stop...
extract_cued_answer scans cues right-to-left, rejects English prose stopwords, and extracts values after 'is'/':'/'=' on heading lines.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 362-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

