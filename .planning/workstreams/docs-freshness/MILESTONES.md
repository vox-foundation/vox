# Milestones — docs-freshness

## v1.3 Self-Maintaining Public Docs (Shipped: 2026-10-13)

**Phases completed:** 6 phases (20, 20.1, 21, 22, 23, 24), 33 plans

**Key accomplishments:**

- Restored `voxlang.org` Cloudflare Pages deployments, added single-issue failure escalation and `ci-liveness` checks, fixed git-derived page dates across symlinked content, added status banners/`noindex` for non-current pages, and isolated research/roadmap pages into the `Internals` sidebar section.
- Shipped collapsible left/right sidebars and responsive wide-article layout on `voxlang.org` with 12/12 Playwright visual-stepper tests across phone, tablet, and desktop in light and dark modes.
- Built deterministic claim extraction (`vox-doc-pipeline::claims`), `vox audit docs`, machine-populated Docs Reality Audit findings, and the corpus prune report (`prune-report.v1.json`).
- Shipped the code-anchored freshness ledger (`ledger.v1.json`), tighten-only baseline ratchet (`baseline.v1.json`), expiring suppressions, PR impact reverse-index comment, and executable tutorial test harness (`TUT-02`).
- Generated `env-vars.generated.md`, `config-keys.generated.md`, `mcp-tools.generated.md`, and 127 crate API pages (`docs/src/api/*.generated.md`) under `ssot-drift`, plus CLI `build_catalog()` compliance and retired-decorator prose checks.
- Delivered reader trust banners and sidebar badges driven by `ledger.v1.json`, consolidated `llms.txt` / `/llms-full.txt`, committed the 10-case golden drift eval set (`golden-drift-eval-set.v1.json`), and ratified the Decision D9 in-house build verdict (`docs-bot-evaluation-and-pivot-rule-2026.md`).

---
