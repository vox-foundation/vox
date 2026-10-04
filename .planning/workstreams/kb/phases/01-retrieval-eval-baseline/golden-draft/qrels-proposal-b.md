approved_by:

# Qrels proposal B (q025..q048)

Agent-proposed (AP-02); the user approves in 01-10. Score: 2 = answers, 1 = relevant. `- [ ]` rows are pool items considered and rejected; flip to `[x]` to accept. Query origin: agent-seed (01-CONTEXT D-02). Pool methods: bm25 = `vox kb eval --query --top 20`; grep = path/content grep excluding docs/src/archive; graph = `vox graph` (stale, so unavailable); seed-sources = seed author intended docs, used as one extra pool source only.

## q025 [policy] is pushing with --no-verify ever acceptable, and what am I expected to do right after?
pool: bm25 (local-ci-pre-push.md #2; contribution-loop.md #11; graphify-capabilities-audit #1 off-topic); grep: `--no-verify` across docs/src -> local-ci-pre-push.md, contributor-hub.md, git-concurrency-policy.md; graph: unavailable; seed-sources: local-ci-pre-push.md; read: local-ci-pre-push Bypass section
answer: "Use only for emergencies or when fixing the hook itself - CI still runs. After pushing with --no-verify, run vox ci pre-push --complete (or --full) locally as soon as possible" (chunk: local-ci-pre-push.md#Local CI parity (pre-push) › Bypass (emergency only))
- [x] doc 2 docs/src/contributors/local-ci-pre-push.md
- [x] chunk 2 docs/src/contributors/local-ci-pre-push.md#Local CI parity (pre-push) › Bypass (emergency only)
- [x] doc 1 docs/src/contributors/contributor-hub.md
- [ ] doc 1 docs/src/architecture/git-concurrency-policy.md  (grep; says the VCS commit_create path passes --no-verify, which is an implementation note, not the human policy)
- [ ] doc 1 docs/src/contributors/contribution-loop.md  (bm25 #11; snapshot-touching PR workflow, no bypass policy)

## q026 [policy] may a crate read TURSO_* environment variables directly to get database credentials?
pool: bm25 (env-vars.md #1, adr/004 #7, #8; vox-db-language-surface.md #3, #6; secrets-ssot.md #15; data-storage-ssot not in top 20); grep: `TURSO_` across docs/src -> data-storage-ssot-2026.md, data-storage-lint-and-ci-spec-2026.md, env-vars.md, adr/004, canonical-runtime-names.md; graph: unavailable; seed-sources: data-storage-ssot-2026.md, env-vars.md; read: data-storage-ssot section 3, env-vars Codex/Turso table, lint spec
answer: "Secret plane is vox-secrets ... No direct env::var(\"TURSO_*\") outside the vox-secrets allowlist." (chunk: data-storage-ssot-2026.md#Data Storage SSOT (2026) › 3. Non-negotiables (decided constraints))
- [x] doc 2 docs/src/architecture/data-storage-ssot-2026.md
- [x] chunk 2 docs/src/architecture/data-storage-ssot-2026.md#Data Storage SSOT (2026) › 3. Non-negotiables (decided constraints)
- [x] doc 1 docs/src/reference/env-vars.md
- [x] chunk 1 docs/src/reference/env-vars.md#Environment variables (SSOT) › Codex / Turso (vox-db, vox-package)
- [x] doc 1 docs/src/architecture/data-storage-lint-and-ci-spec-2026.md
- [x] chunk 1 docs/src/architecture/data-storage-lint-and-ci-spec-2026.md#Data Storage Lint & CI Spec (2026) › 7. .cursor/rules/data-storage-policy.mdc (shipped at HEAD, aligned in this PR)
- [x] doc 1 docs/src/adr/004-codex-arca-turso-ssot.md
- [ ] doc 1 docs/src/reference/vox-db-language-surface.md  (bm25 #3, #6; data-lane crate policy about language surface, not env reads)

## q027 [policy] does vox send any telemetry off the machine without me opting in, and can payloads include source code?
pool: bm25 (adr/023 #2; telemetry-trust-ssot.md #9; telemetry-unification-design-2026.md #5, #20; dashboard-migration-research #1 off-topic); grep: `telemetry` in docs/src/architecture, adr; graph: unavailable; seed-sources: telemetry-trust-ssot.md; read: telemetry-trust-ssot, ADR 023, unification design Trust posture
answer: "No default remote upload. The product does not phone home." and "No source code, secrets, or PII may be included in default telemetry payloads." (chunks: adr/023#Decision; telemetry-trust-ssot.md#Core Policy)
- [x] doc 2 docs/src/architecture/telemetry-trust-ssot.md
- [x] doc 2 docs/src/adr/023-optional-telemetry-remote-upload.md
- [x] chunk 2 docs/src/architecture/telemetry-trust-ssot.md#Telemetry Trust (SSoT) › Core Policy
- [x] chunk 2 docs/src/adr/023-optional-telemetry-remote-upload.md#ADR 023: Optional telemetry remote upload › Decision
- [x] doc 1 docs/src/architecture/telemetry-unification-design-2026.md
- [x] chunk 1 docs/src/architecture/telemetry-unification-design-2026.md#Telemetry unification design 2026 › Trust posture
- [ ] doc 1 docs/src/architecture/dashboard-migration-research-2026.md  (bm25 #1; dashboard decision, only mentions telemetry in passing)

## q028 [policy] how big can a single .rs file get before the toestub audit fails the build?
pool: bm25 (env-vars.md #1 Code-Audit table; toestub-contributor-guide.md #17 Suppression syntax; llm-target-reliability-audit #4); grep: `god_object|god object` across docs/src -> toestub-contributor-guide.md, coding-agents.md, how-to-contribute-parser-hir.md, continuation-prompt-engineering.md, mens-loc-audit-2026-09.md; graph: unavailable; seed-sources: toestub-contributor-guide.md; read: guide god_object section, coding-agents structural limits
answer: "A .rs file exceeds 500 non-blank lines, or a struct/impl has more than 12 methods. Multi-tier thresholds (300/400/500 lines)" (chunk: toestub-contributor-guide.md#TOESTUB contributor guide › Rule-by-rule fix guide › arch/god_object — Error)
- [x] doc 2 docs/src/contributors/toestub-contributor-guide.md
- [x] doc 2 docs/src/contributors/coding-agents.md
- [x] chunk 2 docs/src/contributors/toestub-contributor-guide.md#TOESTUB contributor guide › Rule-by-rule fix guide › arch/god_object — Error
- [x] chunk 2 docs/src/contributors/coding-agents.md#Coding Agent Instructions › Structural limits (enforced by TOESTUB, fail CI)
- [x] doc 1 docs/src/how-to/how-to-contribute-parser-hir.md
- [x] chunk 1 docs/src/how-to/how-to-contribute-parser-hir.md#Contributing — parser and HIR › TOESTUB Considerations
- [ ] doc 1 docs/src/reference/env-vars.md  (bm25 #1; scaling-audit parse-failure knob, not a file-size limit)
- [ ] doc 1 docs/src/architecture/mens-loc-audit-2026-09.md  (grep; a measurement of files over the threshold, not the rule)

## q029 [why] what drove moving model fine-tuning off the Python toolchain?
pool: bm25 (adr/003 #3, #4, #6; journeys/native-training.md #1, #8; voxmens-finetuning-boundaries #5, #7 off-target); grep: path `native-training|adr/003`; graph: unavailable; seed-sources: adr/003-native-training-over-python.md; read: ADR 003 Historical context, journey Curse of Python section
answer: "Environment friction ... Slow iteration: Python-based tokenizer was ~10x slower ... Philosophical mismatch ... CI complexity: Separate Python setup and heavy deps on every CI run" (chunk: adr/003#ADR 003 — Native Rust Training Over Python › Historical context (why we left Python))
- [x] doc 2 docs/src/adr/003-native-training-over-python.md
- [x] chunk 2 docs/src/adr/003-native-training-over-python.md#ADR 003 — Native Rust Training Over Python › Historical context (why we left Python)
- [x] doc 1 docs/src/journeys/native-training.md
- [x] chunk 1 docs/src/journeys/native-training.md#Journey: Native Rust LLM Training › The Curse of Python ML Environments
- [x] chunk 1 docs/src/adr/003-native-training-over-python.md#ADR 003 — Native Rust Training Over Python › Consequences
- [ ] doc 1 docs/src/architecture/voxmens-finetuning-boundaries-research-2026-06-21.md  (bm25 #5, #7; where fine-tuning is worthwhile, not why Python was dropped)
- [ ] doc 1 docs/src/architecture/mens-training-ssot.md  (bm25 #9; pointer page)

## q030 [why] why was the browser-served dashboard dropped in favor of a native desktop shell?
pool: bm25 (adr/024 #1, #8; adr/045 #2; adr/037 #6, #17, #18; vox-gui-native-roadmap #3; dashboard-migration-research #5, #16); grep: path `adr/04[5]|adr/037|adr/024`; graph: unavailable; seed-sources: adr/045-tauri-gui-replaces-axum-dashboard.md; read: ADR 045, ADR 037 context and decision, ADR 024 status
answer: "keeping the CLI commands and the dashboard UI in sync became a manual, error-prone task" and the web-served dashboard needed browser-security handling; replaced by generated Tauri GUI (chunk: adr/045#Context)
- [x] doc 2 docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md
- [x] chunk 2 docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md#ADR 045 — Tauri GUI Replaces Axum Dashboard › Context
- [x] chunk 1 docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md#ADR 045 — Tauri GUI Replaces Axum Dashboard › Decision
- [x] doc 1 docs/src/adr/037-tauri-convergence.md
- [x] chunk 1 docs/src/adr/037-tauri-convergence.md#ADR 037 — Tauri Convergence › Context
- [x] doc 1 docs/src/adr/024-dashboard-axum-spa.md
- [ ] doc 1 docs/src/architecture/dashboard-migration-research-2026.md  (bm25 #5; research on the earlier Axum-SPA migration)
- [ ] doc 1 docs/src/architecture/vox-gui-native-roadmap-2026.md  (bm25 #3; roadmap, not the decision rationale)

## q031 [why] why adopt iroh for mesh networking instead of finishing the home-grown HTTP control plane?
pool: bm25 (adr/047 #1, #3, #5, #9; adr/020 #2; populi.md #4, #11); grep: path `adr/047|iroh`; graph: unavailable; seed-sources: adr/047-iroh-transport.md; read: ADR 047 Context and Alternatives rejected, ADR 020 status
answer: "... writing NAT traversal, hole punching, path migration, key-based addressing, and a congestion-controlled stream protocol. That work is precisely what iroh already vendors." (chunk: adr/047#Context)
- [x] doc 2 docs/src/adr/047-iroh-transport.md
- [x] chunk 2 docs/src/adr/047-iroh-transport.md#ADR 047: iroh QUIC replaces the bespoke populi mesh transport › Context
- [x] chunk 2 docs/src/adr/047-iroh-transport.md#ADR 047: iroh QUIC replaces the bespoke populi mesh transport › Alternatives rejected
- [x] doc 1 docs/src/adr/020-populi-mesh-scaling-transport-default.md
- [x] chunk 1 docs/src/adr/047-iroh-transport.md#ADR 047: iroh QUIC replaces the bespoke populi mesh transport › Decision
- [ ] doc 1 docs/src/architecture/iroh-spike-findings-2026.md  (grep; measured spike results, not the rationale)
- [ ] doc 1 docs/src/reference/populi.md  (bm25 #4; current mesh SSOT, mentions control plane but not the decision)

## q032 [why] why do VoxScripts sent to a peer run in the interpreter rather than wasm or containers?
pool: bm25 (adr/048 #2, #3, #4, #6, #11; voxscript-portability-substrate-research #1, #5, #7; vox-container-vs-wasm #9, #10, #17); grep: path `adr/048|portability|isolation`; graph: unavailable; seed-sources: adr/048-interpreter-is-the-execution-and-sandbox-tier.md; read: ADR 048 Context/Decision, portability research 2.1, container-vs-wasm bottom line
answer: "Pure VoxScripts were defaulting to the native cargo lane ... left the mesh without a real isolation boundary once the WASI / container / MicroVM lanes were deleted or never shipped." (chunk: adr/048#Context)
- [x] doc 2 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md
- [x] chunk 2 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md#ADR 048: Interpreter is the execution and sandbox tier › Context
- [x] chunk 2 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md#ADR 048: Interpreter is the execution and sandbox tier › Decision
- [x] doc 1 docs/src/architecture/voxscript-portability-substrate-research-2026.md
- [x] chunk 1 docs/src/architecture/voxscript-portability-substrate-research-2026.md#VoxScript portability substrate: research and findings (2026-09) › Part 2 — Research findings › 2.1 WASM is the only substrate that delivers bit-identity — and it cannot run Vox's scripts
- [x] doc 1 docs/src/reference/isolation.md
- [ ] doc 1 docs/src/architecture/vox-container-vs-wasm-2026-05-08.md  (bm25 #9, #10; earlier evaluation favouring WASM, superseded by ADR 048, would mislead)

## q033 [why] why are some release workflows exempt from canceling older runs?
pool: bm25 (not in top 20: concurrency-exceptions.md; top hits vox-gui-capability-audit #1, nightly-builds-ssot #2 are off-topic); grep: `cancel-in-progress|concurrency-exceptions` across docs/src -> ci/concurrency-exceptions.md, reference/cli.md; graph: unavailable; seed-sources: ci/concurrency-exceptions.md; read: concurrency-exceptions.md, cli.md workflow-concurrency-guard row
answer: "a tag-push release build produces artifacts for that tag, and a newer tag's run is not a newer attempt at the same work, so 'the latest run wins' ... is simply false there." (chunk: concurrency-exceptions.md#Workflow concurrency exceptions)
- [x] doc 2 docs/src/ci/concurrency-exceptions.md
- [x] chunk 2 docs/src/ci/concurrency-exceptions.md#Workflow concurrency exceptions
- [x] doc 1 docs/src/reference/cli.md
- [x] chunk 1 docs/src/reference/cli.md#Reference: vox CLI (minimal compiler binary) › Build & run › vox ci …
- [ ] doc 1 docs/src/architecture/nightly-builds-ssot.md  (bm25 #2; describes the nightly workflow, not the cancel exemption)
- [ ] doc 1 docs/src/ci/workflow-enumeration.md  (bm25 #5; lists workflows without the concurrency rationale)

## q034 [why] if LLMs aren't actually better at PowerShell, why prefer pwsh for agent shells on Windows?
pool: bm25 (terminal-exec-policy-ssot.md #1, #13; agent-instruction-architecture.md #2, #3, #5; ci/runner-contract.md #4; agent-shell-fluency-eval-design #11, #16); grep: path `terminal-exec|agent-instruction`; graph: unavailable; seed-sources: terminal-exec-policy-ssot.md; read: terminal-exec-policy-ssot IS / IS NOT, agent-instruction-architecture command policy, runner-contract agent vs CI shell
answer: "pwsh is preferred over Bash on Windows agent hosts because it improves policy enforcement and output parsing at the IDE/host boundary, not because LLMs produce better code in PowerShell." (chunk: terminal-exec-policy-ssot.md#What this policy IS)
- [x] doc 2 docs/src/architecture/terminal-exec-policy-ssot.md
- [x] chunk 2 docs/src/architecture/terminal-exec-policy-ssot.md#Terminal Exec Policy SSOT (2026) › What this policy IS
- [x] chunk 1 docs/src/architecture/terminal-exec-policy-ssot.md#Terminal Exec Policy SSOT (2026) › What this policy is NOT
- [x] doc 1 docs/src/contributors/agent-instruction-architecture.md
- [x] chunk 1 docs/src/contributors/agent-instruction-architecture.md#Agent instruction architecture › Command policy strategy (PowerShell-first)
- [x] doc 1 docs/src/ci/runner-contract.md
- [x] chunk 1 docs/src/ci/runner-contract.md#CI runner contract › Agent / local terminal vs CI shell
- [ ] doc 1 docs/src/architecture/agent-shell-fluency-eval-design-2026.md  (bm25 #11; design of an eval that was not run, not the policy rationale)

## q035 [why] why does the lockfile end up with both ring and aws-lc-rs?
pool: bm25 (cryptography-ssot-2026.md #1, #3; iroh-spike-findings-2026.md #2; no-c-no-msvc-2026-05-08.md #4); grep: `aws-lc` across docs/src -> cryptography-ssot-2026.md, iroh-spike-findings-2026.md, crate-consolidation-and-dependency-currency-audit; graph: unavailable; seed-sources: cryptography-ssot-2026.md; read: cryptography-ssot Correction section
answer: "ring is pinned in the root manifest. aws-lc-rs arrives with reqwest 0.13 via chromiumoxide and gix -> jj-lib -> vox-vcs; jj-lib exposes no TLS feature" (chunk: cryptography-ssot-2026.md#Correction (2026-09-04))
- [x] doc 2 docs/src/architecture/cryptography-ssot-2026.md
- [x] chunk 2 docs/src/architecture/cryptography-ssot-2026.md#Cryptography Policy SSoT › Correction (2026-09-04) — this document was unenforced and self-contradictory
- [ ] doc 1 docs/src/architecture/iroh-spike-findings-2026.md  (bm25 #2; says the iroh path has no aws-lc-rs, a negative fact about one dependency)
- [ ] doc 1 docs/src/architecture/crate-consolidation-and-dependency-currency-audit-2026-06-12.md  (grep; mentions aws-lc-rs only as a Windows build hazard for jsonwebtoken)

## q036 [troubleshoot] invoking vox ci ssot-drift by hand flags crates that are already allowlisted, but the git hook passes
pool: bm25 (vox-binary-freshness-and-ssot-2026.md #1; tooling-convergence-findings #2, #17; env-var-ssot-design #8); grep: path `binary-freshness`; graph: unavailable; seed-sources: vox-binary-freshness-and-ssot-2026.md; read: freshness doc sections 1 and 2
answer: "the installed vox.exe was 0.5.0+build.601 while the workspace source was 0.6.0 - an old binary running outdated guard logic and allowlists. ... the git pre-push hook is already safe" (chunk: freshness#1. The incident)
- [x] doc 2 docs/src/contributors/vox-binary-freshness-and-ssot-2026.md
- [x] chunk 2 docs/src/contributors/vox-binary-freshness-and-ssot-2026.md#Vox Binary Freshness & Single-Source-of-Truth (2026) › 1. The incident
- [x] chunk 1 docs/src/contributors/vox-binary-freshness-and-ssot-2026.md#Vox Binary Freshness & Single-Source-of-Truth (2026) › 2. Two underlying problems
- [ ] doc 1 docs/src/architecture/tooling-convergence-findings-2026.md  (bm25 #2; convergence plan for tooling, no stale-binary diagnosis)

## q037 [troubleshoot] cargo errors that a dependency I added to a member crate isn't defined at the root
pool: bm25 (workspace-root-manifest.md #1, #2; dependency-policy.md #5; suite-status-audit #3 off-topic); grep: `workspace.dependencies` across docs/src/ci, contributors; graph: unavailable; seed-sources: ci/workspace-root-manifest.md; read: workspace-root-manifest, dependency-policy single workspace pin
answer: "When Cargo errors with 'not found in workspace.dependencies': add to root [workspace.dependencies] - Internal: path = crates/... External: version/features" (chunk: workspace-root-manifest.md#When Cargo errors ...)
- [x] doc 2 docs/src/ci/workspace-root-manifest.md
- [x] chunk 2 docs/src/ci/workspace-root-manifest.md#Workspace root Cargo.toml (fix forward) › When Cargo errors with "not found in workspace.dependencies"
- [x] chunk 1 docs/src/ci/workspace-root-manifest.md#Workspace root Cargo.toml (fix forward)
- [x] doc 1 docs/src/contributors/dependency-policy.md
- [x] chunk 1 docs/src/contributors/dependency-policy.md#Dependency policy › Rust: single workspace pin
- [ ] doc 1 docs/src/architecture/suite-status-audit-2026-09-21.md  (bm25 #3; audit list, not a how-to fix)

## q038 [troubleshoot] cargo test just sits there warning that a test is still going after a minute, but it never times out
pool: bm25 (runner-contract.md #4; script-tier-timings #1 off-topic; build-broker-usage #3; ci-and-build-loop-findings #7, #20); grep: `has been running for over|terminate-after` -> ci/runner-contract.md only; graph: unavailable; seed-sources: ci/runner-contract.md; read: runner-contract Test hangs section
answer: "cargo test does not enforce per-test timeouts. After ~60 seconds it may print 'has been running for over 60 seconds' - that is only a warning" ; nextest slow-timeout + terminate-after ends a stuck test (chunk: runner-contract#Test hangs)
- [x] doc 2 docs/src/ci/runner-contract.md
- [x] chunk 2 docs/src/ci/runner-contract.md#CI runner contract › Test hangs: cargo test vs cargo nextest
- [ ] doc 1 docs/src/contributors/build-broker-usage.md  (bm25 #3; broker admission waits can look like hangs, but not the per-test timeout question)
- [ ] doc 1 docs/src/architecture/ci-and-build-loop-findings-2026.md  (bm25 #7, #20; hosted CI slowness, not test-level timeouts)

## q039 [troubleshoot] vox run exits with status 77 on my script
pool: bm25 (isolation.md #17 wrong chunk; how-to-train-mens-macos-metal #1 and phase4-runtime-monitors #2 off-topic lexical hits); grep: `\b77\b` near exit/status across docs/src -> reference/isolation.md Exit codes table; graph: unavailable; seed-sources: reference/isolation.md; read: isolation.md Exit codes, Limits, caps grammar
answer: "77 | capability denied (vox: capability denied: <ns>.<method>)" (chunk: isolation.md#Interpreter isolation › Exit codes)
- [x] doc 2 docs/src/reference/isolation.md
- [x] chunk 2 docs/src/reference/isolation.md#Interpreter isolation › Exit codes
- [x] chunk 1 docs/src/reference/isolation.md#Interpreter isolation › --caps grammar
- [x] doc 1 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md
- [ ] doc 1 docs/src/architecture/vox-language-rules-phase4-runtime-monitors-2026.md  (bm25 #2; planning doc for panic traps and fuel, no exit-code table)

## q040 [troubleshoot] worker registration against the populi server keeps coming back 401
pool: bm25 (populi.md #2; populi-quickstart.md #5; mens-serving-ssot #3 off-topic; env-vars Mens #15); grep: `VOX_MESH_TOKEN` across docs/src -> populi-quickstart.md, adr/008, adr/009; graph: unavailable; seed-sources: how-to/populi-quickstart.md; read: quickstart Troubleshooting and Token management, populi.md HTTP errors table
answer: "401 Unauthorized - the Authorization: Bearer header is missing or the token doesn't match the one in the vault. Run vox populi config show ... vox secrets get VOX_MESH_TOKEN" (chunk: quickstart#Troubleshooting)
- [x] doc 2 docs/src/how-to/populi-quickstart.md
- [x] chunk 2 docs/src/how-to/populi-quickstart.md#Populi Quickstart › Troubleshooting
- [x] chunk 1 docs/src/how-to/populi-quickstart.md#Populi Quickstart › Token management
- [x] doc 2 docs/src/reference/populi.md
- [x] chunk 2 docs/src/reference/populi.md#Mesh / Populi SSOT (CPU-first) › Control-plane HTTP errors (stable text bodies)
- [ ] doc 1 docs/src/adr/008-populi-transport.md  (grep; states VOX_MESH_TOKEN makes the server require a bearer, a design note rather than a fix)

## q041 [troubleshoot] budget model selection keeps choosing an expensive Anthropic model as if it were free
pool: bm25 (model-catalog-ssot-2026.md #10 other chunk; free-by-default-audit-2026-05-24.md #2; free-tier-model-selection-and-onboarding-research #4, #7); grep: `is_free` + Anthropic across docs/src -> model-catalog-ssot-2026.md only; graph: unavailable; seed-sources: model-catalog-ssot-2026.md; read: model-catalog-ssot Bug 3 and Wave 0 tasks
answer: "newly discovered Anthropic models enter as is_free: true ... A $75/M token model gets invoked. BudgetManager cannot catch it because cost_per_1k = 0.0." (chunk: model-catalog-ssot#Bug 3)
- [x] doc 2 docs/src/architecture/model-catalog-ssot-2026.md
- [x] chunk 2 docs/src/architecture/model-catalog-ssot-2026.md#Model Catalog SSOT — Architecture & Implementation Plan 2026 › Part 1 — Confirmed Split-Brain Bugs (True Positives) › Bug 3 — AnthropicDirect Registers Expensive Models as is_free: true
- [x] chunk 1 docs/src/architecture/model-catalog-ssot-2026.md#Model Catalog SSOT — Architecture & Implementation Plan 2026 › Part 4 — Implementation Tasks › Wave 0 — Critical Bug Fixes (Regressions Today, No Architecture Change)
- [ ] doc 1 docs/src/architecture/free-by-default-audit-2026-05-24.md  (bm25 #2; audit of is_free call sites, earlier and does not describe the Anthropic defect)
- [ ] doc 1 docs/src/architecture/free-tier-model-selection-and-onboarding-research-2026-08-01.md  (bm25 #4, #7; onboarding design, not the bug)

## q042 [troubleshoot] secrets doctor warns that my API key uses a deprecated alias name - how long will it keep working?
pool: bm25 (secrets-ssot.md #10 Command Surfaces, #16; env-var-ssot-design-and-handoff #1, #7; chunk Transition and Deprecation Window Policy not in top 20); grep: `deprecat|alias` in secrets-ssot.md; graph: unavailable; seed-sources: reference/secrets-ssot.md; read: secrets-ssot Resolution Precedence and Transition and Deprecation Window Policy
answer: "Emit DeprecatedAliasUsed in doctor for legacy aliases. Keep legacy aliases for at least two release trains after warning lands." (chunk: secrets-ssot#Transition and Deprecation Window Policy)
- [x] doc 2 docs/src/reference/secrets-ssot.md
- [x] chunk 2 docs/src/reference/secrets-ssot.md#Secrets SSOT › Transition and Deprecation Window Policy
- [x] chunk 1 docs/src/reference/secrets-ssot.md#Secrets SSOT › Resolution Precedence
- [ ] doc 1 docs/src/architecture/env-var-ssot-design-and-handoff-2026-06-15.md  (bm25 #1; design of the env registry, only mentions deprecated status of rows)
- [ ] doc 1 docs/src/reference/env-vars.md  (grep; per-variable deprecation notes for TURSO aliases, not the general window)

## q043 [multi-hop] adding a brand-new provider API key: where do I register it, and what's the emergency procedure if it leaks?
pool: bm25 (secrets-break-glass-runbook.md #5; secrets-ssot.md #6; free-tier-model-selection #1, #2 off-target); grep: `SecretSpec|spec/ids.rs` and `rotate|reset --force` across docs/src/operations, reference; graph: unavailable; seed-sources: secrets-ssot.md, secrets-break-glass-runbook.md; read: secrets-ssot intro, break-glass runbook, cloudless-ops Incident handling (bm25: not in top 20)
answer: "registry metadata in crates/vox-secrets/src/spec/ defines every accepted managed secret name" ; leak: "Rotate all affected credentials ... through vox secrets reset --force immediately after containment." (chunks: secrets-ssot#Secrets SSOT; cloudless-ops#Incident handling)
Note: the break-glass runbook is about emergency privileged access, not a leaked key; its post-incident rotation step is the closest match, so it is scored 1. The registration procedure checklist lives in root AGENTS.md, which is outside the corpus.
- [x] doc 2 docs/src/reference/secrets-ssot.md
- [x] chunk 2 docs/src/reference/secrets-ssot.md#Secrets SSOT › Secrets SSOT
- [x] doc 2 docs/src/operations/secrets-cloudless-ops-runbook.md
- [x] chunk 2 docs/src/operations/secrets-cloudless-ops-runbook.md#Secrets Cloudless Ops Runbook › Incident handling
- [x] doc 1 docs/src/operations/secrets-break-glass-runbook.md
- [x] chunk 1 docs/src/operations/secrets-break-glass-runbook.md#Secrets Break-Glass Runbook › Post-incident mandatory tasks
- [x] chunk 1 docs/src/reference/secrets-ssot.md#Secrets SSOT › Naming Convention
- [ ] doc 1 docs/src/architecture/free-tier-model-selection-and-onboarding-research-2026-08-01.md  (bm25 #1, #2; provider onboarding research, not the registration or leak procedure)

## q044 [multi-hop] my workflow branches on a random number - will replay reproduce it, and will anything catch that before runtime?
pool: bm25 (expl-actors-workflows.md #1, #10; adr/041 #7; true-workflow-durability-design #5; expl-durable-execution #19; phase5-effects-determinism #16; tut-workflow-durability #18); grep: `random|determinis` in explanation, adr/041, adr/019, tutorials; graph: unavailable; seed-sources: expl-durable-execution.md, adr/041, adr/019; read: expl-actors-workflows Compilation Phases, expl-durable-execution section 4, ADR 041 Decision, tutorial Best Practices
answer: "Determinism lint ... rejecting calls to time.now() or anything on random" inside a workflow body; "For replay to work, the workflow body should stay deterministic" (chunks: expl-actors-workflows#Compilation Phases; expl-durable-execution#4. Determinism Requirements)
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
- [ ] doc 1 docs/src/architecture/true-workflow-durability-design-2026.md  (bm25 #5; design discussion of test gates)
- [ ] doc 1 docs/src/architecture/mesh-phase2-code-mobility-plan-2026.md  (bm25 #3; mesh plan, unrelated lowering task)

## q045 [multi-hop] which bundle should a mesh node install, and what limits apply to scripts it receives from peers?
pool: bm25 (how-to-pick-a-vox-bundle.md #13 Quick-reference; isolation.md #16 Limits; populi-mesh-improvement-backlog #1, #3, #10; plugin-system-redesign #2); grep: `vox-mesh` in how-to-pick-a-vox-bundle.md; `max-disk|mesh receiver` in isolation.md; graph: unavailable; seed-sources: how-to-pick-a-vox-bundle.md, adr/048, isolation.md; read: bundle quick-reference, isolation Limits, ADR 048 Decision
answer: bundle: "Mesh node (inter-process / distributed) | vox-mesh"; limits: "The mesh receiver always supplies --max-disk and --max-files" (32 MiB, 4096 files on mesh) plus steps and depth bounds (chunks: bundle#Quick-reference table; isolation#Limits)
- [x] doc 2 docs/src/how-to/how-to-pick-a-vox-bundle.md
- [x] doc 2 docs/src/reference/isolation.md
- [x] chunk 2 docs/src/how-to/how-to-pick-a-vox-bundle.md#Picking a Vox bundle › Quick-reference table
- [x] chunk 2 docs/src/reference/isolation.md#Interpreter isolation › Limits
- [x] doc 1 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md
- [x] chunk 1 docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md#ADR 048: Interpreter is the execution and sandbox tier › Decision
- [x] chunk 1 docs/src/reference/isolation.md#Interpreter isolation › --caps grammar
- [ ] doc 1 docs/src/architecture/populi-mesh-improvement-backlog-2026.md  (bm25 #1, #3; backlog items, not shipped guidance)
- [ ] doc 1 docs/src/architecture/plugin-system-redesign-2026.md  (bm25 #2; plugin and bundle CLI design, not the install choice)

## q046 [multi-hop] when I pick the Docs chip in the GUI search palette, which crate actually runs the query and over which corpora?
pool: bm25 (gui-navigation.md #1; omni-search-audit-and-roadmap-2026.md #2, #3, #6; search-retrieval-ssot-2026.md #20); grep: `vox_search_query` across docs/src -> omni-search-audit, context-window-management-design, skill-ecosystem-audit; graph: unavailable; seed-sources: gui-navigation.md, search-retrieval-ssot-2026.md; read: gui-navigation Search scopes table, omni-search-audit architecture map, search-retrieval-ssot section 6
answer: "| Docs | chunk, knowledge corpora | Yes |" via backend vox_search_query (vox-gui commands/search.rs into the vox-search stack) (chunk: gui-navigation#Search scopes (OmniSearch vs Search surface))
- [x] doc 2 docs/src/reference/gui-navigation.md
- [x] chunk 2 docs/src/reference/gui-navigation.md#GUI navigation › OmniSearch › Search scopes (OmniSearch vs Search surface)
- [x] doc 2 docs/src/architecture/omni-search-audit-and-roadmap-2026.md
- [x] chunk 2 docs/src/architecture/omni-search-audit-and-roadmap-2026.md#Omni-Search Audit & Roadmap (2026) › 2. Current Search Architecture Map
- [x] chunk 1 docs/src/reference/gui-navigation.md#GUI navigation › OmniSearch
- [x] doc 1 docs/src/architecture/search-retrieval-ssot-2026.md
- [x] chunk 1 docs/src/architecture/search-retrieval-ssot-2026.md#Search & Retrieval SSOT (2026) › 6. MCP tools and GUI
- [ ] doc 1 docs/src/architecture/code-search-omnisearch-research-2026-06-17.md  (bm25 #14, #16, #18; external research and gap matrix, not current wiring)

## q047 [multi-hop] I want to ship local telemetry to our own ingest server - what's the opt-in flow and which settings hold the URL and token?
pool: bm25 (adr/023 #1; env-vars.md #9 Optional telemetry upload; telemetry-unification-design #4); grep: `VOX_TELEMETRY_UPLOAD` across docs/src -> adr/023, env-vars.md, secrets-ssot.md, cli.md; graph: unavailable; seed-sources: adr/023, env-vars.md; read: ADR 023 Decision, env-vars telemetry table, secrets-ssot inventory row, cli.md vox telemetry
answer: "No default remote upload ... explicit CLI invocation (vox telemetry upload) and configured ingest URL"; "VOX_TELEMETRY_UPLOAD_URL" and "VOX_TELEMETRY_UPLOAD_TOKEN" resolved via vox-secrets (chunks: adr/023#Decision; env-vars#Optional telemetry upload)
- [x] doc 2 docs/src/adr/023-optional-telemetry-remote-upload.md
- [x] doc 2 docs/src/reference/env-vars.md
- [x] chunk 2 docs/src/adr/023-optional-telemetry-remote-upload.md#ADR 023: Optional telemetry remote upload › Decision
- [x] chunk 2 docs/src/reference/env-vars.md#Environment variables (SSOT) › Optional telemetry upload (vox telemetry)
- [x] doc 1 docs/src/reference/secrets-ssot.md
- [x] doc 1 docs/src/reference/cli.md
- [x] chunk 1 docs/src/reference/cli.md#Reference: vox CLI (minimal compiler binary) › Tooling › vox telemetry
- [x] doc 1 docs/src/architecture/telemetry-trust-ssot.md
- [ ] doc 1 docs/src/architecture/telemetry-unification-design-2026.md  (bm25 #4; design for a future facade, upload flow unchanged)

## q048 [multi-hop] my PR only bumps Cargo.lock - does CI test the whole workspace, and how do I reproduce that locally first?
pool: bm25 (runner-contract.md #1, #12; affected-crate-selective-ci.md #17 Sentinels; contribution-loop #2; dependency-policy #8, #13); grep: `Cargo.lock` across docs/src/ci -> affected-crate-selective-ci.md; local-ci-pre-push Profiles chunk (bm25: not in top 20); graph: unavailable; seed-sources: ci/affected-crate-selective-ci.md, contributors/local-ci-pre-push.md; read: affected-crate-selective-ci Sentinels and Local parity, local-ci-pre-push profiles
answer: "Sentinels (force full): Cargo.toml (root), Cargo.lock, .cargo/config.toml, rust-toolchain.toml ..." ; "vox ci pre-push --full --since origin/main - mirrors PR scope" (chunks: affected-crate-selective-ci#Sentinels; #Local parity)
- [x] doc 2 docs/src/ci/affected-crate-selective-ci.md
- [x] chunk 2 docs/src/ci/affected-crate-selective-ci.md#Affected-Crate Selective CI › Sentinels (force full)
- [x] chunk 2 docs/src/ci/affected-crate-selective-ci.md#Affected-Crate Selective CI › Local parity
- [x] doc 2 docs/src/contributors/local-ci-pre-push.md
- [x] chunk 1 docs/src/contributors/local-ci-pre-push.md#Local CI parity (pre-push) › Profiles
- [x] doc 1 docs/src/ci/runner-contract.md
- [x] chunk 1 docs/src/ci/runner-contract.md#CI runner contract › Local-first CI (required policy, ENFORCED)
- [ ] doc 1 docs/src/contributors/dependency-policy.md  (bm25 #8, #13; lockfile as execution pin, not CI scope)
- [ ] doc 1 docs/src/contributors/contribution-loop.md  (bm25 #2; snapshot-touching PR flow)
