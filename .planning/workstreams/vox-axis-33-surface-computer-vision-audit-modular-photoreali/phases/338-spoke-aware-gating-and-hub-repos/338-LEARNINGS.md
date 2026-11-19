---
phase: 338
phase_name: "spoke-aware-gating-and-hub-repos"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 10
  lessons: 0
  patterns: 7
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 338 Learnings: spoke-aware-gating-and-hub-repos

## Decisions

### Require >=2 offered tools per episode and expect.tool != offered_tools[0...
Require >=2 offered tools per episode and expect.tool != offered_tools[0] on right_tool cases so trivial controls fail cleanly

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 338-05-SUMMARY.md

---

### Verify nonce echo either as an exact argument value or embedded substrin...
Verify nonce echo either as an exact argument value or embedded substring (e.g. src/${nonce}.vox)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 338-05-SUMMARY.md

---

### Pin 12 shared salvage cases between vox-orchestrator-mcp and vox-eval vi...
Pin 12 shared salvage cases between vox-orchestrator-mcp and vox-eval via salvage_cases_agree

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 338-05-SUMMARY.md

---

### Derive spokes from adapter_card.json train_lanes / spokes via DomainProf...
Derive spokes from adapter_card.json train_lanes / spokes via DomainProfilesFile::spokes_for_lanes rather than trusting static defaults

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 338-06-SUMMARY.md

---

### Restrict verify_publish_gates_with and eval-gate reporting to applying_r...
Restrict verify_publish_gates_with and eval-gate reporting to applying_rows so uninvolved spokes do not falsely pass or block unrelated runs

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 338-06-SUMMARY.md

---

### Keep suites block
Keep suites block: false, require_report: false in shipped configs while enforcing SuiteFloors and McNemar improvement whenever a suite_report.<name>.json file is present

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 338-07-SUMMARY.md

---

### Return paired (base, candidate) SuiteReport structs from run_agent_suite...
Return paired (base, candidate) SuiteReport structs from run_agent_suite and run_rust_bench_suite matching vox_eval::suite_report::suite_verdict

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 338-07-SUMMARY.md

---

### Score rust_bench items via vox_corpus::corpus::rust_bench::{load_items, ...
Score rust_bench items via vox_corpus::corpus::rust_bench::{load_items, run_item} into paired SuiteReport items

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 338-08-SUMMARY.md

---

### Preserve all existing golden model-card snapshots verbatim while adding ...
Preserve all existing golden model-card snapshots verbatim while adding CardFacts dataset citation metadata

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 338-09-SUMMARY.md

---

### Block staging of dataset bundles containing secret patterns or held-out ...
Block staging of dataset bundles containing secret patterns or held-out training-eligible leaks

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 338-09-SUMMARY.md

## Lessons

## Patterns

### schema-validated episode runner
schema-validated episode runner

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 338-05-SUMMARY.md

---

### shared salvage test fixtures
shared salvage test fixtures

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 338-05-SUMMARY.md

---

### lane-to-spoke derivation
lane-to-spoke derivation

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 338-06-SUMMARY.md

---

### applying-row gate filter
applying-row gate filter

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 338-06-SUMMARY.md

---

### paired suite verdicts in eval-gate
paired suite verdicts in eval-gate

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 338-07-SUMMARY.md

---

### sandboxed cargo test suite report
sandboxed cargo test suite report

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 338-08-SUMMARY.md

---

### secret and held-out leakage blocking prior to dataset staging
secret and held-out leakage blocking prior to dataset staging

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 338-09-SUMMARY.md

## Surprises

