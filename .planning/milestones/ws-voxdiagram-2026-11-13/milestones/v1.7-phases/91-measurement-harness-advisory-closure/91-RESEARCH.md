# Phase 91: Measurement Harness & Advisory Closure - Research

**Researched:** 2026-10-27
**Domain:** paired NL-to-diagram LLM evaluation (Rust harness on the `vox_actor_runtime::llm` facade, deterministic parser oracles in Rust and Node, paired-proportion non-inferiority statistics)
**Confidence:** HIGH for in-repo APIs, placement and oracle behaviour; MEDIUM for statistics (formula from cited source, power from simulation run here); LOW for cost (prices not fetched)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Harness shape
- Paired design: the same ≥60 natural-language diagram tasks go to both arms (VoxDiagram, Mermaid) with the same model, same temperature, and grammar cards of comparable length and detail (the VoxDiagram reference/grammar card vs an equivalent Mermaid flowchart + sequence card). Prompts and cards are committed and frozen before the run.
- Task set: graph and sequence tasks in proportion to the docs corpus, spanning the hard cases (punctuation in labels, nested containers, edge labels, self-loops, sequence notes). Each task carries an expected node/edge (or actor/message) set.
- Models come only through `vox_actor_runtime::llm` (registry selection, none pinned, no vendor hostnames). The resolved model id of every call is recorded. Aim for at least two model families if the registry offers them; one is acceptable if that is all that is configured, and the report says so.
- Oracles are deterministic parsers, never an LLM judge: `vox_diagram::check` for VoxDiagram; the real Mermaid parser for Mermaid (the pinned mermaid@11.17.2 Node oracle already used in Phase 84, `docs-astro/tests/unit/mermaid-oracle.test.mjs`), not the heuristic `vox_validate_mermaid`. Research confirms the exact Mermaid entry point.
- Bounded repair: up to 2 repair turns per task per arm, each fed only that arm's own parser diagnostics (VoxDiagram `check` diagnostics/fixes; Mermaid parse error text). Metrics: first-pass validity and validity after repair, per arm, with Wilson 95% CIs; a paired difference with an appropriate paired interval.
- Secondary (reported, not in the decision rule): structural fidelity against the task's expected sets.

#### Decision rule (fixed before the run)
- Written into the plan and the report before any live call: primary endpoint, non-inferiority margin, interval method, and how "non-inferior / inconclusive / inferior" map to the interval. Research proposes the margin and checks that 60+ tasks gives usable power; if not, raise the task count rather than loosen the rule.

#### Reproducibility
- Every raw model output is recorded (JSONL, committed — small). A re-score path reads the recording and reproduces the tallies byte-for-byte with no network; that re-score runs as an ordinary offline test.
- The live run is a separate, explicitly invoked step (ignored test or `vox run` script), never part of default CI.

#### Placement and dependencies
- No new crate edge beyond the two authorized (`vox-doc-pipeline→vox-diagram`, `vox-orchestrator-mcp→vox-diagram`) and no new npm/cargo dependencies. Candidate homes: `vox-orchestrator-mcp` (already depends on `vox-actor-runtime` and `vox-diagram`) or a `.vox` script per VoxScript-first. Research picks; the Mermaid oracle is invoked as a subprocess of the existing Node setup.
- Secrets through Clavis only.

#### Live run gate
- If no LLM provider is configured when execution reaches the live run, stop and report it as a human action. Never fabricate, simulate, or hand-write model outputs as results.

#### Report and advisory (EVAL-02)
- Report at `docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md` (name may be adjusted) with full frontmatter, `kb:summary`, method, decision rule, per-arm tables with CIs, the verdict the data supports, limitations, and the resolved model ids.
- `docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md`: the "LLM Syntax Error Rate" row (line ~141) is marked unsourced in place and links to the measured report. No other rewriting of the advisory.

### Claude's Discretion
- Task authoring format, file layout of the harness and recordings, Mermaid card wording, the exact paired interval method (subject to research).

### Deferred Ideas (OUT OF SCOPE)
- Running the harness nightly in CI.
- Comparing against D2/PlantUML.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| EVAL-01 | A paired NL→diagram harness (≥60 tasks, Wilson CIs, deterministic parser as oracle, models selected via `vox_actor_runtime::llm`, none pinned) reports VoxDiagram vs Mermaid first-pass validity and validity after the bounded repair loop | §LLM facade and model selection; §Placement (vox-cli, zero new edges); §Mermaid oracle; §Statistics (N=200 tasks, Newcombe paired interval, δ=0.10); §Task set; §Recording and offline re-score chain |
| EVAL-02 | The advisory's unsourced error-rate table is annotated as unsourced and linked to the measured EVAL-01 result | §Advisory annotation (the row has no citation anywhere in the file; exact minimal edit given) |
</phase_requirements>

## Project Constraints (from CLAUDE.md / AGENTS.md)

- **LLM boundary:** every LLM call goes through `vox_actor_runtime::llm` (`llm_chat`, `infer_with_retry`); no vendor hostnames, no vendor SDKs; model choice from `vox-orchestrator::models` registry selection. Detector `llm_provider_call` fails on hostnames.
- **No pinned models** (user memory `feedback-no-pinned-models`): do not use `contracts/eval/llm-panel.v1.yaml` (it pins a panel, `pinned_at: "2026-05-15"`), and refuse a run whose `SelectionReason` is `EnvOverride` (`VOX_MODEL_FORCE`).
- **Secrets:** Clavis only (`vox_secrets::resolve_secret`); no `std::env::var` reads of keys. The facade already resolves keys (`LlmConfig::openrouter` → `SecretId::OpenRouterApiKey`).
- **Crate edges:** no new workspace edge; `exceptions` are user-authorized only. Defactor policy allows copying helpers under ~50 lines with `// vox:defactored-from <crate> <date>`.
- **Test-first:** every new `pub fn` in `crates/*/src/**` needs a test in the same file; the failing test comes first.
- **VoxScript-first:** automation glue is `.vox`. The harness is not glue — it needs the LLM facade, which no `.vox` builtin exposes (see §Placement) — so it is Rust, like the precedent `vox model eval-corpus`.
- **Per-file size:** `arch/god_object` errors at 500 non-blank lines per file — split the harness into several files.
- **Docs:** new `docs/src/` pages need frontmatter (`title`, `description`, `category`, `topic`, `domain`, `authorship` + `authorship_model`) and a 40–120 word `<!-- kb:summary -->`; never set `human_attested_by`; never add facet-backlog entries. Verify with `cargo run -p vox-doc-pipeline -- --lint-only --paths <file>`.
- **Docs diagrams:** any diagram added under `docs/src/` must be a `vox-diagram` fence (no new `mermaid` fences — the legacy baseline only shrinks). Mermaid text inside the eval data files is fine (not under `docs/src/`).
- **Local verification is lean:** `cargo check -p <crate>` and that crate's new tests only; hosted CI is the gate. Wrap cargo/pnpm in `timeout`, background long builds.
- **Never edit `docs/src/archive/`**; never hand-regenerate `*.generated.md`.
- **Commits:** branch `milestone/v1.7-voxdiagram`; do not stage `.planning/active-workstream` or `.planning/workstreams/voxdiagram/milestone.lock`; do not push.

## Summary

Everything the harness needs already exists, and all of it is reachable from **`vox-cli` with zero new crate edges**: the facade (`vox-actor-runtime`), registry selection and `ModelSpec → LlmConfig` mapping (`vox-orchestrator`), `vox_diagram::check` / `validate` / `mermaid::export` (through the `vox_doc_pipeline::diagram` re-export that Phase 87 added for exactly this purpose), Wilson and McNemar (`vox-eval::corpus_stats`), and a prior NL→code harness with the same shape (`vox model eval-corpus`: facade call, provider-reported model id, per-sample seeds, verifier-feedback repair turns, fenced-code extraction, infra-error ceiling). `vox-orchestrator-mcp` also reaches facade + diagram, but lacks `vox-eval` and the eval helpers. A `.vox` script cannot do it: the Vox builtin registry has no LLM call at all.

The real Mermaid oracle is `docs-astro/tests/lib/mermaid-oracle.mjs` (jsdom shim + `mermaid.parse`, mermaid 11.17.2). Its exported `accepts()` swallows the error, so the phase adds one export that returns the error text, plus a small stdin/stdout JSONL batch script beside it. Measured here: ~0.8 s one-time startup, ~1.2 ms per parse, and deterministic Jison messages such as `Parse error on line 2: ... Expecting 'TXT', got 'NEWLINE'`. No new npm dependency.

Statistically, **60 tasks is not enough**: at a 10-point margin and true equality, power is only 0.33–0.60 for validity rates of 85–95 % (Monte Carlo here, 20 000 replicates). Pre-register: primary endpoint = first-pass validity on the registry-selected model; paired difference Δ = p(VoxDiagram) − p(Mermaid) with a **Newcombe hybrid-score (Wilson MOVER, method 10) 95 % interval**; margin **δ = 0.10**; verdict non-inferior if L > −0.10, inferior if U < −0.10, else inconclusive; **N = 200 tasks** (184 graph / 16 sequence, the corpus ratio 35:3). This gives power ≥ 0.80 across 85–98 % validity, with type I error measured at 0.023–0.027 against the nominal one-sided 0.025.

**Primary recommendation:** Build the harness as a `vox-cli` module (`commands/model/eval_diagram/`) with an `#[ignore]` live integration test and a normal offline re-score test. Add `paired_difference_ci` to `vox-eval::corpus_stats`. Freeze tasks, cards and rule in `contracts/eval/diagram-authoring/` before any live call. Close the re-score loop across both runtimes with the chain `attempts.jsonl →(Rust) oracle-inputs.jsonl →(Node) oracle-outputs.jsonl →(Rust) tallies.json`, each step byte-checked by an offline test in its own runtime.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Model selection (no pin) | `vox-orchestrator::models` (L3 library) | — | `decide()`/`select()` is the routing SSOT; the harness only passes an intent |
| LLM call, resolved model id, cost | `vox-actor-runtime::llm` (L3 facade) | Clavis (`vox-secrets`) | Sole sanctioned egress; reports the provider's `model` string |
| Harness orchestration, recording, scoring, report rendering | `vox-cli` (L5 surface), new module | `vox-eval` (L2) for statistics | Only crate reaching facade, registry, diagram, and eval stats without a new edge |
| VoxDiagram oracle | `vox-diagram` (L2, via `vox_doc_pipeline::diagram`) | — | Pure deterministic `check()` |
| Mermaid oracle + canonical sets | Node, `docs-astro/tests/lib/` (test-only) | — | The pinned real parser lives only in the Node toolchain |
| Paired CI / NI verdict | `vox-eval::corpus_stats` (L2, pure) | — | Beside the existing `wilson_interval`; pure, fast to test |
| Frozen tasks, cards, recordings, tallies | `contracts/eval/diagram-authoring/` (data) | — | Existing eval-corpus home; `contracts/**` triggers both docs-quality and the full Rust suite |
| Findings report, advisory annotation | `docs/src/architecture/` (docs) | — | Project knowledge SSOT |

## Standard Stack

No new dependencies. Everything below is already in the workspace or `docs-astro/package.json`.

### Core (in-repo)
| Component | Location | Purpose | Evidence |
|-----------|----------|---------|----------|
| `vox_actor_runtime::llm::{llm_chat, infer_with_retry, LlmConfig, LlmChatMessage, LlmResponse}` | `crates/vox-actor-runtime/src/llm/` | The one LLM path | [VERIFIED: crates/vox-actor-runtime/src/llm/mod.rs:15-24] |
| `vox_orchestrator::models::{ModelRegistry::from_cache, select::{decide, ModelSelectionRequest, SelectionIntent, CandidateScope}, llm_config_for_spec, TaskCategory, family::family_key}` | `crates/vox-orchestrator/src/models/` | Unpinned selection + config mapping | [VERIFIED: select.rs:38-51,92-95,998-1001; registry.rs:1301-1354] |
| `vox_doc_pipeline::diagram` (= `vox_diagram`): `check`, `validate`, `ValidateOpts`, `Emit`, `mermaid::export`, `GRAMMAR_CARD` | `crates/vox-diagram/src/lib.rs:56-61` | VoxDiagram oracle, feedback, Mermaid export for fidelity | [VERIFIED: crates/vox-doc-pipeline/src/lib.rs:6-7] |
| `vox_eval::corpus_stats::{wilson_interval, Z_95, paired_compare, mcnemar_exact_p}`, `vox_eval::report::DEFAULT_MAX_INFRA_RATE` | `crates/vox-eval/src/` | Wilson CIs, descriptive McNemar, 5 % infra ceiling | [VERIFIED: corpus_stats.rs:14,46,60,143; report.rs:17-18] |
| `eval_corpus::{extract_vox_code, sample_seed, repair_seed}` | `crates/vox-cli/src/commands/model/eval_corpus.rs:382,435,441` | Fence extraction, per-task seeds | [VERIFIED: read this session] |
| mermaid | 11.17.2 (`docs-astro/package.json` devDependency `"mermaid": "11.17.2"`) | Real Mermaid parser | [VERIFIED: docs-astro/package.json:29; node_modules/mermaid/package.json `"version": "11.17.2"`] |
| jsdom | 30.0.1 (`"jsdom": "30.0.1"`) | DOM shim for mermaid in Node | [VERIFIED: docs-astro/package.json:28] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| vox-cli module | `vox-orchestrator-mcp` ignored test | Reaches facade + diagram, but not `vox-eval` (Wilson/McNemar would be re-implemented or taken from `vox_orchestrator::models::wilson_score_interval`), and not the `eval_corpus` helpers |
| vox-cli module | `.vox` script via `vox run` | Impossible: `crates/vox-compiler/src/builtin_registry.rs` (1754 lines) has 0 case-insensitive matches for `llm`, and `crates/vox-compiler/src/eval/` has none either [VERIFIED: grep -ci this session] |
| `#[ignore]` live test | New `vox model eval-diagram` subcommand | A CLI command churns the command registry, `cli-command-surface.generated.md` and command-compliance; nightly running is deferred, so YAGNI |
| Newcombe method 10 | Tango asymptotic score | Better coverage, but needs a cubic or iterative root (ratesci `tangoci()` uses a cubic-root solution); Newcombe is closed-form and reuses `wilson_interval` |
| Newcombe method 10 | Bonett–Price adjusted Wald | Equally simple; within 0.002 of Newcombe on the worked example below. Not needed — one pre-registered method is enough |

**Installation:** none. Preconditions: `pnpm install --frozen-lockfile` in `docs-astro` (already done on this machine) and an OpenRouter key in Clavis (present).

## Package Legitimacy Audit

This phase installs no external package (cargo or npm). `mermaid@11.17.2` and `jsdom@30.0.1` are existing pinned devDependencies installed since Phase 84.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none new) | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```text
 contracts/eval/diagram-authoring/ (frozen BEFORE the run: tasks.v1.json, cards/, prompt template, decision rule)
        |
        v
 [model resolution]  ModelRegistry::from_cache --> decide(intent CodeGen, CloudOnly) --> ranked candidates
        |                                                        |
        |   probe each candidate in rank order via infer_with_retry (1 tiny call); first that answers = model A;
        |   repeat over candidates whose vendor org != A's  ==> model B (or none: report "one family")
        v
 for model in {A, B}:  for round r in 0..=2:            (round 0 = first pass, 1..2 = repair turns)
        |
        +--> [LLM calls]  llm_chat(LlmConfig locked for the run, temperature 0, seed per task/turn)
        |        task prompt + arm card  --->  raw response  --> append attempts.jsonl (raw, resolved model, tokens, cost)
        |
        +--> [extract] first fenced block, else trimmed text (same rule both arms)
        |
        +--> [oracles, deterministic]
        |      VoxDiagram arm: vox_diagram::check(src)  --> valid? diagnostics JSON (feedback)
        |                       if valid: validate(.., emit Mermaid) --> exported Mermaid text (for fidelity)
        |      Mermaid arm:   batch of texts --> node docs-astro/tests/lib/mermaid-batch.mjs (stdin JSONL)
        |                       --> {ok, error text (feedback), canonical sets}
        |
        +--> invalid & r < 2 ? schedule repair turn with that arm's own diagnostics : done
        v
 [scoring, offline-reproducible]  per-task first-pass / final validity per arm per model
        --> Wilson CI per arm, Newcombe paired CI for delta, NI verdict, McNemar (descriptive), fidelity
        --> tallies.json  --> report tables in docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md

 Offline re-score (no network, ordinary CI):
   attempts.jsonl --(Rust test: recompute)--> oracle-inputs.jsonl  [byte-equal]
   oracle-inputs.jsonl --(Node test: recompute)--> oracle-outputs.jsonl  [byte-equal]
   attempts.jsonl + oracle-outputs.jsonl --(Rust test: recompute)--> tallies.json  [byte-equal]
```

### Recommended file layout
```text
crates/vox-eval/src/corpus_stats.rs           # + paired_difference_ci(), ni_verdict() with tests (pure)
crates/vox-cli/src/commands/model/eval_diagram/
  mod.rs        # types: Task, Arm, AttemptRecord, RunHeader; load tasks + cards; <500 lines
  prompt.rs     # message construction, extraction (reuses eval_corpus::extract_vox_code), feedback text
  oracle.rs     # VoxDiagram verdict via vox_doc_pipeline::diagram; Mermaid batch subprocess (node)
  run.rs        # model resolution (decide + probe), round-based live loop, spend cap, JSONL append
  score.rs      # attempts + oracle outputs -> tallies (Wilson, Newcombe, verdict, fidelity), markdown table
crates/vox-cli/tests/eval_diagram_live.rs     # #[ignore] live run, configured by env (spend cap, N limit)
crates/vox-cli/tests/eval_diagram_rescore.rs  # offline: re-derive oracle-inputs and tallies byte-for-byte
docs-astro/tests/lib/mermaid-oracle.mjs       # + export async function parseError(src) -> null | string
docs-astro/tests/lib/mermaid-batch.mjs        # stdin JSONL {id,text,expected?} -> stdout JSONL {id,ok,error,sets,match}
docs-astro/tests/unit/diagram-authoring-rescore.test.mjs  # offline: re-derive oracle-outputs byte-for-byte
contracts/eval/diagram-authoring/
  README.md, manifest.v1.yaml (hashes, counts, decision rule, freeze commit)
  tasks.v1.json, cards/mermaid.txt (VoxDiagram card = vox_diagram::GRAMMAR_CARD, hash recorded)
  runs/<date>/{attempts.jsonl, oracle-inputs.jsonl, oracle-outputs.jsonl, tallies.json}
```
`contracts/**` is in the docs-quality path filter [VERIFIED: .github/workflows/docs-quality.yml:10], so the Node re-score test runs on any change there. Per AGENTS.md, a `contracts/` change also triggers the full Rust suite. Keep the Node test inside `tests/unit/` because CI runs `node --test "tests/unit/*.test.mjs"` [VERIFIED: docs-quality.yml:189-191].

### Pattern 1: Unpinned model resolution and the resolved id
**What:** pass an intent rather than a model id; lock the served config for the whole run so both arms use the same model.
**Evidence:**
- `decide(request: &ModelSelectionRequest, registry: &ModelRegistry) -> Option<ModelSelectionDecision>` with fields `selected_model`, `alternatives` (top 5 runners-up in scorer order), `outcome: SelectionOutcome { model_id, model_spec, reason, .. }` [VERIFIED: select.rs:55-64,92-95,256-291].
- `CandidateScope { AllProviders, LocalOnly, CloudOnly }` [VERIFIED: select.rs:46-51].
- `SelectionReason::{PremiumAlias, Scored, LocalOnly, EnvOverride { env_var }}` [VERIFIED: select.rs:655-672].
- `llm_config_for_spec(spec: &ModelSpec, task_type: TaskCategory) -> vox_actor_runtime::llm::LlmConfig` maps every `ProviderType` (OpenRouter → `LlmConfig::openrouter`, etc.) and sets `max_tokens` and `cost_per_1k` [VERIFIED: registry.rs:1301-1354].
- `infer_with_retry(options, messages, candidates: Vec<LlmConfig>) -> ActivityResult<Result<(LlmResponse, LlmConfig), String>>` tries candidates in order and returns the served one [VERIFIED: chat.rs:398-440].
- `LlmResponse { content, prompt_tokens, completion_tokens, model, cost_usd, .. }`; `model` is `resp.model` from the provider [VERIFIED: chat.rs:209-220]. `eval_corpus` records it as `provider_model` "(provenance: what actually answered)" [VERIFIED: eval_corpus.rs:570-571].
- Sampling knobs: `LlmConfig.temperature: Option<f32>`, `seed: Option<u64>` ("Providers that do not support seeds ignore it, so a recorded seed is a request, not a reproducibility guarantee") [VERIFIED: types.rs:63-70].

```rust
// Source: patterns from select.rs:92 + registry.rs:1301 + eval_corpus.rs:592-620 (sketch, not compiled)
use vox_orchestrator::models::{ModelRegistry, TaskCategory, llm_config_for_spec};
use vox_orchestrator::models::select::{decide, ModelSelectionRequest, SelectionIntent, CandidateScope, SelectionReason};

let registry = ModelRegistry::from_cache();
let mut req = ModelSelectionRequest::from_intent(SelectionIntent::for_task(TaskCategory::CodeGen));
req.candidate_scope = CandidateScope::CloudOnly;
let d = decide(&req, &registry).ok_or("no model eligible")?;          // None => human-action stop
anyhow::ensure!(!matches!(d.outcome.reason, SelectionReason::EnvOverride { .. }), "VOX_MODEL_FORCE pins a model");
// candidates in rank order: selected first, then alternatives; probe = one tiny llm call each.
let mut cfg = llm_config_for_spec(&d.outcome.model_spec, TaskCategory::CodeGen);
cfg.temperature = Some(0.0);
cfg.seed = Some(sample_seed(42, task_id, 0));                        // eval_corpus.rs:382
cfg.telemetry_task_category = Some("eval-diagram".into());
// LlmResponse.model is the resolved id -> record on every attempt line.
```
For model B, call `registry.rank_with_filter(...)` (pub, returns `Ranking { ranked, .. }` whose entries have `.spec` [VERIFIED: ranking.rs:78-86; select.rs:256-261]). Filter out model A's vendor org and require `ModelRegistry::key_is_present_for(&m)` (used by `decide`, select.rs:126). **Define "family" as the vendor org prefix before `/`** (`qwen/…`, `google/…`). `family::family_key` would count `qwen/qwen-3.6-27b` and `qwen/qwen3-coder-next-32b` as different families [VERIFIED: family.rs:18-60 strips versions, keeps name words].

### Pattern 2: Mermaid oracle as a batch subprocess
**Entry point:** `docs-astro/tests/lib/mermaid-oracle.mjs`. It shims jsdom (`new JSDOM(...)`, no `runScripts`), then `const { default: mermaid } = await import('mermaid'); mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' });`. `accepts(src)` is `try { await mermaid.parse(src); return true } catch { return false }`. `canon(src)` returns id-free canonical sets for `graph|flowchart`, `stateDiagram-v2` and `sequenceDiagram`, and throws on anything else. `sameSets(a, b, msg)` asserts equality, treating graph lists as multisets [VERIFIED: mermaid-oracle.mjs:7-30,121-144].

Add, without new deps:
```js
// docs-astro/tests/lib/mermaid-oracle.mjs — sibling of accepts(); null when valid, Mermaid's own message otherwise
export async function parseError(src) {
  try { await mermaid.parse(src); return null; } catch (e) { return String(e?.message ?? e); }
}
// docs-astro/tests/lib/mermaid-batch.mjs — one process per round; stdin/stdout JSONL, input order preserved
import { createInterface } from 'node:readline';
import { parseError, canon, sameSets } from './mermaid-oracle.mjs';
for await (const line of createInterface({ input: process.stdin })) {
  if (!line.trim()) continue;
  const { id, text, expected } = JSON.parse(line);
  const error = await parseError(text);
  let sets = null, match = null;
  if (error === null) { try { sets = await canon(text); } catch (e) { sets = { unsupported: String(e.message) }; } }
  if (sets && !sets.unsupported && expected) { try { sameSets(sets, expected, id); match = true; } catch { match = false; } }
  process.stdout.write(JSON.stringify({ id, ok: error === null, error, sets, match }) + '\n');
}
```
The Rust side spawns `node docs-astro/tests/lib/mermaid-batch.mjs` (cwd = repo root), writes the batch to stdin and reads the lines back. Before the run, fail fast if `docs-astro/node_modules/mermaid/package.json` is missing or its version is not `11.17.2`, and record that version in the run header. `Command::new("node")` does not match the `no-hardcoded-shell-spawn` pattern `Command::new\(\s*"(cmd|cmd\.exe|powershell|pwsh|sh|bash)"` [VERIFIED: layers.toml:489-500]. Precedent: `crates/vox-integration-tests/src/lib.rs:100` and `vox-cli/.../mcp_wiring.rs:153` spawn `node`.

**Measured here** (probe script in the session scratchpad; it imports the same mermaid 11.17.2 + jsdom shim by absolute path) [VERIFIED: probe run 2026-10-27, node v26.11.1]:

| Input | `mermaid.parse` result |
|-------|------------------------|
| `flowchart TD\n  a --> b` | OK `{"diagramType":"flowchart-v2"}` |
| `a[Label (with parens)] --> b` | `Parse error on line 2:\n...wchart TD  a[Label (with parens)] --> b\n----------------------^\nExpecting 'SQE', 'DOUBLECIRCLEEND', 'PE', ... got 'PS'` |
| `a --> end` | `Parse error on line 2: ... Expecting 'AMP', 'COLON', 'PIPE', ... got 'end'` |
| unterminated `subgraph S` | `Parse error on line 5: ... Expecting 'SEMI', 'NEWLINE', ... 'subgraph', 'end', ...` (long expected-token list) |
| `sequenceDiagram\n  A->>B hello` | `Parse error on line 2:\n...iagram  A->>B hello\n----------------------^\nExpecting 'TXT', got 'NEWLINE'` |
| `not a diagram` | `No diagram type detected matching given configuration for text: not a diagram` |
| `a["say \"hi\""] --> b`, `graph LR\n a -- label --> a` | OK (the parser is lenient) |

Cost: module load ~824 ms once; 300 parses averaged **1.22 ms/parse**. The oracle is never the bottleneck.

### Pattern 3: VoxDiagram oracle and feedback
`check(source: &str) -> CheckReport { schema_version, valid, diagnostics: Vec<Diagnostic>, stats, ir }`. `valid = !has_error`, and warnings never block validity [VERIFIED: check.rs:75,100-113; diag.rs:14-19,195-206]. `Diagnostic` derives `Serialize` with `code, severity, message, span (Location: start,end,line,..), help, fixes: Vec<Fix{span, replacement}>, related` [VERIFIED: diag.rs:21-58]. Repair feedback for this arm = `serde_json::to_string(&report.diagnostics)`, the same diagnostics the `vox_validate_diagram` MCP tool returns to agents. For fidelity, take `validate(src, ValidateOpts { from_mermaid: false, emit: Some(Emit::Mermaid) }).emitted` [VERIFIED: validate.rs:13-31,61] and pass it through the Node `canon`. Phase 84 proved `C(E) == R(IR)` for the exporter, so fidelity for both arms is computed by one canonical path.

Do **not** call `vox_diagram::repair` / `apply_fixes` inside the loop. The model must apply the fixes itself, or the "after repair" endpoint measures the tool, not the agent.

### Pattern 4: Round-based execution
Run all first-pass calls for one model, both arms; then one batch Mermaid oracle call and in-process `check`; then turn-1 repairs for the invalid ones; then a batch; then turn 2. This spawns `node` three times per model and allows bounded concurrency. The facade's AIMD throttle (`vox_llm_egress::throttle`, re-exported at llm/mod.rs:10-13) handles 429 backoff.

### Pattern 5: Recording and the offline re-score chain
- `attempts.jsonl`, line 1 = run header: `{"type":"run", schema_version, freeze_commit, tasks_hash, card hashes, prompt_template_hash, intent, scope, selection: {selected, reason, alternatives}, models: [{slot:"A", requested_id, provider, family}], temperature, seed_base:42, max_tokens, mermaid_version, started_at}`.
- One line per attempt: `{"type":"attempt", task_id, arm:"voxdiagram"|"mermaid", slot:"A"|"B", turn:0..2, seed, requested_model, resolved_model, feedback /* text sent in this turn, null for turn 0 */, raw_response, prompt_tokens, completion_tokens, cost_usd, latency_ms, transport_error}`.
- Do not store the card or system prompt per line. Messages are reconstructible from the frozen files, the previous raw responses and `feedback`, which keeps the file around 1 MB.
- **Rust offline test** (default CI, `tests` job): (1) parse attempts; recompute the extraction and `check` verdict and VoxDiagram feedback for every attempt; assert each recorded `feedback` equals the recomputed diagnostics of the previous turn (this catches oracle drift); assert turn k+1 exists iff turn k was invalid and k < 2. (2) Re-derive `oracle-inputs.jsonl` (Mermaid-arm extracted text, and exported Mermaid of valid VoxDiagram attempts, each with the task's `expected` sets) and `assert_eq!` against the committed bytes. (3) Read the committed `oracle-outputs.jsonl`, compute tallies, `serde_json::to_string_pretty`, `assert_eq!` against committed `tallies.json`. (4) Assert the report contains the rendered markdown table verbatim, which guards against hand-transcription error.
- **Node offline test** (docs-quality job): run `oracle-inputs.jsonl` through the same function the batch script uses and deep-equal / byte-equal against `oracle-outputs.jsonl`.
- `cost_usd` and `latency_ms` are non-deterministic at run time but fixed once recorded, so totals derived from the recording are byte-reproducible.

### Anti-Patterns to Avoid
- **Pinning a model** (`LlmConfig::openrouter("x/y")`, `llm-panel.v1.yaml`, `VOX_MODEL_FORCE`). Refuse `SelectionReason::EnvOverride`.
- **Failing over mid-run** (e.g. `infer_with_retry` with all candidates on every call). One task could be answered by model X in one arm and Y in the other, which breaks pairing. Probe once, lock the `LlmConfig`, and retry transport errors against the same config only.
- **Counting transport errors as invalid diagrams.** They are infra errors: exclude the pair from both arms, report the count, and mark the run incomplete if the rate exceeds `DEFAULT_MAX_INFRA_RATE = 0.05` [VERIFIED: vox-eval/src/report.rs:17-18].
- **Live run as a lib unit test.** `vox-orchestrator-mcp` has a `#[cfg(test)]` ctor that points `VOX_HOME` at an empty temp dir [VERIFIED: vox-orchestrator-mcp/src/lib.rs:260-301]; a lib unit test there would see no model cache or config. Integration tests in `tests/` are not affected. The recommended home, vox-cli, has no such ctor (grep: no `ctor::ctor`/`VOX_HOME` in `crates/vox-cli/src/lib.rs` or `tests/*.rs`).
- **Using the Node oracle's `accepts()` for feedback.** It discards the message.
- **An LLM judge for fidelity.** Fidelity is canonical-set equality via `canon` + `sameSets` only.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Wilson CI | new formula | `vox_eval::corpus_stats::wilson_interval(successes, trials, Z_95)` | Already tested |
| Paired significance (descriptive) | chi-square McNemar | `vox_eval::corpus_stats::paired_compare` (exact McNemar) | Exact at small discordant counts |
| Model choice / config | hardcoded ids, URLs | `decide` + `llm_config_for_spec` | Routing SSOT, Clavis keys, no hostnames |
| LLM call | `reqwest` to a provider | `vox_actor_runtime::llm::llm_chat` | Policy + detector `llm_provider_call` |
| Fence extraction | new regex | `eval_corpus::extract_vox_code` (first fenced block else trimmed text) | Same rule both arms, already used |
| Per-task seeds | ad hoc | `eval_corpus::{sample_seed, repair_seed}` (BLAKE3 framed, 31-bit) | Provider-safe seed range |
| Mermaid parsing / canonical sets | Rust heuristic, `vox_validate_mermaid` | `mermaid-oracle.mjs` (`mermaid.parse`, `canon`, `sameSets`) | The real parser; Phase 84 oracle |
| VoxDiagram canonical sets | copy `tests/support/roundtrip.rs::canon_graph` (~80 lines, test-only) | export valid IR to Mermaid → Node `canon` | Exporter proven lossless in Phase 84; avoids a defactor copy over 50 lines |
| Infra-error ceiling | new constant | `vox_eval::report::DEFAULT_MAX_INFRA_RATE` | Pre-registered 5 % precedent |

## Statistics (pre-registration)

### Notation
For each task i (single model), V_i, M_i ∈ {0,1} = first-pass validity of the VoxDiagram / Mermaid arm. 2×2 counts: a = both valid, b = V only, c = M only, d = neither; n = a+b+c+d.
p₁ = (a+b)/n, p₂ = (a+c)/n, Δ̂ = p₁ − p₂ = (b−c)/n.

### Primary endpoint and rule
- **Primary endpoint:** first-pass validity (turn 0) on the run's **model A** (the registry's top-ranked cloud model that answers the probe). Rationale: it is the direct counterpart of the advisory's "syntax error rate"; it has no tool asymmetry (VoxDiagram's diagnostics are richer than Mermaid's); and post-repair rates sit near the ceiling, where any comparison is uninformative.
- **Interval:** two-sided 95 % Newcombe hybrid-score interval (method 10, Wilson MOVER, no continuity correction) for Δ. Equivalent to a one-sided α = 0.025 NI test.
- **Margin:** δ = 0.10 (10 percentage points).
- **Verdict mapping:**
  - **non-inferior** if L > −0.10
  - **inferior** if U < −0.10
  - **inconclusive** otherwise
  - Descriptive flags printed beside the verdict, never changing it: "superior" if L > 0; "worse but within margin" if U < 0 and L > −0.10.
- **Secondary (reported, no decision):** post-repair validity (turn ≤ 2) with the same interval; per-arm Wilson CIs at turn 0 and final; exact McNemar p; model B's full table under the same rule (replication — if it disagrees with A the report says so; no pooling); structural fidelity (exact canonical-set match, among valid outputs and over all tasks); per-kind (graph/sequence) counts without inference; tokens and cost per arm.
- **Missing data:** a pair with a transport error in either arm after 3 same-model retries is excluded from both arms. Run incomplete if exclusions > 5 %.

### Formulas (implement in `vox-eval::corpus_stats`, no new dependency)
Wilson (existing, corpus_stats.rs:143-161): for s successes of n, z = 1.959963984540054,
centre = (p + z²/2n)/(1 + z²/n), half = z·√(p(1−p)/n + z²/4n²)/(1 + z²/n).

Newcombe method 10. [CITED: ratesci R package source `pairbinci.R`, `xmoverpairci` with `corc = FALSE` and Wilson base; Newcombe 1998, Stat Med 17:2635]:
```text
(l1,u1) = Wilson(a+b, n)      (l2,u2) = Wilson(a+c, n)
phi = (a*d - b*c) / sqrt((a+b)*(c+d)*(a+c)*(b+d)),   phi = 0 if the denominator is 0
L = Δ - sqrt((p1-l1)^2 - 2*phi*(p1-l1)*(u2-p2) + (u2-p2)^2)
U = Δ + sqrt((u1-p1)^2 - 2*phi*(u1-p1)*(p2-l2) + (p2-l2)^2)
```
ratesci verbatim: `lower <- p1phat - pp1hat - sqrt((p1phat - l1)^2 + (u2 - pp1hat)^2 - 2 * (p1phat - l1) * (u2 - pp1hat) * cor_hat)` and `cor_hat <- (x[1] * x[4] - x[2] * x[3]) / sqrt(x1p * n2p * xp1 * np2)` with `if (is.na(cor_hat) | is.infinite(cor_hat)) cor_hat <- 0`.

Signature to add (test-first):
```rust
/// Newcombe (1998) method 10 interval for p1 - p2 from paired 2x2 counts.
pub fn paired_difference_ci(a: usize, b: usize, c: usize, d: usize, z: f64) -> ConfidenceInterval
pub enum NiVerdict { NonInferior, Inconclusive, Inferior }
pub fn ni_verdict(ci: &ConfidenceInterval, margin: f64) -> NiVerdict
```

### Worked numbers (computed this session with the formula above; use as regression vectors)
| a | b | c | d | n | p1 [Wilson] | p2 [Wilson] | φ | Δ | L | U | verdict (δ=0.10) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 120 | 10 | 8 | 12 | 150 | 0.8667 [0.8030, 0.9120] | 0.8533 [0.7879, 0.9011] | 0.5026 | 0.0133 | −0.0439 | 0.0712 | non-inferior |
| 110 | 12 | 8 | 20 | 150 | 0.8133 [0.7434, 0.8676] | 0.7867 [0.7144, 0.8446] | 0.5858 | 0.0267 | −0.0325 | 0.0864 | non-inferior |
| 125 | 3 | 12 | 10 | 150 | 0.8533 | 0.9133 | 0.5421 | −0.0600 | −0.1150 | −0.0098 | inconclusive (worse) |
| 100 | 5 | 30 | 15 | 150 | 0.7000 | 0.8667 | 0.3852 | −0.1667 | −0.2399 | −0.0938 | inconclusive |
| 150 | 0 | 0 | 0 | 150 | 1.0000 [0.9750, 1] | 1.0000 [0.9750, 1] | 0 (den 0) | 0 | −0.0250 | 0.0250 | non-inferior |
| 140 | 0 | 10 | 0 | 150 | 0.9333 | 1.0000 | 0 (den 0) | −0.0667 | −0.1184 | −0.0276 | inconclusive |

Cross-check: Bonett–Price adjusted Wald on row 2 gives [−0.0340, 0.0867] (vs Newcombe [−0.0325, 0.0864]). These vectors come from the same formula, not an independent oracle. Add property tests as well: L ≤ Δ ≤ U; swapping arms (b↔c) negates and swaps the bounds; −1 ≤ L, U ≤ 1; b = c ⇒ Δ = 0.

### Margin justification (δ = 0.10)
1. The advisory itself claims Mermaid's error rate exceeds clean DSLs' by 10–31 points (18–35 % vs 4–8 %), so a 10-point margin is below the smallest gap the advisory asserts. [VERIFIED: advisory line 141]
2. Operationally, a first-pass deficit of at most 10 points costs at most ~0.1 extra repair turn per diagram, and the bounded repair loop exists to absorb exactly that.
3. A tighter margin is unaffordable at sane task counts: δ = 0.075 needs about 1.8× the tasks of δ = 0.10 (the power table shows 0.57–0.78 at N=150).
The margin is a judgement call; see Assumptions A1.

### Power (Monte Carlo, 20 000 replicates per cell, true Δ = 0, Newcombe rule) [VERIFIED: simulation run this session]
P(verdict = non-inferior), δ = 0.10; φ = correlation of the two arms' outcomes within a task (0 = independent; 0.3 = moderate shared difficulty, the realistic case):

| p (both arms) | N=60 φ0 / φ.3 | N=100 | N=120 | N=150 | N=200 |
|---|---|---|---|---|---|
| 0.85 | 0.33 / 0.44 | 0.50 / 0.64 | 0.58 / 0.72 | 0.67 / 0.82 | **0.80 / 0.91** |
| 0.90 | 0.42 / 0.52 | 0.64 / 0.76 | 0.72 / 0.84 | 0.80 / 0.91 | **0.91 / 0.97** |
| 0.95 | 0.60 / 0.71 | 0.84 / 0.92 | 0.90 / 0.96 | 0.96 / 0.99 | **0.99 / 1.00** |
| 0.98 | 0.83 / 0.89 | 0.97 / 0.99 | 0.99 / 1.00 | 1.00 / 1.00 | **1.00 / 1.00** |

Type I error (true Δ = −0.10, so the rule must say non-inferior ≤ 2.5 % of the time): 0.0227–0.0268 across p_M ∈ {0.90, 0.95}, N ∈ {60, 150}, φ ∈ {0, 0.3}. The rule holds its nominal level.

Closed-form check: n ≈ (z₀.₉₇₅ + z₀.₈)²·ψ/δ² = 7.849·ψ/δ², with ψ = P(discordant) ≈ 2p(1−p) under independence. At p = 0.90: ψ = 0.18, n ≈ 141, consistent with the simulated 0.80 at 150. [ASSUMED: standard paired-proportion sample-size approximation; the simulation is the authority.]

**Decision: N = 200 tasks.** 60 is clearly underpowered; 150 holds only for p ≥ 0.90. 200 gives ≥ 0.80 across the plausible 85–98 % range even under independence. Per CONTEXT, the task count is raised rather than the rule loosened. Caveat for the report: if VoxDiagram first-pass validity is well below 85 % (it is a new language seen only through the in-context card), power falls and "inconclusive" becomes likely. Report that honestly.

## Task Set Construction

- **Count and mix:** 200 tasks = **184 graph + 16 sequence**. The docs corpus has 38 `vox-diagram` fences (35 graph, 3 sequence) in 22 files, archive excluded [VERIFIED: grep over docs/src this session]; 200·35/38 = 184.2.
- **Sources:**
  1. ~33 tasks paraphrased from the Phase 84 frozen corpus. Expected sets already exist as `crates/vox-diagram/tests/fixtures/corpus-roundtrip/*.sets.json` (format below).
  2. ~21 tasks from `export-hostile/` (hostile labels and ids: `end`, `o`, `x`, quotes, `<br>`).
  3. The rest from a feature grid, so every hard case has a quota. Suggested minima over 200: punctuation in labels (parens, quotes, colon, slash, `#`, `&`, `<`/`>`, unicode arrow) ≥ 60; nested containers depth 2–3 ≥ 30; edge labels ≥ 80; self-loops ≥ 20; fan-out/fan-in ≥ 30; non-default shapes ≥ 30; Mermaid-hostile words (`end`, ids starting with `o`/`x`) ≥ 15; sequence notes (one and two actors) ≥ 10 of 16; replies ≥ 10 of 16; self-messages ≥ 4.
- **Language-neutral wording:** describe the diagram in prose with every label quoted verbatim, and always state direction ("left to right") so the expected direction is defined. Never mention syntax (no "subgraph", "container", "->"). Both arms get the identical text.
- **Expected-set format (JSON, one array in `tasks.v1.json`):** `{ "id", "kind": "graph"|"sequence", "features": [..], "prompt", "expected" }`, where `expected` uses the Phase 84 canonical sets. Graph: `{kind:'graph', direction:'down'|'right'|'up'|'left', nodes:[[label, shape, containerPath[]]], edges:[[nodeKey, nodeKey, label]], containers:[path[]]}`. Sequence: `{kind:'sequence', actors:[label], events:[['message', i, j, isReply, text] | ['note', [i(,j)], text]]}`. Shapes are `rect|rounded|diamond|cylinder|start|end` [VERIFIED: mermaid-oracle.mjs:48-49,52-118; corpus-roundtrip sets.json example read]. Use JSON, not TOML: Node has no built-in TOML parser and no new npm dep is allowed.
- **Contamination note:** the corpus diagrams were Mermaid in public docs until Phase 90. Memorisation can only favour the Mermaid arm, a conservative bias for a VoxDiagram non-inferiority claim. Report it.
- **Freeze:** commit `tasks.v1.json`, `cards/mermaid.txt`, the prompt template, and the decision rule (in `manifest.v1.yaml` and in the report's Method section) in one commit before any live call. Record its SHA as `freeze_commit` in the run header. Also list all task ids under `held_out_for_training` per the `contracts/eval` convention [VERIFIED: contracts/eval/README.md:23-25].

### Comparable grammar cards
- VoxDiagram card = `vox_diagram::GRAMMAR_CARD` (`include_str!("../grammar-card.txt")`, lib.rs:56): **2189 bytes, 359 words** [VERIFIED: wc this session]. Structure: 1-line intro with fence instruction; a `graph` example (direction, quoted labels, 2 shapes, nested container, edge label, self-loop, dotted path, comments); a `sequence` example (actors, message, note, reply); 6 numbered rules; a 10-line "Do NOT write Mermaid" list; a final "Run the validator; apply every fixes[] entry; repeat until valid."
- Mermaid card (new, `contracts/eval/diagram-authoring/cards/mermaid.txt`) mirrors it one-for-one: intro ("Put it in a mermaid fenced block. Quote every label: id[\"Label\"]"); a `flowchart LR` example covering the same constructs (`["..."]` labels, `[(...)]` cylinder, `{...}` diamond, nested `subgraph id ["Title"] ... end`, `-->|"label"|`, self-loop, `%%` comments); a `sequenceDiagram` example (`participant a as Label`, `->>`, `Note over`, `-->>` reply); 6 numbered rules (quoting, escaping `#quot;`, ids that break the parser (`end`, ids starting `o`/`x` after an arrow), one statement per line, sequence rules, limits); a 10-line "Do NOT" list of Mermaid's own traps (unquoted parens or brackets in labels, lowercase `end` id, `A-->B` with special characters, `style` lines, `&` chains, semicolons, ...); final line "Run the parser; fix the reported line; repeat until it parses."
- **Gate (offline test):** Mermaid card bytes and words within ±10 % of the VoxDiagram card; both cards' examples valid under their own oracle (Node test runs `parseError` over the Mermaid card's fenced examples); card hashes recorded in the run header.
- **Prompt template** (identical except `{LANG}`, `{FENCE}`, `{CARD}`, `{VALIDATOR}`): system = `{CARD}`; user = `Draw this as a {LANG} diagram. {task.prompt}\nReply with only one fenced {FENCE} block.`; repair user turn = `The {LANG} {VALIDATOR} rejected it:\n{feedback}\nReply with only the corrected diagram in one fenced {FENCE} block.` No truncation of feedback in either arm.

## Advisory Annotation (EVAL-02)

**Confirmed unsourced** [VERIFIED: read lines 136-143; `grep -n -i "http\|source\|reference\|citation\|\[\^"` over the 266-line file finds no URL, footnote or reference list]. The row is verbatim:
`| **LLM Syntax Error Rate** | **High (18–35%)** | **Low (4–8%)** | Low (5–10%) | Very Low (2–5%) | **Near Zero (<1%)** |` (line 141).
Line 44 names "*MermaidSeqBench*, *DiagramEval*" without links or figures. MermaidSeqBench exists (arXiv 2511.14967, 132 samples) but scores syntax with an LLM judge over ≤8B models; it does not report these percentages [CITED: arxiv.org/abs/2511.14967 summary]. Out of scope to rewrite (CONTEXT: "No other rewriting").

**Minimal edit (two touches, numbers left in place):**
1. Line 141 first cell → `| **LLM Syntax Error Rate** (unsourced, see note) |`. Other cells unchanged.
2. Directly after the table (before `### Detailed Findings by Candidate:`), add one blockquote:
   `> **Unsourced row.** The "LLM Syntax Error Rate" figures above cite no measurement and should not be read as evidence. For a measured comparison of VoxDiagram and Mermaid authoring validity, see [VoxDiagram authoring reliability findings](voxdiagram-authoring-reliability-findings-2026.md).`

Land it in the same commit as (or after) the report, because `vox ci check-links` validates relative links.

## Report

`docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md`, frontmatter mirroring the advisory: `category: "Architecture SSOTs"`, `topic: "doc-pipeline"`, `domain: "documentation"` [VERIFIED: advisory lines 4-6], plus `status: "research"`, `authorship: "llm-assisted"`, `authorship_model` and a `<!-- kb:summary -->` of 40–120 words. Sections: question; method (tasks, cards, prompt, models with resolved ids, oracles, repair); **decision rule copied verbatim from the freeze commit**; per-arm tables (turn-0 and final validity with Wilson CIs; paired Δ with Newcombe CI and verdict; McNemar p; fidelity; tokens and cost); model B replication; limitations (single sample per task at temperature 0; seeds are requests; Mermaid validity = `parse` acceptance, which is more lenient than render; VoxDiagram `check` also flags semantic errors such as undeclared nodes that Mermaid silently auto-creates; contamination favouring Mermaid; card wording effects). Any diagram in the report must be a `vox-diagram` fence.

## Cost & Runtime Estimate

- **Calls:** per model, 200 tasks × 2 arms = 400 first-pass calls, plus repairs (at 10–20 % first-pass invalid: 40–80 turn-1 and ~20–40 turn-2 calls), ≈ 460–520 calls. Two families ≈ **0.9–1.05 k calls**, plus probes (~10).
- **Tokens:** first pass ≈ card ~550 + template/task ~150 = ~0.7 k prompt [ASSUMED ~4 bytes/token on 2189-byte card] + 200–400 completion. Repair turns ≈ 1.5–2.5 k prompt. Total ≈ 0.9–1.1 M prompt + 0.3–0.4 M completion for a non-reasoning model. A reasoning model can emit thousands of hidden tokens per call: set `max_tokens` 4096 and record `completion_tokens`.
- **Money:** prices not fetched [ASSUMED]. Low single-digit USD at mid-tier OpenRouter prices; ~$10–$45 worst case for a flagship reasoning model. Use `LlmResponse.cost_usd` (provider-reported, else `cost_per_1k`) and a hard spend cap (precedent `eval_corpus --max-spend-usd`). **Default cap $25**; hitting it marks the run incomplete.
- **Wall time:** 3–15 s per call → ~1–4 h sequential; ~15–60 min at concurrency 4. Node oracle: 0.8 s × 6 spawns + ~1.2 ms per parse — negligible.
- **Offline re-score:** Rust ~1 k `check()` calls on small texts (milliseconds) plus JSON; Node ~0.8 s startup + ~1 k parses ≈ 2 s. Both well inside the fast test tiers.

## Common Pitfalls

### Pitfall 1: Registry picks a model that cannot answer
**What goes wrong:** `vox model explain ... --category codegen` selected `qwen/qwen-3.6-27b` with `"provider": "ollama"`, but the local Ollama has only `huihui_ai/qwen3-abliterated:8b` and `huihui_ai/Qwen3.8-abliterated:27b`. Top runners-up include unserved `mens/runs/...` checkpoints [VERIFIED: commands run this session]. The first call would fail.
**Avoid:** `CandidateScope::CloudOnly` (a scope, not a pin), then probe candidates in rank order and lock the first that answers. Record `selection.reason`, `alternatives` and every probe failure in the run header.

### Pitfall 2: Mid-run model drift breaks pairing
**Avoid:** one locked `LlmConfig` per slot; transport errors retry the same config ≤ 3 times, then become infra exclusions. Assert every attempt's `resolved_model` in a slot is identical (OpenRouter may return a dated variant; record it and flag a change).

### Pitfall 3: Asymmetric extraction or prompting
**Avoid:** one extraction function (first fenced block, any info string, else trimmed text), one template, equal-length cards, no truncation of feedback, and identical `max_tokens`, temperature and seed per (task, turn) across arms. Tests assert the template differs only in the four slots.

### Pitfall 4: Node oracle unavailable in the Rust CI job
**Avoid:** never call `node` from the Rust offline test. The chain splits re-scoring by runtime: Rust re-derives inputs and tallies; Node re-derives oracle outputs in docs-quality. A Rust test that skipped when Node is missing could not tell "passed" from "never ran" (AGENTS.md mutation-proof lesson).

### Pitfall 5: Treating `vox_diagram::repair` as the model's repair
**Avoid:** never auto-apply fixes. Optionally report "mechanically repairable" as a descriptive VoxDiagram-only number, outside both endpoints.

### Pitfall 6: Hand-transcribed report numbers
**Avoid:** the harness renders the markdown tables from `tallies.json`; the offline test asserts the report contains them verbatim.

### Pitfall 7: Fabricated or simulated results
**Avoid:** live-run gate. If `decide` returns None or no candidate answers the probe, stop with a human-action message. The offline-test fixture recording (if any, for scorer unit tests) lives under a `tests/fixtures/` path and is named synthetic, never under `runs/`.

### Pitfall 8: `contracts/` change widens CI
A `contracts/**` change triggers the full-workspace Rust suite (~21 min, AGENTS.md CI contract). Batch all contracts commits into one PR push.

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| Interval-overlap "significance" | Exact McNemar for paired binaries (vox-eval module doc) | Already the house rule; extend with an NI interval |
| LLM-judge syntax scoring (MermaidSeqBench) | Deterministic parser oracle | No judge variance; reproducible |
| Pinned eval panels (`llm-panel.v1.yaml`) | Registry-selected, resolved id recorded | Matches the no-pinned-models policy |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | δ = 0.10 is an acceptable non-inferiority margin | Statistics | A user who wants 0.05–0.075 needs ~2–4× the tasks; decide before the freeze commit |
| A2 | Primary endpoint = first-pass on model A (post-repair and model B are secondary) | Statistics | If the user prefers post-repair primary, the ceiling makes "non-inferior" nearly automatic |
| A3 | Paired sample-size approximation n ≈ 7.849·ψ/δ² | Statistics | Low: the simulation is the authority |
| A4 | ~4 bytes/token for the card; price ranges | Cost | Budget only; the spend cap bounds it |
| A5 | CONSORT-style mapping (inferior = whole CI below −δ) | Statistics | Wording only; the mapping is pre-registered either way |
| A6 | `CandidateScope::CloudOnly` is acceptable (scope, not pin) | Model selection | If local models must compete, probing handles failures but results then describe an abliterated local model |
| A7 | "Family" = vendor org prefix | Model selection | Two variants of one vendor could otherwise count as two families |

## Open Questions

1. **Margin and primary endpoint sign-off (A1, A2).** Recommendation: δ = 0.10, first-pass primary; confirm before the freeze commit.
2. **CloudOnly scope (A6).** Recommendation: CloudOnly; log the local candidates it excludes.
3. **Should the eval data be excluded from MENS training corpora?** Recommendation: list task ids under `held_out_for_training` in `manifest.v1.yaml`. Whether a MENS corpus walker reads `contracts/eval/**/*.jsonl` was not checked.
4. **Model registry freshness.** `vox model explain` logged `openrouter model catalog refresh failed error=db error`; the cache still lists 381 models (vendors: openai 65, qwen 47, google 35, mistralai 20, anthropic 15, ...). Recommendation: the executor runs `vox model discover` before the live run if selection looks stale.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| OpenRouter key (Clavis `OPENROUTER_API_KEY`) | live run | ✓ (`vox secrets get` → `sk-o…cc (redacted)`) | — | none → human action |
| Other provider keys (Gemini, OpenAI, Anthropic, Groq, ...) | — | ✗ (all `MissingOptional`) | — | OpenRouter covers many vendors |
| Model catalog cache | selection | ✓ 381 models, ≥ 2 vendor families | — | `vox model discover` |
| Local Ollama | not needed | ✓ (2 abliterated Qwen3 tags) | — | excluded by CloudOnly |
| node | Mermaid oracle | ✓ | v26.11.1 (CI uses 24) | — |
| docs-astro node_modules (mermaid, jsdom) | Mermaid oracle | ✓ | 11.17.2 / 30.0.1 | `pnpm install --frozen-lockfile` |
| vox CLI | env checks | ✓ | 0.6.0+build.7208 | `scripts/vox-dev.sh` |

**Missing dependencies with no fallback:** none. A live run is possible on this machine.
**Missing with fallback:** none relevant.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test / nextest (Rust); `node:test` (docs-astro) |
| Config file | existing; no new config |
| Quick run command | `timeout 600s cargo test -p vox-eval corpus_stats` |
| Full suite command | `cargo nextest run -p vox-cli -E 'test(eval_diagram)'` and `cd docs-astro && node --test tests/unit/diagram-authoring-rescore.test.mjs` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| EVAL-01 | Newcombe CI + NI verdict match worked vectors and properties | unit | `cargo test -p vox-eval paired_difference` | ❌ Wave 0 |
| EVAL-01 | prompt template differs only in 4 slots; cards within ±10 % size; extraction identical for both arms | unit | `cargo test -p vox-cli eval_diagram::prompt` | ❌ Wave 0 |
| EVAL-01 | scorer: synthetic recording → known tallies; infra exclusion; turn-structure invariant | unit | `cargo test -p vox-cli eval_diagram::score` | ❌ Wave 0 |
| EVAL-01 | re-score: attempts → oracle-inputs and → tallies byte-equal; report contains tables | integration (offline) | `cargo test -p vox-cli --test eval_diagram_rescore` | ❌ (after live run) |
| EVAL-01 | Node: oracle-inputs → oracle-outputs byte-equal; Mermaid card examples parse; `parseError` returns text | unit (node) | `node --test tests/unit/diagram-authoring-rescore.test.mjs` | ❌ Wave 0 |
| EVAL-01 | live run | manual / `#[ignore]` | `cargo test -p vox-cli --test eval_diagram_live -- --ignored --nocapture` | ❌ |
| EVAL-02 | advisory row annotated and link resolves | lint | `cargo run -p vox-cli ci check-links`; `cargo run -p vox-doc-pipeline -- --lint-only --paths <both docs>` | n/a |

### Sampling Rate
- **Per task commit:** the touched crate's new tests only (lean local verification).
- **Per wave merge:** `cargo test -p vox-eval`, `cargo test -p vox-cli eval_diagram`, the Node re-score test.
- **Phase gate:** hosted CI green (tests + docs-quality) before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `crates/vox-eval/src/corpus_stats.rs` tests for `paired_difference_ci`, `ni_verdict`
- [ ] `docs-astro/tests/unit/diagram-authoring-rescore.test.mjs` (plus `parseError` test in the existing oracle test file)
- [ ] Mutation proof: break `phi` (force 0) or the verdict comparison and confirm a test fails; corrupt one byte of `oracle-outputs.jsonl` and confirm the Rust re-score fails (AGENTS.md "verify guards by mutation").

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | Model output is untrusted text. `vox_diagram::check` enforces caps (64 KiB, depth 4); Mermaid runs in jsdom with no `runScripts`/resources and `securityLevel: 'strict'`; texts go to `node` via stdin JSON, never argv or a shell |
| V6 Cryptography | no | (seed hashing reuses `sample_seed` → `vox_crypto` facade) |
| V8 Data Protection / secrets | yes | Keys only via Clavis inside the facade; recording stores messages and responses only, never config, headers or keys; grep the JSONL for `sk-` before committing |

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Secret leaked into committed recording | Information disclosure | Never serialize `LlmConfig`; pre-commit grep for key prefixes |
| Hostile output DoS'ing a parser | Denial of service | Size caps in `check`; per-batch subprocess timeout |
| Prompt-injected output asking for tool use | Elevation | No tools passed (`tools: None`); outputs are only parsed |
| Spend runaway | Denial of wallet | Hard `max_spend_usd` cap, checked before each call |

## Sources

### Primary (HIGH confidence)
- In-repo files read this session: `crates/vox-actor-runtime/src/llm/{mod.rs,chat.rs,types.rs}`, `crates/vox-orchestrator/src/models/{select.rs,registry.rs,family.rs}`, `crates/vox-cli/src/commands/model/eval_corpus.rs`, `crates/vox-cli/Cargo.toml`, `crates/vox-eval/src/{corpus_stats.rs,compare.rs,report.rs}`, `crates/vox-diagram/src/{lib.rs,check.rs,diag.rs,validate.rs,mermaid/export.rs}`, `crates/vox-diagram/grammar-card.txt`, `crates/vox-doc-pipeline/src/lib.rs`, `docs-astro/tests/lib/mermaid-oracle.mjs`, `docs-astro/tests/unit/mermaid-oracle.test.mjs`, `docs-astro/package.json`, `.github/workflows/{docs-quality.yml,harness-eval-nightly.yml}`, `contracts/ci/crate-edges.allow.v1.json:2470-2483`, `docs/src/architecture/layers.toml`, `contracts/eval/{README.md,llm-panel.v1.yaml}`, the advisory.
- Commands run: `vox secrets status/get` (redacted), `vox model explain`, `vox model show`, `vox model list`, Ollama `/api/tags`, the mermaid probe, the power and worked-number scripts (session scratchpad).

### Secondary (MEDIUM confidence)
- ratesci R package source `pairbinci.R` (Newcombe MOVER, Bonett–Price, Tango variance): https://raw.githubusercontent.com/cran/ratesci/master/R/pairbinci.R
- ratesci paired-contrasts vignette (SCAS/Tango context): https://archive.linux.duke.edu/cran/web/packages/ratesci/vignettes/paired_contrasts.html
- MermaidSeqBench (arXiv 2511.14967): https://arxiv.org/abs/2511.14967

### Tertiary (LOW confidence)
- Fagerland, Lydersen & Laake 2014 recommendations (not fetched); CONSORT NI verdict mapping (training knowledge).

## Metadata

**Confidence breakdown:**
- Placement / APIs / oracle: HIGH — files read and commands run this session.
- Statistics: MEDIUM-HIGH — formula from cited package source; power and type I error simulated here.
- Cost: LOW — token-based estimate, prices not fetched; the spend cap bounds the risk.

**Research date:** 2026-10-27
**Valid until:** 2026-11-26 (registry contents and prices move faster; re-check selection before the live run)
