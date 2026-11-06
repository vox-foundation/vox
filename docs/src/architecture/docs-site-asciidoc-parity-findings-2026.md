---
title: "Docs Site AsciiDoc-Parity Blueprint: Verified Findings (2026)"
description: "Point-by-point verification of an external AsciiDoc-parity modernization blueprint for voxlang.org against the live docs-astro tree, with the accepted scope and rejected proposals."
category: "Architecture SSOTs"
status: "research"
topic: "doc-governance"
domain: "documentation"
authorship: "llm-assisted"
authorship_model: "claude-opus-5-5"
---

# Docs Site AsciiDoc-Parity Blueprint: Verified Findings (2026)

<!-- kb:summary -->
An external blueprint proposed four AsciiDoc-parity pillars for voxlang.org: richer transclusion, audit-driven banners, contract-backed tables, and workspace attributes. Checked against the live `docs-astro/` tree on 2026-11-05, only transclusion and version derivation held up; both landed. Audit banners already exist (ledger-driven), the proposed stability contract and reality-audit status fields do not exist, and the recommended Starlight plugins need Starlight 0.42 while the repo pins 0.38. Remaining work is tracked as roadmap Phase 342.
<!-- /kb:summary -->

## Verdict by pillar

| Pillar | Blueprint claim | Reality (2026-11-05) | Decision |
|---|---|---|---|
| 1. Transclusion | Upgrade `remark-vox-include.mjs` with tags, ranges, dedent, `::include{}` | Plugin existed, anchors only, no deps | **Done:** tags (`tag::x[]`), `#` leaders, multi-select, `N..M` ranges, dedent, exact names. `::include{}`, `// !hide`, auto titles cut (no call sites). |
| 2. Admonitions | Drive `Banner.astro` from `docs-reality-audit/inventory.v1.json`; add `starlight-github-alerts` | Banner already reads `docs-freshness/ledger.v1.json`; inventory has no status field; findings empty | **Rejected** banner rewrite (owned by `docs-freshness` Phase 32). GitHub alerts **accepted** — 24 docs use them — after the Starlight upgrade. |
| 3. Data tables | `stability-matrix.v1.json` + Content Layer `file()` loaders | Contract does not exist | **Rejected**; use the `*.generated.md` + `ssot-drift` pattern. |
| 4. Attributes / xrefs | `remark-vox-attributes`, `starlight-links-validator@0.27.0` | No `{token}` usage; validator needs Astro ≥7.2.10 / Starlight ≥0.42 | `llms.txt` version now derived from root `Cargo.toml` (**done**). Token plugin rejected. Validator **accepted** after upgrade. |

## Other corrections

- The live remark chain is `remarkVoxInclude` → `remarkDocLinks`; `remarkStripTitleH1` and `voxDiagram` do not exist.
- `.mdx` is already first-class (Phase 323), so "keep everything `.md`" is not a constraint.
- The version SSOT is root `Cargo.toml` `[workspace.package]`, not `CITATION.cff`.
- Namespace collision is real: `vox-lang` (crates.io; English-sentence → NASM compiler) and `MrSmith33/vox` (D-language compiler, BSL-1.0) both exist; `llms.txt` now disambiguates.

## Follow-up

Roadmap Phase 342 (`DOCS-PARITY-02..04`): Starlight ≥0.42 / Astro ≥7.2.10 upgrade, GitHub-alert rendering, build-time link validation plus the two current `remark-doc-links` failures.
