# Contributing to Vox

Welcome. This file is the **short golden path**; deeper policy lives in [`docs/agents/governance.md`](docs/agents/governance.md) and [`AGENTS.md`](AGENTS.md) (secrets / Clavis).

## Quick start

1. Install **Rust** (see root `README.md` and [`docs/src/how-to/how-to-cli-ecosystem.md#installation`](docs/src/how-to/how-to-cli-ecosystem.md#installation)).
2. From the repo root:  
   `cargo check --workspace`
3. Before pushing:  
   `cargo run -p vox-cli -- ci line-endings` on your diff (see [runner contract](docs/src/ci/runner-contract.md)).
4. If you touch CLI flags or help text:  
   `cargo run -p vox-cli -- ci command-compliance`

## Where things live

| Area | Entry |
|------|--------|
| Compiler (lex → HIR) | [`docs/src/explanation/expl-architecture.md`](docs/src/explanation/expl-architecture.md) |
| CLI | [`docs/src/reference/cli.md`](docs/src/reference/cli.md) |
| Mens / Populi HTTP | [`docs/src/reference/populi.md`](docs/src/reference/populi.md) |
| Secrets | [`docs/src/reference/clavis-ssot.md`](docs/src/reference/clavis-ssot.md) |

## First PR checklist

- [ ] `cargo fmt`, `cargo clippy` (as appropriate)
- [ ] Targeted `cargo test -p <crate>` for crates you changed
- [ ] `vox ci line-endings` (or CI will flag)
- [ ] Docs SSOT if you changed user-visible behavior (see [`doc-to-code-acceptance-checklist.md`](docs/src/architecture/doc-to-code-acceptance-checklist.md))

## Deep onboarding

- [Contributing — parser & HIR](docs/src/how-to/how-to-contribute-parser-hir.md)
- [Contributing — Populi operators](docs/src/how-to/how-to-contribute-populi.md)
- [Contributing — Mens training](docs/src/how-to/how-to-contribute-mens.md)
- [First `.vox` app (checkpoints)](docs/src/tutorials/tut-first-vox-app-checkpoints.md)
