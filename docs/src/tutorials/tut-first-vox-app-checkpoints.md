---
title: "Tutorial: first .vox app (checkpoints)"
description: "Checkpoints for a minimal compile/run path"
category: "Tutorials"
status: "current"
sort_order: 3

schema_type: "HowTo"
---

# First `.vox` app — checkpoints

Use this alongside [First full-stack app](../how-to/first-full-stack-app.md) and the [golden examples](../examples/golden.md).

## Checkpoint A — parse

- [ ] Create `app.vox` with a top-level `fn` or copy `examples/golden/hello.vox` from the repository.
- [ ] `vox check app.vox` exits **0** (or fix parse diagnostics).

## Checkpoint B — typecheck + HIR

- [ ] `vox check app.vox` shows no type errors.
- [ ] Optional JSON: `vox check app.vox --json` prints an array of diagnostics; each carries `error_code` (for example `vox/types/type-mismatch`), `severity`, `message`, and a `span`.

## Checkpoint C — build / run (when applicable)

- [ ] `vox build app.vox` or your project’s documented build entry.
- [ ] `vox run …` runs the file (script execution is built into the default `vox` CLI; see the [CLI reference](../reference/cli.md)).

## Checkpoint D — mens (optional)

- [ ] `vox populi serve` local smoke. This needs the separately installed `vox-ml-cli` built with the non-default `populi` feature — see [Installing Vox → Beyond the CLI](../reference/installation.md#beyond-the-cli) — and the [Populi SSOT](../reference/populi.md).

When stuck, capture **full** diagnostic output and cross-check [parser inventory](../reference/parser-ambiguity-inventory.md) and the [CLI reference](../reference/cli.md).
