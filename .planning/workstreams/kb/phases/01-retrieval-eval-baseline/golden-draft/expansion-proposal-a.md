approved_by:

# Expansion proposal A (paraphrase candidates for q001..q024)

Agent-generated (AP-02, Pitfall 8): each candidate is a paraphrase / robustness probe, never a ground-truth query. `- [ ] keep` is left unchecked; the user decides in 01-10. Qrel rows are copied from the source query block in qrels-proposal-a.md (checked rows only). Origin on materialization: llm-expansion.

## A01 [where] (paraphrase of q001) Before I create a fresh package in this repo, is there a cheat sheet that maps functionality to the right folder?

- [ ] keep
- [x] doc 2 docs/src/architecture/where-things-live.md
- [x] doc 1 docs/src/architecture/2026-05-08-naming-and-guards-design.md
- [x] chunk 2 docs/src/architecture/where-things-live.md#Where Things Live › Quick reference: subsystem → crate (by layer)
- [x] chunk 1 docs/src/architecture/where-things-live.md#Where Things Live
- [x] chunk 1 docs/src/architecture/where-things-live.md#Where Things Live › When to NOT add a new crate

## A02 [where] (paraphrase of q002) Old logs mention a daemon under a retired name; which doc lists the official current names and the legacy ones that map onto them?

- [ ] keep
- [x] doc 2 docs/src/architecture/canonical-runtime-names.md
- [x] doc 1 docs/src/architecture/legacy-tombstone-remediation-ledger-2026.md
- [x] doc 1 docs/src/reference/orchestration-unified.md
- [x] chunk 2 docs/src/architecture/canonical-runtime-names.md#Canonical runtime names (daemon, MCP, env) › Orchestrator daemon binary
- [x] chunk 1 docs/src/architecture/canonical-runtime-names.md#Canonical runtime names (daemon, MCP, env)
- [x] chunk 1 docs/src/architecture/legacy-tombstone-remediation-ledger-2026.md#Legacy / tombstone remediation ledger (2026) › Orchestrator naming (vox-dei, vox-dei-d)

## A03 [where] (paraphrase of q003) Is there a prebuilt overnight build I can download and run instead of compiling myself, maybe selected by a release label?

- [ ] keep
- [x] doc 2 docs/src/architecture/nightly-builds-ssot.md
- [x] chunk 2 docs/src/architecture/nightly-builds-ssot.md#Nightly Builds › Using a nightly build locally
- [x] chunk 1 docs/src/architecture/nightly-builds-ssot.md#Nightly Builds › History: the removed nightly-tag.yml

## A04 [where] (paraphrase of q004) Which environment setting points the database layer at a remote hosted SQLite-compatible server?

- [ ] keep
- [x] doc 2 docs/src/reference/env-vars.md
- [x] doc 2 docs/src/how-to/how-to-voxdb-canonical-store.md
- [x] chunk 2 docs/src/reference/env-vars.md#Environment variables (SSOT) › Codex / Turso (vox-db, vox-package)
- [x] chunk 2 docs/src/how-to/how-to-voxdb-canonical-store.md#How to use the canonical VoxDB / Codex store › What is canonical?
- [x] doc 1 docs/src/adr/004-codex-arca-turso-ssot.md

## A05 [where] (paraphrase of q005) For durable runs, which table stores completed step results for later reuse, and which columns identify a row?

- [ ] keep
- [x] doc 2 docs/src/adr/019-durable-workflow-journal-contract-v1.md
- [x] doc 1 docs/src/explanation/expl-durable-execution.md
- [x] chunk 2 docs/src/adr/019-durable-workflow-journal-contract-v1.md#ADR 019: Durable workflow journal contract v1 › Decision
- [x] chunk 1 docs/src/explanation/expl-durable-execution.md#Explanation: Durable Execution › 5. Storage Backend

## A06 [where] (paraphrase of q006) Where is the timing evidence for why committing and pushing locally got so slow?

- [ ] keep
- [x] doc 2 docs/src/architecture/ci-and-build-loop-findings-2026.md
- [x] doc 1 docs/src/architecture/rust-build-loop-ssot-2026.md
- [x] chunk 2 docs/src/architecture/ci-and-build-loop-findings-2026.md#CI and build loop findings (2026-10-03) › Local loop: why a commit or push takes minutes to hours
- [x] chunk 1 docs/src/architecture/rust-build-loop-ssot-2026.md#Rust build loop: where the time goes and what is allowed to change it › Measured on 2026-10-03 (macOS arm64, load average about 3, a worktree on main)

## A07 [where] (paraphrase of q007) According to the taxonomy page, which crate is responsible for sandboxed abilities given to agents, like reading files or making web requests?

- [ ] keep
- [x] doc 2 docs/src/architecture/classification-ssot-2026.md
- [x] chunk 2 docs/src/architecture/classification-ssot-2026.md#Classification Taxonomy SSoT › Core Subsystems

## A08 [how-to] (paraphrase of q008) What are the steps to train our model on a MacBook with an M-series chip using its GPU?

- [ ] keep
- [x] doc 2 docs/src/how-to/how-to-train-mens-macos-metal.md
- [x] doc 1 docs/src/architecture/mens-metal-optimization-ssot-2026.md
- [x] doc 1 docs/src/reference/mens-training.md
- [x] chunk 2 docs/src/how-to/how-to-train-mens-macos-metal.md#How To: Train MENS on macOS Metal › Train the Metal pack
- [x] chunk 1 docs/src/how-to/how-to-train-mens-macos-metal.md#How To: Train MENS on macOS Metal › Install and pre-download
- [x] chunk 1 docs/src/how-to/how-to-train-mens-macos-metal.md#How To: Train MENS on macOS Metal › Run the automation

## A09 [how-to] (paraphrase of q009) Can I cap how many compile jobs the cargo coordinator admits at once on my machine?

- [ ] keep
- [x] doc 2 docs/src/contributors/build-broker-usage.md
- [x] chunk 2 docs/src/contributors/build-broker-usage.md#Build broker — machine-wide cargo coordination › Tunables (env)
- [x] chunk 1 docs/src/contributors/build-broker-usage.md#Build broker — machine-wide cargo coordination › How it works

## A10 [how-to] (paraphrase of q010) I dropped a folder with a SKILL.md into my repo; what makes Vox and other coding assistants find it?

- [ ] keep
- [x] doc 2 docs/src/reference/skill_marketplace.md
- [x] doc 1 docs/src/architecture/skill-ecosystem-interop-research-2026-06-12.md
- [x] doc 1 docs/src/reference/plugin-manifest.md
- [x] chunk 2 docs/src/reference/skill_marketplace.md#Vox Skill Marketplace › Discovery roots (precedence: high → low)
- [x] chunk 1 docs/src/reference/skill_marketplace.md#Vox Skill Marketplace › Two formats (do not conflate)
- [x] chunk 1 docs/src/reference/plugin-manifest.md#Plugin Manifest (Plugin.toml) › AgentSkills Compliance

## A11 [how-to] (paraphrase of q011) What is the procedure to join several of my own computers into a Populi mesh over a private VPN like Tailscale?

- [ ] keep
- [x] doc 2 docs/src/operations/populi-overlay-personal-cluster-runbook.md
- [x] doc 1 docs/src/reference/populi.md
- [x] chunk 2 docs/src/operations/populi-overlay-personal-cluster-runbook.md#Populi overlay personal cluster runbook › Enrollment (high level)
- [x] chunk 1 docs/src/operations/populi-overlay-personal-cluster-runbook.md#Populi overlay personal cluster runbook › Preconditions
- [x] chunk 1 docs/src/reference/populi.md#Mesh / Populi SSOT (CPU-first) › Appendix: TLS / WireGuard transport (P0-T5) › WireGuard / Tailscale Funnel (recommended for off-LAN)

## A12 [how-to] (paraphrase of q012) How can I get randomised-input checks and a crash-hunting harness for a function written in Vox?

- [ ] keep
- [x] doc 2 docs/src/how-to/how-to-unit-test.md
- [x] chunk 2 docs/src/how-to/how-to-unit-test.md#How-To: Test Your Logic › 3. Property Writing with @forall
- [x] chunk 2 docs/src/how-to/how-to-unit-test.md#How-To: Test Your Logic › 4. Fuzzing with @fuzz
- [x] chunk 1 docs/src/how-to/how-to-unit-test.md#How-To: Test Your Logic › Summary

## A13 [how-to] (paraphrase of q013) Can I point vox's chat at a local Ollama model rather than a hosted API?

- [ ] keep
- [x] doc 2 docs/src/how-to/how-to-model-routing.md
- [x] doc 1 docs/src/reference/installation.md
- [x] doc 1 docs/src/reference/mens-serving-ssot.md
- [x] chunk 2 docs/src/how-to/how-to-model-routing.md#Model Routing & Provider Cascade › How Model Selection Works › vox chat (CLI)
- [x] chunk 1 docs/src/how-to/how-to-model-routing.md#Model Routing & Provider Cascade › Provider Cascade
- [x] chunk 1 docs/src/how-to/how-to-model-routing.md#Model Routing & Provider Cascade › How Model Selection Works › Mens / Ollama base URL
- [x] chunk 1 docs/src/reference/installation.md#Installing Vox › AI provider keys › Layer 3: Ollama (optional, local)

## A14 [how-to] (paraphrase of q014) Fresh from-source install done; which commands scaffold an app, check types, and launch it?

- [ ] keep
- [x] doc 2 docs/src/tutorials/tut-getting-started.md
- [x] doc 1 docs/src/tutorials/tut-first-app.md
- [x] chunk 2 docs/src/tutorials/tut-getting-started.md#Getting Started with Vox › Step 2: Create a new project
- [x] chunk 2 docs/src/tutorials/tut-getting-started.md#Getting Started with Vox › Step 4: Type check
- [x] chunk 2 docs/src/tutorials/tut-getting-started.md#Getting Started with Vox › Step 6: Run
- [x] chunk 1 docs/src/tutorials/tut-getting-started.md#Getting Started with Vox › Step 5: Build

## A15 [concept] (paraphrase of q015) Vox has three building blocks, actor, activity and workflow - how do I decide which fits my case?

- [ ] keep
- [x] doc 2 docs/src/explanation/expl-actors-workflows.md
- [x] doc 1 docs/src/reference/ref-async-concurrency.md
- [x] chunk 2 docs/src/explanation/expl-actors-workflows.md#Actors & Workflows › Quick Comparison
- [x] chunk 1 docs/src/explanation/expl-actors-workflows.md#Actors & Workflows › Actors
- [x] chunk 1 docs/src/explanation/expl-actors-workflows.md#Actors & Workflows › Workflows
- [x] chunk 1 docs/src/explanation/expl-actors-workflows.md#Actors & Workflows › Activities

## A16 [concept] (paraphrase of q016) Which storage backend should hold what: the relational database, write-once log files, or hash-addressed objects?

- [ ] keep
- [x] doc 2 docs/src/architecture/data-storage-ssot-2026.md
- [x] chunk 2 docs/src/architecture/data-storage-ssot-2026.md#Data Storage SSOT (2026) › 4. Target state: four-tier data model › 4.1 Tier A — Relational (libSQL/Turso)
- [x] chunk 2 docs/src/architecture/data-storage-ssot-2026.md#Data Storage SSOT (2026) › 4. Target state: four-tier data model › 4.2 Tier B — Append-only JSONL spools
- [x] chunk 2 docs/src/architecture/data-storage-ssot-2026.md#Data Storage SSOT (2026) › 4. Target state: four-tier data model › 4.3 Tier C — Content-addressed artifacts
- [x] chunk 1 docs/src/architecture/data-storage-ssot-2026.md#Data Storage SSOT (2026) › 4. Target state: four-tier data model

## A17 [concept] (paraphrase of q017) What does Socrates do, and which steps does it run before the system replies?

- [ ] keep
- [x] doc 2 docs/src/reference/socrates-protocol.md
- [x] doc 1 docs/src/adr/005-socrates-anti-hallucination-ssot.md
- [x] chunk 2 docs/src/reference/socrates-protocol.md#Socrates protocol — single source of truth › Protocol states
- [x] chunk 1 docs/src/reference/socrates-protocol.md#Socrates protocol — single source of truth

## A18 [concept] (paraphrase of q018) How does the agent tool server tag each call as harmless, local-write, or having an external effect?

- [ ] keep
- [x] doc 2 docs/src/architecture/agentos-ssot-2026.md
- [x] chunk 2 docs/src/architecture/agentos-ssot-2026.md#AgentOS & Agent-Computer Interface SSOT (2026) › 3. Mutation classification

## A19 [concept] (paraphrase of q019) What is the intermediate representation the compiler emits as JSON, and can I rely on its format staying the same?

- [ ] keep
- [x] doc 2 docs/src/reference/vox-ir-specification.md
- [x] chunk 2 docs/src/reference/vox-ir-specification.md#Vox IR Specification
- [x] chunk 2 docs/src/reference/vox-ir-specification.md#Vox IR Specification › Stability guarantees
- [x] chunk 1 docs/src/reference/vox-ir-specification.md#Vox IR Specification › Purpose

## A20 [concept] (paraphrase of q020) If the process dies partway through a workflow, do steps that already completed run again on restart?

- [ ] keep
- [x] doc 2 docs/src/explanation/expl-durable-execution.md
- [x] doc 1 docs/src/tutorials/tut-workflow-durability.md
- [x] chunk 2 docs/src/explanation/expl-durable-execution.md#Explanation: Durable Execution › 2. Recovery via Replay
- [x] chunk 2 docs/src/explanation/expl-durable-execution.md#Explanation: Durable Execution › 3. Exactly-Once Semantics
- [x] chunk 1 docs/src/explanation/expl-durable-execution.md#Explanation: Durable Execution › 1. The Journal System

## A21 [concept] (paraphrase of q021) How is search logic kept in one place so the CLI, orchestrator and MCP server don't each duplicate it?

- [ ] keep
- [x] doc 2 docs/src/architecture/search-retrieval-ssot-2026.md
- [x] chunk 2 docs/src/architecture/search-retrieval-ssot-2026.md#Search & Retrieval SSOT (2026) › 2. Canonical pipeline
- [x] chunk 1 docs/src/architecture/search-retrieval-ssot-2026.md#Search & Retrieval SSOT (2026) › 1. Scope

## A22 [policy] (paraphrase of q022) Is it OK to add the blake3 or sha2 crate to my Cargo.toml directly, or must hashing go through a shared crate?

- [ ] keep
- [x] doc 2 docs/src/architecture/cryptography-ssot-2026.md
- [x] doc 1 docs/src/reference/ref-builtins-hashing.md
- [x] chunk 2 docs/src/architecture/cryptography-ssot-2026.md#Cryptography Policy SSoT › Allowed Primitives
- [x] chunk 1 docs/src/reference/ref-builtins-hashing.md#Hashing & Identity Builtins › Workspace hash algorithm map (Rust tooling)

## A23 [policy] (paraphrase of q023) Can we copy source from an AGPL-licensed repository into ours, and what attribution do we keep when vendoring permissive code?

- [ ] keep
- [x] doc 2 docs/src/adr/026-third-party-code-provenance.md
- [x] doc 1 docs/src/architecture/warp-research-findings-2026.md
- [x] chunk 2 docs/src/adr/026-third-party-code-provenance.md#ADR-026: Third-Party Code Provenance Policy › Decision › 2. Prohibited intake
- [x] chunk 2 docs/src/adr/026-third-party-code-provenance.md#ADR-026: Third-Party Code Provenance Policy › Decision › 3. Attribution requirements for vendored code
- [x] chunk 1 docs/src/adr/026-third-party-code-provenance.md#ADR-026: Third-Party Code Provenance Policy › Decision › 1. Approved intake paths (in priority order)

## A24 [policy] (paraphrase of q024) What header metadata must a new docs page start with, and what are the permitted category names?

- [ ] keep
- [x] doc 2 docs/src/contributors/documentation-governance.md
- [x] doc 1 docs/src/AGENTS.md
- [x] chunk 2 docs/src/contributors/documentation-governance.md#Documentation governance › Taxonomy › Category vocabulary
- [x] chunk 2 docs/src/contributors/documentation-governance.md#Documentation governance › Taxonomy › Frontmatter starter template
- [x] chunk 1 docs/src/contributors/documentation-governance.md#Documentation governance › Taxonomy › Status vocabulary
- [x] chunk 1 docs/src/AGENTS.md#AGENTS.md › Frontmatter requirements (all new pages)
