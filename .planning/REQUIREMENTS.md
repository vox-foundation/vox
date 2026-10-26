# Requirements: Vox Milestone v5.5 (`Phases 307–312`) — T3 CI/TOESTUB Deep Enforcement
**Defined:** 2026-10-25
**Core Value (`v5.5`):** Go deeper than v5.4's change-gated CI / knip / thin-transport / path-label fingerprint by extracting t3code's next tier — contributor vouch trust, privileged preview trust-split, **live** transfer-budget measurement + job-matrix outputs, wall-clock/real-wait/test-in-loop/frame-wait detectors, host-runtime + schema-hoist + DOM-env discipline, and true sidecar fingerprint hashing — without redoing shipped v5.4 gates or Effect-only oxlint rules.
**Evidence:** `~/dev/t3code` (`pr-vouch.yml`, `VOUCHED.td`, `relay-preview{,-cleanup}.yml`, `ci.yml` transfer artifacts, `ci-changes.cjs`, `mobile-fingerprint-check.yml`, `oxlint-plugin-t3code/rules/{no-wall-clock,no-real-time-wait,no-test-in-loop,no-global-process-runtime,no-inline-schema-compile,no-dom-test-environment,no-frame-waits}*`); graph `contracts/reports/t3-ecosystem-surface-graph.v1.json`; store report `docs/t3-deeper-extraction-v5.5.md`.

## v5.5 Requirements (0/24 Complete Across Phases 307–312)

### Phase 307: PR Vouch & Contributor Trust Gate (`T3-TRUST`)
- [ ] **T3-TRUST-01**: Add `.github/VOUCHED.td` (or `contracts/ci/vouched.v1.yaml`) SSOT for trusted/denounced GitHub logins, documented in AGENTS/contributors.
- [ ] **T3-TRUST-02**: Add `.github/workflows/pr-vouch.yml` using mitchellh/vouch (or documented equivalent) on `pull_request_target` + `/recheck-vouch` comments; manage `vouch:trusted|unvouched|denounced` labels.
- [ ] **T3-TRUST-03**: Wire required/advisory context so unvouched authors are visibly labeled without granting fork PRs write beyond label sync (match t3code trust model).
- [ ] **T3-TRUST-04**: Mutation tests: vouch status → label transitions; denounced blocks merge guidance; VOUCHED.td parse fails closed on syntax errors.

### Phase 308: Preview/Privileged Workflow Trust-Split Policy (`T3-PREVIEW`)
- [ ] **T3-PREVIEW-01**: Author `docs/src/ci/privileged-preview-trust-split.md` capturing t3code relay pattern: deploy may run PR code only under environment approval; cleanup uses `pull_request_target` + default-branch checkout and never executes PR code.
- [ ] **T3-PREVIEW-02**: Add `vox ci preview-trust-audit` (or workflow-policy-guard extension) that flags new workflows which combine `pull_request_target` + PR-ref checkout + secrets without an allowlisted trust-split shape.
- [ ] **T3-PREVIEW-03**: Enumerate existing Vox privileged workflows (if any) against the policy; file suppressions with owner+reason only when intentional.
- [ ] **T3-PREVIEW-04**: Contract/tests: intentional mutation introducing unsafe `pull_request_target`+PR-code+secret combo fails the audit.

### Phase 309: Live Transfer-Budget Measurement & Job-Matrix Change Detector (`T3-LIVE-CI`)
- [ ] **T3-LIVE-CI-01**: Replace fixture-only `transfer-budget-report` CI path with artifact emission from real `vox-orchestrator-mcp` transfer-budget tests (or nextest junit/sidecar JSON), uploaded as `thread-transfer-results`-shaped artifact.
- [ ] **T3-LIVE-CI-02**: Update `thread-transfer-report.yml` to prefer live CI artifact over committed fixture; keep fixture as ceiling baseline only; honest `source` field.
- [ ] **T3-LIVE-CI-03**: Extend `check-changed` / new `vox ci ci-changes` to emit per-job booleans (fmt/clippy/ui/docs/…) from path owners — t3code `ci-changes.cjs` analog — and document wiring into PR jobs.
- [ ] **T3-LIVE-CI-04**: Ratchet: CI fails when live sample exceeds ceilings or when required transfer artifact is missing on non-inert PRs.

### Phase 310: TOESTUB Test-Discipline Detectors (Wall-Clock, Real-Wait, Loop, Frame) (`T3-TEST-DET`)
- [ ] **T3-TEST-DET-01**: Extend `test_clock_and_skip` (or new detector) to flag wall-clock reads in tests: `Instant::now`, `SystemTime::now`, `Date.now`, `performance.now` (t3code `no-wall-clock-in-tests`).
- [ ] **T3-TEST-DET-02**: Add real-time wait detector for async timers / `tokio::time::sleep` / Promise timers in tests without TestClock/mock time (t3code `no-real-time-wait-in-tests`).
- [ ] **T3-TEST-DET-03**: Add `no-test-in-loop` analog: `#[test]`/`it(`/`test(` nested under for/for-of in test files; prefer parameterized tables.
- [ ] **T3-TEST-DET-04**: Promote `no-frame-waits-in-browser-tests` from deferred: fail on `waitForTimeout` / bare `page.waitForTimeout` in Playwright e2e under `crates/vox-gui/ui/e2e`.

### Phase 311: Host-Runtime Boundary, Schema Hoist, Knip Dual & DOM-Env Discipline (`T3-HOST-SCHEMA`)
- [ ] **T3-HOST-SCHEMA-01**: New detector banning direct host platform/arch probes outside allowlisted host crates (`std::env::consts::{OS,ARCH}`, `process.platform`/`process.arch`) — t3code `no-global-process-runtime` analog.
- [ ] **T3-HOST-SCHEMA-02**: New detector/hoist lint for hot-path inline schema compile/parse (serde Schema / vox-schema generate call sites rebuilt inside loops/handlers) — t3code `no-inline-schema-compile` analog.
- [ ] **T3-HOST-SCHEMA-03**: Deepen knip: pin local knip version in `vox-gui/ui`, dual `files+dependencies` then exports pass; document schema/unused-export preprocessor backlog.
- [ ] **T3-HOST-SCHEMA-04**: Ban per-file `@vitest-environment` / jsdom imports in unit tests; require project-level env config (t3code `no-dom-test-environment`).

### Phase 312: Sidecar Fingerprint Hash Compare, Surface Graph Regen & v5.5 Scorecard (`T3-FP-GRAPH`)
- [ ] **T3-FP-GRAPH-01**: Upgrade `gui-sidecar-fingerprint.yml` from path-diff labeling to content hash of native/sidecar inputs (Cargo.lock subset, tauri.conf, sidecar binary manifest) comparing PR head vs base — t3code expo fingerprint:generate analog.
- [ ] **T3-FP-GRAPH-02**: Repair `scripts/t3-surface-inventory.vox` compile blockers; regenerate `contracts/reports/t3-ecosystem-surface-graph.v1.json` from current ~/dev clone SHAs.
- [ ] **T3-FP-GRAPH-03**: Publish `contracts/reports/t3-vox-ci-enforcement-scorecard.v1.json` v5.5 rows for TRUST/PREVIEW/LIVE-CI/TEST-DET/HOST-SCHEMA/FP-GRAPH gems.
- [ ] **T3-FP-GRAPH-04**: Docs: architecture closure note + oxlint mapping ledger updates for newly shipped rules; mark prior Partials resolved or still deferred with owners.

## Out of Scope (v5.5)
| Feature | Reason |
|---|---|
| Re-shipping v5.4 check-changed / knip-gui / thin-transport / path-label fingerprint | Already complete; deepen only |
| Effect-TS oxlint (`prefer-catch-tags`, `prefer-effect-fn`, `require-disposable-render`, …) | No Vox Effect runtime |
| Mobile Hermes / Uniwind / glyph / native ktlint-swiftlint toolchain | No Vox RN/Expo mobile app yet |
| Em-dash / low-value comment style lints | Noise; keep deferred |
| CSS `no-unscoped-has` / outset indicators / scroll gutter | GUI polish backlog |

## Traceability
| Requirement | Phase | Status |
|---|---|---|
| `T3-TRUST-01..04` | Phase 307 | Pending |
| `T3-PREVIEW-01..04` | Phase 308 | Pending |
| `T3-LIVE-CI-01..04` | Phase 309 | Pending |
| `T3-TEST-DET-01..04` | Phase 310 | Pending |
| `T3-HOST-SCHEMA-01..04` | Phase 311 | Pending |
| `T3-FP-GRAPH-01..04` | Phase 312 | Pending |

**Coverage:** 24 v5.5 requirements → 6 phases → 0 unmapped

---

## Prior milestones (archived headers live below)

# Requirements: Vox Milestone v5.4 (`Phases 301–306`) — T3 CI/TOESTUB Enforcement Deep-Extraction & Harness Policy Hardening

**Defined:** 2026-10-25
**Core Value (`v5.4`):** Make Vox's CI/CD, TOESTUB/`vox-code-audit`, and agent-ops policy surface as hard as `pingdotgg/t3code`'s oxlint+CI matrix for the gems still missing after v3.9 and Waves 2–5 runtime extractions — change-gated checks, transfer-budget reporting, knip/PR hygiene, kill-safety + multi-surface checklists, thin-transport detectors, and fingerprint/cache/graph closure — without re-extracting shipped router/harness features (`VOX-ROUTER-T3-*`, `VOX-HARNESS-T3-*`, Wave-3 TOESTUB detectors).
**Evidence:** `~/dev/t3code` (`scripts/check-changed.ts`, `oxlint-plugin-t3code` 29 rules, `.github/workflows/{pr-size,pr-vouch,thread-transfer-report,mobile-fingerprint-check}.yml`, `knip.jsonc`, `AGENTS.md`); graph `contracts/reports/t3-ecosystem-surface-graph.v1.json`; audits in agent store `internal/t3*-2026-10-25.md`.

---

## v5.4 Requirements (24/24 Complete Across Phases 301–306)

### 1. Phase 301: Change-Gated CI Path Map & `vox ci check-changed` (`T3-CI-CHANGED`)

- [x] **T3-CI-CHANGED-01**: Port t3code's affected-path decision table (INERT paths + EXTRA_OWNERS / package ownership map) into a Vox contract (`contracts/ci/changed-path-owners.v1.yaml` or equivalent) covering Rust crates, `crates/vox-gui/ui`, workflows, and contracts.
- [x] **T3-CI-CHANGED-02**: Ship `vox ci check-changed` (or `scripts/check-changed.vox`) that diffs against merge-base with `origin/main`, runs only the fmt/lint/typecheck/tests mapped to changed owners, and caches a content-addressed pass key like t3code `check:changed`.
- [x] **T3-CI-CHANGED-03**: Wire change-gating into GitHub PR `linux`/`ui` jobs so inert doc-only / planning-only diffs skip heavy cargo/playwright work without weakening required contexts.
- [x] **T3-CI-CHANGED-04**: Document the owner map + local invocation in `docs/src/ci/runner-contract.md` and add a unit/integration fixture proving an inert path set exits 0 with zero cargo invocations.

### 2. Phase 302: Transfer-Budget Artifacts, PR Comments & Weighted Test Sharding (`T3-CI-BUDGET`)

- [x] **T3-CI-BUDGET-01**: Emit a machine-readable transfer/turn-budget CI artifact from existing `vox-orchestrator-mcp` transfer-budget tests (promote beyond unit-only) comparable to t3code `thread-transfer-results`.
- [x] **T3-CI-BUDGET-02**: Add a trusted-default-branch PR comment publisher (workflow_run pattern from t3code `thread-transfer-report.yml`) that never executes PR code when posting budget deltas vs main baseline.
- [x] **T3-CI-BUDGET-03**: Introduce weighted duration test sharding (seeded weights file akin to t3code `shardWeights.json`) for nextest/`vox-gui` e2e partitions so slow suites stop dominating one shard.
- [x] **T3-CI-BUDGET-04**: Publish before/after shard-balance + budget-delta report under `contracts/reports/` and gate regressions with a ratchet.

### 3. Phase 303: Knip Dual Gate, PR Size Labels & Evidence-Asset Ban (`T3-CI-HYGIENE`)

- [x] **T3-CI-HYGIENE-01**: Add knip (or equivalent unused-export) dual gate for `crates/vox-gui/ui` with workspace schema mirroring t3code `knip.jsonc` (exported types off, entry allowlist, ignoreBinaries).
- [x] **T3-CI-HYGIENE-02**: Ship PR size labeler (XS–XXL) that counts effective non-test LOC (exclude `**/*.{test,spec}.*`, `**/tests/**`, e2e fixtures) via `pull_request_target` + managed labels.
- [x] **T3-CI-HYGIENE-03**: Ban committing PR evidence / screenshot / review-bundle binary assets outside approved artifact paths; enforce via `vox ci` detector or git attribute + CI check (t3code pattern: no screenshot spam in PRs).
- [x] **T3-CI-HYGIENE-04**: Add CONTRIBUTING/AGENTS guidance for size labels + evidence paths; verify with intentional mutation tests that each gate fails closed.

### 4. Phase 304: Agent-Ops Kill-Safety, Hit-Every-Surface Checklist & Worktree Isolation (`T3-AGENT-OPS`)

- [x] **T3-AGENT-OPS-01**: Codify t3code's kill-safety triad in `AGENTS.md` / `docs/agents/governance.md`: never pattern-kill; never write the live userdata/home; never bake localhost origins into bundles — with Vox-specific paths (`VOX_*` homes, Tauri/Vite proxies).
- [x] **T3-AGENT-OPS-02**: Add a mandatory "hit every surface" checklist for harness/GUI changes (entry points, clients, providers/adapters, MCP/agent reachability, contracts, reverse states, connection modes, docs) and a `vox ci`/`vox doctor` advisory that surfaces when a PR touches one surface class without acknowledging others.
- [x] **T3-AGENT-OPS-03**: Worktree setup markers + isolated home (t3code worktree setup script pattern): each git worktree gets a stamped setup marker and isolated config/home so agents cannot clobber the primary install.
- [x] **T3-AGENT-OPS-04**: Suppression / `# toestub-ignore` / lint-disable PRs require human review label or CODEOWNERS gate (t3code `require-suppression-reason` + vouch culture) before merge.

### 5. Phase 305: TOESTUB Thin-Transport, Suppression Review Gate & Snapshot Interning (`T3-TOESTUB-ENF`)

- [x] **T3-TOESTUB-ENF-01**: New `vox-code-audit` detector for thin-transport / no-rpc-permission-bypass beyond MCP (t3code `no-rpc-permission-bypass`): raw RPC/client guard installs only in allowlisted boundary files.
- [x] **T3-TOESTUB-ENF-02**: Extend suppression detectors so every ignore carries a linked reason + issue, and CI fails on unreasoned suppressions introduced in the PR diff (ratchet on Wave-3 `unreasoned_suppression`).
- [x] **T3-TOESTUB-ENF-03**: Wire snapshot / message-ID interning leftover from Wave 2 into the live harness path with tests proving duplicate IDs collapse.
- [x] **T3-TOESTUB-ENF-04**: Map remaining high-value oxlint-plugin-t3code rules not yet covered (`no-low-value-comments`, `no-frame-waits-in-browser-tests`, `no-polling-loop-in-tests`, `require-disposable-render` analogs) into TOESTUB rules or explicit Out-of-Scope ledger rows with owners.

### 6. Phase 306: Tauri Fingerprint Advisory, GUI Cache Layers, Surface Graph Refresh & Scorecard (`T3-CI-CLOSE`)

- [x] **T3-CI-CLOSE-01**: Advisory Tauri/native fingerprint (or `crates/vox-gui` sidecar/native input hash) PR label when a change would break OTA/sidecar reuse — modeled on t3code `mobile-fingerprint-check.yml` (always green; label is the signal).
- [x] **T3-CI-CLOSE-02**: Layer Vite+/pnpm caches for GUI CI jobs under the main-only save policy (`vox ci cache-key-lint` compliant) so `ui` jobs restore node+vite artifacts without poisoning PR cache scope.
- [x] **T3-CI-CLOSE-03**: Repair `scripts/t3-surface-inventory.vox` compile blockers; regenerate `contracts/reports/t3-ecosystem-surface-graph.v1.json` from current `~/dev/{t3code,t3router,pi-t3chat,thom-chat,helioschat}` SHAs.
- [x] **T3-CI-CLOSE-04**: Publish `contracts/reports/t3-vox-ci-enforcement-scorecard.v1.json` + architecture note listing each gem as Shipped / Partial / Deferred with file anchors; update extraction ledger without duplicating v3.9 runtime rows.

## Out of Scope (v5.4)

| Feature | Reason |
|---------|--------|
| Re-implementing `ModelOperationalStatus` / resumable streams / lazy MCP / prompt enhance | Already shipped in v3.9 Phases 208–209 |
| Re-landing Wave 2–5 runtime domain engines | Shipped through v5.1; this milestone is CI/policy only |
| Porting Electron/Expo mobile surfaces from t3code | ADR-045 locks Tauri 2; fingerprint advisory is the adapted gem |
| Full 29-rule oxlint-plugin verbatim | Map high-value rules into TOESTUB; Effect-TS-specific rules stay deferred |
| Replacing nextest with t3code vitest sharding wholesale | Adopt weighted partition idea inside existing nextest/`ui` jobs |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| `T3-CI-CHANGED-01..04` | Phase 301 | Complete |
| `T3-CI-BUDGET-01..04` | Phase 302 | Complete |
| `T3-CI-HYGIENE-01..04` | Phase 303 | Complete |
| `T3-AGENT-OPS-01..04` | Phase 304 | Complete |
| `T3-TOESTUB-ENF-01..04` | Phase 305 | Complete |
| `T3-CI-CLOSE-01..04` | Phase 306 | Complete |

**Coverage:** 24 v5.4 requirements → 6 phases → 0 unmapped

---


# Requirements: Vox Milestones v5.4 (`Phases 301–306`), v5.3 (`Phases 295–300`), v5.2 (`Phases 289–294`), v5.1 (`Phases 283–288`), v5.0 (`Phases 277–282`), v4.9 (`Phases 271–276`) & v4.8 (`Phases 265–270`)

**Defined:** 2026-10-25
**Core Value (`v5.3` — Photorealistic Edge-Softening, Continuous Monumental Aqueduct-to-Sewer Architecture, Sub-Surface Ground Integration & CV Flywheel Score Maximization):** Eliminate sharp diamond coastal cutouts, saw-tooth 2D wall crenellations, overlapping street alpha seams, screen-space rooftop-crossing lines, and 1px PBR Laplacian pixel chatter by introducing feathered radial coastal shoals, smooth macro-shaded PBR stone/marble/water synthesis, true isometric molded curtain walls, continuous 3D `Aqua Claudia` aqueduct bridges, beveled ground-plane hydraulic/sewer conduits (`quadraticCurveTo` elbow fillets), halo-backed geometric activity particles, and positive CV noise reduction (`noiseFloor <= 5.0`, `visualAfterScore >= 0.945`).
**Architecture Spec (`v5.3`):** [`docs/src/architecture/urbs-photorealistic-edge-softening-and-cv-maximization-spec-2026.md`](../docs/src/architecture/urbs-photorealistic-edge-softening-and-cv-maximization-spec-2026.md)
**Core Value (`v5.2` — Smooth Physically-Grounded Lighting, High-Detail Vitruvian Architecture & Grounds, Integrated Roman Hydraulic/Sewer Grid & Animated Shape+Color Activity Conduits):** Eliminate alpha-overlay lighting artifacts and repeating water tile seams, enrich terrace ground architecture (`Cardo`/`Decumanus` street grid, `Opus Sectile` marble forum plazas, vaulted `Substructio` cliff retaining arcades) and `T0`–`T3` building massing (rusticated ground-floor `Fornices` arcades, inter-story `Marcapiano` belt cornices, atrium `Impluvium` rainwater basins, and rooftop `Acroteria` + hypocaust chimneys), integrate the `AQVAE MCP` aqueduct into a city-wide SimCity-style Roman hydraulic supply (`Castellum Divisorium`, pressurized `Fistulae` pipes, bronze sluice valves) and `Cloaca Maxima` sewer/drainage trench network (`hydraulicNetwork.ts`), and animate live activities in progress using multi-channel Color + Geometric Shape flow tokens.
**Architecture Spec (`v5.2`):** [`docs/src/architecture/urbs-hydraulic-plumbing-and-detailed-architecture-spec-2026.md`](../docs/src/architecture/urbs-hydraulic-plumbing-and-detailed-architecture-spec-2026.md)
**Core Value (`v5.1` — T3 Ecosystem Wave 5 Deep Review & End-to-End Parity Closure):** Eliminate every remaining split-brain bug, un-wired domain engine, synthetic fallback, and static-analysis violation uncovered by the Wave 5 adversarial review of `vox-orchestrator`, `vox-orchestrator-mcp`, `vox-code-audit`, and `vox-gui` against the six T3 reference repositories (`t3code`, `t3router`, `pi-t3chat`, `NotT3Chat`, `t3-chat-tomdoestech`, and `t3-cloneathon`).
**Architecture Spec (`v5.1`):** [`docs/src/architecture/t3-vox-wave5-deep-review-and-parity-closure-2026.md`](../docs/src/architecture/t3-vox-wave5-deep-review-and-parity-closure-2026.md)
**Core Value (`v5.0` — Photorealistic Vitruvian Material, Depth-Shading, Atmospheric Lighting & FX Engine):** Transform `VRBS` (`worldRenderer.ts`, `pbrMaterials.ts`, `lightingAndFx.ts`) from flat vector polygons with cartoon strokes into a photorealistic 2.5D Vitruvian architectural diorama featuring procedural natural stone/terracotta/bronze surface textures, projected directional twilight cast shadows, vertical ambient occlusion gradients, specular chamfer highlights, bathymetric coastal water + surf foam, crepuscular god-rays, elevation mist, additive brazier/forge GI pools, live ember/smoke/spray particle FX, and diegetic cliff-apron architrave plaques.
**Architecture Spec (`v5.0`):** [`docs/src/architecture/urbs-photorealistic-lighting-and-materials-spec-2026.md`](../docs/src/architecture/urbs-photorealistic-lighting-and-materials-spec-2026.md)
**Core Value (`v4.9` — Vitruvian Roman Tactical Developer Controller):** Elevate `VRBS` from an architectural observability view into a zero-hop interactive tactical developer controller across 6 high-leverage developer upgrades (*Speculum Codicis* Inline Diff/Diagnostic Loupe, *Concursus Legionum* Multi-Agent Collision Cone Radar, *Pons Foederis* Clickable Viaduct Contract & Layer-Inversion Inspector, *Clausura Ferrea* Hard Agent Scope Quarantine Fencing in `agent_loop.rs`, *Via Temporis* Interactive Milestone Road Turn Time-Scrubber, and *Tabula Praetoria `⌘M`* Split-Dock Tactical Map Workbench Integration) with zero paper code, zero mock fallbacks, and full Tauri/MCP/Git plumbing wiring.
**Architecture Spec (`v4.9`):** [`docs/src/architecture/urbs-tactical-developer-controller-spec-2026.md`](../docs/src/architecture/urbs-tactical-developer-controller-spec-2026.md)
**Core Value (`v4.8` — T3 Wave 4 Live Pipeline End-to-End Closure):** Ensure every T3 Wave 1–3 domain primitive (`salvage_and_stream`, `UserRuleRegistry`, `interpolate_system_prompt_variables`, `context_sanitizer`, `BatteryCostLedger`, `VoxIdAllocatorV2`, `ResponseQualityAnalyzer`, `CommandOutputHub`, `GitPlumbingCheckpointStore`, `ResumableTurnStreamRegistry`, `ThreadRunQueue`, `ConsensusOrchestrator`, `ContextHandoffService`, `GuestShareService`, `ExternalSessionImporter`, `WorkspaceLeaseMap`, `WebhookDedupeAndRateGate`, and `t3Wave3Rpc.ts`) is wired end-to-end on the live production execution path with zero dropped tool calls, zero bypassed guards, and zero silent mock fallbacks when connected to the Tauri/MCP backend.
**Architecture Spec (`v4.8`):** [`docs/src/architecture/t3-vox-wave4-gap-closure-and-live-pipeline-audit-2026.md`](../docs/src/architecture/t3-vox-wave4-gap-closure-and-live-pipeline-audit-2026.md)

---

## v5.3 Requirements (24/24 Complete Across Phases 295–300)

### 1. Phase 295: Feathered Coastal Bathymetry, Smooth Macro-Shaded PBR Materials & Laplacian Noise Suppression (`URBS-EDGE-PBR`)

- [x] **URBS-EDGE-PBR-01**: Replace the hard 4-vertex polygon coastal shelf and cliff shadow cutouts in `paintFullBleedRomanLandscape` (`worldRenderer.ts`) with multi-stop feathered elliptical radial gradients (`rgba(14, 116, 144, 0.34)` → `rgba(6, 182, 212, 0.14)` → `rgba(5, 16, 27, 0)`) and soft organic coastal foam crescents around cliff bases.
- [x] **URBS-EDGE-PBR-02**: Upgrade `buildTravertineTile`, `buildTerracottaTile`, `buildPlazaCobbleTile`, and `buildCliffTile` in `pbrMaterials.ts` to use smooth bilinear/radial macro-shaded stone blocks and soft mortar transitions instead of 1px high-contrast lines that inflate the 4-neighbor Laplacian `noiseFloor`.
- [x] **URBS-EDGE-PBR-03**: Add full-canvas diagonal isometric ocean swell and bathymetric light ribbons in `paintFullBleedRomanLandscape` (`worldRenderer.ts`) so the sea displays natural macro depth without 64px tile repetition or flat-region pixel chatter.
- [x] **URBS-EDGE-PBR-04**: Upgrade `drawIsoBox` and `drawIsoPitchedRoof` (`worldRenderer.ts`) to replace harsh `#1a1613` cartoon ink outlines with soft tonal masonry/terracotta edge bevels and warm sunlit ridge highlights.

### 2. Phase 296: True Isometric Molded Curtain Walls, Seamless `Via Silicea` Street Grid & Coffered `Opus Sectile` Forums (`URBS-TERRACE-CRAFT`)

- [x] **URBS-TERRACE-CRAFT-01**: Replace the jagged axis-aligned 2D `ctx.fillRect` merlons in `drawDistrictCurtainWall` (`worldRenderer.ts`) with true `2:1` isometric molded travertine parapet capstones (`Corona`), recessed arrow-loops (`Ballistaria`), and 3D pyramidal-roofed corner bastions.
- [x] **URBS-TERRACE-CRAFT-02**: Refactor `drawDistrictGroundDetails` (`worldRenderer.ts`) so the `Cardo Maximus` and `Decumanus Maximus` render as a unified recessed stone cross-plaza (`Via Silicea`) with zero overlapping alpha seams, continuous 3D travertine curbstones (`Crepido`), and central intersection drainage grates.
- [x] **URBS-TERRACE-CRAFT-03**: Upgrade the central district Forum in `drawDistrictGroundDetails` (`worldRenderer.ts`) with a recessed travertine border step, concentric porphyry/serpentine marble geometry, and 4 corner Corinthian peristyle columns (`Columnae`) casting soft ground shadows.
- [x] **URBS-TERRACE-CRAFT-04**: Upgrade the terrace cliff rim in `paintFullBleedRomanLandscape` (`worldRenderer.ts`) with a continuous 3D overhanging travertine cornice (`Geison`) that casts a soft drop shadow down the `Substructio` arcade wall.

### 3. Phase 297: Cylindrical & Coffered 3D Architectural Realism for Wonders & `T0`–`T3` Insulae (`URBS-ARCH-REALISM`)

- [x] **URBS-ARCH-REALISM-01**: Upgrade `drawIsoBedrockHarborVault` and `drawIsoPantheonAcropolis` (`worldRenderer.ts`) with smooth multi-stop cylindrical normal-shaded wall gradients, recessed coffered half-dome ribs (`Lacunaria`), and soft rim highlights.
- [x] **URBS-ARCH-REALISM-02**: Upgrade `drawIsoBox` windows (`worldRenderer.ts`) from flat 2D `fillRect` squares to isometric-skewed recessed stone window reveals with warm golden-hour interior radial glow on active/lit buildings.
- [x] **URBS-ARCH-REALISM-03**: Enrich `T2` Basilica and `T3` Temple buildings in `drawIsoFileBuilding` (`worldRenderer.ts`) with a 2-stepped stone stylobate base, shaded portico columns, and a recessed triangular pediment tympanum with a bronze relief boss.
- [x] **URBS-ARCH-REALISM-04**: Upgrade peripheral landmarks (`drawAqueductComplex`, `drawCastrumFortressComplex`, `drawPortusHarborComplex`, `drawIsoMilestoneRoadAndPortcullis` in `worldRenderer.ts`) to use 3D isometric masonry piers, arched stone harbor moles, and smooth water/torch reflections instead of flat 2D `fillRect` primitives.

### 4. Phase 298: Continuous 3D Monumental `Aqua Claudia` Viaduct & Ground-Integrated Beveled Hydraulic/Sewer Grid (`URBS-PLUMBING-3D`)

- [x] **URBS-PLUMBING-3D-01**: Upgrade `drawAqueductComplex` (`worldRenderer.ts`) and `drawRomanHydraulicInfrastructure` (`hydraulicNetwork.ts`) so `AQVAE MCP` forms a continuous 3D isometric stone arcade viaduct (`Aqua Claudia`) with arched piers, spandrel masonry, and an open illuminated water channel (`Specus`) connecting directly into the Acropolis `Castellum Divisorium`.
- [x] **URBS-PLUMBING-3D-02**: Upgrade `drawConduitTrack` in `hydraulicNetwork.ts` to render smooth rounded isometric bends (`quadraticCurveTo` elbow fillets), 3D cylindrical bronze/travertine pipe shading, and soft recessed ground shadow trenches.
- [x] **URBS-PLUMBING-3D-03**: Route blast-radius and dependency hydraulic/sewer conduits on the **terrace ground / viaduct layer beneath 3D buildings** (`Layer 0.85` in `WorldRenderer.paint`) instead of painting screen-space lines across building rooftops in `Layer 6`.
- [x] **URBS-PLUMBING-3D-04**: Render 3D stepped stone water cascades (`Cataracta`) where hydraulic conduits drop between `L3 → L2 → L1 → L0` elevation terraces, and vaulted stone `Cloaca Maxima` portals at cliff bases.

### 5. Phase 299: Illuminated Halo-Backed Shape+Color Activity Particles & Interactive Conduit Semiotics (`URBS-FLOW-SEMIOTICS`)

- [x] **URBS-FLOW-SEMIOTICS-01**: Enhance `drawChevronWaveParticle`, `drawDiamondIngotParticle`, `drawWarningTriangleParticle`, and `drawLaurelSealParticle` (`hydraulicNetwork.ts`) with soft radial glow halos and tapered comet tails for effortless scannability and smooth anti-aliased edges.
- [x] **URBS-FLOW-SEMIOTICS-02**: Scale conduit track gauge, inner stream luminance, and particle cadence in `hydraulicNetwork.ts` proportionally to live activity intensity (MCP server health, git diff churn lines, diagnostic error count, CI runner count).
- [x] **URBS-FLOW-SEMIOTICS-03**: Render glowing bronze manifold rings (`Epistomia`) at district street entry/exit ports in `drawScreenSpaceHydraulicOverlay` (`hydraulicNetwork.ts`) so developers can trace upstream provider and downstream blast-radius ports without lines crossing over building roofs.
- [x] **URBS-FLOW-SEMIOTICS-04**: Upgrade the top-left Roman Hydraulic & Sewer Semiotic Legend Pill in `drawScreenSpaceHydraulicOverlay` (`hydraulicNetwork.ts`) to draw crisp vector geometric shapes (`❯` Chevron, `◆` Diamond, `▲` Triangle, `◉` Laurel Seal) alongside live active channel counts.

### 6. Phase 300: Comprehensive Unit Tests, Semiotic Critique Audit & Playwright CV Flywheel Score Maximization (`URBS-CV-MAX`)

- [x] **URBS-CV-MAX-01**: Expand `pbrMaterials.test.ts`, `lightingAndFx.test.ts`, `hydraulicNetwork.test.ts`, and `worldRenderer.test.ts` to test rounded elbow fillets (`strokeRoundedPolyline`), sub-building ground conduit routing, and low-Laplacian PBR material synthesis (`computeRgbaFrameCvMetrics` `noiseFloor <= 6.5`).
- [x] **URBS-CV-MAX-02**: Tune macro lighting contrast and flat-region Laplacian smoothness so `urbs-civilization` achieves **positive `noiseReductionPct`** (`noiseFloor < 5.1`) and **`visualAfterScore >= 0.942`** in the `@ngi/platform-ssot` CV Flywheel.
- [x] **URBS-CV-MAX-03**: Verify all 32 registered GUI surfaces in `e2e/review/cv-flywheel.spec.ts` pass with `0` horizontal overflow, `0` overlapping controls, `0` unlit landmarks, and `0` serious/critical axe-core violations.
- [x] **URBS-CV-MAX-04**: Capture the verified `v5.3` production screenshot and update the visual semiotics & architecture blueprint carousel.

## v5.2 Requirements (24/24 Complete Across Phases 289–294)

### 1. Phase 289: Smooth Ground-Plane-Recessed Lighting, 2D-Feathered Elliptical Sun-Shafts & Periodic Seamless Sea Synthesis (`URBS-SMOOTH-LIGHT`)

- [x] **URBS-SMOOTH-LIGHT-01**: Upgrade `drawAdditiveLightPools` in `lightingAndFx.ts` and `WorldRenderer.paint` in `worldRenderer.ts` so warm brazier, forge, and active worksite bounce illumination is rendered on the terrace ground plane *before* 3D buildings and wonders are drawn (`drawGroundPlaneIllumination`), eliminating the milky alpha-overlay circles previously painted on top of rooftops.
- [x] **URBS-SMOOTH-LIGHT-02**: Upgrade `drawVolumetricGodRays` in `lightingAndFx.ts` to replace hard-edged 4-point polygon strips with rotated 2D-feathered elliptical radial gradients (`stop 1.0 = rgba(..., 0)` around the full `360°` perimeter) so crepuscular sun-shafts have zero hard lateral polygon boundaries.
- [x] **URBS-SMOOTH-LIGHT-03**: Remove the hard-edged solid `ctx.arc` district pulse circle in `WorldRenderer.drawAnimatedWorldEffects` (`worldRenderer.ts`) and replace it with soft, multi-stop radial ground-plane torch shimmer and warm emissive window/archway rim lighting.
- [x] **URBS-SMOOTH-LIGHT-04**: Upgrade `buildSeaWaterTile` in `pbrMaterials.ts` to use `2π`-periodic trigonometric wave interference (`Math.sin((x / size) * Math.PI * 2)`) and lower the sea pattern overlay alpha in `paintFullBleedRomanLandscape` so the `64×64` water texture tiles seamlessly across the Mediterranean sea with zero visible grid lines.

### 2. Phase 290: High-Detail Terrace Grounds: Orthogonal `Cardo`/`Decumanus` Streets, `Opus Sectile` Marble Plazas, Vaulted `Substructio` Cliff Arcades & Balustrades (`URBS-GROUNDS`)

- [x] **URBS-GROUNDS-01**: Implement `drawDistrictGroundDetails` in `worldRenderer.ts` rendering recessed basalt-paved orthogonal Roman thoroughfares (`Cardo Maximus` and `Decumanus Maximus`) with raised travertine curbstones (`Crepido`), wheel-rut grooves, and stepping-stone pedestrian crosswalks across every district terrace.
- [x] **URBS-GROUNDS-02**: Render an inlaid **`Opus Sectile` Geometric Marble Forum Plaza** around the central District Wonder of each terrace, featuring concentric travertine, porphyry, and verdigris serpentine borders with corner bronze brazier pedestals.
- [x] **URBS-GROUNDS-03**: Upgrade the cliff retaining walls in `paintFullBleedRomanLandscape` (`worldRenderer.ts`) with **Vaulted `Substructio` Blind Arcades, Projecting Ashlar Buttress Pilasters & Carved Stone Drainage Weep-Spouts** along both the SW shadowed and SE sunlit cliff faces.
- [x] **URBS-GROUNDS-04**: Add perimeter stone parapet balustrades, corner Roman boundary obelisks (`Termini`), and manicured cypress/olive planter beds (`Horti`) along terrace edges.

### 3. Phase 291: High-Detail Vitruvian Building Architecture (`T0`–`T3`): Rusticated Ground Arcades (`Fornices`), Belt Cornices, Atrium `Impluvia` & `Acroteria` Roofs (`URBS-ARCH-DETAIL`)

- [x] **URBS-ARCH-DETAIL-01**: Upgrade `drawIsoBox` and `drawIsoFileBuilding` (`worldRenderer.ts`) to support **Ground-Floor Rusticated Stone Arcades (`Fornices`) & Keystone Voussoirs** with warm interior workshop/shopfront lamplight on `T0` Tabernae and `T2` Insulae.
- [x] **URBS-ARCH-DETAIL-02**: Add projecting **Inter-Story Stringcourse Belt Cornices (`Marcapiano`)** and corner ashlar quoining to multi-story `T1`, `T2`, and `T3` buildings so stories are architecturally articulated.
- [x] **URBS-ARCH-DETAIL-03**: Upgrade `T1` Domus courtyard massing in `drawIsoFileBuilding` with a recessed marble **`Impluvium` Rainwater Basin** reflecting sky-cyan water inside the peristyle atrium courtyard, plus an arched prostyle entrance porch.
- [x] **URBS-ARCH-DETAIL-04**: Upgrade `drawIsoPitchedRoof` in `worldRenderer.ts` with sculpted gable **`Acroteria` Finial Ornaments**, stepped pediment tympanum mouldings, and terracotta brick hypocaust chimney vents.

### 4. Phase 292: Integrated Roman Hydraulic Plumbing & Sewer Network (`hydraulicNetwork.ts`): `Aquaeductus` Trunk, `Castellum Divisorium`, `Fistulae` Supply Pipes & `Cloaca Maxima` Sewer Trenches/Outfalls (`URBS-HYDRAULIC`)

- [x] **URBS-HYDRAULIC-01**: Create `crates/vox-gui/ui/src/components/gamify/urbs/hydraulicNetwork.ts` and connect the peripheral `AQVAE MCP` multi-arch aqueduct directly into an upper-terrace **`Castellum Divisorium` (Main Hydraulic Distribution Reservoir)** that branches raised stone `Specus` channels into the city's district network.
- [x] **URBS-HYDRAULIC-02**: Implement **Pressurized `Fistulae` Supply Conduits & Bronze Sluice Valves (`Epistomia`)** in `hydraulicNetwork.ts` that route isometric twin-rail bronze/lead pipes along terrace street curbs from district hydraulic headers to active file worksites and upstream/downstream crate viaducts.
- [x] **URBS-HYDRAULIC-03**: Implement **Recessed `Cloaca Maxima` Arched Stone Sewer Trenches & Bronze Inspection Grates (`Spiracula`)** in `hydraulicNetwork.ts` running along the `Cardo`/`Decumanus` street grid to carry build output, removed-code runoff, and diagnostic error backpressure.
- [x] **URBS-HYDRAULIC-04**: Render **Cliff-Wall `Cloaca Maxima` Arched Sewer Outfall Portals & Stepped Sluice Cascades (`Cataracta`)** at the base of coastal district cliffs, discharging animated water or crimson error runoff plumes into the sea.

### 5. Phase 293: Multi-Channel Color + Geometric Shape Animated Activity Conduits (`AQUA PURA` Chevrons, `IGNIS OPERIS` Diamonds, `CLOACA FAULT` Triangles & `VIA VICTORIAE` Seals) (`URBS-CONDUIT-ANIM`)

- [x] **URBS-CONDUIT-ANIM-01**: Implement `drawAnimatedHydraulicActivityFlows` in `hydraulicNetwork.ts` and wire into `WorldRenderer.drawAnimatedWorldEffects` (`worldRenderer.ts`), encoding **`AQUA PURA` (MCP & Upstream Provider Supply)** as Crystalline Cyan (`#38bdf8` / `#e0f2fe`) **Chevron Wave-Crests (`❯❯`) & Water Droplets** streaming along aqueduct and `Fistulae` supply channels.
- [x] **URBS-CONDUIT-ANIM-02**: Encode **`IGNIS OPERIS` (Active Agent Work-in-Progress & Token Compute Stream)** as Imperial Molten Gold (`#f59e0b` / `#fef08a`) **Articulated Diamond / Rhombus Ingot Packets (`◆◆`)** pulsing along riveted bronze hypocaust conduits from active `LEGIO` agents and `PRAETORIVM` directly to active file worksite scaffolds (`parser.rs`, `select.rs`, etc.).
- [x] **URBS-CONDUIT-ANIM-03**: Encode **`CLOACA FAULT` (Diagnostic Error Backpressure & Blast-Radius Spillway)** as Crimson Vermilion (`#ef4444` / `#fca5a5`) **Spiked Warning Triangles (`▲`) & Counter-Rotating Backpressure Eddies (`◎`)** surging along recessed `Cloaca` sewer trenches between failing worksites (`errors > 0`) and downstream impacted districts (`⚡ BLAST RADIUS`).
- [x] **URBS-CONDUIT-ANIM-04**: Encode **`VIA VICTORIAE` (Verified CI & Git Checkpoint Flow)** as Emerald Verdigris (`#10b981` / `#a7f3d0`) **Circular Laurel Medallions (`◉`)** marching along inlaid marble/brass conduits toward `CASTRVM CI` and `PORTA GIT`.

### 6. Phase 294: Diegetic Hydraulic Sluice-Gate Callouts, Semiotic Consistency Audit, Unit Test Suite & Playwright CV Flywheel Convergence (`URBS-SEMIOTICS-CV`)

- [x] **URBS-SEMIOTICS-CV-01**: Replace the ungrounded screen-space straight dependency lines and giant translucent yellow arrow in `drawScreenSpaceOverlays` (`worldRenderer.ts`) with diegetic **Roman Hydraulic Sluice-Gate (`Cataracta`) & Isometric Conduit Routing** so all connection lines follow isometric street/viaduct/plumbing geometry and maintain 100% visual/action semiotic consistency.
- [x] **URBS-SEMIOTICS-CV-02**: Add a Hydraulic/Plumbing flow legend pill or indicator in `drawScreenSpaceOverlays` / `LudusSandbox.tsx` showing the 4 shape+color conduit channels (`❯ AQUA MCP/Supply`, `◆ IGNIS Agent WIP`, `▲ CLOACA Error Fault`, `◉ VICTORIA Verified CI`) for instant developer scannability.
- [x] **URBS-SEMIOTICS-CV-03**: Create `crates/vox-gui/ui/src/components/gamify/urbs/__tests__/hydraulicNetwork.test.ts` and expand `pbrMaterials.test.ts` and `lightingAndFx.test.ts` to verify seamless periodic sea synthesis, ground-plane illumination, `Aquaeductus`/`Fistulae`/`Cloaca Maxima` network routing, and all 4 shape+color animated activity conduits.
- [x] **URBS-SEMIOTICS-CV-04**: Run end-to-end verification across TypeScript (`tsc --noEmit`), Vitest (`vitest run src/components/gamify`), and the Playwright `@ngi/platform-ssot` CV Flywheel (`urbs-civilization` & `chat-dock-multi`), verifying `visualAfterScore >= 0.94` and `0` regressions.

## v5.1 Requirements (24/24 Complete Across Phases 283–288)

### 1. Phase 283: System Prompt `$variable` Target Fix, Sanitizer Tag Completeness, `ToolRefusalDetector` & Lazy MCP Meta-Tools (`T3-PROMPT-LOOP`)

- [x] **T3-PROMPT-LOOP-01**: Fix `interpolate_system_prompt_variables` wiring in `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` and `preamble_and_retrieval.rs` so `$workspace_root`, `$git_branch`, `$session_id`, `$current_datetime` (full ISO-8601 UTC `YYYY-MM-DDTHH:MM:SSZ`), `$active_file`, `$model_id`, and `$cognitive_profile` are interpolated on `system_prompt` (as well as custom rule blocks), and upgrade `detect_workspace_git_branch` in `system_prompt_vars.rs` to follow `gitdir:` pointers when `.git` is a regular worktree file.
- [x] **T3-PROMPT-LOOP-02**: Expand `escape_user_prompt_special_tokens` in `crates/vox-orchestrator/src/context_sanitizer.rs` to escape `<tool_call>`, `</tool_call>`, `<tool_response>`, `</tool_response>`, `<think>`, and `</think>` in addition to `<|im_start|>`, `<|im_end|>`, `<|endoftext|>`, and `<|system|>`; expand `compact_historical_web_search` to also compact `<WebFetch>...</WebFetch>`; apply `escape_user_prompt_special_tokens` to the active turn's `turn_user_prompt` in `agent_loop.rs` and `message.rs`; and persist `parse_special_entity_tags(&response_text).extracted_reasoning` into `ChatTranscriptEntry::thought_signature` in `persist_and_envelope.rs`.
- [x] **T3-PROMPT-LOOP-03**: Wire `pi-t3chat`-style `ToolRefusalDetector` (`should_nudge_tool_use`: matching capability-refusal phrases when `tools` is non-empty, injecting `[SYSTEM TOOL CORRECTION]` up to `max_corrections = 2`) and `NotT3Chat`'s per-turn search circuit breaker (stripping `vox_search_query` / `web_search` / `vox_kb_search` from available tools once `total_searches >= 3` in a turn) into `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`.
- [x] **T3-PROMPT-LOOP-04**: Wire `sanitize_mcp_tool_search_env` into child MCP process spawning in `crates/vox-orchestrator-mcp/src/external_mcp.rs` (removing hardcoded `ENABLE_TOOL_SEARCH=true`), store a shared `LazyMcpGroupRegistry` on `ServerState`, and expose `vox_list_mcp_servers`, `vox_list_mcp_tools`, and `vox_call_mcp_lazy` in `tool_registry.rs`, `input_schemas.rs`, and `dispatch.rs`.

### 2. Phase 284: Sequenced `ResumableTurnStreamRegistry`, Full `ThreadRunQueue` Lifecycle, Real `ModelSpec` Cost Ledger & `SessionPrivacyPosture` Gate (`T3-STREAM-QUEUE`)

- [x] **T3-STREAM-QUEUE-01**: Replace the primitive `HashMap<String, (String, bool)>` on `ServerState::resumable_streams` with `Arc<PrMutex<vox_orchestrator::models::spec::ResumableTurnStreamRegistry>>`, call `start_turn` once at turn entry (not wiping previous iterations inside `stream_final_answer`), record `TextDelta`, `ReasoningDelta`, `ToolCallStart`, and `Finish` chunks across both `agent_loop` and direct streaming turns, and upgrade `vox_chat_stream_resume` (`after_seq` + `char_offset`) plus add `vox_chat_stream_cancel` (`cancel_turn`) in `history.rs` and `dispatch.rs`.
- [x] **T3-STREAM-QUEUE-02**: Wire the full `ThreadRunQueue` lifecycle in `vox-orchestrator-mcp`: call `register_background_work` / `complete_background_work` around background task/subagent dispatches in `batch_and_eval_gate.rs`, arm `set_limit_recovery` on 429/rate-limit errors, and call `pop_next_follow_up` when a turn settles in `persist_and_envelope.rs` (exposing `next_queued_follow_up` in the turn response envelope).
- [x] **T3-STREAM-QUEUE-03**: Move `BatteryAndCostLedger::check_preflight_balance` after model resolution in `message.rs` (so `"auto"` resolving to a local MENS/Ollama model is recognized as zero-cost), wire `ModelRegistry::compute_turn_cost_delta` (`ModelSpec` pricing, cache read discounts, BYOK, and local/free flags) into `persist_and_envelope.rs`, add rolling 4-hour burst + monthly quota window metrics (`four_hour_pct`, `monthly_pct`, `four_hour_reset_ms`) to `BatteryAndCostLedger`, and expose `vox_battery_status` in `dispatch.rs`.
- [x] **T3-STREAM-QUEUE-04**: Enforce `SessionPrivacyPosture::allows_disk_persistence()` in `persist_and_envelope.rs`: when `privacy_posture == SessionPrivacyPosture::Ephemeral`, skip writing conversation messages, workspace turns, telemetry events, and long-term agent memories to `state.db` (`VoxDb`), keeping the turn in-memory only and reporting `persisted_to_disk: false` in `posture_audit`.

### 3. Phase 285: Worktree Async Lease Mutex Fix, Persistent Webhook Gate, Monotonic Sparse Git Checkpoints, Real `MultiVariantWorktreePool` & Compact Snapshot Fix (`T3-WORKTREE-CKPT`)

- [x] **T3-WORKTREE-CKPT-01**: Upgrade `WorkspaceLeaseMap` in `crates/vox-orchestrator/src/workspace_lease.rs` and `DelegationWorktree::create` in `crates/vox-orchestrator-mcp/src/agy_worktree.rs` to hold a `tokio::sync::MutexGuard` across `.await` during `git worktree add` and `provision_untracked_env_files`; and move `WebhookDedupeAndRateGate` onto `ServerState` (`webhook_gate: Arc<PrMutex<WebhookDedupeAndRateGate>>`) so `submit_webhook_events` in `webhook_intake.rs` preserves its 5-minute TTL dedupe cache and 60/min rate window across 250ms poll ticks (wiring `verify_webhook_signature` and `matches_webhook_json_filter`).
- [x] **T3-WORKTREE-CKPT-02**: Fix sparse Git checkpointing in `batch_and_eval_gate.rs` and `checkpoint_engine.rs`: track a monotonic per-session checkpoint counter on `ServerState` (`checkpoint_turn_counters`) so turns never overwrite `turn/1`, sanitize `:`, `\`, `/`, and `%` in `temp_index` paths for Windows NTFS safety, pass `-p HEAD` (when resolvable) to `git commit-tree`, clean newly created post-checkpoint files during `restore_checkpoint`, and wire `GitPlumbingCheckpointStore::diff_checkpoints` into `handle_vcs_checkpoint_diff_summary`.
- [x] **T3-WORKTREE-CKPT-03**: Persist `muse_pools: Arc<PrMutex<HashMap<String, MultiVariantWorktreePool>>>` on `ServerState`, upgrade `handle_muse_pool_spawn` (`t3_wave3_tools.rs`) to materialize real linked worktrees on disk (`git worktree add --detach` + `provision_untracked_env_files` under `.vox/worktrees/muse/<thread_id>/<slot_id>`), and upgrade `handle_muse_pool_adopt` to apply/record the adopted variant and remove pruned variant worktrees from disk.
- [x] **T3-WORKTREE-CKPT-04**: Fix `orchestrator_status_compact` in `crates/vox-orchestrator-mcp/src/dei_tools/orchestrator_snapshot.rs` and `dispatch.rs` so it compresses the inner status data (eliminating the double `ToolResult` wrapper), filters idle subagents via `parent_agent_id.is_some()` in addition to name heuristics, and wires `LiveBodyDeduper` into `compress_wire_snapshot`.

### 4. Phase 286: UTF-8 Boundary Safety, Multi-Provider Consensus Synthesizer, Recursive Session Discovery & Real Guest Share / Diagnostics (`T3-TOOLS-HARDENING`)

- [x] **T3-TOOLS-HARDENING-01**: Fix the UTF-8 byte-slice panics in `crates/vox-orchestrator/src/context_handoff.rs` (`build_handoff_payload`) and `crates/vox-orchestrator-mcp/src/mcp_tool_access.rs` (`bound_snapshot_metadata`) using char-boundary-safe truncation (`floor_char_boundary`), upgrade `HandoffStrategy::Summarized` to produce a structured digest of earlier turns prior to the verbatim tail, and assign a `VoxIdAllocatorV2` `message_id` to persisted handoff entries.
- [x] **T3-TOOLS-HARDENING-02**: Upgrade `handle_consensus_run` (`t3_wave3_tools.rs`) and `ConsensusOrchestrator` (`crates/vox-orchestrator/src/models/consensus.rs`) to check any configured provider credential (`OpenRouterApiKey`, `GeminiApiKey`, `AnthropicApiKey`, `OpenAiApiKey`, `GroqApiKey`, or local Ollama/Populi endpoint) and run a real consensus synthesis pass (`synthesize_consensus_response`) combining insights across all completed legs.
- [x] **T3-TOOLS-HARDENING-03**: Upgrade `discover_latest_external_jsonl` in `t3_wave3_tools.rs` to recursively walk up to depth 4 under `~/.claude/projects`, `~/.codex/sessions`, and `~/.cursor/chats` (and expose `vox_session_list_external`), assign `VoxIdAllocatorV2` `message_id`s to all imported/guest turns, remove the synthetic fake-invite block from `handle_guest_share_moderate`, and expose `vox_guest_share_redeem` and `vox_guest_share_submit` with wall-clock `now_ms`.
- [x] **T3-TOOLS-HARDENING-04**: Harden `authorize_wave3_call` (`t3_wave3_tools.rs`) so caller args can only narrow (never escalate) `ServerState`'s access mode, persist `PullRequestReviewGuideService` on `ServerState`, and wire real `vox-code-audit` / observer diagnostics for `rel_path` into `handle_file_diff_and_diagnostics` (replacing the hardcoded `"diagnostics": []`).

### 5. Phase 287: Unified `message_id` & Telemetry Across Tauri/MCP, Live Transcript Actions, Mid-Turn Steer Composer & `streamWatchdog.ts` (`T3-GUI-PIPELINE`)

- [x] **T3-GUI-PIPELINE-01**: Preserve `backend_message_id` (`usr:...` / `ast:...`), `battery_spent_bu`, `remaining_balance_bu`, `quality_score`, `flesch_kincaid_grade`, `compaction_audit`, `posture_audit`, and `next_queued_follow_up` in `parse_chat_message_envelope` (`crates/vox-gui/src/commands/chat.rs` & `chat_turn.rs`), and upgrade `history.rs` (`chat_edit_message_in_place`, `chat_regenerate`, `chat_compact_up_to`) to match by `message_id`, `id`, or turn-index fallback while syncing mutations to `state.db` and restoring the turn's Git checkpoint on `chat_regenerate`.
- [x] **T3-GUI-PIPELINE-02**: Upgrade `ChatTranscript.tsx` and `App.tsx`: enable `✎ Edit` on both user and assistant messages, wire `↻ Regenerate` with `execute: true` so it updates `sessionChatStore` with the regenerated turn, add a `✂ Compact Here` action button calling `compactChatUpTo`, wire `/compact` in `handleLoquelaSlash` (`App.tsx`), and wire real forked-session comparison in `handleOpenBranchComparison`.
- [x] **T3-GUI-PIPELINE-03**: Fix the `Enter`-key task-abort bug in `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx` (`handleKeyDown` when `taskInProgress === true`), render `[⚡ Steer Turn]` (`steerChatQueue(..., promoteToSteer: true)`) and `[+ Queue Next]` (`steerChatQueue(..., promoteToSteer: false)`) when a turn/task is in flight, and auto-dispatch `next_queued_follow_up` on turn completion.
- [x] **T3-GUI-PIPELINE-04**: Create `crates/vox-gui/ui/src/lib/chat/streamWatchdog.ts` (`FIRST_EVENT_TIMEOUT_MS = 30_000`, `STREAM_IDLE_TIMEOUT_MS = 120_000`) and wire `resumeChatStream` (`vox_chat_stream_resume`) on session hydration/reconnect; pass `compactionAudit`, `postureAudit`, `batterySpentBu`, and `qualityScore` to `ChatExecutionRail.tsx` and `ContextWindowMeter.tsx`; wire `vcsCheckpointDiffSummary` in `ChatExecutionRail.tsx`; pass `messages`, `onSessionImported`, `sessionImportBundle`, and `sessionListExternal` in `SessionInteropModal.tsx`; wire `vcsMusePoolAdopt` in `ConsensusGridView.tsx`; and add `guestShareMint` / `guestShareSubmit` / `guestShareModerate` in `t3Wave3Rpc.ts`.

### 6. Phase 288: `vox-code-audit` T3 Detector Catalog Registration, `oxlint-plugin-t3code` Rule Parity & 43-Violation Workspace Remediation (`T3-AUDIT-PARITY`)

- [x] **T3-AUDIT-PARITY-01**: Register `vox/lint/unreasoned-suppression`, `vox/test/flaky-wait-and-unreasoned-skip`, `vox/mcp/access-and-error-safety`, and `vox/ui/layout-and-perf-hazards` in `crates/vox-code-audit/src/diagnostics/catalog.rs` (`ALL_KNOWN_IDS`) and populate `diagnostic_id: Some(...)` across all four detector implementations.
- [x] **T3-AUDIT-PARITY-02**: Upgrade `unreasoned_suppression.rs` (adding `.vox` support, Rust `#[expect(`, `// toestub-ignore(...) -- <reason>` enforcement, preceding-line comment exemption, and standard `// @ts-expect-error <explanation>` support) and `test_clock_and_skip.rs` (scanning inline `#[cfg(test)]` modules, unreasoned `.skip(`/`.todo(`/`xit(`, sleep-polling loops inside `while`/`loop`/`for`, and `requestAnimationFrame` waits).
- [x] **T3-AUDIT-PARITY-03**: Upgrade `mcp_access_and_error_safety.rs` (checking `ToolResult::err` with `:?` and thick-handler `Command::new` calls inside MCP tool handler files) and `ui_perf_hazards.rs` (adding `.css` `:has(` and `scrollbar-gutter: stable` + `mx-auto` without `both-edges`), and refactor the 4 raw `std::process::Command::new` sites in `t3_wave3_tools.rs` to use `GitPlumbingCheckpointStore` / `GitExec`.
- [x] **T3-AUDIT-PARITY-04**: Remediate all 43 live workspace violations across `crates/vox-gui/ui/src/`, `crates/vox-cli/`, and `crates/vox-orchestrator-mcp/` (15 unreasoned TS lint suppressions, 4 bare `toestub-ignore` comments, 1 `error = ?e` tracing macro in `harness_issue_judge.rs:137`, 21 `transition-all` classes, and 5 `new ResizeObserver(` sites) and verify `0` findings across `vox-code-audit`, `vox-arch-check`, and `vox-doc-pipeline`.

---

## v5.0 Requirements (24/24 Complete Across Phases 277–282)

### 1. Phase 277: Procedural Natural Surface Material Synthesis (`pbrMaterials.ts`) (`URBS-MAT`)

- [x] **URBS-MAT-01**: Create `crates/vox-gui/ui/src/components/gamify/urbs/pbrMaterials.ts` implementing a deterministic offscreen procedural material synthesis engine (`PbrMaterialAtlas`) that generates cached tileable `CanvasPattern`s for **Pitted Roman Travertine Ashlar (`travertineLit`, `travertineShade`)** with multi-octave value noise, sedimentary pore pitting, staggered ashlar stone blocks, and damp basal moss staining.
- [x] **URBS-MAT-02**: Synthesize **Hand-Molded Roman Terracotta Imbrex & Tegula Roof Tile Patterns (`terracottaLit`, `terracottaShade`)** and **Opus Incertum / Basalt Cobblestone Plaza Patterns (`plazaCobble`, `plazaMarble`)** in `pbrMaterials.ts` with scalloped tile overlap shadows, recessed mortar grooves, and warm stone-to-stone value variation.
- [x] **URBS-MAT-03**: Synthesize **Stratified Craggy Basalt Cliff Patterns (`cliffLit`, `cliffShade`)**, **Weathered Oak Timber Grain (`oakTimber`)**, and **Anisotropic Hammered Bronze / Forged Iron Metallic Shaders (`createBronzeGradient`, `createIronGradient`)** in `pbrMaterials.ts`.
- [x] **URBS-MAT-04**: Add Vitest unit tests in `crates/vox-gui/ui/src/components/gamify/urbs/__tests__/pbrMaterials.test.ts` verifying deterministic procedural noise, pattern synthesis caching, and metallic/surface shader helpers.

### 2. Phase 278: True Isometric Directional Cast Shadows, Contact Ambient Occlusion & Specular Chamfer Bevels (`URBS-DEPTH`)

- [x] **URBS-DEPTH-01**: Implement `drawIsoCastShadow(ctx, px, py, u0, v0, z0, u1, v1, z1)` in `worldRenderer.ts` that projects a soft multi-layer directional twilight shadow polygon onto the terrace plane (`z = z0`) cast by a South-East (`+u, -v`, `36°` altitude) golden-hour sun toward North-West (`-u, +v`).
- [x] **URBS-DEPTH-02**: Upgrade `drawIsoBox` in `worldRenderer.ts` to replace flat solid hex fills with vertical height-shaded `CanvasLinearGradient`s (deep contact ambient occlusion at `z0` transitioning to warm golden-hour rim light at `z1` on SE faces and cool twilight indigo skylight on SW faces) overlaid with the `PbrMaterialAtlas` travertine ashlar texture.
- [x] **URBS-DEPTH-03**: Replace harsh `#120f0c` black cartoon outlines on `drawIsoBox`, `drawIsoPitchedRoof`, `drawIsoColonnade`, and the 8 Vitruvian Roman Wonder mechanisms with **physical specular chamfer highlights** (`1.1px` warm sunlit ivory/gold highlight along top/SE ridges + subtle dark mortar seam on recessed edges).
- [x] **URBS-DEPTH-04**: Add Vitest tests verifying directional cast shadow projection math, height-based gradient shading, and specular chamfer rendering.

### 3. Phase 279: Sculpted Coastal Bathymetry, Surf Foam Ribbons, Craggy Cliff Geology & Cobblestone Terraces (`URBS-TERRAIN`)

- [x] **URBS-TERRAIN-01**: Upgrade `paintFullBleedRomanLandscape` in `worldRenderer.ts` to replace the flat `#0a1019` sea rectangle with a multi-stop **Mediterranean Twilight Bathymetric Depth Field** (abyssal midnight `#040811` → deep Prussian `#091929` → coastal turquoise-teal shelf `#0e3147`) and subtle water wave texture.
- [x] **URBS-TERRAIN-02**: Render **Layered Coastal Surf Foam Ribbons & Cliff-Base Sea Reflections** around every district peninsula where rock cliffs meet the sea, plus cascading mist/foam pools at coastal waterfall bases.
- [x] **URBS-TERRAIN-03**: Upgrade district cliff faces and terrace plazas in `paintFullBleedRomanLandscape` to apply `cliffLit`/`cliffShade` geological strata patterns, vertical erosion fissures, mossy rock ledges, and `plazaCobble` flagstone paving with warm inner vignette bevels.
- [x] **URBS-TERRAIN-04**: Add Vitest tests verifying bathymetric terrain rendering and cliff/terrace material layering.

### 4. Phase 280: Golden-Hour & Twilight Dual-Key Atmospheric Lighting, Volumetric God-Rays, Elevation Mist & Additive GI Pools (`lightingAndFx.ts`) (`URBS-LIGHT`)

- [x] **URBS-LIGHT-01**: Create `crates/vox-gui/ui/src/components/gamify/urbs/lightingAndFx.ts` implementing **Inter-Terrace Atmospheric Depth Mist (`drawElevationAtmosphericMist`)** that renders soft aerial-perspective haze ribbons between `L0` harbor level and elevated `L1..L3` terraces.
- [x] **URBS-LIGHT-02**: Implement **Diagonal Crepuscular Volumetric God-Rays (`drawVolumetricGodRays`)** in `lightingAndFx.ts` casting soft golden-hour light shafts (`globalCompositeOperation = 'screen'`) from the upper-left/SE horizon across the Acropolis, Praetorium, and active worksite districts.
- [x] **URBS-LIGHT-03**: Implement **Additive Point-Light GI Pools (`drawAdditiveLightPools`)** and **Cinematic Viewport Vignette & Tone-Mapping (`drawCinematicPostProcess`)** in `lightingAndFx.ts` so every torch brazier, trip-hammer furnace, burning building, lit window, and cyan aqueduct channel casts warm/cyan bounce light onto surrounding stone.
- [x] **URBS-LIGHT-04**: Add Vitest unit tests in `crates/vox-gui/ui/src/components/gamify/urbs/__tests__/lightingAndFx.test.ts` verifying atmospheric mist, volumetric god-rays, additive GI pools, and post-processing passes.

### 5. Phase 281: Live Particle FX Engine (Forge Embers, Volumetric Smoke Plumes, Waterfall Spray & Mason Chisel Sparks) (`URBS-FX`)

- [x] **URBS-FX-01**: Implement deterministic time-parameterized **Rising Forge & Error Ember Emitters (`drawForgeAndFireEmbers`)** in `lightingAndFx.ts` and wire into `drawAnimatedWorldEffects` in `worldRenderer.ts` for the Acropolis Trip-Hammer Forge, Praetorium braziers, and any building with `errors > 0`.
- [x] **URBS-FX-02**: Implement **Soft Volumetric Drifting Smoke Plumes (`drawVolumetricSmokePlumes`)** and **Waterfall Mist Spray & Harbor Water Caustics (`drawWaterCausticsAndSpray`)** in `lightingAndFx.ts`.
- [x] **URBS-FX-03**: Implement **Active Mason Chisel Sparks & Dust Motes (`drawMasonChiselSparks`)** around active Treadwheel Cranes (`+LoC / -LoC`) and **Shimmering Golden Seal-Token Trails** along active dependency viaducts.
- [x] **URBS-FX-04**: Add Vitest unit tests verifying deterministic particle coordinates, alpha lifecycle fade, and zero-allocation frame rendering.

### 6. Phase 282: Diegetic Cliff-Apron Architrave Plaques, Architecture-First HUD Decluttering & Playwright CV Flywheel Convergence (`URBS-DIEGETIC`)

- [x] **URBS-DIEGETIC-01**: Upgrade `drawDistrictMicroHudBadge` in `worldRenderer.ts` into a **Chiseled Roman Bronze & Travertine Cliff-Apron Architrave Plaque** mounted cleanly along the front cliff retaining wall (`wy + 34`) below the terrace skyline so it never occludes the 3D district wonder or file buildings.
- [x] **URBS-DIEGETIC-02**: Declutter `drawScreenSpaceOverlays` in `worldRenderer.ts` using **Architecture-First Progressive Disclosure**: render floating `drawWorksiteFileDiffSlab` and `drawTacticalErrorDispatchOverlay` callouts only for the currently selected/hovered district or the single highest-priority error/collision target, keeping 85%+ of the 3D architectural diorama completely unobstructed.
- [x] **URBS-DIEGETIC-03**: Upgrade the procedural sprite atlas in `crates/vox-gui/ui/src/components/gamify/urbs/sprites.ts` (`fire-0`, `fire-1`, `ship`, `caravan`, `gate`, `castrum`, `arch`) with multi-stop metallic/stone gradients and soft drop shadows so stamped sprites match the photorealistic 3D world buffer.
- [x] **URBS-DIEGETIC-04**: Run end-to-end verification across Vitest (`pnpm --dir crates/vox-gui/ui exec vitest run src/components/gamify`), TypeScript (`tsc --noEmit`), and Playwright CV Flywheel (`urbs-civilization` & `chat-dock-multi`), verifying `visualAfterScore >= 0.935` and `0` regressions.

## v4.9 Requirements (24/24 Complete Across Phases 271–276)

### 1. Phase 271: Zero-Hop Inline Diff & Compiler Diagnostic Loupe (*Speculum Codicis*) (`URBS-LOUPE`)

- [x] **URBS-LOUPE-01**: Implement Tauri command `workspace_file_diff_and_diagnostics(file_path: String)` in `crates/vox-gui/src/commands/workspace_town.rs` (and register in `crates/vox-gui/src/lib.rs`) returning `FileDiffAndDiagnosticsDto { file_path, added_lines, deleted_lines, unified_diff, diagnostics: Vec<CompilerDiagnosticSpanDto> }` backed by `git diff --unified=3 -- <file_path>` and structured compiler/test diagnostic spans (`file`, `line`, `column`, `severity`, `code`, `message`).
- [x] **URBS-LOUPE-02**: Create `crates/vox-gui/ui/src/components/gamify/urbs/SpeculumLoupePopover.tsx` (`data-testid="urbs-speculum-loupe-popover"`) rendered as a floating hammered-bronze and obsidian optical loupe (`380px x 260px`) anchored directly above any clicked or hovered Treadwheel Crane (`+LoC / -LoC`) or red error building (`N ERR`), displaying color-coded `+` emerald / `-` crimson unified diff hunks and exact `file:line:col` compiler diagnostic spans.
- [x] **URBS-LOUPE-03**: Wire 3 real 1-click developer actions inside `SpeculumLoupePopover.tsx`: `[✓ Stage File]` (`git add -- <file_path>` via Tauri `workspace_stage_file`), `[↺ Revert File]` (`git checkout -- <file_path>` via Tauri `workspace_revert_file`), and `[⚒ Fix Exact Error]` (dispatching `submitAgentTask` pre-seeded with `file_path`, exact `line:col`, compiler error message, and surrounding diff hunk).
- [x] **URBS-LOUPE-04**: Add Rust unit tests in `crates/vox-gui/src/commands/workspace_town.rs` and Vitest component tests in `crates/vox-gui/ui/src/components/gamify/urbs/__tests__/SpeculumLoupePopover.test.tsx` verifying real unified diff extraction, diagnostic rendering, and 1-click Stage/Revert/Dispatch-Fix callbacks.

### 2. Phase 272: Multi-Agent Write-Set Collision & Blast-Radius Cone Radar (*Concursus Legionum*) (`URBS-COLLISION`)

- [x] **URBS-COLLISION-01**: Create `crates/vox-gui/ui/src/components/gamify/urbs/collisionRadar.ts` implementing `computeAgentWriteSetCones(agentTasks, buildings, topologyEdges)` that computes each active agent's claimed write-set (`targetFiles`), downstream blast-radius modules (`1-hop` reverse dependents), and pairwise overlap (`CollisionZone { id, districtId, overlappingFilePaths, agentA, agentB, severity: 'direct_write_conflict' | 'blast_radius_overlap' }`).
- [x] **URBS-COLLISION-02**: Upgrade `crates/vox-gui/ui/src/components/gamify/urbs/worldRenderer.ts` to project semi-transparent Roman surveyor groma cones across each active agent's target district and plant a 3D **Hammered-Bronze Crossed-Swords Collision Standard (`⚠ WRITE-SET COLLISION`)** with an animated crimson hazard boundary on any tile/district with a non-empty `CollisionZone`.
- [x] **URBS-COLLISION-03**: Wire 1-click collision resolution controls in `LudusSandbox.tsx` and `ImperiumInspector.tsx` when a `CollisionZone` is clicked: `[⇄ Serialize Agents]` (calling `steerChatQueue` / task dependency serialization so Agent B waits for Agent A's lock release) and `[🌿 Split Worktree]` (calling `worktreeSpawnVariant` from `t3Wave3Rpc.ts` to isolate Agent B onto a dedicated Git worktree).
- [x] **URBS-COLLISION-04**: Add Vitest unit tests in `crates/vox-gui/ui/src/components/gamify/urbs/__tests__/collisionRadar.test.ts` verifying direct file write-set collisions, 1-hop blast-radius collisions, and `[⇄ Serialize Agents]` / `[🌿 Split Worktree]` action dispatch.

### 3. Phase 273: Clickable Viaduct Cross-Module Contract & Layer-Inversion Inspector (*Pons Foederis*) (`URBS-VIADUCT`)

- [x] **URBS-VIADUCT-01**: Extend `WorkspaceTownSnapshot` in `crates/vox-gui/src/commands/workspace_town.rs` with `edges: Vec<TownDependencyEdge>` (`from_district_id`, `to_district_id`, `imported_symbols: Vec<String>`, `call_site_count: u32`, `from_layer: Option<u8>`, `to_layer: Option<u8>`, `layer_violation: bool`, `broken_contract_errors: u32`) computed by scanning workspace manifest dependencies (`Cargo.toml` / `package.json` / `go.mod` / Python imports) and checking `docs/src/architecture/layers.toml` layer assignments when present.
- [x] **URBS-VIADUCT-02**: Add viaduct segment hit-testing (`hitTestViaductEdge(worldX, worldY, edges, districts)`) in `crates/vox-gui/ui/src/components/gamify/urbs/layout.ts` and render a **Cracked Red Arch Keystone (`⚠ LAYER VIOLATION` / `⚡ BROKEN CONTRACT`)** with crimson mortar fissures in `worldRenderer.ts` whenever `edge.layer_violation` or `edge.broken_contract_errors > 0` is true.
- [x] **URBS-VIADUCT-03**: Add the `PONS CONTRACTVS` (Viaduct Contract Inspector) view to `crates/vox-gui/ui/src/components/gamify/urbs/ImperiumInspector.tsx` (`data-testid="urbs-viaduct-contract-panel"`) displaying source → target module direction, layer numbers (`L1 → L3`), imported symbols/traits, downstream call-site count, and 1-click `[⚒ Repair Contract Mismatch]` task dispatch.
- [x] **URBS-VIADUCT-04**: Add Rust tests in `workspace_town.rs` and Vitest tests in `crates/vox-gui/ui/src/components/gamify/urbs/__tests__/viaductInspector.test.tsx` verifying dependency edge extraction, layer-inversion flagging, viaduct hit-testing, and `PONS CONTRACTVS` rendering.

### 4. Phase 274: Hard Agent Scope Quarantine Fencing (*Clausura Ferrea*) (`URBS-QUARANTINE`)

- [x] **URBS-QUARANTINE-01**: Add `quarantined_paths: Arc<DashSet<String>>` to `ServerState` in `crates/vox-orchestrator-mcp/src/server_state/mod.rs` and expose MCP tools `vox_quarantine_set` (`{ path_prefix: String, quarantined: bool }`) and `vox_quarantine_list` (`{}`) in `tool_registry.rs`, `input_schemas.rs`, and `dispatch.rs`, plus Tauri bridge commands `set_district_quarantine` and `list_district_quarantines` in `crates/vox-gui/src/commands/workspace_town.rs`.
- [x] **URBS-QUARANTINE-02**: Enforce hard pre-execution write rejection in `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`: before executing any mutating tool (`vox_write_file`, `vox_edit_file`, `vox_apply_structured_edit`, `write_file`, `edit_file`, `delete_file`), check whether the normalized target path matches any prefix in `state.quarantined_paths`; if matched, block execution and return a structured `[QUARANTINE GUARDRAIL: Path '<path>' is locked read-only by developer Clausura Ferrea]` tool error so the agent cannot touch quarantined modules.
- [x] **URBS-QUARANTINE-03**: Upgrade `worldRenderer.ts`, `LudusSandbox.tsx`, and `ImperiumInspector.tsx` so `Alt+Click` on a district (or clicking `[🔒 Lock District Scope]` in the inspector) toggles `set_district_quarantine` and renders a 3D **Wrought-Iron Perimeter Portcullis Fence & Bronze Padlock (`🔒 QUARANTINED: READ-ONLY`)** around the district terrace.
- [x] **URBS-QUARANTINE-04**: Add Rust integration tests in `crates/vox-orchestrator-mcp` proving mutating tool calls against a quarantined path are rejected before touching disk while read-only tools (`vox_read_file`) still succeed, plus Vitest UI tests for the quarantine toggle.

### 5. Phase 275: Interactive Milestone Road Turn Time-Scrubber (*Via Temporis* Visual `git bisect`) (`URBS-SCRUBBER`)

- [x] **URBS-SCRUBBER-01**: Extend `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs` with `vox_vcs_checkpoint_diff_summary` (and wire `vcsCheckpointDiffSummary` in `t3Wave3Rpc.ts`) returning per-checkpoint modified file paths, `+LoC / -LoC` deltas, turn index, and timestamp from `GitPlumbingCheckpointStore` (`refs/vox/checkpoints/`).
- [x] **URBS-SCRUBBER-02**: Create `crates/vox-gui/ui/src/components/gamify/urbs/ViaTemporisScrubber.tsx` (`data-testid="urbs-via-temporis-scrubber"`) rendered at the bottom of the `VRBS` canvas along `PORTA GIT`, featuring Roman numeral milestone markers (`I, II, III, IV...`), a draggable **Bronze Sundial Gnomon Scrubber**, per-turn token/BU cost badges, and test-shield state indicators (`PASS` emerald vs. `FAIL` crimson).
- [x] **URBS-SCRUBBER-03**: Wire live historical state preview in `LudusSandbox.tsx`: scrubbing the sundial pointer to Milestone $k$ highlights the exact buildings modified in that turn (`+LoC / -LoC` cranes and test shield state at turn $k$) and provides a 1-click `[↺ Rollback Worktree to Milestone]` CTA wired to `vcsCheckpointRestore`.
- [x] **URBS-SCRUBBER-04**: Add Vitest unit tests in `crates/vox-gui/ui/src/components/gamify/urbs/__tests__/ViaTemporisScrubber.test.tsx` verifying milestone scrubbing, historical building diff preview projection, and `vcsCheckpointRestore` invocation.

### 6. Phase 276: Split-Dock Tactical Map Workbench Integration (*Tabula Praetoria `⌘M`*) (`URBS-DOCK`)

- [x] **URBS-DOCK-01**: Add a 1-click **`[🏛 Tactical Map ⌘M]`** toggle button and `Alt+M` / `⌘M` global shortcut in `crates/vox-gui/ui/src/components/dock/DockWorkspaceShell.tsx` and `ChatSurface.tsx` that opens `LudusSandbox` in a side-by-side `60/40` split-dock alongside the active Chat/Review workbench without leaving the current session.
- [x] **URBS-DOCK-02**: Implement bi-directional district context filtering (`selectedUrbsDistrictFilter` in `useAppStore.ts`): clicking a district in `LudusSandbox` filters the docked `ChatTranscript` / `ReviewView` to tool calls and diffs touching that module's path prefix (with a dismissible `[🏛 District Filter: <name> ×]` pill in the chat header).
- [x] **URBS-DOCK-03**: Wire real-time tool-call mechanism pulses from active chat streaming events into `LudusSandbox.tsx`: live `read_file`/`search` events pulse the target district's **Archimedes Screw Pump**, `write_file`/`edit_file` events spin its **Vitruvian Treadwheel Crane**, `cargo test`/`vitest` events pulse its **Legionary Shield Wall**, and approval prompts drop its **Portcullis Gate**.
- [x] **URBS-DOCK-04**: Run end-to-end verification across Rust (`cargo test -p vox-gui` and `cargo test -p vox-orchestrator-mcp`), Vitest (`pnpm --dir crates/vox-gui/ui exec vitest run`), and Playwright CV Flywheel (`urbs-civilization` & `chat-dock-multi`), verifying `visualAfterScore >= 0.92` and `0` regressions.

## v4.8 Requirements (24/24 Complete Across Phases 265–270)

### 1. Phase 265: `tool:<name>` Fence Salvage Fix, Live `@rule:<name>` Mention Resolution & `$variable` System Prompt Interpolation (`T3-SALVAGE-RULES`)

- [x] **T3-SALVAGE-RULES-01**: Upgrade `salvage_tool_calls_from_text` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs` with a Tier 1.5 regex (`r"(?s)```tool:([A-Za-z0-9_.:-]+)\s*\n(.*?)\s*```"`) that extracts the tool name from the fence header and parses the JSON body as `arguments` (matching `looks_like_streamed_tool_call_envelope`), so `pi-t3chat`-style `tool:<name>` fences are salvaged into executable `ToolCallPayload`s instead of being suppressed and dropped.
- [x] **T3-SALVAGE-RULES-02**: Wire `resolve_user_rule_mentions` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs`) into `resolve_mentions` (and add a workspace/DB rule loader helper) so `@rule:<name>` tags and `RuleActivationMode::Always` rules are stripped and resolved into `<user_rules>` context blocks **before** `MENTION_RE` file-path resolution runs (eliminating false `[ Mention @rule:... not found in workspace ]` warnings).
- [x] **T3-SALVAGE-RULES-03**: Wire `interpolate_system_prompt_variables` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs`) into `prepare_turn_context_and_retrieval` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/message/preamble_and_retrieval.rs`) using live workspace root, active git branch, session ID, active file, model ID, and cognitive profile.
- [x] **T3-SALVAGE-RULES-04**: Add unit and integration tests in `crates/vox-orchestrator-mcp` proving `tool:<name>` fence salvage, `@rule:<name>` live mention stripping + `<user_rules>` injection without file-not-found noise, and `$workspace_root` / `$git_branch` / `$session_id` interpolation on the live turn preparation path.

### 2. Phase 266: Live `context_sanitizer` History Stripping, `vox_chat_compact_up_to`, `BatteryCostLedger` Accounting, `VoxIdAllocatorV2` Message IDs & Real `ResponseQualityAnalyzer` Telemetry (`T3-HIST-BATTERY`)

- [x] **T3-HIST-BATTERY-01**: Wire `vox_orchestrator::context_sanitizer::sanitize_assistant_for_history` (stripping closed `<think>...</think>` and `<WebSearch>...</WebSearch>` blocks from prior assistant turns while preserving `thought_signature` metadata) and `escape_prompt_injection_tokens` (neutralizing raw role-boundary injection delimiters in user/tool messages) into `transcript_entry_to_llm_message` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/conversation.rs`.
- [x] **T3-HIST-BATTERY-02**: Implement `chat_compact_up_to` (`vox_chat_compact_up_to`) in `crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs` (and wire it in `tool_registry.rs`, `input_schemas.rs`, and `dispatch.rs`), supporting both `UpToMessageId` and `FromMessageId` anchor directions while preserving the turn-0 goal anchor and persisting a `[COMPACTED HISTORY SUMMARY]` entry.
- [x] **T3-HIST-BATTERY-03**: Instantiate a shared `BatteryCostLedger` (`1 BU = $0.001`) and `VoxIdAllocatorV2` on `ServerState` (`crates/vox-orchestrator-mcp/src/server_state/mod.rs`), run `check_preflight_balance` before executing `chat_message`, record post-turn `BatteryTransaction` (`record_turn_usage`) in `persist_and_envelope.rs`, and mint collision-free deterministic `usr-*` and `ast-*` message IDs via `VoxIdAllocatorV2::derive`.
- [x] **T3-HIST-BATTERY-04**: Replace the hardcoded `quality_score: Some(1.0)` in `persist_and_build_chat_envelope` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/message/persist_and_envelope.rs`) with the real `ResponseQualityAnalyzer::analyze_response_quality(&params.prompt, &response_text)` normalized score (`quality_score / 100.0`), and expose `quality_score`, `flesch_kincaid_grade`, and `battery_spent_bu` in the turn response envelope's `posture_audit`.

### 3. Phase 267: Live `agent_loop` Integration: `CommandOutputHub` `\r` Normalization, `20 KiB` DOM Shedding, Pre-Write Sparse Git Checkpoints, `ResumableTurnStreamRegistry` & Mid-Turn Steer Draining (`T3-LOOP-CHECKPOINT`)

- [x] **T3-LOOP-CHECKPOINT-01**: Wire `vox_orchestrator::command_output_hub::normalize_terminal_output` (`\r` carriage-return progress-bar folding) and `vox_orchestrator::snapshot::bound_snapshot_metadata` (`20 KiB` DOM/snapshot shedding) into `agent_loop.rs` before `cap_and_summarize_tool_output` runs on tool execution results.
- [x] **T3-LOOP-CHECKPOINT-02**: Wire automatic pre-write sparse Git checkpointing into `agent_loop.rs`: before executing any tool call where `GitPlumbingCheckpointStore::should_sparse_checkpoint(&call.name)` is `true`, capture a sparse checkpoint via a shared `GitPlumbingCheckpointStore` on `ServerState` (`refs/vox/checkpoints/{session_id}/{turn_idx}`).
- [x] **T3-LOOP-CHECKPOINT-03**: Add a shared `ResumableTurnStreamRegistry` (`DashMap<String, Arc<AtomicResumableStreamBuffer>>`) to `ServerState`, append streamed token chunks in `stream_llm_with_pref_and_events` (`salvage_and_stream.rs`), and expose `vox_chat_stream_resume` in `dispatch.rs` so secondary tabs or reconnecting clients can atomically catch up (`catch_up_and_subscribe`) from any character offset.
- [x] **T3-LOOP-CHECKPOINT-04**: Add a shared `ThreadRunQueue` to `ServerState`, expose `vox_chat_queue_steer` (`enqueue` / `promote_to_steer` / `drain`) in `dispatch.rs`, and drain promoted `QueueDisposition::Steer` prompts at the top of each `agent_loop.rs` iteration so users can steer an in-flight multi-step tool loop without cancelling the turn.

### 4. Phase 268: Stateful `t3_wave3_tools.rs`: Concurrent `join_all` LLM Consensus Fan-Out, Live Transcript Hydration/Persistence for `vox_context_handoff` & `vox_session_*` & Persistent `GuestShareService` (`T3-STATEFUL-TOOLS`)

- [x] **T3-STATEFUL-TOOLS-01**: Upgrade `handle_consensus_run` in `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs` so that when pre-computed `legs` are not supplied, it dispatches real concurrent async completions across the requested `models` via `futures_util::future::join_all` (`call_llm_with_pref` with per-leg timeout and graceful deterministic fallback when offline/no API key is configured), scores each leg with `analyze_response_quality`, and synthesizes the winning consensus response.
- [x] **T3-STATEFUL-TOOLS-02**: Upgrade `handle_context_handoff` in `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs` to hydrate `source_thread_id`'s live conversation history from `ServerState` (`context_history_or_hydrate(state, &format!("chat_history:{source_thread_id}"))`) when `turns` is omitted, and persist the generated handoff brief as the initial system/context entry of `chat_history:{new_thread_id}`.
- [x] **T3-STATEFUL-TOOLS-03**: Upgrade `handle_session_import_external`, `handle_session_export_bundle`, and `handle_session_import_bundle` in `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs` to read real external session files (`file_path` or auto-discovered latest JSONL/JSON under `~/.claude/projects`, `~/.codex/sessions`, `~/.cursor/chats`), hydrate real session transcripts from `ServerState` on export, and persist imported turns into `chat_history:{session_id}` + `VoxDb` on import.
- [x] **T3-STATEFUL-TOOLS-04**: Move `GuestShareService` onto `ServerState` (`guest_share_service: Arc<RwLock<GuestShareService>>`) so minted share tokens and pending guest prompts persist across RPC calls, and when `vox_guest_share_moderate` approves a guest prompt (`approve: true`), append the approved guest message directly into `chat_history:{thread_id}`.

### 5. Phase 269: `vox_chat_regenerate` & `vox_chat_edit_message_in_place` Backend MCP Tools, `DelegationWorktree` Lease/Env Wiring, `submit_webhook_events` Dedupe/Rate Gate & Compact `vox_orchestrator_status` (`T3-REGEN-WORKTREE`)

- [x] **T3-REGEN-WORKTREE-01**: Implement `chat_edit_message_in_place` (`vox_chat_edit_message_in_place`, mutating a message's content in-place in `chat_history:{session_id}` and marking `edited_in_place: true` without truncating subsequent turns) and `chat_regenerate` (`vox_chat_regenerate`, truncating from the target message onward and re-invoking `chat_message` with stored `GenerationPostureSnapshot` or `model_override`) in `crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs` and register them in `tool_registry.rs`, `input_schemas.rs`, and `dispatch.rs`.
- [x] **T3-REGEN-WORKTREE-02**: Wire `WorkspaceLeaseMap::global().lease_for(repo_root)` and `provision_untracked_env_files(repo_root, &wt_path)` into `DelegationWorktree::create` in `crates/vox-orchestrator-mcp/src/agy_worktree.rs` so concurrent worktree creations serialize on `.git/index.lock` and automatically receive safe `.env` junction/copy provisioning.
- [x] **T3-REGEN-WORKTREE-03**: Wire `WebhookDedupeAndRateGate::admit_delivery` into `submit_webhook_events` in `crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs` so duplicate `delivery_id`s within the 5-minute TTL window and bursts exceeding 60 deliveries/min per channel are rejected before entering `webhook_inbox`.
- [x] **T3-REGEN-WORKTREE-04**: Wire `args["compact"]` and `args["omit_idle_subagents"]` in `dispatch.rs` (`vox_orchestrator_status`) to call `dei_tools::orchestrator_status_compact(state, omit_idle_subagents)`, and upgrade `crates/vox-orchestrator-mcp/tests/transfer_budget.rs` to test the real `orchestrator_status_compact` and `cap_and_summarize_tool_output` functions on a 900-row / 1.1 MiB fixture.

### 6. Phase 270: `t3Wave3Rpc.ts` Nested Tauri Envelope Unwrapping, Live `ChatTranscript.tsx` In-Place Edit & Posture-Aware Regeneration & Full-Stack Verification (`T3-GUI-ENVELOPE`)

- [x] **T3-GUI-ENVELOPE-01**: Fix `unwrapMcpData` in `crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts` to recursively unwrap Tauri's `{ is_error: boolean, result: unknown }` wrapper, stringified JSON payloads, MCP `{ content: [{ type: 'text', text: '...' }] }` blocks, and `{ success: boolean, data: T, error?: string }` envelopes (throwing on `is_error === true` or `success === false` so real backend responses never fall through to mock objects).
- [x] **T3-GUI-ENVELOPE-02**: Add typed RPC client helpers `editChatMessageInPlace`, `regenerateChatTurn`, `compactChatUpTo`, `resumeChatStream`, and `steerChatQueue` to `crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts` and export them from `crates/vox-gui/ui/src/transport.ts`.
- [x] **T3-GUI-ENVELOPE-03**: Wire in-place assistant message editing (`editChatMessageInPlace` with `[Edited]` badge) and posture-aware message regeneration (`regenerateChatTurn`) into `crates/vox-gui/ui/src/components/chat/ChatTranscript.tsx`.
- [x] **T3-GUI-ENVELOPE-04**: Add comprehensive Vitest unit tests in `crates/vox-gui/ui/src/transport/__tests__/t3Wave3Rpc.test.ts` testing nested Tauri `{ is_error: false, result: { success: true, data: ... } }` and stringified MCP content unwrapping across all RPC helpers, and verify all Rust tests, Vitest suites, `tsc --noEmit`, `vox-code-audit`, `vox-arch-check`, and `vox-doc-pipeline` gates pass with zero errors.

---

## Traceability Matrix

| Requirement ID | Phase | Category | Status |
|---|---|---|---|
| `T3-SALVAGE-RULES-01` | Phase 265 | `tool:<name>` Fence Salvage | Complete |
| `T3-SALVAGE-RULES-02` | Phase 265 | Live `@rule:<name>` Mention Resolution | Complete |
| `T3-SALVAGE-RULES-03` | Phase 265 | Live `$variable` System Prompt Interpolation | Complete |
| `T3-SALVAGE-RULES-04` | Phase 265 | Salvage & Rule/Variable Pipeline Verification | Complete |
| `T3-HIST-BATTERY-01` | Phase 266 | Live `context_sanitizer` in `conversation.rs` | Complete |
| `T3-HIST-BATTERY-02` | Phase 266 | `vox_chat_compact_up_to` Anchor Compaction | Complete |
| `T3-HIST-BATTERY-03` | Phase 266 | Live `BatteryCostLedger` & `VoxIdAllocatorV2` | Complete |
| `T3-HIST-BATTERY-04` | Phase 266 | Real `ResponseQualityAnalyzer` Telemetry | Complete |
| `T3-LOOP-CHECKPOINT-01` | Phase 267 | Live `\r` Normalization & `20 KiB` DOM Shedding | Complete |
| `T3-LOOP-CHECKPOINT-02` | Phase 267 | Automatic Pre-Write Sparse Git Checkpoints | Complete |
| `T3-LOOP-CHECKPOINT-03` | Phase 267 | Live `ResumableTurnStreamRegistry` & RPC | Complete |
| `T3-LOOP-CHECKPOINT-04` | Phase 267 | Mid-Turn `ThreadRunQueue` Steer Draining | Complete |
| `T3-STATEFUL-TOOLS-01` | Phase 268 | Real Concurrent `join_all` LLM Consensus | Complete |
| `T3-STATEFUL-TOOLS-02` | Phase 268 | Stateful `vox_context_handoff` Hydration | Complete |
| `T3-STATEFUL-TOOLS-03` | Phase 268 | Stateful `vox_session_*` File Scan & Persistence | Complete |
| `T3-STATEFUL-TOOLS-04` | Phase 268 | Persistent `ServerState` `GuestShareService` | Complete |
| `T3-REGEN-WORKTREE-01` | Phase 269 | `vox_chat_regenerate` & `vox_chat_edit_message_in_place` | Complete |
| `T3-REGEN-WORKTREE-02` | Phase 269 | `DelegationWorktree` Lease & `.env` Provisioning | Complete |
| `T3-REGEN-WORKTREE-03` | Phase 269 | `submit_webhook_events` Dedupe & Rate Gate | Complete |
| `T3-REGEN-WORKTREE-04` | Phase 269 | Compact `vox_orchestrator_status` & Budget Test | Complete |
| `T3-GUI-ENVELOPE-01` | Phase 270 | `t3Wave3Rpc.ts` Nested Tauri Envelope Unwrapping | Complete |
| `T3-GUI-ENVELOPE-02` | Phase 270 | Typed Regenerate/Edit/Compact/Resume/Steer RPCs | Complete |
| `T3-GUI-ENVELOPE-03` | Phase 270 | `ChatTranscript.tsx` In-Place Edit & Regenerate UI | Complete |
| `T3-GUI-ENVELOPE-04` | Phase 270 | Full-Stack Verification & Gate Closure | Complete |
| `URBS-LOUPE-01` | Phase 271 | `workspace_file_diff_and_diagnostics` Tauri RPC | Complete |
| `URBS-LOUPE-02` | Phase 271 | `SpeculumLoupePopover.tsx` Diff & Diagnostic Loupe | Complete |
| `URBS-LOUPE-03` | Phase 271 | 1-Click Stage / Revert / Dispatch-Fix Actions | Complete |
| `URBS-LOUPE-04` | Phase 271 | Inline Diff & Diagnostic Loupe Tests | Complete |
| `URBS-COLLISION-01` | Phase 272 | `collisionRadar.ts` Write-Set & Blast-Radius Overlap | Complete |
| `URBS-COLLISION-02` | Phase 272 | Surveyor Cones & Crossed-Swords Collision Standard | Complete |
| `URBS-COLLISION-03` | Phase 272 | 1-Click Serialize Agents & Split Worktree Actions | Complete |
| `URBS-COLLISION-04` | Phase 272 | Multi-Agent Collision Radar Tests | Complete |
| `URBS-VIADUCT-01` | Phase 273 | `TownDependencyEdge` & `layers.toml` Inversion Scanner | Complete |
| `URBS-VIADUCT-02` | Phase 273 | Viaduct Hit-Testing & Cracked Red Arch Keystone | Complete |
| `URBS-VIADUCT-03` | Phase 273 | `PONS CONTRACTVS` Viaduct Contract Inspector Panel | Complete |
| `URBS-VIADUCT-04` | Phase 273 | Viaduct Contract & Layer-Inversion Tests | Complete |
| `URBS-QUARANTINE-01` | Phase 274 | `ServerState` `quarantined_paths` & MCP/Tauri Tools | Complete |
| `URBS-QUARANTINE-02` | Phase 274 | Pre-Execution Write Guard in `agent_loop.rs` | Complete |
| `URBS-QUARANTINE-03` | Phase 274 | 3D Wrought-Iron Perimeter Portcullis & `Alt+Click` Lock | Complete |
| `URBS-QUARANTINE-04` | Phase 274 | Hard Quarantine Guard & UI Tests | Complete |
| `URBS-SCRUBBER-01` | Phase 275 | `vox_vcs_checkpoint_diff_summary` Checkpoint Delta RPC | Complete |
| `URBS-SCRUBBER-02` | Phase 275 | `ViaTemporisScrubber.tsx` Sundial Milestone Timeline | Complete |
| `URBS-SCRUBBER-03` | Phase 275 | Historical Building Diff Preview & 1-Click Rollback | Complete |
| `URBS-SCRUBBER-04` | Phase 275 | Milestone Road Time-Scrubber Tests | Complete |
| `URBS-DOCK-01` | Phase 276 | `[🏛 Tactical Map ⌘M]` Split-Dock Workbench Toggle | Complete |
| `URBS-DOCK-02` | Phase 276 | Bi-Directional District Filter with Chat/Review | Complete |
| `URBS-DOCK-03` | Phase 276 | Real-Time Tool-Call Mechanism Animation Pulses | Complete |
| `URBS-DOCK-04` | Phase 276 | Full-Stack & Playwright CV Flywheel Closure | Complete |
| `URBS-MAT-01` | Phase 277 | `PbrMaterialAtlas` Pitted Roman Travertine Ashlar | Complete |
| `URBS-MAT-02` | Phase 277 | Scalloped Terracotta Roof Tiles & Cobblestone Plazas | Complete |
| `URBS-MAT-03` | Phase 277 | Stratified Basalt Cliffs & Anisotropic Bronze Shaders | Complete |
| `URBS-MAT-04` | Phase 277 | Procedural Material Synthesis Unit Tests | Complete |
| `URBS-DEPTH-01` | Phase 278 | `drawIsoCastShadow` Projected Directional Shadows | Complete |
| `URBS-DEPTH-02` | Phase 278 | Vertical Face Gradients & Travertine Texture Overlay | Complete |
| `URBS-DEPTH-03` | Phase 278 | Specular Chamfer Highlights Replacing Cartoon Strokes | Complete |
| `URBS-DEPTH-04` | Phase 278 | Depth Shading & Cast Shadow Tests | Complete |
| `URBS-TERRAIN-01` | Phase 279 | Mediterranean Twilight Bathymetric Sea Depth Field | Complete |
| `URBS-TERRAIN-02` | Phase 279 | Layered Coastal Surf Foam & Cliff-Base Reflections | Complete |
| `URBS-TERRAIN-03` | Phase 279 | Stratified Cliff Geology & Cobblestone Terrace Paving | Complete |
| `URBS-TERRAIN-04` | Phase 279 | Bathymetric Terrain & Cliff Layering Tests | Complete |
| `URBS-LIGHT-01` | Phase 280 | Inter-Terrace Atmospheric Elevation Mist | Complete |
| `URBS-LIGHT-02` | Phase 280 | Diagonal Crepuscular Volumetric God-Rays | Complete |
| `URBS-LIGHT-03` | Phase 280 | Additive Point-Light GI Pools & Cinematic Vignette | Complete |
| `URBS-LIGHT-04` | Phase 280 | Atmospheric Lighting & Post-Processing Tests | Complete |
| `URBS-FX-01` | Phase 281 | Rising Forge & Error Ember Emitters | Complete |
| `URBS-FX-02` | Phase 281 | Volumetric Smoke Plumes & Waterfall Mist/Caustics | Complete |
| `URBS-FX-03` | Phase 281 | Mason Chisel Sparks & Golden Viaduct Token Trails | Complete |
| `URBS-FX-04` | Phase 281 | Deterministic Particle FX Engine Tests | Complete |
| `URBS-DIEGETIC-01` | Phase 282 | Chiseled Cliff-Apron Architrave District Plaques | Complete |
| `URBS-DIEGETIC-02` | Phase 282 | Architecture-First HUD Progressive Disclosure | Complete |
| `URBS-DIEGETIC-03` | Phase 282 | Photorealistic Sprite Atlas Gradient & Shadow Upgrade | Complete |
| `URBS-DIEGETIC-04` | Phase 282 | Full-Stack & Playwright CV Flywheel v5.0 Closure | Complete |
| `T3-PROMPT-LOOP-01` | Phase 283 | `system_prompt` `$variable` Fix & Worktree Branch Detection | Complete |
| `T3-PROMPT-LOOP-02` | Phase 283 | Full `context_sanitizer` Tag Set & `thought_signature` Extraction | Complete |
| `T3-PROMPT-LOOP-03` | Phase 283 | `ToolRefusalDetector` & Per-Turn Search Circuit Breaker | Complete |
| `T3-PROMPT-LOOP-04` | Phase 283 | `LazyMcpGroupRegistry` Meta-Tools & Env Sanitizer | Complete |
| `T3-STREAM-QUEUE-01` | Phase 284 | Live `ResumableTurnStreamRegistry` & `vox_chat_stream_cancel` | Complete |
| `T3-STREAM-QUEUE-02` | Phase 284 | `ThreadRunQueue` `pop_next_follow_up` & Background Settlement | Complete |
| `T3-STREAM-QUEUE-03` | Phase 284 | `ModelRegistry::compute_turn_cost_delta` & `4h`/Monthly Quota | Complete |
| `T3-STREAM-QUEUE-04` | Phase 284 | `SessionPrivacyPosture::Ephemeral` Disk Persistence Gate | Complete |
| `T3-WORKTREE-CKPT-01` | Phase 285 | Async `tokio::sync::Mutex` Worktree Lease & Persistent Webhook Gate | Complete |
| `T3-WORKTREE-CKPT-02` | Phase 285 | Monotonic Sparse Checkpoints, NTFS Sanitization & Clean Restore | Complete |
| `T3-WORKTREE-CKPT-03` | Phase 285 | Real Linked Worktrees in `MultiVariantWorktreePool` | Complete |
| `T3-WORKTREE-CKPT-04` | Phase 285 | Unwrapped `orchestrator_status_compact` & `LiveBodyDeduper` | Complete |
| `T3-TOOLS-HARDENING-01` | Phase 286 | UTF-8 Boundary Safety & `HandoffStrategy::Summarized` Digest | Complete |
| `T3-TOOLS-HARDENING-02` | Phase 286 | Multi-Provider Consensus & Synthesizer Judge Pass | Complete |
| `T3-TOOLS-HARDENING-03` | Phase 286 | Depth-4 External Session Discovery & Real Guest Share RPCs | Complete |
| `T3-TOOLS-HARDENING-04` | Phase 286 | Server-Enforced `authorize_wave3_call` & Real File Diagnostics | Complete |
| `T3-GUI-PIPELINE-01` | Phase 287 | Unified `message_id` & Telemetry Across Tauri/MCP + DB Sync | Complete |
| `T3-GUI-PIPELINE-02` | Phase 287 | Live User+Assistant Edit, Executing Regenerate & `✂ Compact Here` | Complete |
| `T3-GUI-PIPELINE-03` | Phase 287 | Mid-Turn `[⚡ Steer]` / `[+ Queue Next]` & Enter-Key Fix in `Loquela` | Complete |
| `T3-GUI-PIPELINE-04` | Phase 287 | `streamWatchdog.ts`, `resumeChatStream` & Full Modal/Rail Wiring | Complete |
| `T3-AUDIT-PARITY-01` | Phase 288 | T3 Detector Registration in `diagnostics/catalog.rs` | Complete |
| `T3-AUDIT-PARITY-02` | Phase 288 | `unreasoned_suppression` & `test_clock_and_skip` Parity | Complete |
| `T3-AUDIT-PARITY-03` | Phase 288 | `mcp_access_and_error_safety` & `ui_perf_hazards` Parity | Complete |
| `T3-AUDIT-PARITY-04` | Phase 288 | Zero-Violation Workspace Remediation (43 Sites) | Complete |
| `URBS-SMOOTH-LIGHT-01` | Phase 289 | Pre-Building Ground-Plane Bounce Illumination | Complete |
| `URBS-SMOOTH-LIGHT-02` | Phase 289 | 2D-Feathered Elliptical Sun-Shafts (`drawVolumetricGodRays`) | Complete |
| `URBS-SMOOTH-LIGHT-03` | Phase 289 | Soft Torch Shimmer & Window Rim-Light (`drawAnimatedWorldEffects`) | Complete |
| `URBS-SMOOTH-LIGHT-04` | Phase 289 | `2π`-Periodic Seamless Sea Water Tile (`pbrMaterials.ts`) | Complete |
| `URBS-GROUNDS-01` | Phase 290 | Orthogonal `Cardo` & `Decumanus` Basalt Streets & Curbs | Complete |
| `URBS-GROUNDS-02` | Phase 290 | Inlaid `Opus Sectile` Marble Forum Plazas | Complete |
| `URBS-GROUNDS-03` | Phase 290 | Vaulted `Substructio` Cliff Arcades & Buttress Pilasters | Complete |
| `URBS-GROUNDS-04` | Phase 290 | Terrace Parapet Balustrades, Obelisks & Cypress Horti | Complete |
| `URBS-ARCH-DETAIL-01` | Phase 291 | Ground-Floor Rusticated Arcades (`Fornices`) & Keystones | Complete |
| `URBS-ARCH-DETAIL-02` | Phase 291 | Inter-Story Stringcourse Belt Cornices (`Marcapiano`) & Quoins | Complete |
| `URBS-ARCH-DETAIL-03` | Phase 291 | Recessed Atrium `Impluvium` Rainwater Basins (`T1` Domus) | Complete |
| `URBS-ARCH-DETAIL-04` | Phase 291 | Sculpted Rooftop `Acroteria` Finials & Hypocaust Chimneys | Complete |
| `URBS-HYDRAULIC-01` | Phase 292 | Integrated `Aquaeductus` Trunk & `Castellum Divisorium` | Complete |
| `URBS-HYDRAULIC-02` | Phase 292 | Pressurized `Fistulae` Supply Pipes & Bronze Sluice Valves | Complete |
| `URBS-HYDRAULIC-03` | Phase 292 | Recessed `Cloaca Maxima` Sewer Trenches & Inspection Grates | Complete |
| `URBS-HYDRAULIC-04` | Phase 292 | Cliff-Wall `Cloaca` Sea Outfalls & Sluice Cascades | Complete |
| `URBS-CONDUIT-ANIM-01` | Phase 293 | `AQUA PURA` Cyan Chevron Wave-Crest Supply Pulses | Complete |
| `URBS-CONDUIT-ANIM-02` | Phase 293 | `IGNIS OPERIS` Gold Diamond Ingot Agent WIP Stream | Complete |
| `URBS-CONDUIT-ANIM-03` | Phase 293 | `CLOACA FAULT` Crimson Warning Triangle Error Backpressure | Complete |
| `URBS-CONDUIT-ANIM-04` | Phase 293 | `VIA VICTORIAE` Emerald Circular Seal Medallion Flow | Complete |
| `URBS-SEMIOTICS-CV-01` | Phase 294 | Diegetic Hydraulic Sluice-Gate Callouts & Isometric Routing | Complete |
| `URBS-SEMIOTICS-CV-02` | Phase 294 | Multi-Channel Shape+Color Hydraulic Flow Legend | Complete |
| `URBS-SEMIOTICS-CV-03` | Phase 294 | `hydraulicNetwork.test.ts` & Lighting/Material Unit Tests | Complete |
| `URBS-SEMIOTICS-CV-04` | Phase 294 | End-to-End & Playwright CV Flywheel v5.2 Convergence | Complete |
| `URBS-EDGE-PBR-01` | Phase 295 | Feathered Elliptical Coastal Shoals & Surf Gradients | Complete |
| `URBS-EDGE-PBR-02` | Phase 295 | Low-Laplacian Macro-Shaded PBR Stone & Cobble Synthesis | Complete |
| `URBS-EDGE-PBR-03` | Phase 295 | Smooth Specular Ocean Swell Ribbons | Complete |
| `URBS-EDGE-PBR-04` | Phase 295 | Soft Tonal Edge Bevels Replacing Cartoon Ink Outlines | Complete |
| `URBS-TERRACE-CRAFT-01` | Phase 296 | True Isometric Molded Parapet Copings & Beveled Bastions | Complete |
| `URBS-TERRACE-CRAFT-02` | Phase 296 | Seamless Intersection-Free `Cardo` & `Decumanus` Paving | Complete |
| `URBS-TERRACE-CRAFT-03` | Phase 296 | Recessed Multi-Ring `Opus Sectile` Forum & Peristyle Columns | Complete |
| `URBS-TERRACE-CRAFT-04` | Phase 296 | Overhanging Travertine Cliff Cornice (`Geison`) & Drop Shadow | Complete |
| `URBS-ARCH-REALISM-01` | Phase 297 | 3D Cylindrical Normal-Shaded `Tabularium` & Coffered Dome | Complete |
| `URBS-ARCH-REALISM-02` | Phase 297 | Recessed Isometric Window Reveals & Interior Lamplight | Complete |
| `URBS-ARCH-REALISM-03` | Phase 297 | Sculpted Pediment Tympanums, Porticoes & Step Stylobates | Complete |
| `URBS-ARCH-REALISM-04` | Phase 297 | 3D Monumental `AQVAE`, `CASTRVM`, `PORTVS` & `PORTA` Complexes | Complete |
| `URBS-PLUMBING-3D-01` | Phase 298 | Continuous 3D Multi-Tiered `Aqua Claudia` Aqueduct Bridge | Complete |
| `URBS-PLUMBING-3D-02` | Phase 298 | Curved Beveled Isometric Pipe Elbows (`quadraticCurveTo`) | Complete |
| `URBS-PLUMBING-3D-03` | Phase 298 | Sub-Roof Ground-Layer Routing for All Conduits | Complete |
| `URBS-PLUMBING-3D-04` | Phase 298 | Stepped Stone `Cataracta` Sluice-Drops & Vaulted Sea Outfalls | Complete |
| `URBS-FLOW-SEMIOTICS-01` | Phase 299 | Soft Radial Halo Backlighting & Comet Trails on 4 Shape Channels | Complete |
| `URBS-FLOW-SEMIOTICS-02` | Phase 299 | Dynamic Flow Rate, Pipe Gauge & Pressure Pulsing by Load | Complete |
| `URBS-FLOW-SEMIOTICS-03` | Phase 299 | Diegetic `Castellum` Sluice-Gate Manifold & Perimeter Ports | Complete |
| `URBS-FLOW-SEMIOTICS-04` | Phase 299 | Unified Semiotic Legend Pill with Vector Shapes & Counters | Complete |
| `URBS-CV-MAX-01` | Phase 300 | Unit Tests for Beveled Conduits & Low-Noise PBR | Complete |
| `URBS-CV-MAX-02` | Phase 300 | Positive CV Noise Reduction (`noiseFloor < 5.1`) & Score Maximization | Complete |
| `URBS-CV-MAX-03` | Phase 300 | 32-Surface Zero-Regression Verification | Complete |
| `URBS-CV-MAX-04` | Phase 300 | Visual Critique & Before/After Blueprint Documentation | Complete |
