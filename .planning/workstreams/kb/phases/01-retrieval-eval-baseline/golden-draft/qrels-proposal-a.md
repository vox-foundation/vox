approved_by:

# Qrels proposal A (q001..q024)

Agent-proposed (AP-02); the user approves in 01-10. Score: 2 = answers, 1 = relevant. `- [ ]` rows are pool items considered and rejected; flip to `[x]` to accept. Query origin: agent-seed (01-CONTEXT D-02). Pool methods: bm25 = `vox kb eval --query --top 20`; grep = path/content grep excluding docs/src/archive; graph = `vox graph` (stale, so unavailable); seed-sources = seed author intended docs, used as one extra pool source only.

## q001 [where] I'm adding a new subsystem - is there a lookup that tells me which crate it should go in, by layer?
pool: bm25 (where-things-live.md #2, #16; 2026-05-08-naming-and-guards-design.md #7, #9); grep: path `where-things|layers` -> where-things-live.md; graph: unavailable; read: where-things-live intro + layer sections
answer: "Flat lookup table for 'I need to add/find X - where does it go?' ... Quick reference: subsystem -> crate (by layer)" (chunk: where-things-live.md#Where Things Live › Quick reference: subsystem → crate (by layer))
- [x] doc 2 docs/src/architecture/where-things-live.md
- [x] doc 1 docs/src/architecture/2026-05-08-naming-and-guards-design.md
- [x] chunk 2 docs/src/architecture/where-things-live.md#Where Things Live › Quick reference: subsystem → crate (by layer)
- [x] chunk 1 docs/src/architecture/where-things-live.md#Where Things Live
- [x] chunk 1 docs/src/architecture/where-things-live.md#Where Things Live › When to NOT add a new crate
- [ ] doc 1 docs/src/learning/rust-via-vox/lesson-00-syntax-primer.md  (bm25 #1; off-topic lexical match)

## q002 [where] Which page tracks the current names of the daemon binary and env var families versus old aliases like vox-dei-d?
pool: bm25 (canonical-runtime-names.md #1, #2; orchestration-unified.md #4; legacy-tombstone-remediation-ledger-2026.md #9; env-var-ssot-design-and-handoff-2026-06-15.md #3, #5); grep: path `runtime-names|canonical` -> canonical-runtime-names.md; graph: unavailable; read: orchestrator daemon section, ledger and compat matrix
answer: "Canonical `vox-orchestrator-d` ... Deprecated path names (`vox-dei-d`, prose-only 'DEI daemon') ... treat them as aliases of the same IPC peer" (chunk: canonical-runtime-names.md#Canonical runtime names (daemon, MCP, env) › Orchestrator daemon binary)
- [x] doc 2 docs/src/architecture/canonical-runtime-names.md
- [x] doc 1 docs/src/architecture/legacy-tombstone-remediation-ledger-2026.md
- [x] doc 1 docs/src/reference/orchestration-unified.md
- [x] chunk 2 docs/src/architecture/canonical-runtime-names.md#Canonical runtime names (daemon, MCP, env) › Orchestrator daemon binary
- [x] chunk 1 docs/src/architecture/canonical-runtime-names.md#Canonical runtime names (daemon, MCP, env)
- [x] chunk 1 docs/src/architecture/legacy-tombstone-remediation-ledger-2026.md#Legacy / tombstone remediation ledger (2026) › Orchestrator naming (vox-dei, vox-dei-d)
- [ ] doc 1 docs/src/architecture/env-var-ssot-design-and-handoff-2026-06-15.md  (bm25 #3; a design/handoff for env vars, not the page that tracks current names)

## q003 [where] Where can I get a nightly binary without building from source, is there a tag I can pull?
pool: bm25 (nightly-builds-ssot.md #1, #5, #7; vox-binary-freshness-and-ssot-2026.md #3, #10); grep: path `nightly` -> nightly-builds-ssot.md; graph: unavailable; read: nightly-builds-ssot.md
answer: "There is no public nightly release to fetch by tag ... pull the artifact directly from the workflow run (`gh run download`) ... `voxup install --tag` works only for real published release tags" (chunk: nightly-builds-ssot.md#Nightly Builds › Using a nightly build locally)
- [x] doc 2 docs/src/architecture/nightly-builds-ssot.md
- [x] chunk 2 docs/src/architecture/nightly-builds-ssot.md#Nightly Builds › Using a nightly build locally
- [x] chunk 1 docs/src/architecture/nightly-builds-ssot.md#Nightly Builds › History: the removed nightly-tag.yml
- [ ] doc 1 docs/src/contributors/vox-binary-freshness-and-ssot-2026.md  (bm25 #3; binary freshness incident, not nightly retrieval)

## q004 [where] what env var tells vox-db which remote libSQL database to connect to?
pool: bm25 (how-to-voxdb-canonical-store.md #1; env-vars.md #2, #5; adr/004-codex-arca-turso-ssot.md #4); grep: `VOX_DB_URL` across docs/src -> env-vars.md, how-to-voxdb-canonical-store.md, adr/004, codex-http-api.md, voxdb-cutover-runbook.md; graph: unavailable; read: env-vars Codex/Turso table, canonical-store how-to, ADR-004
answer: "`VOX_DB_URL` | Remote libSQL / Turso URL (with `VOX_DB_TOKEN`)" (chunk: env-vars.md#Environment variables (SSOT) › Codex / Turso (vox-db, vox-package))
- [x] doc 2 docs/src/reference/env-vars.md
- [x] doc 2 docs/src/how-to/how-to-voxdb-canonical-store.md
- [x] chunk 2 docs/src/reference/env-vars.md#Environment variables (SSOT) › Codex / Turso (vox-db, vox-package)
- [x] chunk 2 docs/src/how-to/how-to-voxdb-canonical-store.md#How to use the canonical VoxDB / Codex store › What is canonical?
- [x] doc 1 docs/src/adr/004-codex-arca-turso-ssot.md
- [ ] doc 1 docs/src/architecture/data-storage-lint-and-ci-spec-2026.md  (bm25 #3; grep rules, mentions env reads only as lint targets)

## q005 [where] which DB table holds the per-activity replay log for durable workflows, and what is it keyed on?
pool: bm25 (adr/019-durable-workflow-journal-contract-v1.md #1; expl-durable-execution.md #6, #9, #14; true-workflow-durability-design-2026.md #2); grep: `workflow_activity_log` across docs/src; graph: unavailable; read: ADR-019 Decision, expl-durable-execution section 5
answer: "durable replay is keyed by `(run_id, workflow_name, activity_id)` in `workflow_activity_log`" (chunk: adr/019...#ADR 019: Durable workflow journal contract v1 › Decision)
- [x] doc 2 docs/src/adr/019-durable-workflow-journal-contract-v1.md
- [x] doc 1 docs/src/explanation/expl-durable-execution.md
- [x] chunk 2 docs/src/adr/019-durable-workflow-journal-contract-v1.md#ADR 019: Durable workflow journal contract v1 › Decision
- [x] chunk 1 docs/src/explanation/expl-durable-execution.md#Explanation: Durable Execution › 5. Storage Backend
- [ ] doc 1 docs/src/architecture/true-workflow-durability-design-2026.md  (bm25 #2; design discussion, does not name the table or its key)

## q006 [where] where's the measured breakdown of why local commits and pushes were taking minutes to hours?
pool: bm25 (ci-and-build-loop-findings-2026.md #2, #7, #14; rust-build-loop-ssot-2026.md #8; work-loss-audit-and-handoff-2026-05-24.md #3; mens-cloud-training-economics-2026-09-11.md #1); grep: `minutes to hours|syspolicyd` in docs/src/architecture; graph: unavailable; read: findings Local loop table, rust-build-loop-ssot measured sections
answer: "Hooks build binaries, pre-push runs clippy --workspace (2.5 h observed), build jobs 24 vs broker cap 6, kernel-time CPU, 48.5 min vox-cli debug build" (chunk: ci-and-build-loop-findings-2026.md#CI and build loop findings (2026-10-03) › Local loop: why a commit or push takes minutes to hours)
- [x] doc 2 docs/src/architecture/ci-and-build-loop-findings-2026.md
- [x] doc 1 docs/src/architecture/rust-build-loop-ssot-2026.md
- [x] chunk 2 docs/src/architecture/ci-and-build-loop-findings-2026.md#CI and build loop findings (2026-10-03) › Local loop: why a commit or push takes minutes to hours
- [x] chunk 1 docs/src/architecture/rust-build-loop-ssot-2026.md#Rust build loop: where the time goes and what is allowed to change it › Measured on 2026-10-03 (macOS arm64, load average about 3, a worktree on main)
- [ ] doc 1 docs/src/architecture/work-loss-audit-and-handoff-2026-05-24.md  (bm25 #3; commits/work loss audit, different topic)
- [ ] doc 1 docs/src/architecture/mens-cloud-training-economics-2026-09-11.md  (bm25 #1; lexical match on "default is local", unrelated)

## q007 [where] In the component taxonomy, which crate owns agent capabilities like file or network access?
pool: bm25 (classification-ssot-2026.md #1; plugin-system-deep-audit-2026-05-08.md #4); grep: `component taxonomy|classification` in docs/src; graph: unavailable; read: classification-ssot Core Subsystems table
answer: "Capabilities | `vox-skills` | Isolated agent capabilities (e.g., file system access, network requests)" (chunk: classification-ssot-2026.md#Classification Taxonomy SSoT › Core Subsystems)
- [x] doc 2 docs/src/architecture/classification-ssot-2026.md
- [x] chunk 2 docs/src/architecture/classification-ssot-2026.md#Classification Taxonomy SSoT › Core Subsystems
- [ ] doc 1 docs/src/architecture/plugin-system-deep-audit-2026-05-08.md  (bm25 #4; sandbox model of plugins, not the crate taxonomy)

## q008 [how-to] how do I fine-tune MENS on an Apple Silicon laptop using Metal?
pool: bm25 (how-to-train-mens-macos-metal.md #9, #14; mens-metal-optimization-ssot-2026.md #2, #7, #8, #10, #11; burn-necessity-audit-2026-05-08.md #1, #6); grep: path `metal|mens-train|macos` -> mens-training.md (not in bm25 top 20); graph: unavailable; read: macos-metal how-to, mens-training Metal lines
answer: "`vox mens train --backend qlora --tokenizer hf --device metal --model $SPIKE_MODEL_ID --data-dir examples/mens/metal-e2e ...` after building vox-ml-cli with gpu features" (chunk: how-to-train-mens-macos-metal.md#How To: Train MENS on macOS Metal › Train the Metal pack)
- [x] doc 2 docs/src/how-to/how-to-train-mens-macos-metal.md
- [x] doc 1 docs/src/architecture/mens-metal-optimization-ssot-2026.md
- [x] doc 1 docs/src/reference/mens-training.md
- [x] chunk 2 docs/src/how-to/how-to-train-mens-macos-metal.md#How To: Train MENS on macOS Metal › Train the Metal pack
- [x] chunk 1 docs/src/how-to/how-to-train-mens-macos-metal.md#How To: Train MENS on macOS Metal › Install and pre-download
- [x] chunk 1 docs/src/how-to/how-to-train-mens-macos-metal.md#How To: Train MENS on macOS Metal › Run the automation
- [ ] doc 1 docs/src/architecture/burn-necessity-audit-2026-05-08.md  (bm25 #1; Burn framework audit, not a how-to)

## q009 [how-to] how do I limit how many cargo builds the broker lets run at once on this box?
pool: bm25 (build-broker-usage.md #1-#6, #10; ci-and-build-loop-findings-2026.md #7, #17); grep: path `broker` -> build-broker-usage.md; graph: unavailable; read: build-broker-usage Tunables
answer: "`VOX_BROKER_MAX_CONCURRENT` - optional upper bound on simultaneous builds ... `VOX_BROKER_RESERVED_SLOTS` - slots subtracted from the host cap" (chunk: build-broker-usage.md#Build broker — machine-wide cargo coordination › Tunables (env))
- [x] doc 2 docs/src/contributors/build-broker-usage.md
- [x] chunk 2 docs/src/contributors/build-broker-usage.md#Build broker — machine-wide cargo coordination › Tunables (env)
- [x] chunk 1 docs/src/contributors/build-broker-usage.md#Build broker — machine-wide cargo coordination › How it works
- [ ] doc 1 docs/src/architecture/ci-and-build-loop-findings-2026.md  (bm25 #7; reports the broker cap value but does not say how to set it)

## q010 [how-to] how do I get a plain SKILL.md folder picked up by vox and by the other agent harnesses?
pool: bm25 (none of the answering docs in top 20; how-to-ai-agents.md #1 only); grep: `agentskills|\.agents/skills` across docs/src -> skill_marketplace.md, plugin-manifest.md, skill-ecosystem-interop-research-2026-06-12.md; graph: unavailable; seed-sources: skill-ecosystem-interop-research-2026-06-12.md; read: skill_marketplace Discovery roots, plugin-manifest AgentSkills section
answer: "Discovery roots (precedence high to low): `<workspace>/.vox/skills/`, `.cursor/skills/`, `.agents/skills/`, `.claude/skills/`, then `~/` equivalents; each skill is `<root>/<name>/SKILL.md`" (chunk: skill_marketplace.md#Vox Skill Marketplace › Discovery roots (precedence: high → low))
- [x] doc 2 docs/src/reference/skill_marketplace.md
- [x] doc 1 docs/src/architecture/skill-ecosystem-interop-research-2026-06-12.md
- [x] doc 1 docs/src/reference/plugin-manifest.md
- [x] chunk 2 docs/src/reference/skill_marketplace.md#Vox Skill Marketplace › Discovery roots (precedence: high → low)
- [x] chunk 1 docs/src/reference/skill_marketplace.md#Vox Skill Marketplace › Two formats (do not conflate)
- [x] chunk 1 docs/src/reference/plugin-manifest.md#Plugin Manifest (Plugin.toml) › AgentSkills Compliance
- [ ] doc 1 docs/src/how-to/how-to-ai-agents.md  (bm25 #1; defining agent roles, not SKILL.md folders)

## q011 [how-to] steps to run a populi cluster across my own machines linked by Tailscale or WireGuard
pool: bm25 (populi-overlay-personal-cluster-runbook.md #1, #6; populi.md #2, #9, #18; adr/047-iroh-transport.md #8); grep: `tailscale|wireguard|overlay` in docs/src/operations, reference, how-to; graph: unavailable; read: overlay runbook, populi.md WireGuard appendix
answer: "Enrollment: bring up the overlay, deploy the control plane reachable by overlay peers, point workers at `VOX_MESH_CONTROL_ADDR` with the overlay URL, join + heartbeat, bootstrap tokens" (chunk: populi-overlay-personal-cluster-runbook.md#Populi overlay personal cluster runbook › Enrollment (high level))
- [x] doc 2 docs/src/operations/populi-overlay-personal-cluster-runbook.md
- [x] doc 1 docs/src/reference/populi.md
- [x] chunk 2 docs/src/operations/populi-overlay-personal-cluster-runbook.md#Populi overlay personal cluster runbook › Enrollment (high level)
- [x] chunk 1 docs/src/operations/populi-overlay-personal-cluster-runbook.md#Populi overlay personal cluster runbook › Preconditions
- [x] chunk 1 docs/src/reference/populi.md#Mesh / Populi SSOT (CPU-first) › Appendix: TLS / WireGuard transport (P0-T5) › WireGuard / Tailscale Funnel (recommended for off-LAN)
- [ ] doc 1 docs/src/adr/047-iroh-transport.md  (bm25 #8; transport decision, rejected-alternatives mention only)

## q012 [how-to] how do I write property tests or a fuzz target for a function in a .vox file?
pool: bm25 (how-to-unit-test.md #1, #19; semantic-coverage-remediation-plan-2026-06-13.md #2); grep: `@forall|@fuzz` in docs/src/how-to; graph: unavailable; read: how-to-unit-test sections 3 and 4
answer: "`@forall fn test_addition_commutative(a: int, b: int) to Unit { assert(...) }` generates random inputs; `@fuzz fn fuzz_parser(input: str)` drives the native fuzzer" (chunks: how-to-unit-test.md#How-To: Test Your Logic › 3. Property Writing with @forall and › 4. Fuzzing with @fuzz)
- [x] doc 2 docs/src/how-to/how-to-unit-test.md
- [x] chunk 2 docs/src/how-to/how-to-unit-test.md#How-To: Test Your Logic › 3. Property Writing with @forall
- [x] chunk 2 docs/src/how-to/how-to-unit-test.md#How-To: Test Your Logic › 4. Fuzzing with @fuzz
- [x] chunk 1 docs/src/how-to/how-to-unit-test.md#How-To: Test Your Logic › Summary
- [ ] doc 1 docs/src/architecture/semantic-coverage-remediation-plan-2026-06-13.md  (bm25 #2; plan on how to write compiler tests, not a user how-to)

## q013 [how-to] make vox chat use a model I pulled into Ollama instead of a cloud provider
pool: bm25 (how-to-model-routing.md #8, #11, #18; coding-agent-local-model-ux-comparison-2026-07-30.md #1, #6; vox-harness docs #2-#5); grep: `ollama` counts across docs/src reference/how-to -> installation.md, mens-serving-ssot.md; graph: unavailable; read: model-routing vox chat and cascade sections, installation Layer 3
answer: "Check for Ollama -> fall back to local inference at localhost:11434 (VOX_POPULI_LOCAL_OLLAMA_URL, POPULI_URL, OLLAMA_URL); Vox auto-detects a pulled model" (chunk: how-to-model-routing.md#Model Routing & Provider Cascade › How Model Selection Works › vox chat (CLI))
- [x] doc 2 docs/src/how-to/how-to-model-routing.md
- [x] doc 1 docs/src/reference/installation.md
- [x] doc 1 docs/src/reference/mens-serving-ssot.md
- [x] chunk 2 docs/src/how-to/how-to-model-routing.md#Model Routing & Provider Cascade › How Model Selection Works › vox chat (CLI)
- [x] chunk 1 docs/src/how-to/how-to-model-routing.md#Model Routing & Provider Cascade › Provider Cascade
- [x] chunk 1 docs/src/how-to/how-to-model-routing.md#Model Routing & Provider Cascade › How Model Selection Works › Mens / Ollama base URL
- [x] chunk 1 docs/src/reference/installation.md#Installing Vox › AI provider keys › Layer 3: Ollama (optional, local)
- [ ] doc 1 docs/src/architecture/coding-agent-local-model-ux-comparison-2026-07-30.md  (bm25 #1; compares other tools such as Continue.dev, not Vox usage)

## q014 [how-to] I just built vox from source, walk me through making a new project, type-checking it and running it
pool: bm25 (tut-getting-started.md and tut-first-app.md not in top 20; v1-readiness-status-2026-07.md #1; faq.md #7); grep: path `getting-started|quickstart|first-app` -> tut-getting-started.md, tut-first-app.md; `vox init|vox check` in docs/src/tutorials; graph: unavailable; seed-sources: tut-getting-started.md; read: both tutorials
answer: "`vox init my-app`, then `vox check src/main.vox`, `vox build src/main.vox -o dist`, `vox run src/main.vox`" (chunks: tut-getting-started.md Steps 2, 4, 6)
- [x] doc 2 docs/src/tutorials/tut-getting-started.md
- [x] doc 1 docs/src/tutorials/tut-first-app.md
- [x] chunk 2 docs/src/tutorials/tut-getting-started.md#Getting Started with Vox › Step 2: Create a new project
- [x] chunk 2 docs/src/tutorials/tut-getting-started.md#Getting Started with Vox › Step 4: Type check
- [x] chunk 2 docs/src/tutorials/tut-getting-started.md#Getting Started with Vox › Step 6: Run
- [x] chunk 1 docs/src/tutorials/tut-getting-started.md#Getting Started with Vox › Step 5: Build
- [ ] doc 1 docs/src/architecture/v1-readiness-status-2026-07.md  (bm25 #1; readiness status summary, not a walkthrough)

## q015 [concept] actor vs activity vs workflow in Vox, when would I reach for each?
pool: bm25 (expl-actors-workflows.md #7, #11, #15; ref-async-concurrency.md #10; adr/028-deprecate-stub-durability-grammar.md #1, #6, #19); grep: path `durab|workflow` -> adr/019, 021, 028, 041, expl-actors-workflows via read; graph: unavailable; seed-sources: expl-actors-workflows.md; read: Quick Comparison, ref-async-concurrency
answer: "Quick Comparison: Actor = `fn ActorName_Handler(state, ...)`, lives in memory, runtime checkpoints; Workflow = replayable journal in Codex; Activity = individual retryable step, idempotent" (chunk: expl-actors-workflows.md#Actors & Workflows › Quick Comparison)
- [x] doc 2 docs/src/explanation/expl-actors-workflows.md
- [x] doc 1 docs/src/reference/ref-async-concurrency.md
- [x] chunk 2 docs/src/explanation/expl-actors-workflows.md#Actors & Workflows › Quick Comparison
- [x] chunk 1 docs/src/explanation/expl-actors-workflows.md#Actors & Workflows › Actors
- [x] chunk 1 docs/src/explanation/expl-actors-workflows.md#Actors & Workflows › Workflows
- [x] chunk 1 docs/src/explanation/expl-actors-workflows.md#Actors & Workflows › Activities
- [ ] doc 1 docs/src/adr/028-deprecate-stub-durability-grammar.md  (bm25 #1; the stub-grammar removal gate, superseded by ADR-041 per AGENTS policy)

## q016 [concept] what kinds of data go in libSQL vs append-only logs vs content-addressed blobs?
pool: bm25 (data-storage-ssot-2026.md #1, #3, #20; context-window-management-design-2026-06-20.md #2, #14); grep: `content-addressed|four-tier` across docs/src (only data-storage-ssot defines the tiers); graph: unavailable; seed-sources: data-storage-ssot-2026.md; read: four-tier section 4.1-4.3
answer: "Tier A relational libSQL for anything with identity/relations; Tier B append-only JSONL spools for hot-path event streams; Tier C content-addressed artifacts" (chunks: data-storage-ssot-2026.md#...› 4. Target state: four-tier data model › 4.1 / 4.2 / 4.3)
- [x] doc 2 docs/src/architecture/data-storage-ssot-2026.md
- [x] chunk 2 docs/src/architecture/data-storage-ssot-2026.md#Data Storage SSOT (2026) › 4. Target state: four-tier data model › 4.1 Tier A — Relational (libSQL/Turso)
- [x] chunk 2 docs/src/architecture/data-storage-ssot-2026.md#Data Storage SSOT (2026) › 4. Target state: four-tier data model › 4.2 Tier B — Append-only JSONL spools
- [x] chunk 2 docs/src/architecture/data-storage-ssot-2026.md#Data Storage SSOT (2026) › 4. Target state: four-tier data model › 4.3 Tier C — Content-addressed artifacts
- [x] chunk 1 docs/src/architecture/data-storage-ssot-2026.md#Data Storage SSOT (2026) › 4. Target state: four-tier data model
- [ ] doc 1 docs/src/architecture/context-window-management-design-2026-06-20.md  (bm25 #2; context-window design, not the storage tiers)

## q017 [concept] Socrates protocol: what is it and what states does a question move through?
pool: bm25 (socrates-protocol.md #1, #8; adr/005-socrates-anti-hallucination-ssot.md #9; cli-gui-surface-coverage-map-2026.md #2); grep: path `socrates` -> socrates-protocol.md, adr/005; graph: unavailable; read: Protocol states
answer: "Protocol states: Retrieve, Verify, Calibrate, Gate (RiskDecision Answer/Ask/Abstain), Persist" (chunk: socrates-protocol.md#Socrates protocol — single source of truth › Protocol states)
- [x] doc 2 docs/src/reference/socrates-protocol.md
- [x] doc 1 docs/src/adr/005-socrates-anti-hallucination-ssot.md
- [x] chunk 2 docs/src/reference/socrates-protocol.md#Socrates protocol — single source of truth › Protocol states
- [x] chunk 1 docs/src/reference/socrates-protocol.md#Socrates protocol — single source of truth
- [ ] doc 1 docs/src/architecture/cli-gui-surface-coverage-map-2026.md  (bm25 #2; surface coverage map, incidental Socrates mention)

## q018 [concept] how are MCP tool calls labeled by side-effect level, e.g. read-only vs touching the network?
pool: bm25 (agentos-ssot-2026.md #1; mcp_serverless_research.md #2); grep: `mutation_kind` across docs/src -> agentos-ssot, where-things-live, vox-stdlib-gap-audit, ai-first-fixtures-research; graph: unavailable; read: agentos-ssot section 3
answer: "Every MCP tool dispatch SHOULD attach `aci.mutation_kind`: read_only, local_mutation, external_side_effect (network, spend, host-global), unknown" (chunk: agentos-ssot-2026.md#AgentOS & Agent-Computer Interface SSOT (2026) › 3. Mutation classification)
- [x] doc 2 docs/src/architecture/agentos-ssot-2026.md
- [x] chunk 2 docs/src/architecture/agentos-ssot-2026.md#AgentOS & Agent-Computer Interface SSOT (2026) › 3. Mutation classification
- [ ] doc 1 docs/src/explanation/mcp_serverless_research.md  (bm25 #2; serverless MCP deployment research, no side-effect labels)

## q019 [concept] what does the Vox IR represent and how stable is it meant to be?
pool: bm25 (vox-ir-specification.md not in top 20: bm25 misses; faq.md #1 only); grep: path `ir-spec|vox-ir` -> vox-ir-specification.md; graph: unavailable; seed-sources: vox-ir-specification.md; read: whole spec head, Purpose and Stability sections
answer: "The canonical, machine-verifiable JSON bundle for a program after type checking; v2.x keeps a predictable module-level shape, breaking changes bump version" (chunks: vox-ir-specification.md root, Stability guarantees)
- [x] doc 2 docs/src/reference/vox-ir-specification.md
- [x] chunk 2 docs/src/reference/vox-ir-specification.md#Vox IR Specification
- [x] chunk 2 docs/src/reference/vox-ir-specification.md#Vox IR Specification › Stability guarantees
- [x] chunk 1 docs/src/reference/vox-ir-specification.md#Vox IR Specification › Purpose
- [ ] doc 1 docs/src/explanation/faq.md  (bm25 #1; generic FAQ architecture answer, does not define the IR)

## q020 [concept] after a crash, what happens to activities that already finished when the workflow resumes?
pool: bm25 (expl-durable-execution.md #3, #11; tut-workflow-durability.md #13; expl-actors-workflows.md #11, #15; mesh-phase2-code-mobility-plan-2026.md #1); grep: `resume|replay|skip` in tut-workflow-durability and journeys; graph: unavailable; seed-sources: expl-durable-execution.md; read: expl-durable-execution sections 1-3
answer: "Recovery: restart with the same run_id and stable activity ids, load stored results for activities already recorded complete, skip them and continue with the remaining steps" (chunk: expl-durable-execution.md#Explanation: Durable Execution › 2. Recovery via Replay)
- [x] doc 2 docs/src/explanation/expl-durable-execution.md
- [x] doc 1 docs/src/tutorials/tut-workflow-durability.md
- [x] chunk 2 docs/src/explanation/expl-durable-execution.md#Explanation: Durable Execution › 2. Recovery via Replay
- [x] chunk 2 docs/src/explanation/expl-durable-execution.md#Explanation: Durable Execution › 3. Exactly-Once Semantics
- [x] chunk 1 docs/src/explanation/expl-durable-execution.md#Explanation: Durable Execution › 1. The Journal System
- [ ] doc 1 docs/src/architecture/mesh-phase2-code-mobility-plan-2026.md  (bm25 #1; acceptance criteria of a mesh plan, not user-facing semantics)

## q021 [concept] how do the CLI, orchestrator and MCP tools avoid each carrying their own retrieval implementation?
pool: bm25 (search-retrieval-ssot-2026.md #2, #3, #6; dead-crate-fate-plan-2026-05-08.md #1, #9, #15); grep: path `retriev|search` -> search-retrieval-ssot-2026.md among many; graph: unavailable; seed-sources: search-retrieval-ssot-2026.md; read: Canonical pipeline section
answer: "Rule: CLI, orchestrator, MCP, and chat preamble must not fork retrieval logic; they call into `vox-search` with a `SearchRuntimeContext` and `SearchPolicy`" (chunk: search-retrieval-ssot-2026.md#Search & Retrieval SSOT (2026) › 2. Canonical pipeline)
- [x] doc 2 docs/src/architecture/search-retrieval-ssot-2026.md
- [x] chunk 2 docs/src/architecture/search-retrieval-ssot-2026.md#Search & Retrieval SSOT (2026) › 2. Canonical pipeline
- [x] chunk 1 docs/src/architecture/search-retrieval-ssot-2026.md#Search & Retrieval SSOT (2026) › 1. Scope
- [ ] doc 1 docs/src/architecture/dead-crate-fate-plan-2026-05-08.md  (bm25 #1; per-crate fate analysis, incidental)

## q022 [policy] can I depend on blake3 or sha2 directly in my crate, or does hashing have to go through something else?
pool: bm25 (cryptography-ssot-2026.md #1; ref-builtins-hashing.md #2; adr-042-vox-populi-types.md #3, #8); grep: path `crypto` -> cryptography-ssot-2026.md; graph: unavailable; read: cryptography-ssot, hashing reference
answer: "All cryptographic logic MUST use the `vox-crypto` crate ... Hashing: use `sha2` or `blake3` via pure-Rust crates" (chunk: cryptography-ssot-2026.md#Cryptography Policy SSoT › Allowed Primitives)
- [x] doc 2 docs/src/architecture/cryptography-ssot-2026.md
- [x] doc 1 docs/src/reference/ref-builtins-hashing.md
- [x] chunk 2 docs/src/architecture/cryptography-ssot-2026.md#Cryptography Policy SSoT › Allowed Primitives
- [x] chunk 1 docs/src/reference/ref-builtins-hashing.md#Hashing & Identity Builtins › Workspace hash algorithm map (Rust tooling)
- [ ] doc 1 docs/src/architecture/adr-042-vox-populi-types.md  (bm25 #3; types-crate ADR, hashing only incidental)

## q023 [policy] are we allowed to vendor code from an AGPL project, and what provenance must we record for MIT/Apache copies?
pool: bm25 (adr/026-third-party-code-provenance.md #1, #2, #6, #10, #13; warp-research-findings-2026.md #5, #7, #11; skill_marketplace.md #3); grep: path `provenance|license` -> adr/026, skill-marketplace-security-and-provenance research; graph: unavailable; seed-sources: adr/026; read: ADR-026 Decision sections 1-3
answer: "Any file from a project declared AGPL-3.0-only must not be copied or vendored; vendored Apache/MIT code keeps its SPDX header and records upstream, upstream_commit, upstream_license, vendored date" (chunks: adr/026 Decision 2 and 3)
- [x] doc 2 docs/src/adr/026-third-party-code-provenance.md
- [x] doc 1 docs/src/architecture/warp-research-findings-2026.md
- [x] chunk 2 docs/src/adr/026-third-party-code-provenance.md#ADR-026: Third-Party Code Provenance Policy › Decision › 2. Prohibited intake
- [x] chunk 2 docs/src/adr/026-third-party-code-provenance.md#ADR-026: Third-Party Code Provenance Policy › Decision › 3. Attribution requirements for vendored code
- [x] chunk 1 docs/src/adr/026-third-party-code-provenance.md#ADR-026: Third-Party Code Provenance Policy › Decision › 1. Approved intake paths (in priority order)
- [ ] doc 1 docs/src/reference/skill_marketplace.md  (bm25 #3; skill bundling license note, not the code-vendoring policy)

## q024 [policy] what frontmatter does a new page under docs/src need and which category values are accepted?
pool: bm25 (documentation-governance.md #1; docs/src/AGENTS.md #8; adr/002-diataxis-doc-architecture.md #2); grep: path `governance|AGENTS` and `category` in documentation-governance; graph: unavailable; seed-sources: documentation-governance.md; read: Taxonomy subsections, docs/src/AGENTS.md frontmatter section
answer: "Use one of these `category` values in frontmatter (Category vocabulary table); required keys title, description, category, status, training_eligible" (chunks: documentation-governance.md Taxonomy subsections; docs/src/AGENTS.md)
- [x] doc 2 docs/src/contributors/documentation-governance.md
- [x] doc 1 docs/src/AGENTS.md
- [x] chunk 2 docs/src/contributors/documentation-governance.md#Documentation governance › Taxonomy › Category vocabulary
- [x] chunk 2 docs/src/contributors/documentation-governance.md#Documentation governance › Taxonomy › Frontmatter starter template
- [x] chunk 1 docs/src/contributors/documentation-governance.md#Documentation governance › Taxonomy › Status vocabulary
- [x] chunk 1 docs/src/AGENTS.md#AGENTS.md › Frontmatter requirements (all new pages)
- [ ] doc 1 docs/src/adr/002-diataxis-doc-architecture.md  (bm25 #2; Diataxis layout decision, not frontmatter vocabulary)
