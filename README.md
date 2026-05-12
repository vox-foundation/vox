<div align="center">
  <img src="docs/src/assets/vox_hero_banner.jpeg" alt="Vox - The human voice acting as the great nerve of intelligence" width="100%" />

  <br><br>

  <p><strong>One <code>.vox</code> file compiles to a database schema, a typed server, a browser app, and the artifacts to deploy them.</strong> Initiated by Bertrand Reyna-Brainerd.</p>

  <p><a href="https://vox-lang.org"><strong>vox-lang.org</strong></a></p>
</div>

<p align="center">
  <a href="https://vox-lang.org"><img src="https://img.shields.io/badge/docs-vox--lang.org-blue?style=flat-square" alt="Documentation"/></a>
  <a href="https://github.com/vox-foundation/vox/commits/main"><img src="https://img.shields.io/github/last-commit/vox-foundation/vox?style=flat-square&label=updated" alt="Last Updated"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green?style=flat-square" alt="License"/></a>
  <a href="https://vox-lang.org/feed.xml"><img src="https://img.shields.io/badge/RSS-updates-orange?style=flat-square" alt="RSS Feed"/></a>
</p>

---

<!-- Code examples in this file mirror examples/golden/*.vox -->
<!-- Run: vox check examples/golden/*.vox to verify -->

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

Vox is what falls out when you design the language *after* the model: collapse the duplications, push errors into the type system, draw the browser/server boundary in one place, and build durability and tool exposure into the grammar instead of layering them on top.

<div align="center">
  <img src="docs/src/assets/old_internet_knot.png" alt="A symbolic representation of the old internet as a massive, complex, tangled knot of glowing wires and chaotic data streams." width="80%">
  <p><em>Symbolic representation of the old internet: a tangled knot of integrations and boilerplate.</em></p>
</div>
<!-- ANCHOR_END: why_vox -->

## Install

> [!IMPORTANT]
> **Pre-Release Status:** Vox has not yet reached version 1.0. Pre-compiled binaries and installers are not yet available. To use Vox today, you must build it from source.

### Building from Source (Current)

Requires **Rust** and **Cargo**.

```bash
git clone https://github.com/vox-foundation/vox.git
cd vox
cargo install --path crates/vox-cli
```

*To build the native Tauri GUI (`vox-gui`), you will also need Node.js and `pnpm`, as well as Tauri's system dependencies for your platform. Run `cargo build -p vox-gui`.*

### Package Managers (Coming in v1.0)

Once version 1.0 is released, the following installation methods will be supported:

**macOS**

```bash
brew install vox-foundation/vox/vox
```

**Linux (Debian/Ubuntu)**

```bash
curl -fsSLO https://github.com/vox-foundation/vox/releases/latest/download/vox-cli-amd64.deb
sudo dpkg -i vox-cli-amd64.deb
```

**Windows** — download the `.msi` from the Releases page [4].

```bash
vox init my-app
cd my-app
vox run src/main.vox
```

### Ecosystem & Plugins

Vox is highly modular. The core binary covers compile, run, bundle, package. Heavier capabilities are provided through optional **CLI Extensions** and **Runtime Plugins**.

#### CLI Extensions
These ship as separate binaries that `vox` dispatches to from `$PATH`; if missing, `vox` tells you what to install.

| Extension | Adds | Purpose |
|---|---|---|
| `vox-ml-cli` | `vox mens`, `vox oratio`, `vox populi`, `vox speech`, `vox train` | Rust-native ML frameworks (Candle [5], Whisper [6], HF hub [7]) for training and serving without Python. |
| `vox-schola` | `vox schola`, `vox scientia` | Autonomous AI research, fact-checking, and capability-map subsystems. |
| `vox-gui` | `vox gui` | Native Tauri desktop application and visual environment. |

#### Runtime Plugins (Agent Skills)
The Vox AgentOS dynamically loads capabilities through a stable ABI using `vox-plugin-host`. There are currently 27 first-party plugins granting your agents access to the outside world:

- **Machine Learning & Audio**: `mens-candle-cuda` (NVIDIA acceleration), `mens-candle-metal` (Apple Silicon acceleration), `nvml-probe`, `oratio`, `oratio-mic`, `populi-mesh`
- **Execution Sandboxes**: `runtime-container` (Docker), `runtime-wasm`, `script-execution`
- **Agent Skills**: `skill-compiler`, `skill-git`, `skill-memory`, `skill-orchestrator`, `skill-rag`, `skill-testing`, `skill-testing-validate`, `skill-v0`, `browser`, `noop-skill`
- **Core Infrastructure**: `api`, `catalog`, `cloud`, `host`, `types`, `webhook`
- **Publishing**: `publication`, `grammar-export`

*→ See the Plugin Catalog [8] for detailed tool signatures.*

## The CLI

The full CLI surface, including every `vox ci`, `vox populi`, and `vox mens` subcommand, lives at `docs/src/reference/cli.md` [9]. Run `vox commands --recommended` for first-time discovery.

---

<div align="center">
  <img src="docs/src/assets/archive/vox_unification_diagram.png" alt="Vox architecture unification vs. legacy fragmentation">
</div>

<!-- ANCHOR: how_vox -->
## How Vox works

### Pillar 1: One source of truth

```vox
@table type Task {
    title: str
    done:  bool
    owner: str
}
```

The declaration is the schema [10], the wire format [11], and the typed client. `@index Task.by_owner on (owner)` lives next to it. Migrations [10] come from the diff against the previous schema.

→ `@table` reference [12] · migration guide [13]

### Pillar 2: Errors in the type system

```vox
@endpoint(kind: query)
fn recent_tasks() to list[Task] {
    return db.Task.where({ done: false }).limit(10)
}

@endpoint(kind: mutation)
fn add_task(title: str, owner: str) to Result[Id[Task]] {
    if title == "" { return Error("title required") }
    return Ok(db.insert(Task, { title: title, done: false, owner: owner }))
}
```

A `Result[T]` caller must handle both arms — no exceptions, no `null`, no implicit propagation. The compiler refuses to build code that drops `Error`. `vox-lsp` [14] surfaces the same diagnostics live in the editor.

`@endpoint(kind: …)` is the unified form of the older `@query` / `@server` / `@mutation` decorators (April 2026 grammar collapse).

→ decorator reference [12]

### Pillar 3: One file → running deployment

```vox
component TaskPage(tasks: List[Task]) {
    view: column() {
        tasks.map(fn(t) { row() { text() { t.title } } })
    }
}

routes { "/" to TaskPage }
```

`vox build` emits React [15]/TSX [16] components, a generated `vox-client.ts` RPC bridge, and — via `vox-deploy-codegen` [17] — Dockerfile, Compose, Kubernetes, Fly, Coolify, and systemd targets, all derived from the same module graph. External React, TanStack, or mobile apps can import the emitted components or call the endpoints over the bridge.

→ external interop plan [18] · deployment [19]

### Pillar 4: Durability, agents, skills

`@durable` lowers to checkpointed execution under `vox-workflow-runtime` [20] — retried on transient faults, restarted on node death. [1, 2] `@mcp.tool` exposes a function to any Model Context Protocol [21] client. [3]

```vox
@durable
fn charge_card(amount: int) to Result[str] {
    if amount > 1000 { return Error("amount too large") }
    return Ok("tx_123")
}

@mcp.tool "Process a durable checkout"
fn checkout(amount: int) to Result[str] {
    return charge_card(amount)
}
```

<div align="center">
  <img src="docs/src/assets/durable_essentialist_loop.webp" alt="Durable execution loop: commit, execute, recover, complete" width="60%">
</div>

The same primitives drive multi-agent work. `vox-orchestrator` [22] routes tasks to agents by file affinity and ten policy modules (tier cascade, plan-mode trigger, risk matrix, budget gate, circuit breaker, calibration, …). Capabilities are extensible: 27 first-party plugins (compiler, git, memory, RAG, testing, Mens-Candle-CUDA/Metal, WASM and OCI runtimes) load through `vox-plugin-host` [23] behind a stable ABI.

→ orchestration policy research [24] · `vox-skills` [25]

### Pillar 5: Built for LLM authorship

The shape of the four pillars above is downstream of one decision: *design the language after the model*. Three subsystems make that concrete.

- **Grammar-constrained decoding.** `vox-constrained-gen` [26] is an Earley/PDA decoder [27] with a deadlock watchdog. Token-stream constraint, not post-hoc validation — invalid Vox cannot be sampled.
- **Measurable detectors.** Rules live in `rules.v1.yaml` [28] with a JSON Schema and an F1 bench scorer [29] over fixture corpora. Stub, hollow-fn, victory-claim, AI-laziness, secret, magic-value, deprecated-symbol, and effect-system rules are all scored against ground truth, not vibes.
- **Local training.** Vox is new; mainstream languages saturate the public training corpus, Vox doesn't. `vox populi` runs QLoRA [30] fine-tunes and OpenAI-compatible serving on detected CUDA / Metal / WebGPU — Burn [31] + Candle [5], no Python. Requires the `gpu` cargo feature.

→ `examples/golden/` [32] · Rosetta comparison [33] · why Vox for AI [34]

<div align="center">
  <img src="docs/src/assets/vox_new_internet.png" alt="A symbolic representation of a new AI-governed internet." width="80%">
  <p><em>The new Vox internet: sound waves flowing into an AI governance matrix that outputs structured code and interfaces.</em></p>
</div>

---

### Engineering invariants

Properties enforced on the project itself, invisible from the language surface:

- **Layered crate graph.** All 101 workspace crates declare a layer (L0 pure types → L5 surfaces) in `layers.toml` [35]. `vox-arch-check` [36] blocks inversions, fan-in violations, LoC budget overruns, and orphaned modules.
- **Sandboxed execution.** `vox-wasm-engine` [37] (Wasmtime [38]), `vox-container` [39] (OCI [40]), `vox-bounded-fs` [41] (size-capped reads), `vox-exec-grammar` [42] (shell risk classifier). Tiers are selectable on `vox run`.
- **Declared capabilities.** `vox-capability-registry` [43] gates what tools can do; `vox-identity` [44] signs with ed25519 [45] against a trust ledger; `vox-secrets` [46] is the only path to a secret value.
<!-- ANCHOR_END: how_vox -->

---

## Automation: VoxScript-first

Project automation is `.vox`, not `.ps1` / `.sh` / `.py`. The same file runs on Windows, Linux, and macOS; it's type-checked before execution (`vox check scripts/foo.vox`); it emits `vox.script.*` telemetry [47]; and it can run in a WASM sandbox for untrusted input.

```bash
vox run scripts/clean-cache.vox
vox run --isolation wasm scripts/process-untrusted-data.vox
```

Other commands worth knowing:

- `vox share publish file.vox` — short-lived public preview tunnel (Cloudflare, localhost.run, Tailscale).
- `vox audit` — runs the rule pack against your tree.
- `vox telemetry doctor` — diagnoses `VOX_TELEMETRY` and per-sink wiring.

---

## Mesh and provider routing

Cross-machine work is opt-in. Nodes advertise CPU/CUDA/Metal/VRAM on startup and the orchestrator routes training and inference jobs to whichever machines can take them. Agent-to-agent messages are in-process by default; the `populi-transport` feature enables relay. Both ends declare the same Vox type, so wire mismatches fail at compile time.

```bash
VOX_MESH_ENABLED=1 VOX_MESH_NODE_ID=my-node vox populi serve
vox populi status --quotas
```

Local models (Ollama) and the major cloud providers go through one policy layer with per-provider quotas and disclosure rules. See the model routing how-to [48].

---

## Stability

<!-- ANCHOR: tier_table -->
Workspace `0.5.0` — pre-1.0. Surfaces are graded by how reproducibly an LLM can target them: data and tool contracts lock first, rendering surfaces last.

🟢 Stable · 🟡 Preview · 🚧 Experimental

| Surface | Tier | Notes |
|:---|:---|:---|
| Compiler engine | 🟢 | AST [49], HIR [50], type checker [51], LSP [14], codegen [52]. |
| `@table` & data layer | 🟢 | Schema [10], migrations [10], `db.*` query builder, wire types. |
| `@mcp.tool` / `@mcp.resource` | 🟢 | MCP protocol compliance. |
| Surface syntax | 🟡 | Top-level forms (`@endpoint(kind: …)`, `@durable`, bare `workflow`/`activity`/`actor`) defined in `AGENTS.md` [53]. |
| Endpoints | 🟡 | Unified `@endpoint` is recent. |
| Code-audit rule pack | 🟡 | See Pillar 5 [54]. |
| RAG & knowledge curation | 🟡 | `vox scientia` [55], Socrates guards. |
| Durable execution | 🟡 | Grammar locked; `vox-workflow-runtime` [20] behavior maturing. |
| Local training (MENS) | 🟡 | Hardware coverage expanding (`vox-mens` [56]). |
| Web UI & rendering | 🟡 | Vox-native reactivity [57] for greenfield; React [15] TSX + `vox-client.ts` for interop. |
| Distributed node mesh | 🚧 | Cross-machine routing is pre-1.0 design. |

v1.0 criteria: `docs/src/architecture/v1-release-criteria.md` [58]. Roadmap: GUI-native phases [59]. History: `CHANGELOG.md` [60].
<!-- ANCHOR_END: tier_table -->

Phase status: 2–6 done (primitive collapse, grammar unification, compiler/GUI milestones); Phase 7 mostly done (TASK-7.3 bundler swap deferred to Phase 9); Phase 8 corpus migration done, TASK-8.2 awaits an operator MENS run; Phase 9 route-pipeline restoration landed. Retired symbols: `AGENTS.md` retired-surfaces table [53].

---

## Documentation

Docs follow the **Diátaxis** framework.

| Intent | Start here |
|---|---|
| Learning | Getting Started [61] · First full-stack app [62] |
| Task recipes | How-To Guides [63] · AI Agents & MCP [64] |
| Understanding | Why Vox for AI [34] · Compiler architecture [65] |
| Reference | CLI [9] · Decorators [12] |
| Architecture | Master index [66] · Contributor hub [67] |
| Operations | Deployment [19] · CI runner [68] |

---

## Contributing

Start at the Contributor Hub [67]. The Contribution Loop [69] explains the write → verify → train cycle. If CI flags a gate failure, the TOESTUB Guide [70] covers the common causes. Undocumented surfaces are tracked in `DOC_GAPS.md` [71].

---

## CI gates beyond the rule pack

The rule pack (Pillar 5) covers detectors. A handful of CI guards live outside it because they enforce repo invariants, not code patterns:

| Guard | Blocks | Run |
|---|---|---|
| `vox arch-check` | layer inversions, fan-in violations, LoC budgets, orphans | always |
| `vox ci secret-env-guard` | raw `std::env::var` for secrets | always |
| `vox ci sync-ignore-files` | `.voxignore` drift to `.cursorignore` / `.aiignore` / `.aiexclude` | always |
| `vox-drift-check` | multi-language workspace drift | pre-push |

Rationale and the full detector inventory live in `AGENTS.md` [53].

---

<!-- ANCHOR: community_license -->
## Backing, license, contact

Funded via Open Collective [72] — every transaction is public. Sponsorships fund developer grants, MENS training hardware, and academic bounties.

Apache 2.0 [73]: commercial use, patent grant, modification with attribution. `LICENSE` [74].

Discussion: GitHub Discussions [75]. Changelogs and ADRs: RSS [76].
<!-- ANCHOR_END: community_license -->

---

---

## References

**[1]** Fateev, M., & Abbas, S. (2019). *Temporal*. Temporal Technologies. <https://temporal.io>

**[2]** Armstrong, J. (2003). *Making reliable distributed systems in the presence of software errors* [Ph.D. thesis, Royal Institute of Technology, Stockholm]. <https://erlang.org/download/armstrong_thesis_2003.pdf>

**[3]** Anthropic. (2024). *Model Context Protocol*. <https://modelcontextprotocol.io>

**[4]** https://github.com/vox-foundation/vox/releases

**[5]** https://github.com/huggingface/candle

**[6]** https://en.wikipedia.org/wiki/Whisper_(speech_recognition_system)

**[7]** https://huggingface.co/docs/hub/index

**[8]** docs/src/reference/plugin-catalog.generated.md

**[9]** docs/src/reference/cli.md

**[10]** crates/vox-db/

**[11]** crates/vox-types/

**[12]** docs/src/reference/ref-decorators.md

**[13]** docs/src/how-to/how-to-database.md

**[14]** crates/vox-lsp/

**[15]** https://react.dev/

**[16]** https://www.typescriptlang.org/

**[17]** crates/vox-deploy-codegen/

**[18]** docs/src/architecture/external-frontend-interop-plan-2026.md

**[19]** docs/src/reference/deployment-compose.md

**[20]** crates/vox-workflow-runtime/

**[21]** https://modelcontextprotocol.io

**[22]** crates/vox-orchestrator/

**[23]** crates/vox-plugin-host/

**[24]** docs/src/architecture/autonomous-orchestration-policy-research-2026.md

**[25]** crates/vox-skills/

**[26]** crates/vox-constrained-gen/

**[27]** https://en.wikipedia.org/wiki/Earley_parser

**[28]** crates/vox-rule-pack/rules/rules.v1.yaml

**[29]** https://en.wikipedia.org/wiki/F-score

**[30]** https://arxiv.org/abs/2305.14314

**[31]** https://github.com/tracel-ai/burn

**[32]** examples/golden/

**[33]** docs/src/explanation/expl-rosetta-inventory.md

**[34]** docs/src/explanation/why-vox-for-ai.md

**[35]** docs/src/architecture/layers.toml

**[36]** crates/vox-arch-check/

**[37]** crates/vox-wasm-engine/

**[38]** https://wasmtime.dev/

**[39]** crates/vox-container/

**[40]** https://opencontainers.org/

**[41]** crates/vox-bounded-fs/

**[42]** crates/vox-exec-grammar/

**[43]** crates/vox-capability-registry/

**[44]** crates/vox-identity/

**[45]** https://en.wikipedia.org/wiki/EdDSA#Ed25519

**[46]** crates/vox-secrets/

**[47]** crates/vox-telemetry/

**[48]** docs/src/how-to/how-to-model-routing.md

**[49]** crates/vox-compiler/

**[50]** crates/vox-compiler/src/hir/

**[51]** crates/vox-compiler/src/typeck/

**[52]** crates/vox-codegen/

**[53]** AGENTS.md

**[54]** crates/vox-rule-pack/

**[55]** crates/vox-schola/

**[56]** crates/vox-ml-cli/

**[57]** https://en.wikipedia.org/wiki/Reactive_programming

**[58]** docs/src/architecture/v1-release-criteria.md

**[59]** docs/src/architecture/gui-native-roadmap-status-2026.md

**[60]** CHANGELOG.md

**[61]** docs/src/tutorials/tut-getting-started.md

**[62]** docs/src/how-to/first-full-stack-app.md

**[63]** docs/src/how-to/

**[64]** docs/src/how-to/how-to-ai-agents.md

**[65]** docs/src/explanation/expl-architecture.md

**[66]** docs/src/architecture/architecture-index.md

**[67]** docs/src/contributors/contributor-hub.md

**[68]** docs/src/ci/runner-contract.md

**[69]** docs/src/contributors/contribution-loop.md

**[70]** docs/src/contributors/toestub-contributor-guide.md

**[71]** docs/src/api/DOC_GAPS.md

**[72]** https://opencollective.com/vox-foundation

**[73]** https://www.apache.org/licenses/LICENSE-2.0

**[74]** https://github.com/vox-foundation/vox/blob/main/LICENSE

**[75]** https://github.com/vox-foundation/vox/discussions

**[76]** https://vox-lang.org/feed.xml

