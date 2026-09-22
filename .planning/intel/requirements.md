# Requirements

Synthesized from PRD classifications. This corpus contains exactly one classified PRD:
`docs/src/architecture/dead-crate-fate-plan-2026-05-08.md` — a per-crate disposition plan for the
2026-05-08 workspace crate audit. Its `status` frontmatter is `research` (not locked/Accepted), but
the body is prescriptive with concrete per-crate acceptance criteria, so it is extracted as
requirements grouped by disposition ("fate") category rather than as one requirement per crate
(20 crates), to avoid 20 near-duplicate entries while preserving every crate-level acceptance
criterion verbatim.

## REQ-dead-crate-delete
- source: docs/src/architecture/dead-crate-fate-plan-2026-05-08.md
- description: Delete workspace crates with zero consumers and no remaining conceptual fit, consolidating any worthwhile types into their true owning crate first.
- acceptance: DELETE vox-schola (remove crate + workspace member; verify `cargo tree -p vox-cli` has no reference). DELETE vox-scientia-core and vox-scientia-social (pure pass-through facades over vox-publisher; verify no `vox_scientia_core::*`/`vox_scientia_social::*` imports remain). DELETE vox-scientia-ingest, but only after removing its mandatory `vox-cli` dependency first (audit usage, inline into vox-publisher's scholarly-external-jobs feature or gate behind a flag, then delete). DELETE vox-socrates-policy after migrating `ConfidencePolicy`/`ComplexityBand`/`RiskBand` types into vox-orchestrator-types. DELETE vox-spool (zero consumers; inline the JSONL helper elsewhere if needed). DELETE vox-tools (superseded by vox-capability-registry + vox-plugin-host dispatch pattern). DELETE vox-mcp-meta after migrating `A2A_MESSAGE_TYPES` constants into vox-orchestrator-types and wiring vox-mcp-registry directly. DELETE vox-browser and vox-audio-ingress (listed in executive summary; no consumers).
- scope: workspace crate lifecycle, DELETE fate, crate management

## REQ-dead-crate-wire-up
- source: docs/src/architecture/dead-crate-fate-plan-2026-05-08.md
- description: Wire up functionally-complete but never-adopted crates into their intended call path instead of deleting working code.
- acceptance: WIRE-UP vox-exec-grammar — add dependency from vox-container (or vox-cli-core) and call `vox_exec_grammar::risk::classify` inside the existing exec-policy gate per the ADR-026 contract (`contracts/terminal/exec-policy.v1.yaml`). WIRE-UP vox-mcp-registry — add dependency from vox-orchestrator, use `TOOL_REGISTRY` to validate/enumerate MCP tool names, then delete the now-redundant vox-mcp-meta wrapper. RECLASSIFY vox-search from DEAD to CORE (documentation-only correction: vox-cli and vox-orchestrator depend on it unconditionally; it is already wired). RECLASSIFY vox-doc-inventory from DEAD to CORE (vox-cli/Cargo.toml line 138 depends on it unconditionally via two binary targets).
- scope: workspace crate lifecycle, WIRE-UP-AS-IS fate, classification corrections

## REQ-dead-crate-extract-to-plugin
- source: docs/src/architecture/dead-crate-fate-plan-2026-05-08.md
- description: Move complete-but-CORE-inappropriate crates into the plugin architecture so their functionality survives without bloating the default CLI compile.
- acceptance: EXTRACT vox-grammar-export into a new `vox-plugin-grammar-export` plugin implementing the `GrammarExportPlugin` ABI, dispatched from vox-constrained-gen via the plugin host when available. EXTRACT vox-webhook into `vox-plugin-webhook`, reversing its current direct `vox-orchestrator` dependency into a dispatch-via-plugin call through `WebhookOrchestratorBridge` and `OrchestratorInboxItem`.
- scope: workspace crate lifecycle, EXTRACT-TO-PLUGIN fate, plugin ABI

## REQ-dead-crate-keep-frozen
- source: docs/src/architecture/dead-crate-fate-plan-2026-05-08.md
- description: Explicitly preserve crates whose disposition is to remain as-is without further action in this plan's scope.
- acceptance: KEEP-FROZEN vox-workflow-runtime, vox-integration-tests, vox-test-harness — no action required by this plan.
- scope: workspace crate lifecycle, KEEP-FROZEN fate

## REQ-dead-crate-misplaced
- source: docs/src/architecture/dead-crate-fate-plan-2026-05-08.md
- description: Resolve the two crates flagged MISPLACED (wrong architectural tier) rather than DEAD (unused).
- acceptance: REWRITE-AS-PLUGIN vox-ssg (misplaced as a CORE crate; belongs in the plugin architecture). Complete the in-progress vox-oratio extraction (already underway; finishing it removes the last direct Candle dependency bleed from CORE).
- scope: workspace crate lifecycle, MISPLACED fate

## REQ-dead-crate-catalog-cleanup
- source: docs/src/architecture/dead-crate-fate-plan-2026-05-08.md
- description: Confirm catalog-only ghost entries (already removed from the actual workspace) are fully reflected in catalog.toml with no further code action needed.
- acceptance: Verify execution-api and stub-check are absent from the workspace and any remaining catalog.toml comments accurately describe them as already cleaned up.
- scope: workspace crate lifecycle, catalog hygiene
