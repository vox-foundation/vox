---
phase: 78
phase_name: "text-in-text-out-multi-spoke-verification-round-trip-evals"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 8
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 78 Learnings: text-in-text-out-multi-spoke-verification-round-trip-evals

## Decisions

### Implemented extract_fenced_or_raw to cleanly evaluate raw or fenced text...
Implemented extract_fenced_or_raw to cleanly evaluate raw or fenced text-in/text-out model completions across all spokes.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 78-01-SUMMARY.md

---

### Implemented verify_voxlang_roundtrip (SPOKE-01) combining vox_compiler f...
Implemented `verify_voxlang_roundtrip` (`SPOKE-01`) combining `vox_compiler` frontend + `Interpreter` execution with AST construct coverage scoring.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 78-01-SUMMARY.md

---

### Implemented verify_rust_roundtrip (SPOKE-02) combining syn::parse_file +...
Implemented `verify_rust_roundtrip` (`SPOKE-02`) combining `syn::parse_file` + stub rejection with workspace source-policy detectors (banned crypto crate imports, hardcoded LLM vendor hostnames, raw secret env reads).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 78-01-SUMMARY.md

---

### Implemented verify_shell_roundtrip (SPOKE-03) combining classify_shell_s...
Implemented `verify_shell_roundtrip` (`SPOKE-03`) combining `classify_shell_safety_floor`, secret-leak detection, delimiter balance, and JSON output schema validation.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 78-01-SUMMARY.md

---

### Standardized MensSpokeScorecardV1 with schema_version mens_spoke_scoreca...
Standardized MensSpokeScorecardV1 with schema_version mens_spoke_scorecard_v1 and both JSON and Markdown table outputs.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 78-02-SUMMARY.md

---

### Implemented McpToolSpec and verify_mcp_roundtrip (SPOKE-04) validating s...
Implemented `McpToolSpec` and `verify_mcp_roundtrip` (`SPOKE-04`) validating single or multi-step MCP tool invocations and goal completion.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 78-02-SUMMARY.md

---

### Implemented AgentHarnessSpec and verify_agent_harness_roundtrip (SPOKE-0...
Implemented `AgentHarnessSpec` and `verify_agent_harness_roundtrip` (`SPOKE-05`) validating `TaskPlan` JSON structure, file-scope boundaries, token/step budgets, and verification gates.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 78-02-SUMMARY.md

---

### Implemented MensSpokeScorecardV1, evaluate_multi_spoke_suite, canonical_...
Implemented `MensSpokeScorecardV1`, `evaluate_multi_spoke_suite`, `canonical_five_spoke_golden_suite`, and `contracts/reports/mens-spoke-scorecard.v1.json` (`SPOKE-06`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 78-02-SUMMARY.md

## Lessons

## Patterns

## Surprises

