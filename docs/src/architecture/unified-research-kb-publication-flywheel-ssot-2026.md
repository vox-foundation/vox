---
title: "Unified Research, Federated Knowledge Base & Autonomous Discovery-to-Publication Flywheel (SSOT)"
description: "Canonical architecture for the closed-loop workflow connecting Active Research, the 3-layer Federated Knowledge Base, Discovery Review, and Scientia 7-stage Publication Studio — offline standalone or in tandem."
category: "Architecture SSOTs"
status: "current"
topic: "scientia-research"
domain: "knowledge"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Unified Research, Federated Knowledge Base & Autonomous Discovery-to-Publication Flywheel

<!-- kb:summary -->
Defines the canonical 5-stage closed-loop workflow bridging Active Research (`ResearchView`), the 3-layer Federated Knowledge Base (`MemoryView`: `vox_baseline`, `~/.vox/library/` `global_library`, and `docs/wiki/` `repo_local`), Discovery triage (`DiscoveryInbox` and `DiscoveryReview`), and the 7-stage `Scientia` Publication Studio (`PublicationsView`). Approved and published manuscripts back-index into `knowledge_nodes` (`pub:{id}`), enabling standalone offline use alongside seamless tandem discovery, curation, and publication.
<!-- /kb:summary -->

## Overview: Closing the Discovery-to-Knowledge Loop

Vox integrates research execution, untrusted evidence quarantine, multi-tier knowledge retrieval, and publication-grade `arXiv.org` manuscript synthesis into one auditable loop — while preserving clean entry and exit points for standalone usage.

```vox-diagram
graph {
  direction: right
  node R: "1. Active Research (ResearchView)"
  node Q: "2. Discovery Review (DiscoveryInbox / DiscoveryReview)"
  node K: "3. 3-Layer Federated KB (MemoryView)"
  node P: "4. Scientia Studio (PublicationsView)"

  R -> Q: "Promote finding + Claims"
  K -> R: "Context Retrieval & Grounding"
  K -> Q: "file_gui_finding (target_scope)"
  Q -> P: "Open in Studio (vox://navigate-surface)"
  P -> K: "Approved / Published pub:{publication_id}"
}
```

## Five-Stage Closed-Loop Architecture

| Stage | Surface / Module | Backend Contract & Storage | Handoff / Transition |
|---|---|---|---|
| **1. Active Research** | `ResearchView` | `research_sessions`, `research_artifacts`, `publication_claims` | `Promote` passes `"finding-{session_id}"`; matches `"finding-{session_id}-{digest}"` |
| **2. Evidence Quarantine & Review** | `DiscoveryInbox`, `DiscoveryReview` | `kb_findings` (`FindingOrigin`), `scientia_finding_candidates` | `"Open review"` dispatches `vox://navigate-surface` (`scientia`) + `vox://scientia-open-studio` |
| **3. 3-Layer Federated KB** | `MemoryView` | `knowledge_nodes`, `CorpusTier` (`docs_src`, `wiki`, `global_library`) | Scope filter (`All Layers`, `Repo-Local`, `Global Library`, `Vox Baseline Canon`) + `"To Research"` |
| **4. 7-Stage Publication Studio** | `PublicationsView`, `DirectPublicationDrawer` | `publication_manifests`, `scientia_publication_queue`, `advance_publication_stage_in_db` | `"Open in Publications Studio"` deep-links `publicationId`; renders IMRaD + BibTeX + `.tar.gz` |
| **5. Knowledge Back-Indexing** | `crates/vox-scientia/src/review_flow.rs` | `knowledge_nodes` (`id = "pub:{publication_id}"`, `node_type = "research_synthesis"`) | Auto-upserted when stage reaches `approved` or `published`, feeding future Research queries |

## Three-Layer Federated Knowledge Base Model

The KB separates baseline documentation truth from user-curated learnings and project-specific working knowledge (`crates/vox-search/src/kb_ids.rs`, `crates/vox-search/src/promotion.rs`):

1. **Layer 1 — `vox_baseline` Canon (`CorpusTier::DocsSrc`, repo `docs/src/`):** Immutable-by-convention core language, compiler, runtime, and governance reference documentation packaged with Vox. Any project or workflow inherits this baseline out of the box without duplicating core docs.
2. **Layer 2 — `global_library` (`CorpusTier::GlobalLibrary`, `~/.vox/library/promoted/`):** User-scoped cross-repository library for validated methodologies, scientific syntheses, and reusable engineering patterns promoted with `PromotionTargetScope::GlobalLibrary`.
3. **Layer 3 — `repo_local` (`CorpusTier::Wiki`, `docs/wiki/promoted/` & scoped `knowledge_nodes`):** Repository-isolated domain notes, local evidence, and project-specific manuscripts kept private to the current repository (`target_scope = "repo_local"`).

## Tandem Operation vs. Standalone Expansion Patterns

### Using Tools in Tandem (Full Discovery-to-Publication Flywheel)

1. Start in **Knowledge Base (`MemoryView`)** by choosing a retrieval scope (`Repo-Local`, `Global Library`, or `Vox Baseline Canon`) to inspect baseline coverage, or ingest domain papers/text with an explicit `target_scope` (`repo_local` or `global_library`) which both indexes a searchable node and files a companion `kb_findings` quarantine audit trail (`file_gui_finding`).
2. Jump directly via **"To Research"** into **Active Research (`ResearchView`)** to run multi-provider or local grounded investigation against indexed canon and local corpus context.
3. Click **Promote** on a high-signal finding or research session: `promote_candidate_to_publication` matches `"finding-{session_id}"` prefixes to `"finding-{session_id}-{digest}"` rows and preserves the `candidate_class` (`empirical_science`, `algorithmic_improvement`, `bug_fix`, `benchmark_report`) saved in `confidence_json`.
4. Triage in **Scientia Discovery Review** (`DiscoveryInbox` → `DiscoveryReview`), verify claims, and click **"Open in Publications Studio"** to jump straight to `PublicationsView` with `publicationId` focused.
5. Advance through the 7 lifecycle stages (`draft -> doi_reserved -> orcid_attributed -> approved -> submitted -> published`) and export the validated `arXiv.org` `.tar.gz` bundle. Upon reaching `approved` or `published`, Vox automatically back-indexes `pub:{publication_id}` into `knowledge_nodes` as `research_synthesis` so subsequent research queries immediately retrieve verified discoveries.

### Using Tools Standalone or Expanding Into Your Own KB

- **Vox Baseline as Seed + Repo Expansion (`repo_local`):** Keep `vox_baseline` as the canonical platform baseline while accumulating all new scientific findings, domain corpora, and `research_synthesis` back-links strictly inside `repo_local` (`docs/wiki/promoted/` + `target_scope: "repo_local"`). Zero cross-repo leakage occurs.
- **Graduating Reusable Knowledge (`global_library`):** Promote domain-agnostic protocols, verified benchmarks, and published syntheses into `~/.vox/library/promoted/` (`target_scope: "global_library"`) so every new repository inherits your personal scientific corpus alongside `vox_baseline`.
- **Standalone Surface Execution:** `ResearchView` runs ad-hoc investigations without requiring publication; `MemoryView` ingests/searches independently without activating `Scientia`; `PublicationsView` (`DirectPublicationDrawer`) authors and compiles `arXiv.org` manuscripts from any manual IMRaD draft even offline.

## Key Implementation Contracts

- **Unified Ingest Routing:** `crates/vox-gui/src/commands/knowledge.rs` (`ingest_text`, `ingest_url`) forwards `target_scope` to `crates/vox-orchestrator-mcp/src/gui_ops/knowledge.rs`, preserving `ToolResult::ok(node_id)` (`String`) return parity while filing best-effort `kb_findings` records.
- **Prefix & Class Promotion Fidelity:** `crates/vox-scientia/src/review_flow.rs` (`promote_candidate_to_publication`) resolves `"finding-{session_id}"` prefix lookups and extracts `"candidate_class"` from `confidence_json` before falling back to SQL column defaults.
- **Synthesis Back-Indexing:** `advance_publication_stage_in_db` upserts `knowledge_nodes` (`id = "pub:{publication_id}"`, `node_type = "research_synthesis"`) when entering `approved` or `published`.
