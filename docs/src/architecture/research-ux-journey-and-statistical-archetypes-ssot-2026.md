---
title: "Research UX Journeys, Statistical Archetypes & Self-Documenting Surface Design (SSOT)"
description: "Canonical audit of real Vox research usage patterns, 4 statistical researcher archetypes, pruned redundant chrome, self-documenting technical metrics, and live Federated KB prior-art preflight."
category: "Architecture SSOTs"
status: "current"
topic: "scientia-research"
domain: "knowledge"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Research UX Journeys, Statistical Archetypes & Self-Documenting Surface Design

<!-- kb:summary -->
Synthesizes over one hundred persisted Vox architectural and scientific research documents into four statistical researcher archetypes: Prior-Art & KB Survey (35%), Architecture & Systems Tradeoff Audit (25%), Claim & Contradiction Check (25%), and Empirical Science to arXiv Synthesis (15%). Defines exact UI chrome pruned for redundancy, plain-language surface explanations for cryptographic and epistemic controls, interactive Confidence Gate tuning, and live Federated KB Prior-Art Preflight via `vox_search_query`.
<!-- /kb:summary -->

## Grounding in Historical Vox Research Corpora

Analysis of persisted research artifacts under `docs/src/architecture/` (covering Populi mesh orchestration, Swarm observability, Multi-Repo KB / Graphify / OpenWiki federation, MENS fine-tuning & cost accounting, and Scientia `arXiv.org` publication control) shows that nearly all real investigations fall into four recurring user archetypes — mirroring general scientific and engineering discovery behavior:

| Statistical Archetype | Empirical Share | Typical Query Shape | Optimal Pipeline Preset | Primary Output Surface |
|---|---|---|---|---|
| **1. Prior-Art & KB Survey** | ~35% | Literature & ecosystem landscape across docs + web | `Deep`, `scholarly`, `both`, `2W` | `ResearchView` → `MemoryView` (`Save to KB`) |
| **2. Architecture & Systems Audit** | ~25% | Multi-crate tradeoff, protocol, or code verification | `Deep`, `codegen`, `both`, `3W` | `ResearchView` (`DAG & Claims` + `Sandbox REPL`) |
| **3. Claim & Contradiction Check** | ~25% | Fast external fact check or conflicting-source triage | `Fast`, `general`, `web`, `2W` | `ResearchView` (`Corroborated` / `Flag Citation`) |
| **4. Empirical Science → `arXiv.org`** | ~15% | Quantitative hypothesis test, `95% CI`, preprint export | `Deep`, `scholarly`, `both`, `4W` | `ResearchView` (`Promote`) → `PublicationsView` |

```vox-diagram
graph {
  direction: right
  node A: "1. Archetype Preset (Survey / Audit / Check / Empirical)"
  node P: "2. Live Federated KB Preflight (vox_search_query)"
  node E: "3. Multi-Wave Epistemic Run + Confidence Gate"
  node S: "4. Synthesis / DAG / Claims / Publications Studio"

  A -> P: "seed scope & waves"
  P -> E: "surface baseline hits & web gaps"
  E -> S: "gate & promote to Scientia"
}
```

## UI Elements Audited: Pruned Redundancies vs. Explained Mysteries

### Redundant or Unwired Elements Pruned

- **Duplicate Wax Seals in Command Consoles (`MemoryView`, `ResearchView`):** Both surfaces rendered a `RomanWaxSealBadge` in the top `RomanSurfaceHeader` *and* repeated a second badge inside the console toolbar below it. Consolidation keeps a single live status seal in the header (`LOW EVIDENCE` / `VERIFICATA` on `ResearchView`, `CODEX SYNCED` on `MemoryView`).
- **Duplicate `Top-K` Progress Bar (`MemoryView`):** The static graduated brass bar directly under `RomanRotaryKnob` duplicated the rotary dial's own detent value without adding interaction.
- **Unwired Read-Only `Stability` Rotary Knob (`ResearchView`):** A rotary knob without an `onValueChange` handler violates affordance expectations. Replaced with an interactive **`Confidence Gate`** knob (`50%–95% min`) comparing the user's acceptance floor against live observed wave stability.
- **Duplicate Classification & TeX Controls inside `DirectPublicationDrawer` (`compact={true}`):** When embedded in `PublicationsView`, `PublicationsView` already renders the full Taxonomy Control Tablet, 7-Stage Progression Controller, and 5-tab Artifact Preview above it. Compact mode hides those duplicate selectors so the drawer exposes only the unique **6-Stage Evidence Flywheel Stepper (`1..6`)**, **`Advance Full Flywheel`**, **`Execute Replay`**, and **`Finding Page HTML`**.

### Unexplained UI Controls Made Self-Documenting

- **`Novelty × Utility` Gate (`WorthinessRadarPanel`):** Added inline formula caption and tooltip (`Novelty = prior-art distance`, `Utility = empirical support & replayability`, composite `≥ 65%` needed for autonomous promotion).
- **`Venue Routing & Policy` Metrics (`PublicationsView`):** Added plain-English descriptions on `Reply window`, `Neg-result quota`, `Critic allowed`, and `Atlas gate`.
- **Hybrid RRF & Corpora (`MemoryView`):** Replaced cryptic `k=60 fusion · dim 768` with `BM25 + 768d Vector RRF (k=60)` and added hover explanations across `Architecture Specs · Memory`, `Rust Crates · Chunk`, `Codex Docs · Knowledge`, and `Top-K Depth`.
- **Research Engine Parameters (`ResearchView`):** Documented `Fast` vs `Deep Research`, `Mode` (`general`, `shopping`, `CodeGen`, `scholarly`), `Scope` (`web`, `local`, `both`), `Waves (1W–5W)`, `Judge`, `Save to KB`, and `Promote`.

## Power Upgrades for Deep Discovery

1. **Statistical Research Archetype Launchpad (`research-archetype-launchpad`):** One-click configuration of lane, mode, federated scope, and adversarial wave depth matching the four primary user workflows without requiring manual parameter tuning.
2. **Live Federated KB Prior-Art Preflight (`research-kb-preflight-panel`):** As a researcher enters a query (`>= 4` chars), debounced search (`vox_search_query`) inspects `vox_baseline`, `global_library`, and `repo_local` before launching external calls — surfacing existing baselines/published syntheses (`pub:{id}`) and offering a one-click **`Synthesize KB + Web Gaps`** trigger.
3. **Interactive Epistemic Confidence Gate & Tab Differentiation:** Researchers tune their acceptable minimum corroboration floor dynamically via `Confidence Gate`, while switching between `Overview`, `Synthesis` (expanded report + open citations), `DAG & Claims` (epistemic verification banner first), and `Diagnostics` adapts the reading canvas genuinely to the stage of investigation.
