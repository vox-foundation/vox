# Phase 414: Root Hygiene, Salvage Purge, CWD Bug Fix & Backlog Slicing - Context

**Gathered:** 2026-11-13
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Purge untracked scratch files, literal `~` directory, `.remediation-notes/`, `.vox/salvage/vox-mens-compare`, and `crates/vox-ml-cli/mens/`; fix the CWD-relative `mens/` path bug in `crates/vox-ml-cli`; verify `.github/workflows/link_checker.yml` consolidation and record disposition for the 12 open Dependabot PRs; and emit an atomic commit-slice manifest (`contracts/reports/v71-atomic-commit-slices.v1.json`) partitioning the dirty working tree without running `git add` or `git commit`.

</domain>

<decisions>
## Implementation Decisions

### Exact Deletion Scope (`HYGIENE-ROOT-01` & `HYGIENE-ROOT-02`)
- Delete only the explicitly enumerated scratch/salvage/polluted paths:
  - `/Users/brbrainerd/dev/vox/file`
  - `/Users/brbrainerd/dev/vox/test_syntax.vox`
  - `/Users/brbrainerd/dev/vox/scratch_extract.vox`
  - `/Users/brbrainerd/dev/vox/fix-harness-report.md`
  - `/Users/brbrainerd/dev/vox/.remediation-notes`
  - `/Users/brbrainerd/dev/vox/~` (always quoted full path `"/Users/brbrainerd/dev/vox/~"`, never bare `~`)
  - `/Users/brbrainerd/dev/vox/.vox/salvage/vox-mens-compare`
  - `/Users/brbrainerd/dev/vox/crates/vox-ml-cli/mens`
- Add `.vox/salvage/` and `.remediation-notes/` to `.voxignore` (and `.remediation-notes/` to `.gitignore`), then run `vox ci sync-ignore-files`. Do NOT gitignore `crates/*/mens/` so any future CWD regression remains immediately visible in `git status`.

### `vox-ml-cli` CWD `mens/` Bug Fix (`HYGIENE-ROOT-02`)
- `crates/vox-ml-cli/src/commands/mens/pipeline.rs` currently ignores its `data_dir: PathBuf` argument in several stages (`Generate`, `Extract`, `Replay`, `ResearchGen`, `ReviewIngest`, `ReviewDatasetBuild`, `ReviewEvalPackBuild`, `ReviewToDpo`, `AgentTraceIngest`, `BackTranslate`, `Rft`), hardcoding `PathBuf::from("mens/data/...")` relative to process CWD (which is `crates/vox-ml-cli/` under `cargo test -p vox-ml-cli` or standalone sub-crate runs).
- Derive all stage `mix_sources/*`, `synthetic.jsonl`, and `research-lane-sft.jsonl` paths from `&data_dir`, resolve workspace-relative `bench`/`pool`/`crates`/`docs` paths via `vox_corpus::training::contract::resolve_from_workspace`, and normalize default `hub::Dataset` staging paths via `resolve_from_workspace` in `crates/vox-ml-cli/src/commands/mens/hub.rs`.
- Add a unit test in `pipeline.rs` proving pipeline stage path resolution honors a custom `data_dir` (e.g., `tempdir`) and never targets CWD `mens/data`.

### `link_checker.yml` & Dependabot PR Disposition + Commit Slicing (`HYGIENE-ROOT-03` & `HYGIENE-ROOT-04`)
- Verify `.github/workflows/link_checker.yml` is fully superseded by `.github/workflows/docs-quality.yml` (`lycheeverse/lychee-action@v2` blocking internal link gate + selftest) and `.github/workflows/docs-drift-refresh.yml` (`lycheeverse/lychee-action@v2` + `scripts/lychee-triage.vox` external link triage).
- Record an explicit disposition table for all 12 open Dependabot PRs (`#692..#721`: 6 mergeable minor/patch grouped PRs vs 6 major bumps requiring compatibility review).
- Write `contracts/reports/v71-atomic-commit-slices.v1.json` partitioning dirty/untracked paths into dependency-ordered atomic commit slices without touching git index/commits (`00-core.md` Rule 6).

</decisions>
