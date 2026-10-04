approved_by:

# Expansion proposal B (paraphrase candidates for q025..q048 plus coverage-gap candidates)

Agent-generated (AP-02, Pitfall 8): paraphrases are robustness probes and gap questions are coverage probes, never ground-truth queries. `- [ ] keep` is left unchecked; the user decides in 01-10. Paraphrase qrel rows are copied from the source block in qrels-proposal-b.md (checked rows only); gap blocks were pooled the same way as Task 1 (bm25 top 12 on the gap wording, path/content grep, reading; graph unavailable). Origin on materialization: llm-expansion, including every `(gap: ...)` block.

## B01 [policy] (paraphrase of q025) Under what circumstances may I bypass the pre-push checks, and what must I run once the push is done?

- [ ] keep
- [x] doc 2 docs/src/contributors/local-ci-pre-push.md
- [x] chunk 2 docs/src/contributors/local-ci-pre-push.md#Local CI parity (pre-push) › Bypass (emergency only)
- [x] doc 1 docs/src/contributors/contributor-hub.md

## B02 [policy] (paraphrase of q026) Is it OK for my crate to pull Turso connection secrets straight out of the process environment?

- [ ] keep
- [x] doc 2 docs/src/architecture/data-storage-ssot-2026.md
- [x] chunk 2 docs/src/architecture/data-storage-ssot-2026.md#Data Storage SSOT (2026) › 3. Non-negotiables (decided constraints)
- [x] doc 1 docs/src/reference/env-vars.md
- [x] chunk 1 docs/src/reference/env-vars.md#Environment variables (SSOT) › Codex / Turso (vox-db, vox-package)
- [x] doc 1 docs/src/architecture/data-storage-lint-and-ci-spec-2026.md
- [x] chunk 1 docs/src/architecture/data-storage-lint-and-ci-spec-2026.md#Data Storage Lint & CI Spec (2026) › 7. .cursor/rules/data-storage-policy.mdc (shipped at HEAD, aligned in this PR)
- [x] doc 1 docs/src/adr/004-codex-arca-turso-ssot.md

## B03 [policy] (paraphrase of q027) Will vox ever phone home with usage data unless I turn that on, and could what it uploads contain my code?

- [ ] keep
- [x] doc 2 docs/src/architecture/telemetry-trust-ssot.md
- [x] doc 2 docs/src/adr/023-optional-telemetry-remote-upload.md
- [x] chunk 2 docs/src/architecture/telemetry-trust-ssot.md#Telemetry Trust (SSoT) › Core Policy
- [x] chunk 2 docs/src/adr/023-optional-telemetry-remote-upload.md#ADR 023: Optional telemetry remote upload › Decision
- [x] doc 1 docs/src/architecture/telemetry-unification-design-2026.md
- [x] chunk 1 docs/src/architecture/telemetry-unification-design-2026.md#Telemetry unification design 2026 › Trust posture

## B04 [policy] (paraphrase of q028) What is the line-count ceiling for a Rust source file before the code-shape linter turns red?

- [ ] keep
- [x] doc 2 docs/src/contributors/toestub-contributor-guide.md
- [x] doc 2 docs/src/contributors/coding-agents.md
- [x] chunk 2 docs/src/contributors/toestub-contributor-guide.md#TOESTUB contributor guide › Rule-by-rule fix guide › arch/god_object — Error
- [x] chunk 2 docs/src/contributors/coding-agents.md#Coding Agent Instructions › Structural limits (enforced by TOESTUB, fail CI)
- [x] doc 1 docs/src/how-to/how-to-contribute-parser-hir.md
- [x] chunk 1 docs/src/how-to/how-to-contribute-parser-hir.md#Contributing — parser and HIR › TOESTUB Considerations

## B05 [why] (paraphrase of q029) What were the reasons for abandoning Python-based training when building the model tuning pipeline?

- [ ] keep
- [x] doc 2 docs/src/adr/003-native-training-over-python.md
- [x] chunk 2 docs/src/adr/003-native-training-over-python.md#ADR 003 — Native Rust Training Over Python › Historical context (why we left Python)
- [x] doc 1 docs/src/journeys/native-training.md
- [x] chunk 1 docs/src/journeys/native-training.md#Journey: Native Rust LLM Training › The Curse of Python ML Environments
- [x] chunk 1 docs/src/adr/003-native-training-over-python.md#ADR 003 — Native Rust Training Over Python › Consequences

## B06 [why] (paraphrase of q030) What made the team retire the localhost web UI for orchestration and ship a desktop app instead?

- [ ] keep
- [x] doc 2 docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md
- [x] chunk 2 docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md#ADR 045 — Tauri GUI Replaces Axum Dashboard › Context
- [x] chunk 1 docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md#ADR 045 — Tauri GUI Replaces Axum Dashboard › Decision
- [x] doc 1 docs/src/adr/037-tauri-convergence.md
- [x] chunk 1 docs/src/adr/037-tauri-convergence.md#ADR 037 — Tauri Convergence › Context
- [x] doc 1 docs/src/adr/024-dashboard-axum-spa.md

## B07 [why] (paraphrase of q031) What was the reasoning for building the peer-to-peer layer on iroh rather than continuing our own HTTP-based coordinator?

- [ ] keep
- [x] doc 2 docs/src/adr/047-iroh-transport.md
- [x] chunk 2 docs/src/adr/047-iroh-transport.md#ADR 047: iroh QUIC replaces the bespoke populi mesh transport › Context
- [x] chunk 2 docs/src/adr/047-iroh-transport.md#ADR 047: iroh QUIC replaces the bespoke populi mesh transport › Alternatives rejected
- [x] doc 1 docs/src/adr/020-populi-mesh-scaling-transport-default.md
- [x] chunk 1 docs/src/adr/047-iroh-transport.md#ADR 047: iroh QUIC replaces the bespoke populi mesh transport › Decision

## B08 [why] (paraphrase of q032) What is the rationale for running scripts received from other nodes in the built-in interpreter instead of a WebAssembly or container sandbox?

- [ ] keep
- [x] doc 2 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md
- [x] chunk 2 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md#ADR 048: Interpreter is the execution and sandbox tier › Context
- [x] chunk 2 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md#ADR 048: Interpreter is the execution and sandbox tier › Decision
- [x] doc 1 docs/src/architecture/voxscript-portability-substrate-research-2026.md
- [x] chunk 1 docs/src/architecture/voxscript-portability-substrate-research-2026.md#VoxScript portability substrate: research and findings (2026-09) › Part 2 — Research findings › 2.1 WASM is the only substrate that delivers bit-identity — and it cannot run Vox's scripts
- [x] doc 1 docs/src/reference/isolation.md

## B09 [why] (paraphrase of q033) What is the reasoning behind a few publishing pipelines being allowed to run to completion even when a newer commit lands?

- [ ] keep
- [x] doc 2 docs/src/ci/concurrency-exceptions.md
- [x] chunk 2 docs/src/ci/concurrency-exceptions.md#Workflow concurrency exceptions
- [x] doc 1 docs/src/reference/cli.md
- [x] chunk 1 docs/src/reference/cli.md#Reference: vox CLI (minimal compiler binary) › Build & run › vox ci …

## B10 [why] (paraphrase of q034) Given there is no evidence that models write PowerShell better, what justifies steering agents toward pwsh on Windows?

- [ ] keep
- [x] doc 2 docs/src/architecture/terminal-exec-policy-ssot.md
- [x] chunk 2 docs/src/architecture/terminal-exec-policy-ssot.md#Terminal Exec Policy SSOT (2026) › What this policy IS
- [x] chunk 1 docs/src/architecture/terminal-exec-policy-ssot.md#Terminal Exec Policy SSOT (2026) › What this policy is NOT
- [x] doc 1 docs/src/contributors/agent-instruction-architecture.md
- [x] chunk 1 docs/src/contributors/agent-instruction-architecture.md#Agent instruction architecture › Command policy strategy (PowerShell-first)
- [x] doc 1 docs/src/ci/runner-contract.md
- [x] chunk 1 docs/src/ci/runner-contract.md#CI runner contract › Agent / local terminal vs CI shell

## B11 [why] (paraphrase of q035) How did two different Rust TLS crypto backends both land in Cargo.lock?

- [ ] keep
- [x] doc 2 docs/src/architecture/cryptography-ssot-2026.md
- [x] chunk 2 docs/src/architecture/cryptography-ssot-2026.md#Cryptography Policy SSoT › Correction (2026-09-04) — this document was unenforced and self-contradictory

## B12 [troubleshoot] (paraphrase of q036) Running the drift check manually complains about crates we already whitelisted, yet the commit hook is green. What is going on?

- [ ] keep
- [x] doc 2 docs/src/contributors/vox-binary-freshness-and-ssot-2026.md
- [x] chunk 2 docs/src/contributors/vox-binary-freshness-and-ssot-2026.md#Vox Binary Freshness & Single-Source-of-Truth (2026) › 1. The incident
- [x] chunk 1 docs/src/contributors/vox-binary-freshness-and-ssot-2026.md#Vox Binary Freshness & Single-Source-of-Truth (2026) › 2. Two underlying problems

## B13 [troubleshoot] (paraphrase of q037) Cargo says the dependency I put in my crate's manifest can't be resolved from the top-level manifest. How do I fix it?

- [ ] keep
- [x] doc 2 docs/src/ci/workspace-root-manifest.md
- [x] chunk 2 docs/src/ci/workspace-root-manifest.md#Workspace root Cargo.toml (fix forward) › When Cargo errors with "not found in workspace.dependencies"
- [x] chunk 1 docs/src/ci/workspace-root-manifest.md#Workspace root Cargo.toml (fix forward)
- [x] doc 1 docs/src/contributors/dependency-policy.md
- [x] chunk 1 docs/src/contributors/dependency-policy.md#Dependency policy › Rust: single workspace pin

## B14 [troubleshoot] (paraphrase of q038) My Rust test run prints a notice that one test has been going for over a minute and then keeps hanging forever - is there a timeout?

- [ ] keep
- [x] doc 2 docs/src/ci/runner-contract.md
- [x] chunk 2 docs/src/ci/runner-contract.md#CI runner contract › Test hangs: cargo test vs cargo nextest

## B15 [troubleshoot] (paraphrase of q039) A script I launched with vox run terminates with code 77 - what does that code signify?

- [ ] keep
- [x] doc 2 docs/src/reference/isolation.md
- [x] chunk 2 docs/src/reference/isolation.md#Interpreter isolation › Exit codes
- [x] chunk 1 docs/src/reference/isolation.md#Interpreter isolation › --caps grammar
- [x] doc 1 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md

## B16 [troubleshoot] (paraphrase of q040) My node's attempt to join the mesh coordinator gets rejected as unauthorized, how do I diagnose it?

- [ ] keep
- [x] doc 2 docs/src/how-to/populi-quickstart.md
- [x] chunk 2 docs/src/how-to/populi-quickstart.md#Populi Quickstart › Troubleshooting
- [x] chunk 1 docs/src/how-to/populi-quickstart.md#Populi Quickstart › Token management
- [x] doc 2 docs/src/reference/populi.md
- [x] chunk 2 docs/src/reference/populi.md#Mesh / Populi SSOT (CPU-first) › Control-plane HTTP errors (stable text bodies)

## B17 [troubleshoot] (paraphrase of q041) The cost-aware model picker keeps selecting a pricey Claude model because it thinks it costs nothing - why?

- [ ] keep
- [x] doc 2 docs/src/architecture/model-catalog-ssot-2026.md
- [x] chunk 2 docs/src/architecture/model-catalog-ssot-2026.md#Model Catalog SSOT — Architecture & Implementation Plan 2026 › Part 1 — Confirmed Split-Brain Bugs (True Positives) › Bug 3 — AnthropicDirect Registers Expensive Models as is_free: true
- [x] chunk 1 docs/src/architecture/model-catalog-ssot-2026.md#Model Catalog SSOT — Architecture & Implementation Plan 2026 › Part 4 — Implementation Tasks › Wave 0 — Critical Bug Fixes (Regressions Today, No Architecture Change)

## B18 [troubleshoot] (paraphrase of q042) My secrets health check says the environment variable name I used for an API key is an outdated alias; how long before support for it ends?

- [ ] keep
- [x] doc 2 docs/src/reference/secrets-ssot.md
- [x] chunk 2 docs/src/reference/secrets-ssot.md#Secrets SSOT › Transition and Deprecation Window Policy
- [x] chunk 1 docs/src/reference/secrets-ssot.md#Secrets SSOT › Resolution Precedence

## B19 [multi-hop] (paraphrase of q043) I'm wiring up credentials for a new LLM vendor: which registry do I add it to, and what do I do if that credential gets exposed?

- [ ] keep
- [x] doc 2 docs/src/reference/secrets-ssot.md
- [x] chunk 2 docs/src/reference/secrets-ssot.md#Secrets SSOT › Secrets SSOT
- [x] doc 2 docs/src/operations/secrets-cloudless-ops-runbook.md
- [x] chunk 2 docs/src/operations/secrets-cloudless-ops-runbook.md#Secrets Cloudless Ops Runbook › Incident handling
- [x] doc 1 docs/src/operations/secrets-break-glass-runbook.md
- [x] chunk 1 docs/src/operations/secrets-break-glass-runbook.md#Secrets Break-Glass Runbook › Post-incident mandatory tasks
- [x] chunk 1 docs/src/reference/secrets-ssot.md#Secrets SSOT › Naming Convention

## B20 [multi-hop] (paraphrase of q044) If my durable workflow picks a branch using randomness, will a resumed run take the same path, and does the compiler warn me before I ship?

- [ ] keep
- [x] doc 2 docs/src/explanation/expl-actors-workflows.md
- [x] doc 2 docs/src/explanation/expl-durable-execution.md
- [x] doc 2 docs/src/adr/041-durable-functions-completion-2026.md
- [x] chunk 2 docs/src/explanation/expl-actors-workflows.md#Actors & Workflows › Compilation Phases
- [x] chunk 2 docs/src/explanation/expl-durable-execution.md#Explanation: Durable Execution › 4. Determinism Requirements
- [x] chunk 2 docs/src/adr/041-durable-functions-completion-2026.md#ADR 041: Durable functions completion (workflow, activity, actor, @scheduled) › Decision
- [x] doc 1 docs/src/architecture/vox-language-rules-phase5-effects-determinism-2026.md
- [x] chunk 1 docs/src/architecture/vox-language-rules-phase5-effects-determinism-2026.md#Vox Language Rules — Phase 5: Effect System & Workflow Determinism (2026-05-09) › Task 3: Workflow determinism hard rule
- [x] doc 1 docs/src/tutorials/tut-workflow-durability.md
- [x] chunk 1 docs/src/tutorials/tut-workflow-durability.md#Tutorial: Workflow Durability › 5. Best Practices
- [x] doc 1 docs/src/adr/019-durable-workflow-journal-contract-v1.md

## B21 [multi-hop] (paraphrase of q045) For a machine that joins the compute mesh, which distribution package do I install, and what resource caps are enforced on code sent by other peers?

- [ ] keep
- [x] doc 2 docs/src/how-to/how-to-pick-a-vox-bundle.md
- [x] doc 2 docs/src/reference/isolation.md
- [x] chunk 2 docs/src/how-to/how-to-pick-a-vox-bundle.md#Picking a Vox bundle › Quick-reference table
- [x] chunk 2 docs/src/reference/isolation.md#Interpreter isolation › Limits
- [x] doc 1 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md
- [x] chunk 1 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md#ADR 048: Interpreter is the execution and sandbox tier › Decision
- [x] chunk 1 docs/src/reference/isolation.md#Interpreter isolation › --caps grammar

## B22 [multi-hop] (paraphrase of q046) In the desktop app's quick-search box, selecting the documentation filter runs which backend component, and which indexes does it search?

- [ ] keep
- [x] doc 2 docs/src/reference/gui-navigation.md
- [x] chunk 2 docs/src/reference/gui-navigation.md#GUI navigation › OmniSearch › Search scopes (OmniSearch vs Search surface)
- [x] doc 2 docs/src/architecture/omni-search-audit-and-roadmap-2026.md
- [x] chunk 2 docs/src/architecture/omni-search-audit-and-roadmap-2026.md#Omni-Search Audit & Roadmap (2026) › 2. Current Search Architecture Map
- [x] chunk 1 docs/src/reference/gui-navigation.md#GUI navigation › OmniSearch
- [x] doc 1 docs/src/architecture/search-retrieval-ssot-2026.md
- [x] chunk 1 docs/src/architecture/search-retrieval-ssot-2026.md#Search & Retrieval SSOT (2026) › 6. MCP tools and GUI

## B23 [multi-hop] (paraphrase of q047) How do I turn on sending my local diagnostics to a server I control, and where are the endpoint and credential configured?

- [ ] keep
- [x] doc 2 docs/src/adr/023-optional-telemetry-remote-upload.md
- [x] doc 2 docs/src/reference/env-vars.md
- [x] chunk 2 docs/src/adr/023-optional-telemetry-remote-upload.md#ADR 023: Optional telemetry remote upload › Decision
- [x] chunk 2 docs/src/reference/env-vars.md#Environment variables (SSOT) › Optional telemetry upload (vox telemetry)
- [x] doc 1 docs/src/reference/secrets-ssot.md
- [x] doc 1 docs/src/reference/cli.md
- [x] chunk 1 docs/src/reference/cli.md#Reference: vox CLI (minimal compiler binary) › Tooling › vox telemetry
- [x] doc 1 docs/src/architecture/telemetry-trust-ssot.md

## B24 [multi-hop] (paraphrase of q048) If a pull request touches nothing but the lockfile, does the pipeline run every crate's tests, and how can I mimic that on my own machine before pushing?

- [ ] keep
- [x] doc 2 docs/src/ci/affected-crate-selective-ci.md
- [x] chunk 2 docs/src/ci/affected-crate-selective-ci.md#Affected-Crate Selective CI › Sentinels (force full)
- [x] chunk 2 docs/src/ci/affected-crate-selective-ci.md#Affected-Crate Selective CI › Local parity
- [x] doc 2 docs/src/contributors/local-ci-pre-push.md
- [x] chunk 1 docs/src/contributors/local-ci-pre-push.md#Local CI parity (pre-push) › Profiles
- [x] doc 1 docs/src/ci/runner-contract.md
- [x] chunk 1 docs/src/ci/runner-contract.md#CI runner contract › Local-first CI (required policy, ENFORCED)

## B25 [how-to] (gap: journeys) I'd like a start-to-finish walkthrough where I define a database model and a UI together without hand-writing any API layer - where do I begin?
pool: bm25 (tut-first-app.md #1, #5; journeys/fullstack-data.md #3; tut-getting-started #7, #8); grep: path `journeys|tut-first`; graph: unavailable; read: journeys/fullstack-data.md, tut-first-app headings

- [ ] keep
- [x] doc 2 docs/src/journeys/fullstack-data.md
- [x] doc 2 docs/src/tutorials/tut-first-app.md
- [x] chunk 2 docs/src/journeys/fullstack-data.md#Journey: One-File Full-Stack Data › The Vox Paradigm: No API Layer
- [x] chunk 1 docs/src/tutorials/tut-first-app.md#Tutorial: Building a Collaborative Task List › 2. Define the Data Model
- [x] doc 1 docs/src/tutorials/tut-getting-started.md
- [ ] doc 1 docs/src/architecture/boilerplate-reduction-design-brief-2026.md  (bm25 #4; ranked design brief, not a walkthrough)

## B26 [how-to] (gap: learning) Is there a set of exercises for practising how Rust moves and borrows values, written around this project's own code?
pool: bm25 (lesson-01 #2, #6, #9; lesson-00 #1; notes-session-01 #12); grep: path `learning/rust-via-vox`; graph: unavailable; read: learning dir headings

- [ ] keep
- [x] doc 2 docs/src/learning/rust-via-vox/lesson-01-ownership-pattern-drill.md
- [x] chunk 2 docs/src/learning/rust-via-vox/lesson-01-ownership-pattern-drill.md#Lesson 1 — Ownership in Practice: Reading Code Like a Reviewer
- [x] doc 1 docs/src/learning/rust-via-vox/lesson-00-syntax-primer.md
- [x] doc 1 docs/src/learning/rust-via-vox/notes-session-01.md
- [x] chunk 1 docs/src/learning/rust-via-vox/notes-session-01.md#Rust via Vox — Session 1 Notes: Ownership, Errors, Enums › 3. Stack vs Heap (and why ownership exists)
- [ ] doc 1 docs/src/architecture/vox-memory-model-audit-and-value-optimization-2026-06-05.md  (bm25 #7; Vox's own value semantics, not Rust ownership practice)

## B27 [where] (gap: api) Where do contributors keep the running list of things in the language that still lack proper documentation?
pool: bm25 (api/DOC_GAPS.md #1 on the related CLI-gap query; not top 12 on a wording-matched run); grep: path `api/` -> DOC_GAPS.md, vox-codegen-ts.md; graph: unavailable; read: DOC_GAPS.md

- [ ] keep
- [x] doc 2 docs/src/api/DOC_GAPS.md
- [x] chunk 2 docs/src/api/DOC_GAPS.md#Known Documentation Gaps & Backlog
- [x] chunk 1 docs/src/api/DOC_GAPS.md#Known Documentation Gaps & Backlog › High Priority
- [ ] doc 1 docs/src/contributors/documentation-governance.md  (bm25 weak hit; governance of authority, not a gap backlog)

## B28 [where] (gap: api) The vox-codegen-ts crate name keeps showing up in old notes - where does TypeScript output generation actually live now?
pool: bm25 (not in top 12 for the first wording; vox-codegen-ts.md only found by path); grep: `vox-codegen-ts` path and content -> api/vox-codegen-ts.md; graph: unavailable; read: api/vox-codegen-ts.md

- [ ] keep
- [x] doc 2 docs/src/api/vox-codegen-ts.md
- [x] chunk 2 docs/src/api/vox-codegen-ts.md#vox-codegen-ts API (deprecated stub)
- [x] chunk 1 docs/src/api/vox-codegen-ts.md#vox-codegen-ts API (deprecated stub) › Overview

## B29 [troubleshoot] (gap: ref) I followed a link to docs/src/ref/env-vars.md and it is nearly empty - where is the actual list of environment variables?
pool: bm25 (ref/env-vars.md #2, #9 on a related query; reference/env-vars.md via grep); grep: path `ref/env-vars`; graph: unavailable; read: ref/env-vars.md, ref/env-vars-ssot.md, reference/env-vars.md

- [ ] keep
- [x] doc 2 docs/src/reference/env-vars.md
- [x] chunk 2 docs/src/reference/env-vars.md#Environment variables (SSOT)
- [x] doc 1 docs/src/ref/env-vars.md
- [x] doc 1 docs/src/ref/env-vars-ssot.md
- [ ] doc 1 docs/src/architecture/env-var-ssot-design-and-handoff-2026-06-15.md  (bm25 #12; design of the registry, not the list)

## B30 [where] (gap: top-level files) There is a ref-cli page at the root of the docs folder that says legacy - which page is the canonical command reference?
pool: bm25 (reference/cli.md #4 and api/DOC_GAPS.md #1 on a related query); grep: path `ref-cli`; graph: unavailable; read: docs/src/ref-cli.md, reference/cli.md

- [ ] keep
- [x] doc 2 docs/src/reference/cli.md
- [x] chunk 2 docs/src/reference/cli.md#Reference: vox CLI (minimal compiler binary)
- [x] doc 1 docs/src/ref-cli.md
- [x] chunk 1 docs/src/ref-cli.md#CLI reference (redirect)
- [ ] doc 1 docs/src/reference/command-compliance.md  (bm25 #3; projection pipeline for the command registry, not the canonical reference)

## B31 [concept] (gap: case-studies) Is there an end-to-end account of taking a brand-new Vox web app from an empty directory to a live host?
pool: bm25 (case-studies/v0.5-production-deployment.md #2; cli-vox-deploy.md #1, #3); grep: path `case-studies`; graph: unavailable; read: case study headings and scenario, how-to-deploy headings

- [ ] keep
- [x] doc 2 docs/src/case-studies/v0.5-production-deployment.md
- [x] chunk 2 docs/src/case-studies/v0.5-production-deployment.md#Case Study: First Production Deployment (v0.5 Preview Gate) › Deployment Execution
- [x] chunk 1 docs/src/case-studies/v0.5-production-deployment.md#Case Study: First Production Deployment (v0.5 Preview Gate) › Scenario
- [x] doc 1 docs/src/how-to/how-to-deploy.md
- [x] doc 1 docs/src/reference/cli-vox-deploy.md
- [ ] doc 1 docs/src/architecture/mobile-rn-expo-implementation-spec-2026.md  (bm25 #6; mobile build artifacts, unrelated)

## B32 [where] (gap: examples) Where is a page of complete Vox programs that CI actually validates, covering a table, a read query and a write mutation?
pool: bm25 (examples/golden.md #4 earlier run on the MCP chunk; how-to-database.md via grep); grep: path `examples/golden`; graph: unavailable; read: golden.md headings and CRUD section

- [ ] keep
- [x] doc 2 docs/src/examples/golden.md
- [x] chunk 2 docs/src/examples/golden.md#Golden Examples: Working Vox Code › CRUD API — Table, Query, Mutation, and Endpoint
- [x] chunk 1 docs/src/examples/golden.md#Golden Examples: Working Vox Code
- [x] doc 1 docs/src/how-to/how-to-database.md

## B33 [concept] (gap: versioning) How is the version string that vox prints assembled - what are the build count and the commit hash parts?
pool: bm25 (vox-binary-freshness-and-ssot-2026.md #2, #3 on a related query); grep: `VOX_BUILD_NUMBER|\+build\.` across docs/src -> vox-binary-freshness-and-ssot-2026.md, where-things-live.md, others; graph: unavailable; read: freshness section 2

- [ ] keep
- [x] doc 2 docs/src/contributors/vox-binary-freshness-and-ssot-2026.md
- [x] chunk 2 docs/src/contributors/vox-binary-freshness-and-ssot-2026.md#Vox Binary Freshness & Single-Source-of-Truth (2026) › 2. Two underlying problems
- [x] chunk 1 docs/src/contributors/vox-binary-freshness-and-ssot-2026.md#Vox Binary Freshness & Single-Source-of-Truth (2026) › 3. Design — automatic staleness prevention › 3.1 Build-number freshness self-check (primary)
- [ ] doc 1 docs/src/architecture/where-things-live.md  (grep; mentions build metadata crate in passing)

## B34 [how-to] (gap: language syntax) How do I define a persistent record type in a Vox file and then read rows back through a typed read function?
pool: bm25 (journeys/fullstack-data.md #1; tut-getting-started #7; how-to-database.md and vox-db-language-surface.md via grep); grep: path `how-to-database|vox-db-language-surface`; graph: unavailable; read: how-to-database Defining a Table, vox-db-language-surface Declarations

- [ ] keep
- [x] doc 2 docs/src/how-to/how-to-database.md
- [x] doc 2 docs/src/reference/vox-db-language-surface.md
- [x] chunk 2 docs/src/how-to/how-to-database.md#How-To: The Database Layer › Defining a Table
- [x] chunk 2 docs/src/reference/vox-db-language-surface.md#Vox database language surface (canonical) › Declarations
- [x] chunk 1 docs/src/how-to/how-to-database.md#How-To: The Database Layer › Guarding Reads/Writes with query and mutation
- [x] doc 1 docs/src/tutorials/tut-getting-started.md
- [x] chunk 1 docs/src/tutorials/tut-getting-started.md#Getting Started with Vox › Step 3: Write a notes app
- [ ] doc 1 docs/src/journeys/fullstack-data.md  (bm25 #1; motivation for the one-file approach, snippet only)

## B35 [where] (gap: CLI surface) Which page enumerates all vox subcommands in one place, and is it hand-written?
pool: bm25 (cli-command-surface.generated.md not in top 12; cli.md #4 and command-compliance.md #3 on a related query); grep: path `cli-command-surface`; graph: unavailable; read: generated page header, cli.md

- [ ] keep
- [x] doc 2 docs/src/reference/cli-command-surface.generated.md
- [x] chunk 2 docs/src/reference/cli-command-surface.generated.md#CLI command surface (generated)
- [x] doc 2 docs/src/reference/cli.md
- [x] chunk 1 docs/src/reference/cli.md#Reference: vox CLI (minimal compiler binary)
- [x] doc 1 docs/src/reference/command-compliance.md
- [x] chunk 1 docs/src/reference/command-compliance.md#Command compliance › Command projection pipeline (SSOT chain)
