# Phase 470 Plan: End-to-End Polyglot Repo Onboarding Proof, Concept-Art Plates & Multi-Pass Before/After CV Flywheel Convergence

**Workstream:** `universal-kb-openwiki-v79`
**Phase:** `470`
**Requirements:** `CV-01`, `CV-02`, `CV-03`, `CV-04`

## Target Files
- `crates/vox-gui/ui/public/assets/romanesque/v79-concept-{openwiki-studio,multirepo-atlas}-{carrara,slate}.jpg`
- `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts`
- `crates/vox-gui/ui/e2e/review/v79CvFlywheelConvergence.test.ts` (new, `<350` LoC)
- `crates/vox-cli/tests/v79_polyglot_flywheel_proof.rs` (new)
- `crates/vox-search/src/promotion.rs` (document-relative citation containment fix for `docs/wiki/*.md` Tier-2 proposals)
- `crates/vox-doc-pipeline/src/pipeline/openwiki_heal.rs` (sanitize `GIT_*` env vars and annotate read-only `git diff --name-only HEAD` probe for `vox-arch-check`)
- `contracts/reports/v79-universal-kb-openwiki-flywheel-proof.v1.json` (new)
- `docs/src/architecture/universal-kb-graphify-openwiki-multirepo-research-2026.md`
- `docs/src/architecture/where-things-live.md`
- `docs/src/architecture/docs-system-ssot.md`

## Steps
1. Create `470-CONTEXT.md` and `470-PLAN.md`.
2. Provision `v79-concept-*` Carrara & Slate plates, wire `conceptArtPlateForSurface` in `vlmVisionFlywheel.ts`, and implement + run `v79CvFlywheelConvergence.test.ts` (`CV-02`, `CV-03`).
3. Implement `crates/vox-cli/tests/v79_polyglot_flywheel_proof.rs`, write `contracts/reports/v79-universal-kb-openwiki-flywheel-proof.v1.json`, format with `rustfmt --edition 2024`, and verify passing via `cargo test -p vox-cli --test v79_polyglot_flywheel_proof` (`CV-01`).
4. Promote `universal-kb-graphify-openwiki-multirepo-research-2026.md` (`status: "current"`), update `where-things-live.md` + `docs-system-ssot.md`, and run `vox-doc-pipeline --lint-only`, `vox-arch-check`, and Vitest guard suites (`CV-04`).
5. Write `470-{VERIFICATION,REVIEW,SUMMARY}.md`, mark `v7.9` `6/6` phases (`24/24` requirements, `100%`) complete in `REQUIREMENTS.md`, `ROADMAP.md`, `STATE.md`, and `state.json`, and run the home-path literal scan.
