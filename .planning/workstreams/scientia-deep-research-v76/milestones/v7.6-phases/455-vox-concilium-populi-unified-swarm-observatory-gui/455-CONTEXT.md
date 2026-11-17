# Phase 455 Context — Vox Concilium & Populi Unified Swarm Observatory GUI

**Approved by Operator:** 2026-11-15 (`"1 approved 2 add viagsd-phase"`)

## Locked Operator Decisions

1. **`5 -> 2` Surface Consolidation Approved:**
   - Merge `SubAgentsView.tsx`, `AgentFlow.tsx`, and `AgentTrajectoryBraid.tsx` into **Mode 1: Vox Concilium** (Unified Causal Swarm Observatory).
   - Merge `MeshView.tsx` and `PopuliMeshBentoView.tsx` into **Mode 2: Vox Populi** (Hybrid Compute & Swarm Placement Topology).
   - Keep `SURFACE_REGISTRY` deep-link aliases/redirects intact so existing capability links (`surface.mesh`, `surface.subagents`, etc.) resolve cleanly.
2. **Backend Causal Event Contract Approved (Phase 454 prerequisite):**
   - Stamp `(swarm_seq, channel, traceparent, span_id, compute_tier, file_lock_prefix, workspace_mode, parked_since_seq)` across `AgentEvent`, `BulletinBoard`, `DbA2AMessage`, and `RemoteTaskEnvelope`.
3. **Visual Direction & Pilot Controls Approved:**
   - Obsidian dark surface + warm Roman gold trim + semantic emerald/amber/coral telemetry palette.
   - Four first-class operator controls in the Live Agent Lens: `Park (0 tok/s)`, `Steer @agent` (`#seq` doorbell post), `Fork Warm Context` (`jj share` + KV prefix locality), and `Hot-Swap Model / Compute Node` (Cloud Bandit <-> Local MENS GPU <-> Populi QUIC Peer).

## Reference Artifacts & Architecture Research

- Persistent Repository Research Doc: `docs/src/architecture/swarm-observability-and-populi-mesh-research-2026.md`
- Approved Concept Art Proposal: `/Users/brbrainerd/.gemini/antigravity/brain/dd81d5dc-d58a-4584-a0ad-1761e1a1f535/vox_concilium_concept_art_proposal.md`
- Interactive HTML Prototype: `/Users/brbrainerd/.gemini/antigravity/brain/dd81d5dc-d58a-4584-a0ad-1761e1a1f535/vox_concilium_concept_prototype.html`
- High-Resolution Concept Renders:
  - `/Users/brbrainerd/.gemini/antigravity/brain/dd81d5dc-d58a-4584-a0ad-1761e1a1f535/vox_concilium_ui_concept_1794792064030.jpg`
  - `/Users/brbrainerd/.gemini/antigravity/brain/dd81d5dc-d58a-4584-a0ad-1761e1a1f535/vox_populi_mesh_swarm_topology_ui_v2_1794792098519.jpg`
