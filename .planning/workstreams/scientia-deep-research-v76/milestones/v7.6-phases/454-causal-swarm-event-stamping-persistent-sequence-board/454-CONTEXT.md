# Phase 454 Context — Causal Swarm Event Stamping & Persistent Sequence Board

**Status:** Locked from Operator Approval (`"1 approved 2 add viagsd-phase"`, `2026-11-15`) & Architecture Research (`docs/src/architecture/swarm-observability-and-populi-mesh-research-2026.md`)
**Workstream:** `.planning/workstreams/scientia-deep-research-v76/`

## Locked Decisions

1. **`CONC-01` — Canonical L0 Causal Swarm Stamp (`vox-orchestrator-types`):**
   - Define pure L0 `ComputeTier` (`CloudBandit`, `LocalMensGpu`, `PopuliQuicPeer`, `InlineCpu`), `SwarmWorkspaceMode` (`Inherit`, `Share`, `Branch`), and `CausalSwarmStamp`:
     `(swarm_seq: u64, channel: String, traceparent: String, span_id: String, compute_tier: ComputeTier, file_lock_prefix: Option<String>, workspace_mode: SwarmWorkspaceMode, parked_since_seq: Option<u64>)`.
   - Stamp `#[serde(default, skip_serializing_if = "Option::is_none")] pub causal_stamp: Option<CausalSwarmStamp>` across `AgentEvent` (`vox-orchestrator::events`), `BulletinBoard` sequenced envelopes (`vox-orchestrator::bulletin`), `DbA2AMessage`, and `RemoteTaskEnvelope` (`vox-orchestrator::a2a::envelope`) with 100% backward-compatible serde defaults.

2. **`CONC-02` — Append-Only `#seq` Shared Sequence Board & `0 tok/s` Doorbell Wakeups:**
   - Extend `BulletinBoard` with a monotonic `AtomicU64` `#seq` sequence counter, bounded replay ring (`SequenceBoardEntry`), `#channel` normalization (`#main`, `#parser`, `#verify`, `#scientia`, etc.), `@agent` / `@all` / `@#channel` mention extraction, and `ParkedAgentRegistry` (`waiting` at `0 tok/s` with `parked_since_seq: u64` + `tokio::sync::Notify` wakeup when mentioned).
   - Persist `#seq` board entries into Codex `a2a_messages` (`msg_type = "concilium_seq_board"`) with `read_board(since_seq, seqs, channel, limit)` cursoring in `vox-db` (`ops_a2a.rs`).

3. **`CONC-03` — Pilot Steering RPCs (`/api/v2/concilium/*` + MCP + `vox-gui` `mission_control`):**
   - Expose read and write endpoints under `/api/v2/concilium/*` in `vox-orchestrator-mcp::http_gateway::concilium_api` (mounted in `mod.rs`) plus MCP tool `vox_concilium_control` and `vox-gui::commands::mission_control` (`daemon`-class, zero `VoxDb` handles):
     - `GET /api/v2/concilium/snapshot` (agents, causal swimlane spans, now-playing strip summary, parked `0 tok/s` counts)
     - `GET /api/v2/concilium/board?since_seq=&seqs=&channel=&limit=` (filtered `#seq` Shared Sequence Board)
     - `GET /api/v2/concilium/placement` (Local MENS GPU, Populi QUIC Peer, Cloud Bandit Router node cards + docked `@agent` pills + `FileAffinityMap` Lock Matrix)
     - `POST /api/v2/concilium/park` (`park_agent` at `0 tok/s` recording `parked_since_seq`)
     - `POST /api/v2/concilium/steer` (`steer_agent` appending `#seq` directive + `@agent` doorbell wakeup)
     - `POST /api/v2/concilium/fork` (`fork_agent` branching warm context + `share` workspace mode preserving local MENS KV-cache prefix metadata)
     - `POST /api/v2/concilium/swap-compute` (`swap_agent_compute` hot-migrating worker tier across `CloudBandit`, `LocalMensGpu`, and `PopuliQuicPeer`).

4. **`CONC-04` — Zero-Token Rust Watchdog (`concilium_watchdog.rs`):**
   - Deterministic Rust watchdog evaluates `circuit_breaker::CircuitBreakerState`, `AttentionBudget`, lease TTL expiry, and `FileAffinityMap` lock contention, posting zero-token structured `#seq` system alerts (`#ops` / `#main`) and waking `@monitor` ONLY when deterministic recovery thresholds trip (`TripReason` or unresolved lock contention).
