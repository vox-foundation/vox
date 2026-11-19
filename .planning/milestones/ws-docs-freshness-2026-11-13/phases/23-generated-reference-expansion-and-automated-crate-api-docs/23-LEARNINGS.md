---
phase: 23
phase_name: "generated-reference-expansion-and-automated-crate-api-docs"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 15
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
---

# Phase 23 Learnings: generated-reference-expansion-and-automated-crate-api-docs

## Decisions

### Generated docs/src/reference/env-vars.generated.md from contracts/config...
Generated `docs/src/reference/env-vars.generated.md` from `contracts/config/env-vars.v1.yaml` (851 variables across all crates).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-01-SUMMARY.md

---

### Updated read_env_vars_ssot_doc in crates/vox-cli/src/commands/ci/command...
Updated `read_env_vars_ssot_doc` in `crates/vox-cli/src/commands/ci/command_compliance/docs_sync.rs` to read the generated file.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-01-SUMMARY.md

---

### Replaced hand-maintained docs/src/reference/env-vars.md with a concise p...
Replaced hand-maintained `docs/src/reference/env-vars.md` with a concise pointer to the generated reference.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-01-SUMMARY.md

---

### Generated docs/src/reference/config-keys.generated.md from contracts/con...
Generated `docs/src/reference/config-keys.generated.md` from `contracts/config/registry.v1.yaml` (110 knobs), stating intended readers.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-01-SUMMARY.md

---

### Generated docs/src/reference/mcp-tools.generated.md from contracts/mcp/t...
Generated `docs/src/reference/mcp-tools.generated.md` from `contracts/mcp/tool-registry.canonical.yaml` (340 tools), stating intended readers.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-01-SUMMARY.md

---

### Implemented crates/vox-cli-ci/src/crate_api_docs.rs.
Implemented `crates/vox-cli-ci/src/crate_api_docs.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-02-SUMMARY.md

---

### Uses cargo_metadata to discover workspace crates, versions, dependencies...
Uses `cargo_metadata` to discover workspace crates, versions, dependencies, and feature flags.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-02-SUMMARY.md

---

### Extracts crate documentation (//!), public items (pub mod, pub struct, p...
Extracts crate documentation (`//!`), public items (`pub mod`, `pub struct`, `pub enum`, `pub trait`, `pub fn`, `pub type`, `pub const`), and docstrings (`///`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-02-SUMMARY.md

---

### Generates docs/src/api/<crate-name>.generated.md with valid frontmatter ...
Generates `docs/src/api/<crate-name>.generated.md` with valid frontmatter (`category: "API Reference — Crates"`, `status: current`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-02-SUMMARY.md

---

### Generates docs/src/api/index.generated.md with links and summaries of al...
Generates `docs/src/api/index.generated.md` with links and summaries of all 127 crates.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-02-SUMMARY.md

---

### Implemented check_catalog_top_level_command_presence in crates/vox-cli/s...
Implemented `check_catalog_top_level_command_presence` in `crates/vox-cli/src/commands/ci/command_compliance/validators.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-03-SUMMARY.md

---

### Wired into command_compliance::run in crates/vox-cli/src/commands/ci/com...
Wired into `command_compliance::run` in `crates/vox-cli/src/commands/ci/command_compliance/mod.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-03-SUMMARY.md

---

### Validates that every top-level clap command in the CLI catalog exists as...
Validates that every top-level clap command in the CLI catalog exists as a registered operation in `contracts/operations/catalog.v1.yaml`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-03-SUMMARY.md

---

### Added 9 retired decorator forms to contracts/documentation/retired-symbo...
Added 9 retired decorator forms to `contracts/documentation/retired-symbols.v1.yaml`: `@table`, `@query`, `@mutation`, `@server`, `@tool`, `@resource`, `@form`, `@index`, `@endpoint`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-03-SUMMARY.md

---

### Added line-level migration cue filtering (retired, removed, deprecated, ...
Added line-level migration cue filtering (`retired`, `removed`, `deprecated`, `instead`, `→`, `no longer`, `superseded`, `replaced by`, `migration`, `formerly`, `legacy`, `never emit`, `not @`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 23-03-SUMMARY.md

## Lessons

## Patterns

## Surprises

