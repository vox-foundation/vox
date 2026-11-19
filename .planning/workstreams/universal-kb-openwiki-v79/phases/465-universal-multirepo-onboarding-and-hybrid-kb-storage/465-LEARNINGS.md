---
phase: 465
phase_name: "universal-multirepo-onboarding-and-hybrid-kb-storage"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 465 Learnings: universal-multirepo-onboarding-and-hybrid-kb-storage

## Decisions

### Extended RepoAccessMode with ReadOnlyLocal, RepoCapability with WriteFil...
Extended `RepoAccessMode` with `ReadOnlyLocal`, `RepoCapability` with `WriteFile` (`#[serde(alias = "write")]`), and repository descriptors with `read_only` and `external_cache_dir`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 465-SUMMARY.md

---

### Added ActiveRepoContext, HarnessOnboardSummary, vox_user_home_dir, globa...
Added `ActiveRepoContext`, `HarnessOnboardSummary`, `vox_user_home_dir`, `global_library_root[_under]` (`~/.vox/library`), `external_repo_cache_root[_under]` (`~/.vox/repos/<id>`), `save_repo_catalog`, `onboard_repository_harness[_with_home]`, `add_repository_to_catalog[_with_home]`, `remove_repository_from_catalog`, `switch_active_repository[_with_home]`, and `get_active_repository_context[_with_home]`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 465-SUMMARY.md

---

### Updated repo_query_text, repo_query_file, and repo_query_history in crat...
Updated `repo_query_text`, `repo_query_file`, and `repo_query_history` in `crates/vox-repository/src/repo_catalog/query.rs` to allow both `Local` and `ReadOnlyLocal` repositories.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 465-SUMMARY.md

---

### Added scaffold_vox_project_with_options[_and_home] and scaffold_vox_proj...
Added `scaffold_vox_project_with_options[_and_home]` and `scaffold_vox_project_under_repo_with_options` in `crates/vox-project-scaffold/src/lib.rs` to wire `--harness` onboarding.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 465-SUMMARY.md

---

### Added --harness on vox init, --read-only / --external-cache-only on vox ...
Added `--harness` on `vox init`, `--read-only` / `--external-cache-only` on `vox repo init` and `vox catalog add`, `--active` on `vox repo status`, and `--switch` / `--add` / `--remove` / `--read-only` on `vox repo catalog refresh`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 465-SUMMARY.md

## Lessons

## Patterns

## Surprises

