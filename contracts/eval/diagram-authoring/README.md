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
| `manifest.v1.yaml` | Counts, BLAKE3 hashes of the four frozen inputs, oracles, MENS status. | hand-maintained, gated |
| `runs/<name>/` | Recordings of live runs and their derived oracle files and tallies (from 91-04). | live run, then Rust/Node |

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

About a third of the graph references are adapted from the Phase 84 docs-corpus fixtures
(`crates/vox-diagram/tests/fixtures/corpus-roundtrip/`) and the export-hostile fixtures. That corpus
was public Mermaid until Phase 90, so memorisation can only favour the Mermaid arm: the findings
report lists this among its limitations.

## Re-score chain and its CI homes

```text
tasks -> reference-inputs (Rust) -> reference-outputs (Node)
attempts + reference-outputs -> oracle-inputs (Rust) -> oracle-outputs (Node)
attempts + oracle-outputs -> tallies.json + tables.md (Rust)
```

- Rust half: `crates/vox-cli/tests/it/eval_diagram_rescore.rs` and `eval_diagram_tasks.rs`, run by
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

## MENS isolation

The dataset sits where MENS does not read: `contracts/eval/**` is listed as deliberately not a root
in `mens/config/vox-source-pool.yaml`, whose pool reads `.vox` files only. The docs extractor
(`vox-corpus` `extract_docs`) walks `docs/src` only. So no `held_out_for_training` list is needed
(`manifest.v1.yaml` `mens.ingested: false`).

## Oracle versions

A recorded run re-scores only against the oracle versions it was recorded with. A change to
vox-diagram diagnostics or a mermaid bump makes the re-score fail on purpose: re-bless the derived
files deliberately and say so in the report.
