---
phase: 341-harness-and-agent-spokes
plan: 01
subsystem: mens-corpus-agents
tags: [mens, agents, tool_selection, argument_generation, leakage]
provides:
  - Deterministic zero-leakage unit test `zero_leakage_against_agent_episodes_eval` in `tool_selection_synth.rs`
  - Deterministic zero-leakage unit test `zero_leakage_against_agent_episodes_eval` in `argument_generation_synth.rs`
affects: [vox-corpus]
key-files:
  modified:
    - crates/vox-corpus/src/corpus/tool_selection_synth.rs
    - crates/vox-corpus/src/corpus/argument_generation_synth.rs
completed: 2026-11-13
---

# Phase 341 Plan 01 Summary: Harness & Agent Spoke Corpora (Zero-Leakage Guards)

- Added [tool_selection_synth::tests::zero_leakage_against_agent_episodes_eval](file:///Users/brbrainerd/dev/vox/crates/vox-corpus/src/corpus/tool_selection_synth.rs) asserting that generated `tool_selection_synth` task prompts have zero overlap against `user_request` prompts in [episodes.v1.jsonl](file:///Users/brbrainerd/dev/vox/mens/data/agent_episodes/episodes.v1.jsonl).
- Added [argument_generation_synth::tests::zero_leakage_against_agent_episodes_eval](file:///Users/brbrainerd/dev/vox/crates/vox-corpus/src/corpus/argument_generation_synth.rs) asserting that generated `argument_generation_synth` task prompts have zero overlap against `user_request` prompts in [episodes.v1.jsonl](file:///Users/brbrainerd/dev/vox/mens/data/agent_episodes/episodes.v1.jsonl).
- **Verification**: `cargo test -p vox-corpus --lib -- tool_selection argument_generation rust_authoring rust_bench vox_reasoning_synth` (`22 passed; 0 failed; 2 ignored; 285 filtered out`).
