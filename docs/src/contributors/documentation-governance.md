---
title: "Documentation governance"
description: "Authority map, taxonomy, status vocabulary, and maintenance rules for Vox documentation."
category: "contributor"
status: "current"
sort_order: 10
last_updated: 2026-03-28
training_eligible: true
---

# Documentation governance

This page defines how Vox documentation is organized and how to keep it from drifting.

## Authority map

| Surface | Primary audience | Owns | Must not become |
| --- | --- | --- | --- |
| [`README.md`](../../../README.md) | evaluators, first-time visitors | short front door, quick start, tone, links into the book | a second FAQ or architecture dump |
| [`docs/src/index.md`](../index.md) | site visitors | site landing page, current product narrative, reader-first navigation | a contributor policy page |
| [`docs/src/explanation/faq.md`](../explanation/faq.md) | readers and evaluators | common product and architecture questions | a troubleshooting runbook |
| [`docs/src/how-to/troubleshooting-faq.md`](../how-to/troubleshooting-faq.md) | operators and contributors | operational fixes and environment troubleshooting | the main public FAQ |
| [`AGENTS.md`](../../../AGENTS.md) | contributors and agents | required cross-tool contributor policy, secret-management entry point, short architecture pointers | the general table of contents for the whole repo or a tool-specific troubleshooting log |
| `docs/src/reference/` | readers and contributors | lookup material, contracts mirrored in prose, stable operational references | speculative planning or marketing copy |
| `docs/src/architecture/` | contributors | current architecture, SSOT notes, rationale, research, roadmaps | quick-start or beginner onboarding |
| `docs/src/contributors/` | contributors | contributor hub, documentation governance, contributor-facing process guidance | public product marketing |
| [`docs/agents/`](../../agents/) | contributors and automation | inventories, governance, machine-oriented support docs | duplicated public documentation |
| [`contracts/`](../../../contracts/) | code and CI | machine-readable SSOT | long-form human explanation |

## Taxonomy

Folder placement communicates ownership. Frontmatter communicates how a page should appear in the book.

### Category vocabulary

Use one of these `category` values in frontmatter:

| `category` | Meaning |
| --- | --- |
| `getting-started` | first-stop pages and front-door onboarding |
| `tutorial` | guided learning |
| `how-to` | goal-oriented instructions |
| `explanation` | conceptual understanding |
| `reference` | stable lookup information |
| `adr` | architecture decisions |
| `architecture` | current architecture, SSOTs, research indexes, roadmaps |
| `ci` | CI and quality-specific references |
| `contributor` | contributor-facing governance and process docs |

Alias compatibility exists for a few legacy values, but new docs should use the canonical forms above.

### Status vocabulary

Use `status` when the distinction matters to readers:

| `status` | Use for |
| --- | --- |
| `current` | documented behavior or process the repo actively relies on |
| `experimental` | implemented but intentionally unstable or gated |
| `legacy` | still present but not the preferred path |
| `research` | investigation, findings, or synthesis not equivalent to shipped behavior |
| `roadmap` | future-facing implementation plans |
| `deprecated` | retained only for migration or compatibility notice |

Do not use `status` to make aspirational pages sound shipped.

## Placement guide

When adding or moving a page:

1. If the source of truth is machine-readable, put the contract in `contracts/` and link to it from `docs/src/reference/`.
2. If the subject is a communication protocol or transport boundary, make the machine-readable artifact discoverable from `contracts/index.yaml` and mirror it from one canonical `docs/src/reference/` page.
3. If the page teaches or explains the user-facing language, keep it in `docs/src/`.
4. If the page is mainly for contributors or automation, prefer `docs/src/contributors/` or `docs/agents/`.
5. If the page is research or planning, keep it under `docs/src/architecture/` and label it clearly with `status: research` or `status: roadmap`.
6. If a page exists only as a compatibility stub, make it a short redirect and avoid duplicating the canonical content.

## Claim policy

Forward-facing docs should describe the architecture that exists now.

Prefer:

- "Vox documents a compiler pipeline that generates Rust and TypeScript artifacts."
- "Mens currently defaults to code-oriented training lanes."
- "This page is research, not a claim that the capability is fully shipped."

Avoid:

- "Vox already does everything in this section automatically" unless the code path is current and documented.
- "Mens answers architecture questions" unless that retrieval or QA path is explicitly wired and tested.
- "SSOT" in titles when the page is only a convenience summary or index.

## Maintenance protocol

Use this lightweight review matrix for high-drift surfaces:

| If you change | Also review |
| --- | --- |
| `crates/vox-cli/src/**` command surface | [`docs/src/reference/cli.md`](../reference/cli.md), command-compliance docs, contributor references that mention the command |
| secret or env handling | [`AGENTS.md`](../../../AGENTS.md), [Clavis SSOT](../reference/clavis-ssot.md) |
| agent instruction layering or shell-discipline policy | [`AGENTS.md`](../../../AGENTS.md), [Agent instruction architecture](agent-instruction-architecture.md), and relevant tool-specific overlays such as `GEMINI.md` |
| doc structure, nav, or new pages | this page, [`docs/src/adr/002-diataxis-doc-architecture.md`](../adr/002-diataxis-doc-architecture.md), [`docs/src/SUMMARY.md`](../SUMMARY.md) |
| architecture claims | [Doc-to-code acceptance checklist](../architecture/doc-to-code-acceptance-checklist.md), relevant explanation/reference pages |
| contracts or schema-backed behavior | matching `contracts/` files and the mirrored reference pages |
| communication protocols, transport routes, or streaming semantics | [`contracts/communication/protocol-catalog.yaml`](../../../contracts/communication/protocol-catalog.yaml), [Communication protocols reference](../reference/communication-protocols.md), and the owning protocol page such as MCP / Populi / runtime docs |
| Mens training or corpus behavior | [Mens native training SSOT](../reference/mens-training.md), [Mens training data contract](../reference/mens-training-data-contract.md) |
| Codex `research_metrics`, mesh/cost telemetry env knobs, or telemetry trust boundaries | [Telemetry and research_metrics contract](../reference/telemetry-metric-contract.md), [env-vars](../reference/env-vars.md), [Telemetry trust SSOT](../architecture/telemetry-trust-ssot.md) |
| **`vox-vscode/`** (extension host, webview UI, Oratio/MCP wiring) | [`vox-vscode/README.md`](../../../vox-vscode/README.md), [VS Code ↔ MCP compatibility](../reference/vscode-mcp-compat.md); speech capture / Oratio pages when capture or tool surfaces change |

## Review cadence

- Front door surfaces: review on every material product-language or contributor-experience change.
- Architecture and reference pages: review when the owning code path changes.
- Research and roadmap pages: keep their status current even if the implementation does not move.
- Contributor and governance pages: review whenever CI, inventory rules, or workflow expectations change.

## Related

- [Contributor hub](contributor-hub.md)
- [Doc-to-code acceptance checklist](../architecture/doc-to-code-acceptance-checklist.md)
- [Architectural governance (TOESTUB)](../../agents/governance.md)
- [Doc inventory verifier](../reference/doc-inventory.md)
