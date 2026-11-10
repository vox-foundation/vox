<!--
  README-SYNC-MAP (Single Source of Ownership — do not duplicate across repos):
  - Language, compiler, runtime, and suite architecture belong in `vox-foundation/vox/README.md`.
  - GUI workbench & 32 surface inventory belong in `vox-foundation/vox-axis/README.md`.
  - This file (`vox-foundation/voxlang.org/README.md` == `vox/docs-astro/README.md`) owns ONLY the
    documentation build pipeline (`vox-doc-pipeline` + Astro/Starlight), taxonomy facets, and the
    `eval.voxlang.org` browser sandbox.
  - Status states are strictly: Working / Prototype / Research / Not started.
-->
<div align="center">
  <img src="assets/diagrams/vox_hero_banner.webp" alt="voxlang.org — Public Documentation, Compiler-Verified Doctests, and Live Browser Eval Sandbox" width="100%" />

  <p>
    <strong>The public documentation site, curated knowledge-base surface, and live browser playground for Vox</strong><br />
    <em>Astro 7.4 + Starlight 0.42 · Compiler-verified <code>.vox</code> doctests · Diátaxis + W3C SKOS taxonomy · Coolify <code>eval.voxlang.org</code> sandbox</em>
  </p>

  <p>
    <sub>Led by <strong>Bertrand Reyna-Brainerd</strong> · Stewarded by the <strong>Vox Foundation</strong> (in formation) · Apache-2.0 · <strong>Pre-1.0 Research Prototype</strong></sub>
  </p>

  <p>
    <a href="https://voxlang.org"><strong>voxlang.org</strong></a> &nbsp;·&nbsp;
    <a href="https://eval.voxlang.org/health"><strong>eval.voxlang.org</strong></a> &nbsp;·&nbsp;
    <a href="https://github.com/vox-foundation/vox"><strong>vox-foundation/vox</strong></a> &nbsp;·&nbsp;
    <a href="https://github.com/vox-foundation/vox-axis"><strong>vox-foundation/vox-axis</strong></a>
  </p>

  <p>
    <img src="https://img.shields.io/badge/status-research%20prototype-red?style=flat-square" alt="Status: research prototype" />&nbsp;
    <a href="https://github.com/vox-foundation/voxlang.org/commits/main"><img src="https://img.shields.io/github/last-commit/vox-foundation/voxlang.org?style=flat-square&label=updated" alt="Last Updated" /></a>&nbsp;
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green?style=flat-square" alt="License" /></a>
  </p>
</div>

> [!WARNING]
> **Last reviewed: 2026-11-10 (`v6.8`).** `voxlang.org` is the public documentation and interactive playground surface for the [Vox programming language](https://github.com/vox-foundation/vox) (`0.6.0` workspace). All 900+ pages are authored under [`docs/src/`](https://github.com/vox-foundation/vox/tree/main/docs/src), validated by `vox-doc-pipeline` (frontmatter taxonomy, `kb:summary` blocks, executable `.vox` doctests, and `why_vox` homepage sync), built with Astro 7.4.5 and Starlight 0.42.9, deployed to Cloudflare Pages (`voxlang.org`), and backed by the Coolify deterministic interpreter sandbox (`eval.voxlang.org`).

---

## Animated Visual Tour (`voxlang_site_tour.webp`)

<div align="center">
  <img src="assets/tours/voxlang_site_tour.webp" alt="Animated visual tour of voxlang.org architecture, single-file full-stack model, and suite pillars" width="94%" />
  <p>
    <strong>voxlang.org Visual Tour (Animated WebP)</strong><br />
    <em>Cycling through the Vox hero identity, single-file 3-layer projection model, 6 suite pillars, collaborative task example, and compiler-verified documentation pipeline.</em>
  </p>
</div>

---

## Architecture & Deployment Pipeline

<div align="center">
  <img src="assets/diagrams/voxlang_pipeline_diagram.webp" alt="voxlang.org Pipeline: vox-doc-pipeline to Astro Starlight to Cloudflare Pages and Coolify eval.voxlang.org" width="92%" />
  <p>
    <strong>Compiler-Verified Documentation &amp; Live Eval Pipeline (Imperial Roman Carrara White &amp; Gold Schematic)</strong><br />
    <em>Every markdown page in <code>docs/src/</code> passes <code>vox-doc-pipeline</code> (W3C SKOS taxonomy facets[^2], Nanopublication provenance[^3], <code>.vox</code> doctest compilation, VoxDiagram rendering) before Astro 7.4 + Starlight 0.42 builds static assets for Cloudflare Pages (<code>voxlang.org</code>) and connects interactive playground islands to Coolify (<code>eval.voxlang.org</code>)[^4].</em>
  </p>
</div>

---

## What This Repository Coordinates

| Subsystem | Programmer Summary | Key Files / Commands |
|:---|:---|:---|
| **1. Astro 7.4 + Starlight 0.42 Site** | Builds 900+ documentation pages organized by Daniele Procida's **Diátaxis** quadrants[^1] (Tutorials, How-To Guides, Reference, Architecture/Explanation) plus `.well-known/llms.txt`[^5], with frontmatter-driven sidebar generation (`sidebar.mjs`) and fatal internal-link checking (`starlight-links-validator`). | `docs-astro/astro.config.mjs`, `docs-astro/src/utils/sidebar.mjs`, `pnpm --dir docs-astro build` |
| **2. Compiler-Verified Doctests (`vox-doc-pipeline`)** | Every fenced `vox` code block is type-checked by `vox check` unless explicitly marked `// vox:skip <reason>`, preventing doc-rot. Also validates W3C SKOS `topic`/`domain` prefLabels[^2] and Nanopublication-style `authorship` provenance[^3]. | `cargo run -p vox-doc-pipeline -- --lint-only` |
| **3. Live Browser Eval Sandbox (`eval.voxlang.org`)** | Interactive `.vox` code blocks execute against `https://eval.voxlang.org/v1/eval`, running `vox run --interp --caps deterministic` inside a capability-sandboxed Coolify container[^4] with zero ambient filesystem/network/secret access. | `vox-eval.compose.yml`, `.github/workflows/deploy-hetzner.yml`, `vox ci coolify-eval` |
| **4. Installer Bootstrap (`/voxup`, `/voxup.ps1`)** | Serves cross-platform POSIX and PowerShell bootstrap scripts verified on every deploy by Playwright smoke tests. | `docs-astro/public/voxup`, `docs-astro/tests/smoke.spec.ts` |

---

## Tradeoffs & Maintainability

- **Single-source page authoring:** Do not duplicate language tutorials or GUI reference tables into this README. Edit the canonical Markdown under [`docs/src/`](https://github.com/vox-foundation/vox/tree/main/docs/src); `vox-doc-pipeline` and `sidebar.mjs` propagate changes automatically.
- **No hand-edited generated reference pages:** `docs/src/reference/*.generated.md` are emitted from Rust source by `cargo run -p vox-cli -- ci command-sync` and `ci generate-plugin-catalog-docs`.
- **Homepage `why_vox` sync:** The `<!-- ANCHOR: why_vox -->` block in `vox/README.md` and `{/* SYNC-FROM-README: why_vox */}` in `docs/src/index.mdx` are enforced byte-for-byte by `vox-doc-pipeline`.

---

## Status

| Component | State | Checked | Notes |
|:---|:---|:---|:---|
| **Astro 7.4.5 / Starlight 0.42.9 build** | Working | 2026-11-10 | Builds 907+ pages cleanly; GitHub-style alerts render as native asides; broken internal links fail the build via `starlight-links-validator` 0.27.0. |
| **`vox-doc-pipeline` frontmatter & doctest lint** | Working | 2026-11-10 | Enforces `title`, `description`, `category`, `topic`, `domain`, `authorship`, and `<!-- kb:summary -->` against `contracts/documentation/kb-taxonomy.v1.yaml`. |
| **Cloudflare Pages deploy (`voxlang.org`)** | Working | 2026-11-10 | Deployed via `wrangler pages deploy dist --project-name=vox-docs`, followed by live Playwright smoke verification (`tests/smoke.spec.ts`). |
| **Coolify Eval Sandbox (`eval.voxlang.org`)** | Working | 2026-11-10 | Managed via `vox ci coolify-eval sync-compose --compose vox-eval.compose.yml --domains https://eval.voxlang.org`. |
| **Ludus Leaderboard Arena (`VoxLeaderboardArena.astro`)** | Prototype | 2026-11-10 | Verifies Ed25519-signed `VerifiedLeaderboardSnapshotV1` payloads from `ops.voxlang.org`. |

---

## Trying It & Keeping Docs in Sync

```bash
# 1. Lint documentation frontmatter, KB facets, and doctests:
cargo run -p vox-doc-pipeline --manifest-path /Users/brbrainerd/dev/vox/Cargo.toml -- --lint-only

# 2. Build the Starlight site:
./scripts/build.sh

# 3. Verify Coolify eval sandbox discovery & cross-repo parity:
cargo run -p vox-cli --manifest-path /Users/brbrainerd/dev/vox/Cargo.toml -- ci coolify-eval discover
cargo run -p vox-cli --manifest-path /Users/brbrainerd/dev/vox/Cargo.toml -- ci cross-repo-parity
```

---

## Intellectual Lineage, Prior Art & References

[^1]: **Diátaxis Documentation Architecture:** Information architecture separating Tutorials, How-To Guides, Reference, and Explanation follows Daniele Procida, *Diátaxis: A systematic approach to technical documentation authoring* ([diataxis.fr](https://diataxis.fr/)).
[^2]: **SKOS Controlled Vocabulary (`kb-taxonomy.v1.yaml`):** Hierarchical `topic` and `domain` `prefLabel` / `broader` / `related` facets follow W3C *SKOS Simple Knowledge Organization System Reference* (Miles & Bechhofer, W3C Recommendation, 2009).
[^3]: **Nanopublication Provenance (`authorship`, `origin`, `kb:summary`):** Attaching machine-verifiable assertion, provenance, and publication metadata to each knowledge artifact draws from Paul Groth, Andrew Gibson, & Jan Velterop, *"The Anatomy of a Nanopublication"*, *Information Services & Use* 30(1–2), 2010.
[^4]: **Capability-Sandboxed Interpreter (`eval.voxlang.org`):** Executing untrusted browser snippets under `--caps deterministic` with zero ambient authority follows Dennis & Van Horn (1966) object-capability discipline and the [Bytecode Alliance WASI capability model](https://wasi.dev/).
[^5]: **Machine-Readable LLM Discovery (`.well-known/llms.txt`):** Exposing a structured Markdown index of documentation for coding agents follows Jeremy Howard's [`llms.txt` proposal](https://llmstxt.org/) (Answer.AI, 2024).
