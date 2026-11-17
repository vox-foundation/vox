# Milestones

## v7.6 Unified Light ↔ Deep Research Continuum, Extreme-Data Streaming, Socrates/Novelty Verification & Continuous Direct Publication Flywheel (Shipped: 2026-11-16)

**Phases completed:** 9 phases, 14 plans, 17 tasks

**Key accomplishments:**

- Phase 447 Plan 03 unified `vox-publisher` and `vox-cli-research` onto the canonical `L1`/`L3` novelty/embedder types and captured the deterministic pre-v7.6 baseline (`contracts/reports/scientia-v76-before-baseline.v1.json`) across all 6 evaluation axes and 9 live bug reproductions.
- Phase 448 Plan 01 unified Chat and Deep Research behind one orthogonal 4-tier continuum (`Off | Auto | Fast | Deep`), wired warm-seeded `T1 -> T2 -> T3` CRAG escalation across all 4 domain modes, and stabilized day-level KV-cache system prompt prefixes.
- Phase 449 Plan 01 landed sequenced SSE/WS research deltas (`resync_required` recovery), SHA3-256 chunk spilling & compact IPC payloads, `O(batch_size)` Arrow/CSV `FrameChunkCursor` streaming, and virtualized multi-wave/DAG/tabular views in Axis GUI.
- Phase 450 Plan 01 retired production mock claim checking, `None => Novel` false positives, Epistemic Ouroboros self-ingestion, direct `sha2`/`sha3` imports in SCIENTIA producers/prior-art, and air-gapped scholarly deadlocks across Wave D (`p450-verifier`, `p450-trust-ouroboros`, `p450-novelty-disc`).
- Phase 451 Plan 01 shipped the DB-hydrated Direct Publication (`> Nanopublication`) flywheel (`PUBL-01..04`) behind a 4-layer human `ApprovalToken` gate and eliminated all CLI-DB `execute_command` subprocess spawns across Axis Scientia/Publications Studio (`STUD-01..02`).
- Phase 452 Plan 01 closed all 9 live `v7.6` bug probes (`9/9` resolved), proved zero-regression paired improvement across all 6 evaluation axes via `vox scientia v76-audit --stage after --compare`, mounted `<BeforeAfterAuditInspector />` with virtualized `similar` diff hunks in Axis GUI, and landed the quarantined Next-Lessons pipeline with full batched Wave F verification passing.
- Phase 453 Plan 01 separated OpenRouter synthesis reasoning budgets from visible output budgets (`LIVE-09`), hardened `extract_claims` Markdown stripping plus contrastive/index-preserving claim verification (`LIVE-10`), and scoped `audit_citations` and `research_cache` against ungrounded citations and stale abstentions (`LIVE-11`).
- Phase 453 Plan 02 implemented [`daemon_db_proxy.rs`](crates/vox-cli-research/src/daemon_db_proxy.rs) (`397` LoC), allowing `vox research` and `vox scientia` commands to transparently proxy through `vox-orchestrator-d` whenever canonical `vox.db` is `flock`-locked.
- Phase 454 Plan 01 introduced canonical `vox-orchestrator-types::concilium` causal DTOs (`CONC-01`), monotonic `#seq` doorbell persistence in `vox-db` (`CONC-02`), and zero-token deterministic `concilium_watchdog.rs` (`361` LoC).
- Phase 454 Plan 02 shipped the daemon HTTP v2 Concilium & Populi control plane in [`concilium_api.rs`](crates/vox-orchestrator-mcp/src/http_gateway/concilium_api.rs) (`398` LoC) (`CONC-03`) and wired typed Tauri `mission_control` IPC handlers in `vox-gui` without direct database handles (`CONC-04`).
- Phase 455 Plan 01 shipped `<ConciliumView />` and consolidated `AgentFlow`, `SubAgentsView`, and `AgentTrajectoryBraid` onto unified W3C traceparent swimlanes, `#seq` bulletin routing, and live pilot controls (`CONC-05`, `CONC-06`).
- Phase 455 Plan 02 delivered `<PopuliTopologyView />` with docked swarm workers, `FileAffinityMap` lock matrices, compute hot-swap dispatch controls (`CONC-07`), and consolidated `PopuliMeshBentoView` + `MeshView` with event-driven placement updates and zero `5000ms` polling (`CONC-08`).

---
