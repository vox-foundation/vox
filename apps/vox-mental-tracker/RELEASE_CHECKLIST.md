# Release checklist — vox-mental-tracker vX.Y.Z

Tag: `vox-mental-tracker-vX.Y.Z`
Commit: `<sha>`
Date: `<YYYY-MM-DD>`
Released by: `<name>`

## Programmatic gates

Run `bash apps/vox-mental-tracker/scripts/release_check.sh` from a clean working tree.

- [ ] G1 — Vitest: `<N>` tests passed
- [ ] G2 — Playwright: `<N>` tests passed (with `BASE_URL=<url>`)
- [ ] G3 — `vox check`: 0 errors / 0 warnings
- [ ] G4 — Contracts: `<N>` JSON + `<N>` YAML files parsed cleanly

CI run link: `<url>`

## Manual gates

- [ ] G5 — Android debug build produced; smoke-tested home + voice flow on `<device model>`
- [ ] G6 — iOS simulator build produced (or gap documented in release notes; reason: `<…>`)
- [ ] G7 — `docs/user/privacy.md` audited against current data flows; no stale claims
- [ ] G8 — `docs/architecture/failure-modes-research-2026.md` audited; new surfaces covered

## Rollout

- [ ] Tag pushed
- [ ] GitHub release created with this checklist attached
- [ ] Users of prior version notified of any data-format / privacy changes (via release notes)

## Notes

`<freeform notes for this release: notable changes, known issues, follow-up work>`
