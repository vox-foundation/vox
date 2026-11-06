---
title: "Tolerant CLI Ergonomics & Precomputed Semantic Routing Research (2026)"
description: "Architectural evaluation of zero-runtime-LLM CLI tolerance—prefix unfolding, order-agnostic verb-noun normalization, precomputed semantic alias matrices, and ACI-gated execution safety—for human ergonomics and LLM determinism."
category: "Architecture SSOTs"
status: "research"
topic: "cli-surface"
domain: "platform"
authorship: "llm-assisted"
authorship_model: "Gemini 4 Argon"
training_eligible: false
training_rationale: "Research synthesis and architectural design; not human-attested for MENS training."
owner: "team-platform"
review_cadence: "90d"
code_anchors:
  - crates/vox-cli/src/main.rs
  - crates/vox-cli/src/lib.rs
  - crates/vox-cli/src/command_catalog.rs
  - crates/vox-cli/src/fuzzy.rs
  - crates/vox-cli-core/src/command_contract.rs
  - crates/vox-cli-core/src/command_registry_model.rs
  - crates/vox-capability-registry/src/explain.rs
  - crates/vox-capability-registry/src/index.rs
  - crates/vox-search/src/embeddings.rs
  - crates/vox-search/src/fuzzy_trigram.rs
  - crates/vox-actor-runtime/src/llm/embed.rs
  - crates/vox-cli/src/commands/ci/operations_catalog.rs
  - crates/vox-cli/src/commands/ci/capability_index/mod.rs
  - contracts/operations/catalog.v1.yaml
  - contracts/cli/command-registry.yaml
  - contracts/capabilities/capability-index.v1.json
related:
  - docs/src/architecture/agentos-ssot-2026.md
  - docs/src/architecture/llm-first-shell-nushell-skills-mcp-research-2026.md
  - docs/src/contributors/documentation-governance.md
---

# Tolerant CLI Ergonomics & Precomputed Semantic Routing Research (2026)

<!-- kb:summary -->
This document investigates how to make the `vox` CLI maximally forgiving of human memory—supporting unambiguous prefix completions, out-of-order subcommand and flag tokens, synonym auto-aliasing, and typo recovery—without invoking an LLM on the runtime hot path or degrading determinism for LLM agents. It evaluates five architectural mechanisms, defines a build/CI-time semantic compiler powered by `vox_actor_runtime::llm` and Gemini 3.5 Flash-Lite / Gemini Embedding 2, establishes an edit-vs-semantic drift firewall, and specifies a side-effect-aware execution contract gated by `side_effect_class` and TTY/agent detection.
<!-- /kb:summary -->

## 1. Executive Summary & Core Thesis

A large, multi-domain CLI surface (`vox` currently spans 200+ operations across `app`, `workflow`, `ai`, `interop`, `data`, and `platform` lanes, plus Latin domain groupings like `fabrica`, `mens`, `ars`, `diag`, and `clavis`) creates a fundamental tension between **human recall** and **machine determinism**:

- **Humans** think associatively, forget exact hierarchy order (`vox list secrets` vs. `vox secrets list`), substitute synonyms (`vox remove pkg` vs. `vox rm pkg` vs. `vox uninstall pkg`), interleave flags before subcommands (`vox --json secrets get KEY` vs. `vox secrets --json get KEY`), and prefer short prefixes (`vox sec doc`).
- **LLMs and CI scripts** require a stable, canonical schema (`vox <noun> <verb> --flags`), deterministic exit codes, zero interactive prompts in non-TTY pipes, and actionable structured error diagnostics when they hallucinate a flag or subcommand from another tool's pretraining distribution (e.g., `cargo`, `npm`, `git`, `kubectl`).

**Architectural Verdict:**
1. **Never put an LLM on the synchronous CLI parse hot path.** Even a fast cloud call (`gemini-3.1-flash-lite` or `gemini-3.5-flash-lite`) adds 150–400 ms of network latency, requires connectivity/credentials, and introduces non-deterministic routing. Local weight loading adds cold-start memory overhead inappropriate for a `<5 ms` CLI binary dispatch.
2. **Use LLMs and Embeddings at *CI / Contract-Sync Time* as a "Semantic Alias & Synonym Compiler", and use Pure-Rust Deterministic Cascade Resolution at *Runtime*.**
3. **Gate Auto-Execution Strictly by `side_effect_class` and Confidence Tier.** Syntactic normalizations (flag hoisting, unambiguous prefix expansion, registered canonical synonyms) can auto-execute safely (or auto-execute `read_only` commands with a `stderr` notice), whereas fuzzy/semantic distance matches on `local_mutation` or `remote_mutation` commands **must never silently auto-execute**.

---

## 2. Current Codebase Baseline (Verified 2026-11-01)

Vox already possesses roughly 70% of the foundational infrastructure needed for a zero-latency tolerant CLI, spread across four crates:

| Subsystem | Code Anchor | Current Capability | Gap for Tolerant CLI Execution |
|---|---|---|---|
| **CLI Parser & Dispatch** | [`crates/vox-cli/src/main.rs`](../../../crates/vox-cli/src/main.rs), [`crates/vox-cli/src/lib.rs`](../../../crates/vox-cli/src/lib.rs) | `VoxCliRoot::parse()` via `clap` v4; pre-clap manual argv scan for ML binary delegation (`vox-ml-cli`) | `clap` fails hard on out-of-order subcommand tokens (`vox get secrets`), unknown synonyms, or global flags placed in unexpected positions; `infer_subcommands` / `infer_long_args` are not enabled globally. |
| **Operations & Command SSOT** | [`contracts/operations/catalog.v1.yaml`](../../../contracts/operations/catalog.v1.yaml), [`crates/vox-cli/src/commands/ci/operations_catalog.rs`](../../../crates/vox-cli/src/commands/ci/operations_catalog.rs) | Canonical `OperationRow` with `id`, `title`, `description`, `intent_tags`, `side_effect_class`, `reversible`, `latin_aliases`, `cli.path`, and `mcp.name` | `intent_tags` and `latin_aliases` are not compiled into `clap` aliases or a runtime argv normalizer; flag-level aliases (`--format` vs `--output`) are not indexed in the catalog. |
| **Capability Index & Fuzzy Explain** | [`crates/vox-capability-registry/src/explain.rs`](../../../crates/vox-capability-registry/src/explain.rs), [`contracts/capabilities/capability-index.v1.json`](../../../contracts/capabilities/capability-index.v1.json) | `resolve()` runs exact alias matching + token-bag overlap (`0.9 * hit / len`) + `strsim::jaro_winkler` (threshold `0.75`) over embedded `capability-index.v1.json` | Used only when the user explicitly runs `vox explain <query>`; not wired into `VoxCliRoot::try_parse()` error recovery or pre-parse argv normalization. |
| **Fast Fuzzy & Local Embeddings** | [`crates/vox-cli/src/fuzzy.rs`](../../../crates/vox-cli/src/fuzzy.rs), [`crates/vox-actor-runtime/src/llm/embed.rs`](../../../crates/vox-actor-runtime/src/llm/embed.rs) | `nucleo::Matcher` in `vox-cli`; deterministic 256-dim FNV-1a unigram/bigram/trigram L2-normalized `local_embed_text()` in `vox-actor-runtime` | Neither is connected to CLI parse failure recovery or build-time semantic synonym expansion. |

---

## 3. Why Naïve Semantic Auto-Aliasing Fails ("Beware Drift")

The prompt notes: *"one idea is we can precompute semantic distance between terms and create an autoaliaser, but beware drift; it would have to be automateable and not require the full execution of an llm every time you use a command"*.

In CLI design, pure vector-similarity auto-aliasing suffers from four well-documented failure modes if applied without structural constraints:

1. **The Antonym & Co-hyponym Trap (Cosine Proximity != Intent Equivalence):**
   - In dense embedding spaces, antonym pairs (`start` / `stop`, `add` / `remove`, `lock` / `unlock`, `import` / `export`, `enable` / `disable`) share almost identical distributional contexts and routinely score **0.78–0.88 cosine similarity**.
   - An unconstrained nearest-neighbor auto-aliaser that maps an unknown verb to the closest vector will catastrophically alias a destructive verb (`clean`, `drop`, `purge`) to a constructive or diagnostic one—or vice versa.
2. **Surface Growth & Voronoi Cell Shift (Silent Version Drift):**
   - Suppose in v0.4, `vox deploy` is the closest semantic neighbor to a user typing `vox ship` (similarity `0.81`).
   - In v0.5, a contributor adds `vox ci release-build` or `vox share publish`. The Voronoi partition of the embedding space shifts: `vox ship` now lands closer to `vox share publish` (`0.83`). A script or muscle memory relying on the implicit semantic alias silently changes target across versions.
3. **Cross-Axis Confusion (Verb vs. Noun vs. Flag):**
   - Measuring semantic distance against flat strings conflates *actions* (`verify`, `check`, `lint`) with *objects* (`policy`, `secrets`, `db`, `schema`).
4. **LLM Agent Poisoning:**
   - When an LLM agent invokes `vox` via `vox_run_shell` or a script and hallucinates a command (`vox db migrate --dry-run`), silent fuzzy execution hides the schema error from the agent or executes the wrong subcommand. Agents need **strict contract enforcement + structured machine-readable redirects** so they can self-repair on the next turn.

---

## 4. Better Ideas: The 5-Stage Zero-Runtime-LLM Resolution Architecture

Instead of a single fuzzy/semantic hammer, the optimal architecture separates **deterministic compile-time/runtime normalization** (Stages 1–3) from **precomputed semantic synonym tables** (Stage 4) and **LLM-assisted CI mining** (Stage 5).

```vox-diagram
graph {
    direction: down
    node argv: "Raw Argv (std::env::args)"
    node stage0: "Stage 0: Execution Context Probe (TTY vs CI/LLM Agent)"
    node stage1: "Stage 1: Exact Clap Parse + Prefix Unfolding (infer_subcommands / infer_long_args)"
    node dispatch: "Execute Handler (< 1 ms)"
    node stage2: "Stage 2: Order-Agnostic Token & Flag Normalizer (Verb↔Noun Swap, Global Flag Hoisting)"
    node gate: "Side-Effect & Context Gate"
    node stage3: "Stage 3: Committed Alias & Cross-CLI Rosetta Table (contracts/cli/semantic-aliases.v1.json)"
    node stage4: "Stage 4: Hybrid Lexical + Precomputed Concept Vector Ranker (< 2 ms In-Memory)"
    node autoexec: "Auto-Execute + Stderr Canonical Hint"
    node prompt: "Single-Keystroke Confirmation [Y/n]"
    node diag: "Structured Diagnostic + Exact Copy-Paste / JSON Redirect"
    argv -> stage0
    stage0 -> stage1
    stage1 -> dispatch: "Success"
    stage1 -> stage2: "Clap Error (InvalidSubcommand / UnknownArgument)"
    stage2 -> gate: "Unique Canonical Path Found"
    stage2 -> stage3: "No Structural Match"
    stage3 -> gate: "High-Confidence Alias Hit"
    stage3 -> stage4: "Unmatched"
    gate -> autoexec: "Tier A: Exact Reorder / Prefix + Read-Only"
    gate -> prompt: "Tier B: Mutating OR Ambiguous + Interactive TTY"
    gate -> diag: "Tier C: Non-TTY / LLM Agent / Low Confidence"
    stage4 -> gate
}
```

### Idea 1: Native Prefix Unfolding & Global Flag Hoisting (Stage 1 — 0 ms Overhead)

Before inventing custom matchers, enable and tighten `clap`'s built-in deterministic tolerance on `VoxCliRoot` and its subcommands:

- **Unambiguous Prefix Completion (`infer_subcommands(true)` + `infer_long_args(true)`):**
  - Allows `vox sec doc` → `vox secrets doctor`, `vox comp --rec` → `vox commands --recommended`, `vox fab ch` → `vox fabrica check`.
  - **Safety property:** `clap` only resolves a prefix when it matches **strictly one** subcommand or long flag at that tree node. If `vox c` matches `check`, `compile`, `config`, and `ci`, `clap` immediately rejects it and lists the colliding candidates.
- **Universal Flag Hoisting (`global = true` + Pre-Parse Flag Float):**
  - Humans and LLMs frequently place `--json`, `--quiet`, `--verbose`, `--color`, or `--help` at the end of a command (`vox secrets list --json`) or between subcommand tokens (`vox --json mens train`).
  - Marking all root `GlobalOpts` fields with `#[arg(long, global = true)]` in `vox-cli-core` eliminates positional flag errors across all 200+ subcommands.

### Idea 2: Order-Agnostic Command Normalization — "Bag-of-Command-Tokens" (Stage 2 — `<0.2 ms`)

A major source of human CLI friction is remembering whether a CLI uses `<noun> <verb>` (`vox secrets list`, `vox container run`, `vox skill search`) or `<verb> <noun>` (`vox list secrets`, `vox run container`, `vox search skill`), or forgetting an intermediate umbrella token (`vox arena-join` instead of `vox ars ludus arena-join`).

Because the entire `vox` command tree is small (~250 leaf paths in `contracts/cli/command-registry.yaml`), we can normalize token order **deterministically without any LLM or vector math**:

1. **Leading Command-Token Extraction:**
   - When `VoxCliRoot::try_parse_from(&argv)` returns `ErrorKind::InvalidSubcommand`, split `argv` into:
     - `flags`: tokens starting with `-` (and their attached values, looked up via `CommandCatalogArgument::takes_value`).
     - `cmd_candidates`: leading non-flag, non-path tokens (tokens that do not contain `/`, `\`, `.vox`, `.rs`, `.json`, `.toml`, or exist on disk via `Path::exists`).
     - `positional_tail`: remaining arguments (file paths, queries, IDs).
2. **Permutation & Suffix Matching against `CommandCatalog`:**
   - **Rule 2a (Verb-Noun / Token Permutation):** Sort the token set of `cmd_candidates` and compare against the sorted `path` tokens of every `CommandCatalogEntry`.
     - Example: User types `vox list secrets` → token set `{"list", "secrets"}` matches `["secrets", "list"]` uniquely → rewrites to `vox secrets list`.
     - Example: User types `vox doctor secrets` → token set `{"doctor", "secrets"}` matches `["secrets", "doctor"]` uniquely → rewrites to `vox secrets doctor`.
   - **Rule 2b (Unique Leaf Elision):** If a user omits an umbrella namespace (e.g. `vox socrates-metrics` instead of `vox codex socrates-metrics`, or `vox command-compliance` instead of `vox ci command-compliance`), and the leaf subcommand name is **globally unique** across `CommandCatalog`, resolve directly to the unique parent path.
   - **Rule 2c (Flag-to-Subcommand & Subcommand-to-Flag Duals):**
     - Humans often type `vox commands --search foo` as `vox commands search foo`, or `vox features --domain ai` as `vox features ai`. Inspecting `CommandCatalogEntry::arguments` allows a single remaining positional token on a flag-only subcommand to bind to its primary filter flag when unambiguous.

### Idea 3: The "Cross-CLI Rosetta Stone" & Decomposed Verb/Noun Synonym Matrix (Stage 3 — `<0.5 ms`)

Instead of computing semantic similarity between whole command strings at runtime, decompose the vocabulary into **constrained Grammatical Slots** in `contracts/operations/catalog.v1.yaml`:

- **Canonical Action Verbs (~25 total in Vox):**
  - `list` $\leftarrow$ `{ls, show, dir, enumerate}`
  - `remove` $\leftarrow$ `{rm, del, delete, uninstall, drop}`
  - `add` $\leftarrow$ `{install, create, new, register}`
  - `check` $\leftarrow$ `{verify, validate, lint, typecheck}`
  - `doctor` $\leftarrow$ `{diag, diagnose, health, healthcheck}`
  - `explain` $\leftarrow$ `{describe, info, whatis, man}`
  - `update` $\leftarrow$ `{up, refresh, sync}`
- **Canonical Domain Nouns (~30 total in Vox):**
  - `secrets` $\leftarrow$ `{secret, clavis, creds, credentials, keys, vault, env-secrets}`
  - `skill` $\leftarrow$ `{skills, ars}`
  - `package` / `pm` $\leftarrow$ `{pkg, packages, deps, dependencies}`
  - `mesh` $\leftarrow$ `{populi, cluster, nodes, swarm}`
  - `speech` $\leftarrow$ `{oratio, stt, transcribe, voice, audio}`
  - `db` $\leftarrow$ `{database, codex, sqlite, turso}`
- **Cross-Ecosystem Idiom Redirects ("Muscle-Memory Bridges"):**
  - Developers and LLMs constantly bring muscle memory from `cargo`, `npm`/`pnpm`, `git`, `docker`, and `kubectl`.
  - A small declarative table maps foreign idioms to their Vox equivalents:
    - `vox clippy` $\rightarrow$ `vox audit code` (or `vox stub-check`)
    - `vox install <pkg>` $\rightarrow$ `vox add <pkg>` (project dependency) vs `vox upgrade` (toolchain)
    - `vox get pods` / `vox nodes` $\rightarrow$ `vox populi status` / `vox explain mesh`
    - `vox fmt --check` $\rightarrow$ `VOX_FMT_CHECK=1 vox run scripts/fmt.vox`

Because slots are combined compositionally (`NounSynonym(t1) + VerbSynonym(t2)`), `vox rm secret FOO` deterministically normalizes to `vox secrets remove FOO` via two hash-map lookups (`O(1)`), with **zero risk of antonym collision**.

### Idea 4: Build-Time LLM & Embedding Compiler with Ratcheted Lockfile (Stage 4 & 5)

Where *do* LLMs and embeddings shine? **In an offline/CI generator (`vox ci cli-synonyms --write`) that mines, scores, and audits aliases against `contracts/operations/catalog.v1.yaml` and commits a verified, human-reviewable lockfile (`contracts/cli/semantic-aliases.v1.json`).**

#### How the Precomputation Pipeline Works

1. **Input Surface Extraction:**
   - Read every `OperationRow` from `contracts/operations/catalog.v1.yaml` and every `CommandCatalogEntry` (including flags and `about` strings) from `command_catalog::build_catalog()`.
2. **Candidate Generation via `vox_actor_runtime::llm` (Gemini 3.5 Flash-Lite Structured Output):**
   - When a new CLI operation is added (or lacks a synonym entry in the lockfile), the generator invokes `vox_actor_runtime::llm::llm_chat` configured for `gemini-3.5-flash-lite` (or `gemini-3.8-flash`) using JSON Schema structured output (`response_format: { type: "text", mime_type: "application/json", schema: ... }`).
   - The prompt asks the model to generate:
     - 3–5 natural-language developer phrasings (`intent_phrases`),
     - likely CLI muscle-memory aliases from `cargo`/`git`/`npm`/`kubectl`,
     - flag synonyms (e.g., `--out` / `--output-dir` for `-o`),
     - **Explicit Negative / Antonym Exclusions (`must_not_match`)** — e.g., for `secrets set`, explicitly listing `get`, `delete`, `rm`, `list` as forbidden aliases.
3. **Cross-Encoder / Embedding Margin Verification (`gemini-embedding-2` or `local_embed_text`):**
   - Every candidate alias $a$ for target operation $T_i$ is embedded via `vox_search::embeddings::EmbeddingService` (using `gemini-embedding-2` at 768 dims in connected CI, or `local_embed_text` offline).
   - **The Drift & Collision Guard (Margin Rule):**
     An alias $a$ is accepted into the auto-redirect tier **if and only if**:
     $$\cos(e(a), e(T_i)) \ge \tau_{\text{min}} \quad \text{AND} \quad \cos(e(a), e(T_i)) - \max_{j \neq i} \cos(e(a), e(T_j)) \ge \Delta_{\text{margin}}$$
     where $\tau_{\text{min}} = 0.82$ and $\Delta_{\text{margin}} = 0.08$.
   - If a newly added command $T_k$ in a future PR encroaches on $a$ such that the margin drops below $\Delta_{\text{margin}}$, `vox ci command-compliance` flags the collision at PR time! The alias is either pinned explicitly in `catalog.v1.yaml` or demoted from "auto-resolve" to "disambiguation shortlist" (`Did you mean X or Y?`). **This completely solves silent semantic drift.**
4. **Compact Binary Embedding Quantization for Fallback Free-Text Search:**
   - For arbitrary multi-word queries (`vox how do i rotate api keys`), we can precompute 256-bit binary-quantized (1-bit sign) or int8 embeddings for all ~250 CLI operations and embed them via `include_bytes!` in `vox-capability-registry`.
   - Runtime memory footprint: $250 \text{ ops} \times 64 \text{ bytes} = \mathbf{16 \text{ KB}}$ total binary size, searched via hardware `popcnt` (XOR + Hamming distance) in **$<50\ \mu\text{s}$** with zero network calls.

---

## 5. Preserving & Enhancing Usability for LLMs (Dual-Persona Contract)

A critical requirement from the prompt is: *"without making it less usable by LLMs"*.

Tolerance that helps a human in an interactive terminal can **harm** an LLM agent if not conditioned on the caller's execution environment. We enforce a strict **Dual-Persona Execution Contract**:

| Dimension | Human Interactive Persona (`stdin.is_terminal() && stdout.is_terminal() && !is_agent_env()`) | LLM Agent / CI Script Persona (`--json`, `CI=true`, `VOX_MCP=1`, or non-TTY pipe) |
|---|---|---|
| **Unambiguous Prefix (`vox sec list`)** | Executes; prints dim `stderr` note: `→ vox secrets list` | Executes; emits canonical command in `stderr` / ACI metadata |
| **Verb↔Noun Swap (`vox list secrets`)** | Auto-executes if `side_effect_class == "read_only"`; prompts `[Y/n]` if mutating | **Never silently executes mutating swaps.** Returns exit code `2` with machine-parsable JSON/text redirect: `error[CLI-SWAP]: unknown command 'vox list secrets'. Canonical command: 'vox secrets list'` |
| **Precomputed Synonym (`vox clavis doctor`)** | Executes (`clavis` is an explicit registered alias) | Executes (`clavis` is an explicit registered alias) |
| **Heuristic / Fuzzy Typo (`vox secrts lst`)** | If `read_only` and Jaro-Winkler $\ge 0.90$ with margin $\ge 0.15$: executes with warning or prompts `[Y/n]`. If mutating: always prompts `[Y/n]`. | **Hard fail (exit code `2`)** with structured `did_you_mean` array + exact flag signature so the LLM updates its tool/shell call in 1 turn without corrupting state. |
| **Interactive Prompts (`[Y/n]` chooser)** | Enabled when TTY is attached | **Strictly forbidden** (prevents hanging headless subagents / `timeout 900s` rules). |

### Why Structured "Self-Repair" Diagnostics Beat Silent Fuzzy Execution for LLMs

When an LLM runs `vox build --release app.vox` (hallucinating Cargo's `--release` flag on `vox build`, which actually belongs on `vox compile`), silently ignoring or guessing the flag causes subtle bugs. Instead, when `VoxCliRoot::try_parse()` fails on an unknown flag or subcommand, Vox should emit a **1-turn self-repair diagnostic** derived from `CommandCatalog` and `CapabilityIndex`:

```text
error: unexpected argument '--release' found on 'vox build'

  note: 'vox build' emits TypeScript output and does not take '--release'.
  help: to compile a native release binary, use:
        vox compile app.vox --release
  help: valid flags for 'vox build <FILE>':
        -o, --out-dir <DIR>    Output directory for generated TypeScript
        --v0                   Resolve @v0 component annotations
```

When `--json` or `VOX_CLI_DIAG_FORMAT=json` is active (or inside `vox_run_shell` ACI envelopes), this error serializes directly into `aci.error_recovery`:

```json
{
  "error_code": "CLI_FLAG_ON_WRONG_SUBCOMMAND",
  "attempted": ["vox", "build", "--release", "app.vox"],
  "canonical_suggestion": ["vox", "compile", "app.vox", "--release"],
  "side_effect_class": "local_mutation",
  "auto_executed": false
}
```

---

## 6. Side-Effect-Aware Execution Matrix (`side_effect_class` Integration)

Every operation in [`contracts/operations/catalog.v1.yaml`](../../../contracts/operations/catalog.v1.yaml) already carries a `side_effect_class` (`read_only`, `local_mutation`, `remote_mutation`) and `reversible: Option<bool>`. Combining this metadata with the resolution stage yields a deterministic safety policy:

| Resolution Mechanism | `read_only` (e.g., `explain`, `commands`, `status`, `list`, `check`, `doctor`) | `local_mutation` + `reversible: true` (e.g., `fmt`, `build`, `add`) | `local_mutation` (irreversible) or `remote_mutation` (e.g., `remove`, `clean`, `deploy`, `cutover`, `publish`) |
|---|---|---|---|
| **Stage 1: Exact / Registered Alias (`fab`, `clavis`)** | Execute immediately | Execute immediately | Execute immediately |
| **Stage 1b: Unambiguous Prefix (`vox sec list`)** | Execute + dim `stderr` canonical echo | Execute + dim `stderr` canonical echo | Prompt `[Y/n]` in TTY (unless opted in via `cli.prefix_auto_exec = true`); fail with hint in non-TTY |
| **Stage 2: Exact Token Permutation (`vox list secrets`)** | Execute + `stderr` canonical echo (`→ vox secrets list`) | Prompt `[Y/n]` in TTY; fail with exact rewrite in non-TTY | Never auto-execute; prompt `[Y/n]` in TTY or fail with exact rewrite |
| **Stage 3: Precomputed Synonym (`vox verify codex`)** | Execute + `stderr` canonical echo (`→ vox codex verify`) | Prompt `[Y/n]` in TTY; fail with exact rewrite in non-TTY | Never auto-execute; show canonical command |
| **Stage 4: Fuzzy / Edit-Distance / Vector Shortlist** | Show ranked shortlist (or execute top hit only if `vox explain` is the target) | Show ranked shortlist + `[1..N / n]` interactive picker in TTY | Show ranked shortlist; require explicit re-invocation |

---

## 7. Concrete Implementation Design for the Vox Workspace

### 7.1 Schema Extension in `contracts/operations/catalog.v1.yaml`

Extend `OperationRow` (or `CliProjection`) in [`crates/vox-cli/src/commands/ci/operations_catalog.rs`](../../../crates/vox-cli/src/commands/ci/operations_catalog.rs) with optional, generator-assisted synonym fields:

```yaml
# Example row in contracts/operations/catalog.v1.yaml
- id: secrets.doctor
  title: Secrets Doctor
  description: Audit secret resolution health and workflow profile readiness.
  product_lane: platform
  side_effect_class: read_only
  reversible: true
  intent_tags: [clavis, credentials, diagnose, healthcheck, env, keys]
  cli:
    path: [secrets, doctor]
    status: active
    verb_aliases: [check, diagnose, audit, verify, health]
    noun_aliases: [secret, clavis, creds, keys]
    foreign_idioms: ["vault status", "auth status"]
```

### 7.2 Pre-Parse Argv Normalizer in `vox-cli` (`crates/vox-cli/src/argv_normalize.rs`)

Instead offorking `clap`, intercept errors from `VoxCliRoot::try_parse_from(args)` in [`crates/vox-cli/src/main.rs`](../../../crates/vox-cli/src/main.rs):

```text
// architectural pseudocode illustrating the pre-parse normalizer flow
fn parse_with_tolerance(raw_args: list[str]) -> Result[VoxCliRoot, CliDiagnostic] {
    // Fast path: 99% of valid LLM/script calls and exact human calls pay 0 overhead.
    match VoxCliRoot.try_parse_from(raw_args) {
        Ok(root) => return Ok(root),
        Err(clap_err) => {
            // Stage 2: Try order-agnostic token permutation + global flag hoisting
            if let Some(rewritten) = normalize_token_order_and_flags(raw_args) {
                if should_auto_execute(rewritten.capability_id, rewritten.confidence) {
                    eprintln("→ running canonical: " + rewritten.display_cmd)
                    return VoxCliRoot.try_parse_from(rewritten.args)
                }
                return Err(CliDiagnostic.suggest_rewrite(rewritten))
            }
            // Stage 3 & 4: Consult precomputed synonym table + CapabilityIndex::resolve
            return Err(enrich_clap_error_with_capability_index(clap_err, raw_args))
        }
    }
}
```

Key properties of this design:
- **Zero happy-path overhead:** Valid commands execute `VoxCliRoot::try_parse_from` once and never allocate the catalog or synonym tables.
- **Cold-path budget `< 2 ms`:** Only when `try_parse_from` fails does `vox` consult the embedded `CommandCatalog` / `CapabilityIndex` (`bundled_index()`) and the precomputed alias map.
- **Single SSOT:** Both `vox explain <query>` and CLI parse recovery call the exact same `vox_capability_registry::resolve` and `CommandCatalog` structures.

### 7.3 Automated Precomputation & Drift Gate (`vox ci cli-synonyms`)

To keep synonym precomputation automated while preventing drift:

1. **Generator (`vox ci cli-synonyms --write`):**
   - Walks `contracts/operations/catalog.v1.yaml` and `command_catalog::build_catalog()`.
   - Uses `vox_actor_runtime::llm::llm_chat` (routed to `gemini-3.5-flash-lite` with structured JSON schema output) **only for operations whose content hash `(id, title, description, cli.path, arguments)` has changed** since the last lockfile generation.
   - Computes pairwise similarity and antonym exclusions using `vox_search::embeddings` (`gemini-embedding-2` or deterministic `local_embed_text`).
   - Writes `contracts/cli/semantic-aliases.v1.json` (sorted keys, deterministic formatting).
2. **CI Drift & Collision Gate (`vox ci command-compliance`):**
   - Runs purely offline in milliseconds during `vox ci ssot-drift` / `vox ci command-compliance`:
     - Verifies that every alias in `semantic-aliases.v1.json` points to an active `cli.path` in `command-registry.yaml`.
     - Verifies that no alias collides with any canonical command name, `latin_ns`, or `visible_alias`.
     - Verifies that no two operations claim the same `(noun_alias, verb_alias)` tuple without an explicit disambiguation rule.
     - Checks the **Antonym Banlist** (`add` $\not\leftrightarrow$ `remove`, `start` $\not\leftrightarrow$ `stop`, `lock` $\not\leftrightarrow$ `unlock`, `set` $\not\leftrightarrow$ `get`, `import` $\not\leftrightarrow$ `export`, `enable` $\not\leftrightarrow$ `disable`).
3. **Telemetry-Driven Alias Mining (Optional Closed Loop):**
   - When a user triggers a Stage 2/3/4 recovery and confirms the suggested command `[Y/n]`, local telemetry (`vox-telemetry`) can record an anonymized `(attempted_tokens_hash, accepted_capability_id)` counter in `VoxDb` (`research_metrics`), surfacing real-world developer typos and mental-model mismatches for promotion into `catalog.v1.yaml`.

---

## 8. Summary Comparison of Approaches

| Approach | Runtime Latency | Offline / Heremetic? | Drift Risk | Antonym Safety | LLM Agent Impact | Verdict |
|---|---|---|---|---|---|---|
| **Runtime LLM Call on Every CLI Invocation** | 150–800 ms | No (requires API key / local server) | High (model updates change parse behavior) | Medium | Breaks determinism & latency budgets | **Reject** |
| **Runtime LLM Call Only on Parse Error** | 0 ms happy path; 200–600 ms on error | No | Medium | Medium | Slows agent error-recovery loops | **Opt-in Flag Only (`vox --assist` / `vox ?`)** |
| **Naïve Embedding Nearest-Neighbor Auto-Aliaser** | `< 2 ms` | Yes (if vectors embedded) | **Critical** (Voronoi cell shift + antonym proximity) | **Poor** (`add` $\approx$ `remove`) | Dangerous silent mis-execution | **Reject** |
| **5-Stage Deterministic Normalizer + CI-Precomputed Semantic Lockfile (Proposed)** | **`0 ms` happy path; `< 1.5 ms` error path** | **100% Offline at runtime** | **Zero** (locked in `contracts/cli/` + margin-checked in CI) | **Guaranteed** (slot decomposition + antonym banlist + `side_effect_class` gate) | **Improves LLM self-repair** via structured canonical redirects | **Recommended Architecture** |

---

## 9. GSD Milestone & Phase Mapping (`Phase 331` — `CLI-TOL-01..04`)

Tracked in [`.planning/ROADMAP.md`](../../../.planning/ROADMAP.md) and [`.planning/REQUIREMENTS.md`](../../../.planning/REQUIREMENTS.md) under **Phase 331: Tolerant CLI Ergonomics & Precomputed Semantic Routing (`CLI-TOL-01..04`)** (phase directory: [`.planning/phases/331-tolerant-cli-ergonomics-precomputed-semantic-routing/`](../../../.planning/phases/331-tolerant-cli-ergonomics-precomputed-semantic-routing/)):

- **Plan `331-01` (`CLI-TOL-01`) — Zero-Cost Structural Tolerance (`vox-cli` & `vox-cli-core`):**
  - Enable `#[command(infer_subcommands = true, infer_long_args = true)]` on `VoxCliRoot`.
  - Ensure all `GlobalOpts` flags (`--json`, `--quiet`, `--verbose`, `--color`) carry `global = true`.
  - Switch `VoxCliRoot::parse()` in `crates/vox-cli/src/main.rs` to `VoxCliRoot::try_parse()` and wire fallback diagnostics through `vox_capability_registry::resolve(bundled_index(), ...)`.
- **Plan `331-02` (`CLI-TOL-02`, `CLI-TOL-03`) — Order-Agnostic Token & Flag Normalizer + Dual-Persona Safety Gate (`crates/vox-cli/src/argv_normalize.rs`):**
  - Implement permutation matching (`vox <verb> <noun>` $\rightarrow$ `vox <noun> <verb>`), unique-leaf elision, and wrong-subcommand flag detection over `command_catalog::build_catalog()`.
  - Enforce the `side_effect_class` + TTY execution gate (auto-run `read_only` permutations with a `stderr` notice; prompt `[Y/n]` on TTY for mutations; emit structured canonical redirect `aci.error_recovery` on non-TTY/LLM calls).
- **Plan `331-03` (`CLI-TOL-04`) — Precomputed Semantic Synonym Lockfile (`contracts/cli/semantic-aliases.v1.json`):**
  - Add `verb_aliases`, `noun_aliases`, and `foreign_idioms` to `contracts/operations/catalog.v1.yaml`.
  - Build `vox ci cli-synonyms --write` using `vox_actor_runtime::llm` (`gemini-3.5-flash-lite` structured output + `gemini-embedding-2` / `local_embed_text` margin verification) and enforce collision/antonym checks in `vox ci command-compliance`.
