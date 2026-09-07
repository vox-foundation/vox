<div align="center">
  <img src="docs/src/assets/vox_hero_banner.jpeg" alt="Vox - The human voice acting as the great nerve of intelligence" width="100%" />

  <br><br>

  <p><strong>One <code>.vox</code> file compiles to a database schema, a typed server, and a browser UI</strong> (Rust + TypeScript). Pre-1.0, workspace <strong>0.6.0</strong>. Initiated by Bertrand Reyna-Brainerd.</p>

  <p><a href="https://voxlang.org"><strong>voxlang.org</strong></a></p>
</div>

<p align="center">
  <a href="https://voxlang.org"><img src="https://img.shields.io/badge/docs-voxlang.org-blue?style=flat-square" alt="Documentation"/></a>
  <a href="https://github.com/vox-foundation/vox/commits/main"><img src="https://img.shields.io/github/last-commit/vox-foundation/vox?style=flat-square&label=updated" alt="Last Updated"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green?style=flat-square" alt="License"/></a>
  <a href="https://voxlang.org/feed.xml"><img src="https://img.shields.io/badge/RSS-updates-orange?style=flat-square" alt="RSS Feed"/></a>
</p>

---

<div align="center">
  <blockquote>
    <p><em>"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence!"</em></p>
    <p>— Nathaniel Hawthorne, <em>The House of the Seven Gables</em> (1851)</p>
  </blockquote>
</div>

---

<!-- ANCHOR: why_vox -->
## Why Vox

Mainstream languages predate LLMs by decades. They tolerate implicit state — nulls, exceptions, schemas restated three times across the stack. That's tractable for a person; it's a minefield for a statistical code generator. A million-token context window doesn't help when most of it is integration boilerplate.

<div align="center">
  <img src="docs/src/assets/old_internet_knot_abstract.png" alt="A diagram illustrating the complexity of traditional web development fragmentation." width="80%" />
  <p>
    <strong>Fragmentation in Traditional Web Development</strong><br />
    Traditional development requires restating data models and logic across frontend, API, backend, and database layers. This duplication creates significant maintenance overhead and increases the risk of integration drift.
  </p>
</div>

Vox is what falls out when you design the language *after* the model: collapse the duplications, push errors into the type system, draw the browser/server boundary in one place, and build durability and tool exposure into the grammar instead of layering them on top.
<!-- ANCHOR_END: why_vox -->

## What works today

Surfaces are graded on the [stability matrix](https://voxlang.org/reference/stability/). The default `vox` binary is the CLI (compile, run, bundle, package). Heavier lanes are optional.

- **Compiler & fullstack codegen** (🟣 Mature / 🔵 Stable): one `.vox` file can emit SQL schema, a typed API server, and React/TSX. Deployment is a separate, unevenly mature step — see the [FAQ](docs/src/explanation/faq.md).
- **CLI & DX** (🟣 Mature): `vox check`, `vox build`, `vox run`, `vox doctor`, `vox audit`, `vox ci`.
- **MCP tools** (🔵 Stable): 300+ first-party tools in the registry. Not a production SLA for every tool.
- **Durable workflows** (🔵 interpreter / 🟡 codegen): journal-backed replay on the interpreted path; generated Rust workflows are not full durable state machines.
- **Local inference & QLoRA** (🟡 Preview / 🟠 Emergent): Rust-native Candle/Burn path — no Python glue on that path. CUDA needs `cargo vox-cuda-release`; not in the default binary. Loss-parity for training is still in progress.
- **Operator GUI** (🟡 Preview): `vox-gui` is a Tauri **operator console** (dashboard, agent flow). It is **not** a compiler that turns your `.vox` app into a native desktop binary. Needs the GUI sidecar build.
- **Mesh** (🟠 Emergent, opt-in): node discovery exists. Automatic off-process routing is experimental and default-off.
- **Also in the repo** (uneven maturity): `vox graph`, SCIENTIA, `vox-search`, `vox-term`, plugin ABI, `vox audit`, skills (agentskills.io), WASM isolation. See [where things live](docs/src/architecture/where-things-live.md).

## Not yet

- Published installers (`voxup`, Homebrew, `.msi`, `.deb`) — no published GitHub Release assets
- Production mesh / automatic hardware routing
- Generated durable-workflow codegen at interpreter parity
- A `.vox` → Tauri app compiler
- Public benchmark leaderboard

## Install

Vox is pre-1.0. **Build from source.** There are no published GitHub Releases today; the `voxup` one-liner is not an end-to-end path (the live script URL has 404'd, and `voxup` ignores draft CI releases).

```bash
git clone https://github.com/vox-foundation/vox.git
cd vox
cargo install --locked --path crates/vox-cli
vox doctor
```

Requires Rust **1.98.1** (`rust-toolchain.toml`). Windows, optional subsystems, Docker, and packaging status: **[Installing Vox](docs/src/reference/installation.md)** — the canonical page.

### Quick Start
```bash
vox init my-app
cd my-app
vox run src/main.vox
```

## The CLI

The full CLI surface, including every `vox ci`, `vox populi`, and `vox mens` subcommand, lives at [`docs/src/reference/cli.md`](docs/src/reference/cli.md). Run `vox commands --recommended` for first-time discovery.

---

### Ecosystem & plugins

Heavier capabilities — Rust-native ML training/serving, the operator GUI, and bundled agent skills (git, memory, RAG, testing, container/WASM runtimes, and more) — load as optional extensions. `vox` tells you if one is required but missing.

Full extension and skill catalog, kept current automatically: **[Plugin Catalog](docs/src/reference/plugin-catalog.generated.md)**.

Project automation itself is `.vox`, not `.ps1`/`.sh`/`.py` — scripts are type-checked and cross-platform (`vox run scripts/clean-build-artifacts.vox`).

Cross-machine orchestration (mesh) is **opt-in** and Emergent. See the [model routing how-to](docs/src/how-to/how-to-model-routing.md).

---

## Stability & Path to 1.0

Vox is marching toward a production-hardened v1.0 release. Surfaces are graded by architectural stability — a representative slice:

| Feature Area | Status |
|:---|:---|
| Compiler Core | 🟣 Mature |
| Database Engine | 🔵 Stable |
| Durable Runtime | 🔵 Stable (interpreter) / 🟡 Preview (codegen) |
| Native GUI (operator console) | 🟡 Preview |
| Distributed Mesh | 🟠 Emergent |

Full per-surface matrix, all tiers explained, and v1.0 release criteria: **[voxlang.org/reference/stability](https://voxlang.org/reference/stability/)**.

Roadmap execution minimizes syntactic redundancy to stabilize the compiler primitives prior to v1.0. Retired symbols: [`AGENTS.md` retired-surfaces table](AGENTS.md).

---

## Documentation

Full docs, organized by intent (tutorials, how-to guides, reference, architecture): **https://voxlang.org**

---

## Contributing

Start at the [Contributor Hub](docs/src/contributors/contributor-hub.md). The [Contribution Loop](docs/src/contributors/contribution-loop.md) explains the write → verify → train cycle. If CI flags a gate failure, the [TOESTUB Guide](docs/src/contributors/toestub-contributor-guide.md) covers the common causes. Undocumented surfaces are tracked in [`DOC_GAPS.md`](docs/src/api/DOC_GAPS.md).

---

Beyond the rule pack, CI enforces repo-wide invariants — layer boundaries (`vox audit arch`), secret hygiene, generated-file drift, and more. Full detector inventory and rationale: [`AGENTS.md`](AGENTS.md).

---

## Backing, license, contact

Community-backed via [Open Collective](https://opencollective.com/vox-foundation) — the ledger is public. Early-stage: sponsorships, when they exist, fund developer grants, MENS training hardware, and academic bounties.

[Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0): commercial use, patent grant, modification with attribution. [`LICENSE`](https://github.com/vox-foundation/vox/blob/main/LICENSE).

Discussion: [GitHub Issues](https://github.com/vox-foundation/vox/issues) · [Contributor Hub](docs/src/contributors/contributor-hub.md). Changelogs and ADRs: [RSS](https://voxlang.org/feed.xml).
