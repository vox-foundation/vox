# Milestones

## v1.0 Architectural Housekeeping (Shipped: 2026-09-30)

**Phases completed:** 6 phases, 23 plans, 11 tasks

**Key accomplishments:**

- D-03 verification pass confirms all 10 REQ-dead-crate-delete crates and their prerequisite type/logic migrations already landed; repointed the stale all-features CI matrix entry; operator ratified SC#3's actual host-crate disposition.
- Verified catalog.toml's execution-api/stub-check ghost-entry bookkeeping is already accurate and added a KEEP-FROZEN disposition comment for the three frozen crates, closing ROADMAP Phase 1 Success Criterion 4.
- Ratified ADR-045 as Accepted across all three GSD artifacts (doc body, intel classification, decisions index) and recorded ADR-037's desktop-convergence clause as implemented with three cited, verified code paths — zero code changes, pure documentation ratification.
- Audited vox-gui's command surface (clean — no violations), recorded the finding and one documented exemption in the surface-ownership contract, wrote the Vox-native/React authoring boundary in a machine-readable field plus matching prose, and fixed the one reference page still naming a deleted crate as the canonical GUI.

---

## v1.3 Self-Maintaining Public Docs (Shipped: 2026-10-13)

**Phases completed:** 6 phases (Phases 20, 20.1, 21, 22, 23, 24), 33 plans, 32 verified

**Key accomplishments:**

- Restored `voxlang.org` Cloudflare Pages deployments with fast local staging, added single-issue failure escalation and `ci-liveness` checks, fixed git-derived page dates across symlinked content, added frontmatter status banners and `noindex` for non-current pages, and isolated research/roadmap pages into the `Internals` sidebar section.
- Shipped collapsible left hover-rail and right table-of-contents sidebars with responsive wide-article layout on `voxlang.org`, verified by 12/12 Playwright visual-stepper tests across mobile, tablet, and desktop viewports in light and dark modes.
- Built deterministic claim extraction (`vox-doc-pipeline::claims`), `vox audit docs` CLI command, machine-populated Docs Reality Audit findings, and the corpus prune report (`prune-report.v1.json`) auditing all 972 doc pages.
- Shipped the code-anchored freshness ledger (`ledger.v1.json`), tighten-only baseline ratchet (`baseline.v1.json`), expiring suppressions, PR impact reverse-index comment (`vox ci pr-impact`), and executable `.vox` tutorial test harness (`TUT-02`).
- Generated `env-vars.generated.md`, `config-keys.generated.md`, `mcp-tools.generated.md`, and 127 crate API documentation pages (`docs/src/api/*.generated.md`) under `ssot-drift`, plus CLI `build_catalog()` compliance verification and retired-decorator prose checks.
- Delivered reader trust banners and sidebar badges driven by `ledger.v1.json`, consolidated `llms.txt` and `/llms-full.txt`, committed the 10-case golden drift evaluation set (`golden-drift-eval-set.v1.json`), and ratified the Decision D9 in-house build verdict (`docs-bot-evaluation-and-pivot-rule-2026.md`).

---

