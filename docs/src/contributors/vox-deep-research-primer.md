---
title: "Vox Deep-Research Primer (Claude & Gemini)"
description: "Paste-ready context block and prompt templates that brief Claude Research and Gemini Deep Research on what Vox is, so external research tasks stay on-topic and grounded."
category: "Contributors"
status: "current"
---

# Vox Deep-Research Primer

Use this page when you run deep research **outside** the repo (claude.ai Research,
Gemini Deep Research). Those agents have never seen Vox, and "vox" is a heavily
overloaded word online. Paste **§1 Context block** first, then one template
from **§2**.

Facts below are accurate as of 2026-09-21 (workspace `0.6.0`). Re-check the
README and the [stability matrix](https://voxlang.org/reference/stability/)
before reusing this after a version bump.

---

## 1. Context block (paste verbatim)

```text
<vox_context>
WHAT VOX IS
Vox is a pre-1.0, AI-first, full-stack programming language and toolchain.
One `.vox` source file compiles to a database schema (SQL), a typed API server
(Rust), and a browser UI (React/TSX). The thesis: mainstream languages predate
LLMs and restate the same data model across DB, API, and frontend; Vox is
designed *after* the model — it collapses those duplications, pushes errors
into the type system, draws the browser/server boundary in one place, and puts
durability and tool exposure (MCP) into the grammar.

- Repo:        https://github.com/vox-foundation/vox  (Apache-2.0)
- Docs/site:   https://voxlang.org   (RSS: https://voxlang.org/feed.xml)
- Initiated by Bertrand Reyna-Brainerd.
- Implementation: Rust 2024 edition (toolchain 1.98.1), ~140-crate Cargo
  workspace; compiler is a monolith (`vox-compiler`: lexer, parser, HIR,
  typechecker, codegen). Frontend output is React/TSX; desktop operator
  console is Tauri 2.
- Status: build from source only; no published releases/installers yet.

LANGUAGE SHAPE (current syntax)
Rule: bare-keyword blocks declare scope; decorators modify declarations.
Bare keywords: type, fn, component, state_machine, routes, module, actor,
workflow, activity, and data-layer keywords table, query, mutation, server,
tool, resource, form, index. Decorators: @pure, @uses(net), @auth(...),
@require, @test, @durable, @scheduled, @deprecated.
Types: Result[T], Option, Id[T], ADTs, `?` error propagation, `match`.

  table Note { title: str  content: str }
  query get_notes() to int { return len(db.Note.all()) }
  server health() to Result[str] { return Ok("ok") }
  component App() { view: text() { "Hello Vox" } }

  activity charge_card(amount: int) to Result[str] { ... }
  workflow checkout(amount: int) to Result[str] {
      let tx = charge_card(amount)?
      return Ok(tx)
  }

MATURITY (do not overstate)
- Mature/Stable: compiler + full-stack codegen, CLI (vox check/build/run/
  doctor/audit/ci), database layer, 300+ first-party MCP tools, durable
  workflows on the interpreter path (journal-backed replay).
- Preview: durable workflows via Rust codegen, local inference/QLoRA
  (Rust-native Candle/Burn, no Python), Tauri operator GUI.
- Emergent/not yet: P2P mesh routing, published installers, public
  benchmark leaderboard, .vox -> native desktop app compiler.

INTERNAL SUBSYSTEM NAMES (Latin-flavored)
- MENS: fine-tuning/training lane; trains a Vox-specialized code model
  (active base: Qwen3-8B, QLoRA). Tests in the repo double as training data
  (test pass rate is a planned GRPO reward term).
- Populi: control plane + peer-to-peer mesh for distributed inference/GPU work.
- SCIENTIA: research/evidence framework (includes a deep-research pipeline).
- Codex / Arca: data store facade / low-level SQL (SQLite/Turso/libSQL) layer.
- Clavis (vox-secrets): secret management. TOESTUB: anti-stub quality gate.
- Orchestrator (vox-orchestrator): agent task dispatch; all LLM calls go
  through a model-agnostic facade with a scored model registry.

DISAMBIGUATION — these are NOT Vox-the-language; discard results about them:
Vox Media / vox.com; VOX (Spanish political party); Vox guitar amps; MagicaVoxel
`.vox` voxel files; Vox ML / audio "vox" datasets (VoxCeleb, VoxPopuli —
note the name clash with Populi); "vox" voice-activated / speech products;
older unrelated projects named "Vox" on GitHub. When searching, anchor on
"voxlang", "vox-foundation", or "Vox programming language".

RETIRED — do not recommend or treat as current:
`@component fn`, `@endpoint`, `@table`/`@query`/`@mutation`-style decorators
(now bare keywords), `@py.import` / Python glue, `@native`, Capacitor (now
Tauri 2), crates vox-dei, vox-ars, vox-ludus, vox-dashboard, vox-oratio,
Qwen 2.5 models, `--isolation wasm|container|microvm`.
</vox_context>
```

---

## 2. Prompt templates

Both engines do better with: an explicit role, one research question, scope
limits, the *decision* the research feeds, and a fixed output shape. Fill the
`{{…}}` slots.

### 2a. Claude (claude.ai → Research on)

Claude follows XML-tagged structure well and will ask clarifying questions if
the goal is vague — so front-load the goal.

```text
{{paste §1 vox_context}}

<role>You are a senior programming-language and AI-systems researcher advising
the Vox maintainers.</role>

<question>{{one precise question, e.g. "What durable-execution designs
(Temporal, Restate, DBOS, Inngest, Azure Durable Functions) handle workflow
versioning without replay breakage, and which fits a language where workflow
determinism is enforced by a compiler lint?"}}</question>

<decision>This research feeds: {{the concrete Vox decision, ADR, or plan}}.
Optimize for what changes that decision, not for completeness.</decision>

<scope>
- Time window: prefer sources from {{2024-2026}}; flag anything older.
- Prefer primary sources: papers (arXiv/ACM/USENIX), official docs, source
  code, maintainer posts. Treat vendor marketing and SEO blogs as low-trust.
- Out of scope: {{…}}.
</scope>

<method>
1. Map the landscape first (3-8 candidate approaches), then go deep on the
   2-3 most relevant to Vox's constraints in <vox_context>.
2. For every non-trivial claim, cite the source inline. If a claim is your
   inference, label it "Inference:".
3. Actively look for counter-evidence and failure reports, not just docs.
4. Say "unknown" rather than guess. Do not invent Vox features — if something
   about Vox matters and isn't in <vox_context>, list it under Open Questions.
</method>

<output>
1. TL;DR (<=5 bullets, each ending with a recommendation for Vox).
2. Comparison table: approach | how it works | evidence quality | fit for Vox.
3. Detailed findings with citations.
4. Recommendation + the single biggest risk.
5. Open questions to verify inside the Vox repo.
6. Bibliography (title, author/org, date, URL).
Markdown only, no preamble.
</output>
```

### 2b. Gemini Deep Research

Gemini drafts a **research plan** before running. Always open and edit that
plan — delete off-topic branches (especially any "Vox Media" / voxel / VoxPopuli
steps) before clicking *Start research*. Plain headed sections work better
than XML here.

```text
CONTEXT
{{paste §1 vox_context}}

GOAL
Produce a decision-grade research report answering: {{question}}.
It will be used to decide: {{decision}}.

RESEARCH PLAN REQUIREMENTS
- Your plan must search only for the programming-language/AI-tooling sense of
  the topic. Exclude Vox Media, voxel .vox files, VoxCeleb/VoxPopuli datasets,
  and political results.
- Cover: {{sub-question 1}}; {{sub-question 2}}; {{sub-question 3}}.
- Source priority: peer-reviewed papers > official docs/specs > source repos
  and changelogs > engineering blogs > forums. Note publication dates.
- Include at least one section on failure modes, criticisms, or negative
  results.

CONSTRAINTS
- Do not describe Vox beyond the CONTEXT above; list unknowns instead.
- Distinguish shipped/production evidence from research prototypes and
  announcements.
- Recency: {{2024-2026}} unless a foundational older source is essential.

OUTPUT FORMAT
Executive summary (<=200 words) -> comparison table -> findings by
sub-question -> recommendations specific to Vox (ranked, each with
effort/risk) -> open questions -> cited sources list.
```

### 2c. Follow-up turns (both engines)

- *Verification pass:* "List the 5 claims in your report that most affect the
  recommendation. For each, quote the supporting source passage and rate
  confidence high/medium/low."
- *Gap pass:* "What would a skeptical Rust/PL maintainer say is missing or
  wrong? Research those gaps."
- *Handoff:* "Rewrite the recommendations as a checklist a coding agent in the
  Vox repo could act on, naming what to verify in the codebase first."

---

## 3. Bringing results back

External reports are untrusted input. Before acting on one, verify any claim
about Vox itself against the repo, then land the durable findings as a doc
under `docs/src/architecture/` (`*-research-2026.md`, with frontmatter) per
`AGENTS.md` §Research and Documentation Storage. Existing in-repo research on
the deep-research product space:
[deep-research-competitive-landscape-2026-08-01.md](../architecture/deep-research-competitive-landscape-2026-08-01.md).
