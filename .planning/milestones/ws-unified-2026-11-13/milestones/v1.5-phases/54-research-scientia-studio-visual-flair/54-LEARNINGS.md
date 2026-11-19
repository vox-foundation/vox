---
phase: 54
phase_name: "research-scientia-studio-visual-flair"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 54 Learnings: research-scientia-studio-visual-flair

## Decisions

### Close Gap 6 from [scientia-publication-and-deep-research-claims-vs-reali...
Close Gap 6 from [`scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`](../../../../docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md): 1. **Typed Frontend Research API (`crates/vox-gui/ui/src/api/research.ts`)**: Extend `startResearchAsync` and `ResearchSession` types with `domainMode` (`'general' | 'shopping' | 'codegen'`), `scope` (`'web' | 'local' | 'both'`), `waves` (`1..3`), `siteScope`, `verifyCompilation`, `epistemic_dag`, `waves_summary`, `shopping_matrix`, `codegen_verification`, and `usage`. 2. **Power Control Bar & Domain Result Cards in `ResearchView.tsx`**: - Add a sleek, compact power control bar beneath the search input in `ResearchView.tsx` allowing users to switch `Domain Mode` (`General` | `Shopping Matrix` | `CodeGen Verified`), `Scope` (`Web` | `Local Repo` | `Hybrid Both`), `Waves` (`1W` | `2W` | `3W`), and optional `site:` filter. - Render `ShoppingDecisionCard.tsx` (weighted criteria bars + ranked option scores table) when `shopping_matrix` is present, and `CodeGenVerificationCard.tsx` (`rustc`/`cargo check` compilation status badge, repair wave count, and diagnostics snippet) when `codegen_verification` is present.

**Rationale:** Primary phase objective established in plan.
**Source:** 54-01-PLAN.md

---

### Close Gap 7 from [scientia-publication-and-deep-research-claims-vs-reali...
Close Gap 7 from [`scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`](../../../../docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md) and §1.3 from [`deep-research-full-surface-audit-and-roadmap-2026.md`](../../../../docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md): 1. **Live Backend Stage Mapping in `ResearchView.tsx`**: Map the live `session.status` (`planning`, `retrieving`, `verifying_claims`, `auditing_citations`, `synthesizing`, `persisting_artifact`, `completed`, `failed`) and `research_session_events` into `deriveStages(session.status)` so `PipelineTimeline` advances through each stage in real time during a run instead of freezing on Stage 0. 2. **Mount `MultiWaveProgressTimeline.tsx` & Live Telemetry Ribbon**: Mount the existing `MultiWaveProgressTimeline.tsx` component inside `ResearchView.tsx` (fed by `session.waves_summary` and `wave_*` events) and add a tasteful live observability ribbon displaying `Wave N/M · Stability Δ · Verified Claims X/Y · Contradictions C · Est. Cost $Z` across both `ResearchView.tsx` and `ResearchTracePanel.tsx`. 3. **Chat `force_research` & `research_scope` Forwarding + Non-Blocking Stage Events (`chat_turn.rs`, `SCIGUI-05`)**: Add `force_research: Option<bool>` and `research_scope: Option<String>` to `ChatTurnInput` and forward them in `sync_tool_args` in `crates/vox-gui/src/commands/chat_turn.rs` so Chat's research toggle and `/research` scope controls reach `vox_chat_message` and emit live stage events to `ResearchTracePanel.tsx`.

**Rationale:** Primary phase objective established in plan.
**Source:** 54-02-PLAN.md

## Lessons

## Patterns

## Surprises

