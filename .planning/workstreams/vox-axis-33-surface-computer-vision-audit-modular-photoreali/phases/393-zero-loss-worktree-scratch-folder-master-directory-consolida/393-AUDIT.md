# Phase 393 Forensic Audit: Worktrees, Scratch Folders & Sibling Directories Across `~/dev`

**Audited:** 2026-11-09
**Scope:** `/Users/brbrainerd/dev/vox*` (Primary Priority) + all sibling repositories, `_wt/`, `.worktrees/`, `.claude/worktrees/`, and `_scratch/` in `/Users/brbrainerd/dev` (`~/dev`).
**Policy:** Supersedes `D15` per operator directive (`D37`): carefully salvage all unique unmerged code, schemas, plans, and dirty diffs into the canonical parent master folders (`/Users/brbrainerd/dev/<repo>`), then remove and prune secondary worktrees and build/scratch trash (`~176+ GB` total reclaimable).

---

## 1. Vox Worktree & Scratch Directory Inventory (`/Users/brbrainerd/dev/vox*`)

| Directory Path | Role / Branch | Disk Size | HEAD Commit | Dirty / Untracked | Cherry / Merge Status vs `main` | Bucket | Resolution Plan |
|---|---|---|---|---|---|---|---|
| `/Users/brbrainerd/dev/vox` | Primary Master (`main`) | ~100+ GB | `cb059eeace` | Active working tree | Canonical Master | **MASTER** | Target destination for all salvaged Vox work. |
| `/Users/brbrainerd/dev/vox-mens-compare` | Worktree (`gsd/mens-compare-316`) | **158 GB** (`143 GB` `target/`, `10 GB` `.sccache/`, `3.4 GB` `mens/`) | `4ad5670efa` | 123 dirty/untracked files | 59 commits on `gsd/mens-compare-316` | **Bucket B (`NEEDS-SALVAGE`)** | Copy missing kernel (`decode_attention.rs`), schemas (`gate-receipt.v2.schema.json`, `spoke-gate-state.v1.schema.json`), `.planning/phases/337..345` plans/summaries/reviews, and dirty patch into `/Users/brbrainerd/dev/vox`; keep branch ref `gsd/mens-compare-316`; then `rm -rf` `target/` + `.sccache/` and `git worktree remove --force`. |
| `/Users/brbrainerd/dev/vox-voxdiagram` | Worktree (`fix/docs-unit-tests-after-voxdiagram`) | **5.2 GB** | `11c505a9b4` | 3 tracking files | Already in `main` | **Bucket A (`SAFE-DISCARD`)** | `git -C /Users/brbrainerd/dev/vox worktree remove --force /Users/brbrainerd/dev/vox-voxdiagram` |
| `/Users/brbrainerd/dev/vox/.worktrees/pr-669` | Worktree (`feat/chat-tool-relevance-onto-main`) | **863 MB** | `834e72a8e4` | 0 (clean) | Merged in `main` as `ccabeae26` (PR #686) | **Bucket A (`SAFE-DISCARD`)** | `git -C /Users/brbrainerd/dev/vox worktree remove /Users/brbrainerd/dev/vox/.worktrees/pr-669` |
| `/Users/brbrainerd/dev/vox-readme-main` | Worktree (`docs/why-decisions`) | **301 MB** | `1f59a21f8d` | 0 (clean) | Already in `main` | **Bucket A (`SAFE-DISCARD`)** | `git -C /Users/brbrainerd/dev/vox worktree remove /Users/brbrainerd/dev/vox-readme-main` |
| `/Users/brbrainerd/dev/vox/.worktrees/docs-sidebar-chevrons` | Worktree (`fix/docs-sidebar-seam-chevrons`) | **301 MB** | `f47e64a5a7` | 0 (clean) | Already in `main` | **Bucket A (`SAFE-DISCARD`)** | `git -C /Users/brbrainerd/dev/vox worktree remove /Users/brbrainerd/dev/vox/.worktrees/docs-sidebar-chevrons` |
| `/Users/brbrainerd/dev/vox-mens-compare-p335-clean` | Worktree (detached at `4ad5670ef`) | **281 MB** | `4ad5670efa` | 0 (clean) | Clean detached mirror of `vox-mens-compare` | **Bucket A (`SAFE-DISCARD`)** | `git -C /Users/brbrainerd/dev/vox worktree remove /Users/brbrainerd/dev/vox-mens-compare-p335-clean` |
| `/Users/brbrainerd/dev/vox/.worktrees/finalize-chat-loop` | Partial dir (no `.git`) | **12 KB** | None | 1 file (identical to `main`) | 100% byte-identical in `main` | **Bucket C (`BROKEN/PRUNABLE`)** | Remove directory & `.worktrees/` once empty. |
| `/Users/brbrainerd/dev/vox/.claude/worktrees/research-detection-chat-gui-05d826` | Empty skeleton (no `.git`) | **0 B** | None | 0 files | Empty `docker/` dir | **Bucket C (`BROKEN/PRUNABLE`)** | Remove directory & `.claude/worktrees/` once empty. |
| `/Users/brbrainerd/dev/vox-integration` | Empty folder in `~/dev` | **0 B** | None | 0 files | Empty | **Bucket C (`BROKEN/PRUNABLE`)** | `rmdir /Users/brbrainerd/dev/vox-integration` |

### Exact Vox Artifacts to Salvage from `vox-mens-compare` & `_scratch/{p335,p339_cards}` before Removal:
1. `crates/vox-plugin-mens-candle-metal/src/decode_attention.rs` (Metal decode attention kernel)
2. `contracts/mens/gate-receipt.v2.schema.json`
3. `contracts/mens/spoke-gate-state.v1.schema.json`
4. `.planning/phases/340-multi-spoke-hub-repo-layout-and-model-selector/` (`340-01-PLAN.md`, `340-02-PLAN.md`, `340-CONTEXT.md`)
5. `.planning/phases/341-harness-and-agent-spokes/` (`341-01-PLAN.md`, `341-02-PLAN.md`, `341-CONTEXT.md`)
6. `.planning/phases/342-rust-spoke/` (`342-01-PLAN.md`, `342-02-PLAN.md`, `342-CONTEXT.md`)
7. `.planning/phases/343-ladder-wide-re-gating-and-multi-spoke-promotion/` (`343-01-PLAN.md`, `343-02-PLAN.md`, `343-CONTEXT.md`)
8. `.planning/phases/344-mens-debt-sweep/` (`344-01-PLAN.md`, `344-CONTEXT.md`)
9. `.planning/phases/345-mens-regression-root-cause-fixes/` (`345-01-PLAN.md`, `345-01-SUMMARY.md`)
10. `.planning/phases/337..339` completion summaries (`337-01..12-SUMMARY.md`, `338-01..03-SUMMARY.md`, `339-REVIEW*.md`)
11. Full git diff patch of `vox-mens-compare` (`git -C /Users/brbrainerd/dev/vox-mens-compare diff HEAD > .vox/salvage/vox-mens-compare-dirty.patch`) + `_scratch/p335` & `_scratch/p339_cards` copied into `mens/runs/salvage_p335_p339/`.

---

## 2. Sibling `~/dev` Repositories & Scratch Folders Inventory

| Path | Parent Master Repo | Disk Size | Status | Bucket | Resolution Plan |
|---|---|---|---|---|---|
| `/Users/brbrainerd/dev/velocicode-numpy` | `/Users/brbrainerd/dev/velocicode` | 320 KB | Clean, 1 commit ahead (`9fe792f` adding `"numpy>=2.0"`) | **Bucket B (`NEEDS-SALVAGE`)** | Merge `fix/declare-numpy` (`9fe792f`) into `velocicode` `main`, then `git worktree remove`. |
| `/Users/brbrainerd/dev/brFinancier-subdup` | `/Users/brbrainerd/dev/brFinancier` | **465 MB** | Clean, 0 commits ahead | **Bucket A (`SAFE-DISCARD`)** | `git -C ~/dev/brFinancier worktree remove ~/dev/brFinancier-subdup` |
| `/Users/brbrainerd/dev/Resonance-ci-fix-forward` | `/Users/brbrainerd/dev/Resonance` | **2.1 GB** | Branch `fix/ci-python-environment` (7 commits ahead + dirty files) | **Bucket B (`NEEDS-SALVAGE`)** | Save dirty diff patch into `~/dev/Resonance/.salvage/Resonance-ci-fix-forward.patch` (preserving branch `fix/ci-python-environment` in `~/dev/Resonance`), remove worktree, and run `git worktree prune` (also cleans missing `/private/tmp/resonance-runtime-prev-base`). |
| `/Users/brbrainerd/dev/fableforge-infra-pr` | `/Users/brbrainerd/dev/fableforge` | **3.9 GB** | Commit `b0928526` already in `origin/main` | **Bucket A (`SAFE-DISCARD`)** | `git -C ~/dev/fableforge worktree remove --force ~/dev/fableforge-infra-pr` |
| `/Users/brbrainerd/dev/_wt/ff-replicate-slugs` & `ff-runner-doc` | `/Users/brbrainerd/dev/fableforge` | **2.2 GB** | Clean, 0 commits ahead | **Bucket A (`SAFE-DISCARD`)** | `git -C ~/dev/fableforge worktree remove` both. |
| `/Users/brbrainerd/dev/_wt/ff-{backend-persist,legacy-specs,sfx,backend-convo}` | `/Users/brbrainerd/dev/fableforge` | **15.6 GB** | Named branches `wt/112-*` (`ff-backend-convo` has 5 dirty files) | **Bucket B (`NEEDS-SALVAGE`)** | Export `ff-backend-convo` dirty patch to `~/dev/fableforge/.salvage/ff-backend-convo.patch` (all 4 `wt/112-*` branch refs already live in `~/dev/fableforge/.git/refs/heads/wt/`!), remove `_wt/*` worktree checkouts, and `rmdir ~/dev/_wt`. |
| `~/dev/fableforge/.claude/worktrees/*` (10 clean + 6 active) | `/Users/brbrainerd/dev/fableforge` | **~16 GB** | 10 clean merged; 6 active branches/diffs | **Bucket A & B** | Save any dirty diffs from the 6 active worktrees into `~/dev/fableforge/.salvage/`, keep their git branch refs, remove all 16 `.claude/worktrees/*` directories, and run `git worktree prune`. |
| `~/dev/gigme/.claude/worktrees/*` & `.worktrees/*` + `_scratch/gigme-phase179` | `/Users/brbrainerd/dev/gigme` | ~1.5 GB | 3 clean (`e2e-hydration`, `keen-hopper`, `work-history-fields`); 2 dirty (`test-fast-shared-workers`, `narrow-nav-review`) + `_scratch/gigme-phase179` | **Bucket A & B** | Save dirty patches + `_scratch/gigme-phase179` into `~/dev/gigme/.salvage/`, keep branch refs, remove worktrees, and run `git worktree prune`. |
| `~/dev/ramada/.worktrees/*` & `.claude/worktrees/*` + `~/dev/.worktrees/ramada-release-reports-20261027` | `/Users/brbrainerd/dev/ramada` | ~2 GB | 2 clean (`before-v49`, `revoke-connect`); 2 active (`v1.5-milestone`, `event-discovery-v1-1`) + Oct 26 release reports | **Bucket A & B** | Save dirty patch from `event-discovery-v1-1` + release reports into `~/dev/ramada/.salvage/`, keep branch refs `v1.5-milestone` & `claude/v4.5-unified...`, remove worktree dirs, and run `git worktree prune`. |
| `~/dev/_scratch/wip-backups/wip-20261105-161304.patch` | `/Users/brbrainerd/dev/mudlet-dragonball-evolution` | 12 KB | Backup patch for `wanderbot` flee state | **Bucket B (`NEEDS-SALVAGE`)** | Copy into `~/dev/mudlet-dragonball-evolution/.salvage/wip-20261105-161304.patch`. |
| `~/dev/*.log` (Sept 25–27 stdout logs: `flip-*`, `wA..wD-*`, `wave-*`, `t16-*`, `p1/p2-*`) | `~/dev` root | ~250 KB | Obsolete Sept 25–27 test stdout logs | **Bucket C (`BROKEN/PRUNABLE`)** | Remove stale log files from `~/dev/` root. |
| `~/dev/memory` & `~/dev/antigravity-skill-packs` | `~/dev` root | 948 KB | Persistent developer memory & skill packs | **PROTECTED** | **DO NOT TOUCH.** |
