---
phase: 343
phase_name: "docs-system-ssot-and-automated-drift-control"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 10
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts: []
---

# Phase 343 Learnings: docs-system-ssot-and-automated-drift-control

## Decisions

### Extracted fn workspace_crate_names(root
Extracted `fn workspace_crate_names(root: &Path) -> Result<HashSet<String>>` to strictly read package names from `crates/*/Cargo.toml`, bubbling errors instead of silently skipping unreadable manifests.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 343-05-SUMMARY.md

---

### Removed the dead check on deleted docs/src/architecture/orphan-surface-i...
Removed the dead check on deleted `docs/src/architecture/orphan-surface-inventory.md`, along with `parse_workspace_crate_block` and `CRATE_LINE_RE`. Crate-roster parity is handled by `vox-arch-check` Rule 12.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 343-05-SUMMARY.md

---

### Replaced the retired architecture index check with an immediate recreati...
Replaced the retired architecture index check with an immediate recreation guard on `docs/src/architecture/architecture-index.md`, matching the `research-index.md` check.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 343-05-SUMMARY.md

---

### Removed the stale training_eligible warning keyed on hand-written last_u...
Removed the stale `training_eligible` warning keyed on hand-written `last_updated` (which the linter forbids).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 343-05-SUMMARY.md

---

### Made AGENTS.md, GEMINI.md, CLAUDE.md, and .github/copilot-instructions.m...
Made `AGENTS.md`, `GEMINI.md`, `CLAUDE.md`, and `.github/copilot-instructions.md` mandatory root guides (missing file causes fatal error).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 343-05-SUMMARY.md

---

### Refactored vox_doc_pipeline::claims::prune::build_inbound_link_graph to ...
Refactored `vox_doc_pipeline::claims::prune::build_inbound_link_graph` to take `(repo_root: &Path, docs_dir: &Path, extra_sources: &[PathBuf])`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 343-06-SUMMARY.md

---

### Strips fenced code blocks (  ) and inline code spans ( ..
Strips fenced code blocks (` ``` `) and inline code spans (`` `...` ``) before scanning Markdown links so code examples do not generate false inbound links.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 343-06-SUMMARY.md

---

### Ignores archive paths (/archive/) and ignores self-links (target_rel == ...
Ignores archive paths (`/archive/`) and ignores self-links (`target_rel == source_rel`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 343-06-SUMMARY.md

---

### Normalizes repo_root and docs_dir paths to avoid prefix stripping errors...
Normalizes `repo_root` and `docs_dir` paths to avoid prefix stripping errors with relative root paths.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 343-06-SUMMARY.md

---

### Adds root guides (AGENTS.md, README.md, CONTRIBUTING.md, CLAUDE.md, GEMI...
Adds root guides (`AGENTS.md`, `README.md`, `CONTRIBUTING.md`, `CLAUDE.md`, `GEMINI.md`) into `extra_sources` so inbound links from top-level repository guides count toward page reachability.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 343-06-SUMMARY.md

## Lessons

## Patterns

## Surprises

