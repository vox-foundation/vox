---
phase: 368
phase_name: "mens-corpus-pipeline-studio-gui"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
---

# Phase 368 Learnings: mens-corpus-pipeline-studio-gui

## Decisions

### Generated /Users/brbrainerd/.gemini/antigravity/brain/dd5364d3-6351-4d67...
Generated `/Users/brbrainerd/.gemini/antigravity/brain/dd5364d3-6351-4d67-85be-8138e2691240/concept_mens_pipeline_studio_romanesque_1794153153510.jpg` establishing the three-arch *Cursus Honorum* layout (`I · Mix Sources & Auto-DAG`, `II · Live JSONL Record Inspector & AST Editor`, `III · Ladder Checkpoints & Gate`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 368-01-SUMMARY.md

---

### Upgraded [crates/vox-corpus/src/corpus/mix/mod.rs](file:///Users/brbrain...
Upgraded [`crates/vox-corpus/src/corpus/mix/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-corpus/src/corpus/mix/mod.rs) to auto-synthesize missing `vox_reasoning_traces.jsonl` (`120` compiler-verified CoT traces) **before** incremental fingerprint comparison and re-exported `calculate_file_fingerprint` and `calculate_config_fingerprint`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 368-01-SUMMARY.md

---

### Implemented [crates/vox-orchestrator-mcp/src/gui_ops/mens_corpus.rs](fil...
Implemented [`crates/vox-orchestrator-mcp/src/gui_ops/mens_corpus.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/gui_ops/mens_corpus.rs) and exposed 4 Tauri commands in [`crates/vox-gui/src/commands/mens_run_reports.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/mens_run_reports.rs) (`mens_corpus_studio_snapshot`, `mens_corpus_read_jsonl`, `mens_corpus_save_jsonl_row`, `mens_corpus_rebuild_mix`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 368-01-SUMMARY.md

---

### Built [MensCorpusStudioPanel.tsx](file:///Users/brbrainerd/dev/vox/crate...
Built [`MensCorpusStudioPanel.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Models/MensCorpusStudioPanel.tsx) and mounted it in [`MensTrainingView.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Models/MensTrainingView.tsx).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 368-01-SUMMARY.md

---

### Added [MensCorpusStudioPanel.test.tsx](file:///Users/brbrainerd/dev/vox/...
Added [`MensCorpusStudioPanel.test.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Models/__tests__/MensCorpusStudioPanel.test.tsx) (`1 passed`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 368-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

