---
phase: 116
phase_name: "github-ci-red-gate-and-db-digest-remediation"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 4
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
---

# Phase 116 Learnings: github-ci-red-gate-and-db-digest-remediation

## Decisions

### CI-GREEN-01 (contracts/db/baseline-version-policy.yaml)
**`CI-GREEN-01` (`contracts/db/baseline-version-policy.yaml`):** Updated `repository_baseline_digest_hex` to `"0x2d24ad28ca1cbba8cab84f2fcc5fa0f150d04a361b56d248a7901442fba05071"` matching `vox_db::schema::baseline_sql()` Keccak256 (`0` baseline digest warnings in `vox ci ssot-drift`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 116-01-SUMMARY.md

---

### CI-GREEN-02 (crates/voxup/src/profiles.rs, crates/_public.toml)
**`CI-GREEN-02` (`crates/voxup/src/profiles.rs`, `crates/_public.toml`):** Updated `public_toml_crates` to parse both `[[crates]]` table `name` entries and string arrays, and added Tier 2 plugin/installer crates (`vox-plugin-types`, `vox-plugin-api`, `vox-plugin-sdk`, `voxup`) to `crates/_public.toml` (`cargo test -p voxup --test distribution_parity`: `10 passed; 0 failed`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 116-01-SUMMARY.md

---

### CI-GREEN-03 (docs/src/community/leaderboard.mdx, docs-astro/src/componen...
**`CI-GREEN-03` (`docs/src/community/leaderboard.mdx`, `docs-astro/src/components/VoxLeaderboardArena.astro`, `.planning/workstreams/unified/ROADMAP.md`):** Replaced HTML comments with MDX `{/* kb:summary */}` in `leaderboard.mdx`, escaped curly braces in `VoxLeaderboardArena.astro`, and fixed relative link resolution in `.planning/workstreams/unified/ROADMAP.md` (`pnpm --dir docs-astro build` built `855` pages in `38.30s`; `vox ci check-links` verified `5402` internal links).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 116-01-SUMMARY.md

---

### CI-GREEN-04 (.github/workflows/cr-l8-corpus-feedback.yml)
**`CI-GREEN-04` (`.github/workflows/cr-l8-corpus-feedback.yml`):** Replaced the 14-minute `vox-cli` compilation in `scripts-check` with lightweight `vox-langtool` (`cargo build -p vox-langtool --bin vox-langtool` + `./target/debug/vox-langtool check "$f"`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 116-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

