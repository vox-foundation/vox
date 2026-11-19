# Requirements: Milestone v8.9 (`adversarial-docs-graphics-crosslink-overhaul-v89`)

| ID | Category | Requirement | Phase | Status |
|---|---|---|---|---|
| `FIX-01` | Engine | Initialize Milestone `v8.9` and update `remarkStripTitleH1` (`SKIP_TYPES` skips `mdxjsEsm` + `mdxFlowExpression`) with unit tests | 526 | `[x]` |
| `FIX-02` | Formatting | Remove raw backtick leaks in JSX string props (`index.mdx`) and raw HTML showcase blocks (`how-to-axis-workbench.md`, `how-to-scientia-publication.md`, `tut-deep-research-and-scientia.mdx`) | 526 | `[x]` |
| `FIX-03` | Verifier | Fix quoted positional arg tokenization (`"` and `'`) in `scripts/docs/tutorial-verify.vox` | 526 | `[x]` |
| `FIX-04` | Engine | Verify `remark-strip-title-h1.test.mjs` passes with `.mdx` AST coverage | 526 | `[x]` |
| `GFX-01` | Infographics | Produce Plate 1 (`federated-kb-openwiki-arch.webp`) via Playwright Chromium + Sharp fallback after `image-generator` `429 RESOURCE_EXHAUSTED` and inspect via `read_file` | 527 | `[x]` |
| `GFX-02` | Infographics | Generate Plate 2 (`scientia-research-arxiv-flywheel.webp`) via `image-generator` and inspect for visual/text fidelity | 527 | `[x]` |
| `GFX-03` | Infographics | Produce Plate 3 (`non-modal-doc-studio-formats.webp`) via Playwright Chromium + Sharp fallback after `image-generator` `429 RESOURCE_EXHAUSTED` and inspect via `read_file` | 527 | `[x]` |
| `GFX-04` | Infographics | Convert and stage all 3 infographics in both `docs/src/assets/axis/` and `docs-astro/public/assets/axis/` | 527 | `[x]` |
| `PED-01` | Tutorials | Deepen `tut-deep-research-and-scientia.mdx` with flywheel infographic, Archetype matrix, realistic JSON outputs, and Troubleshooting section | 528 | `[x]` |
| `PED-02` | Tutorials | Deepen `tut-knowledgebase-and-openwiki.mdx` with architecture + format studio infographics, `.vox/repositories.yaml`, 3-tab Tier-1/Tier-2 diff example, Parser Matrix, and Common Pitfalls | 528 | `[x]` |
| `PED-03` | Tutorials | Fix nested backticks in `tut-knowledgebase-and-openwiki.mdx` (`openwiki` prose & line 167 `vox-diagram`) | 528 | `[x]` |
| `PED-04` | Tutorials | Add SKOS facets, `kb:summary`, and cross-links to `tut-first-app.md`, `tut-ui-integration.md`, and `tut-actor-basics.md` | 528 | `[x]` |
| `GAP-01` | Reference | Rewrite `docs/src/reference/ref-search.md` covering Pagefind, Omnibar (`Cmd/Ctrl+K`), `vox-search` RRF + PPR, CLI/MCP, and `vox-diagram` | 529 | `[x]` |
| `GAP-02` | Reference | Upgrade `docs/src/reference/gui-navigation.md` replacing `../../superpowers/` links and documenting Non-Modal Doc Dock, Concilium/Populi, and Repo Switcher | 529 | `[x]` |
| `GAP-03` | Reference | Upgrade `external-repositories.md`, `scientia-ssot-handbook.md`, and `ref-syntax.md` removing stale `../archive/` links, fixing typos/versions (`0.6.0`), and adding SKOS facets | 529 | `[x]` |
| `GAP-04` | Concepts | Upgrade `explanation/glossary.md` fixing Actor/Workflow definitions + `.vox` snippets and adding 7 `v7.9–v8.7` terms | 529 | `[x]` |
| `GAP-05` | Concepts | Upgrade `explanation/faq.md` & `why-vox-for-ai.md` adding Federated KB/OpenWiki, Scientia, and Doc Studio coverage | 529 | `[x]` |
| `GAP-06` | Concepts | Upgrade `explanation/expl-architecture.md` & `expl-ai-orchestration.md` replacing ASCII art with `vox-diagram` + KB/OpenWiki cross-links | 529 | `[x]` |
| `GAP-07` | Contributors | Refresh `contributors/vox-deep-research-primer.md` to current `2026-11-18` state and link native Research tutorial | 529 | `[x]` |
| `GAP-08` | Hygiene | Replace truncated `description:` strings across 16 reader-facing files | 529 | `[x]` |
| `QA-01` | Regression | Create `docs-astro/tests/build-output/v89-quality.spec.ts` verifying `.mdx` + `tutorials/` + `how-to/` + `explanation/` + `contributors/` + `index.html` routes & `#anchors` | 530 | `[x]` |
| `QA-02` | Regression | Verify every `<img>` in reader-facing built routes resolves to an existing non-empty file in `docs-astro/dist/` | 530 | `[x]` |
| `QA-03` | Regression | Verify zero duplicate `<h1>` tags on `.mdx` tutorial pages | 530 | `[x]` |
| `QA-04` | Regression | Verify zero literal unrendered backticks inside `.vox-showcase-card` or Starlight `.card` blocks | 530 | `[x]` |
| `GATE-01` | Verification | Tighten facet backlog (`--tighten-facet-backlog`, `0` insertions) and pass full-repo `./target/debug/vox-doc-pipeline --lint-only` (`0` hard errors) | 531 | `[x]` |
| `GATE-02` | Verification | Run `scripts/docs/tutorial-verify.vox` updating `contracts/documentation/tutorial-verification.v1.json` and pass `node --test docs-astro/tests/unit/*.test.mjs` (`0` failures) | 531 | `[x]` |
| `GATE-03` | Verification | Pass `pnpm --dir docs-astro build` (`0` errors, `All internal links are valid.`) and Playwright `tests/build-output` + `tests/suite-portal-showcase.spec.ts` | 531 | `[x]` |
| `GATE-04` | Closeout | Verify `0` truncated descriptions or `/Users/brbrainerd` matches across `docs/src/` and close Milestone `v8.9` | 532 | `[x]` |
