# `contracts/eval/diagram-authoring/` — VoxDiagram vs Mermaid authoring eval (EVAL-01)

Do agents author VoxDiagram at least as reliably as Mermaid? Every task here goes to both arms
with the same model, the same template and grammar cards of comparable size. Each arm's output
is judged by a deterministic parser, never an LLM: `vox-diagram` check for VoxDiagram, the pinned
real Mermaid parser for Mermaid.

## Files

| File | Role | Derived by |
|---|---|---|
| `tasks.v1.jsonl` | One task per line, `{id, kind, prompt, reference}`. The prompt is plain prose, identical for both arms. The reference is a VoxDiagram source that draws what the prompt says. | hand-authored |
| `cards/voxdiagram.txt` | Byte copy of `crates/vox-diagram/grammar-card.txt`, frozen so later card edits do not change what was measured. | copied |
| `cards/mermaid.txt` | Mermaid card mirroring the VoxDiagram card section for section (bytes and words within 10 percent). | hand-authored |
| `prompt-template.txt` | The one template both arms share. `[user]` is the first message, `[repair]` a repair turn. The system message is the arm's card. Only `{LANG}`, `{FENCE}` and `{VALIDATOR}` differ between arms. | hand-authored |
| `reference-inputs.jsonl` | Mermaid export of every reference. | Rust |
| `reference-outputs.jsonl` | The real Mermaid parser's canonical sets of each export: the task's expected node/edge (actor/message) sets. Never hand-written. | Node |
| `manifest.v1.yaml` | Counts, BLAKE3 hashes of the four frozen inputs, the pre-registered `decision_rule`, oracles, MENS status. | hand-maintained, gated |
| `runs/<name>/` | Recordings of live runs and their derived oracle files and tallies. | live run, then Rust/Node |
| `probe-spend.jsonl` | The probe ledger: one line per model probe, answered or not, with its cost. | live run |

## Authoring rules (gated by `crates/vox-cli/tests/it/eval_diagram_tasks.rs`)

- Ids `g001`.. (graph) then `s001`.. (sequence), sorted, one compact JSON object per line.
- Every label, edge label, group title, party name, message and note text of the expected sets
  appears double-quoted verbatim in the prompt, and every quoted span is one of them.
- A graph prompt states its direction as exactly one of `top to bottom`, `left to right`,
  `bottom to top`, `right to left`.
- Outside quoted labels a prompt never names syntax: no arrow or bracket tokens and none of the
  words subgraph, container, flowchart, mermaid, vox, node, actor, participant.
- Labels have no line breaks; references check clean; prompts are unique.
- Tasks are hand-authored data. Never derive a prompt or a reference from a model's output.
- The set holds 200 tasks (184 graph, 16 sequence), equal to the manifest's counts, and meets the
  hard-case quotas. `quotas` measures them from `reference-outputs.jsonl` (the oracle's canonical
  sets), never from tags, and prints every count.

50 of the 184 graph references (g011-g060) are adapted from the Phase 84 docs-corpus fixtures
(`crates/vox-diagram/tests/fixtures/corpus-roundtrip/`) and the export-hostile fixtures. That corpus
was public Mermaid until Phase 90, so memorisation can only favour the Mermaid arm: the findings
report lists this among its limitations.

## Re-score chain and its CI homes

```text
tasks -> reference-inputs (Rust) -> reference-outputs (Node)
attempts + reference-outputs -> oracle-inputs (Rust) -> oracle-outputs (Node)
attempts + oracle-outputs -> tallies.json + tables.md (Rust)
```

- Rust half: `crates/vox-cli/tests/it/eval_diagram_rescore.rs`, `eval_diagram_tasks.rs` and
  `eval_diagram_frozen.rs` (cards, template, manifest hashes), run by
  `ci.yml` job `tests` (nextest on affected crates; a `contracts/` change runs the full suite).
- Node half: `docs-astro/tests/unit/diagram-authoring-rescore.test.mjs`, run by
  `docs-quality.yml` job `docs-quality`, step "Docs unit tests".

Neither half runs the other runtime. Each re-derives the files it owns and compares them
byte-for-byte.

## Bless order

After editing tasks, regenerate the derived files on their owning side, then read the diffs:

```text
EVAL_DIAGRAM_BLESS=1 cargo nextest run -p vox-cli --test it -E 'test(/eval_diagram_rescore/)'
EVAL_DIAGRAM_BLESS=1 pnpm --dir docs-astro exec node --test tests/unit/diagram-authoring-rescore.test.mjs
EVAL_DIAGRAM_BLESS=1 cargo nextest run -p vox-cli --test it -E 'test(/eval_diagram_rescore/)'
```

Then update `files` in `manifest.v1.yaml` with the hashes the manifest gate prints. Never edit a
derived file by hand, and never regenerate a recording (`runs/*/attempts.jsonl`).

## Live run

The live run is the ignored test `eval_diagram_live::live_run` (code:
`crates/vox-cli/src/commands/model/eval_diagram/{run,live,prereg}.rs`). It never runs in CI. Run it
once, as plan 91-05 describes, with the freeze commit set to the commit whose subject starts with
`docs(91-04): pre-register`:

```text
EVAL_DIAGRAM_FREEZE_COMMIT=<freeze commit> \
  cargo test -p vox-cli --test it -- --ignored --exact eval_diagram_live::live_run --nocapture
```

Optional: `EVAL_DIAGRAM_MAX_SPEND_USD` (default and maximum 25) and `EVAL_DIAGRAM_RUN_NAME` (default
the UTC date). Before any model call it refuses, with a HUMAN ACTION REQUIRED message, unless the
freeze commit is that pre-registration commit, an ancestor of HEAD, and nothing in this directory
(except `runs/` and `probe-spend.jsonl`) or in the findings report changed since it; it also checks
the manifest rule and hashes, the report's prereg block, mermaid 11.17.2, a fresh run directory and
the remaining budget. Models come from the registry's CodeGen selection restricted to cloud
providers; nothing is pinned.

The $25 cap covers every call, probes included, across all runs: spend is the sum of the attempt
costs under `runs/` and the costs in `probe-spend.jsonl`. Before each call the run reserves the
largest per-call cost it has seen. A call that fails after it may have been billed is charged that
amount, and the run stops instead if nothing has been priced yet. A probe that may have been billed
without a usable cost stops probing. The run `2026-10-27` predates these rules and charged its 9
failed calls $0 (see the findings report). A run name must be one directory name of letters, digits,
`.`, `_` and `-`. **Never delete a run directory or
`probe-spend.jsonl`**: their spend counts against the cap, and deleting them would let a later run
spend it again.

## MENS isolation

The dataset sits where MENS does not read: `contracts/eval/**` is listed as deliberately not a root
in `mens/config/vox-source-pool.yaml`, whose pool reads `.vox` files only. The docs extractor
(`vox-corpus` `extract_docs`) walks `docs/src` only. So no `held_out_for_training` list is needed
(`manifest.v1.yaml` `mens.ingested: false`).

## Oracle versions

A recorded run re-scores only against the oracle versions it was recorded with. A change to
vox-diagram diagnostics or a mermaid bump makes the re-score fail on purpose: re-bless the derived
files deliberately and say so in the report.
