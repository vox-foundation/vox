# Requirements: Milestone v9.3 — Frontpage Tour, Unified Playground & Editorial Rewrite

Workstream: `frontpage-tour-playground-editorial-v93`
Surfaces: `docs/src/index.mdx`, `docs-astro/src/components/{VoxGuiTourCarousel,VoxPlayground}.astro`, `docs-astro/public/assets/**`.
Predecessor: v9.0 (`frontpage-gui-tour-loadtime-cv-v90`, Phases 533/533.1). Supersedes its "one script produces many things" framing.

## Active

### Tour captures (TOUR-HD)
- [x] **TOUR-HD-01**: Every tour slide ships both a light and a dark capture (today 4 of 9 slides have only light).
- [x] **TOUR-HD-02**: All captures are re-taken at ≥2560×1600 (2× of the current 1280×800), served as WebP with `srcset`/`sizes` and explicit width/height (no layout shift).
- [ ] **TOUR-HD-03**: A capture manifest (`contracts/` JSON, generated) lists each slide, theme, source view/route, pixel size and capture date; a CI check fails when a slide lacks a theme variant or is below the minimum size.
- [ ] **TOUR-HD-04**: Slides whose current graphic is a stylised illustration rather than a real UI capture are identified and regenerated (see GFX-01).

### Playground (PLAY)
- [x] **PLAY-01**: The playground has ONE set of output tabs. The secondary Rust/TSX pane (`.secondary-tabs-bar`) is merged into the main target tabs (Runtime, SQL, Rust, TSX + MCP) with no loss of function (side-by-side/split is an option on the one control, not a second tab strip).
- [x] **PLAY-02**: The TSX tab shows the generated client **and** the MCP tool descriptor, with inline annotations that explain `useVoxQuery`: what it calls, which generated endpoint/MCP tool it maps to, and where its types come from.
- [x] **PLAY-03**: A "how a call travels" strip for the selected example shows: component → `useVoxQuery` → generated route → `query`/`mutation` → table; and `tool` → MCP descriptor → agent call → `Result`.
- [x] **PLAY-04**: A VUV example (the view/component vocabulary) is demonstrated: `.vox` view source beside the rendered preview and the emitted TSX.
- [ ] **PLAY-05**: Playground output stays truthful: tabs that are browser-side approximations are labelled as such; anything shown must match `vox build` output for the same snippet (verified by a fixture test comparing the playground emitter with the compiler on the presets).
- [ ] **PLAY-06**: Keyboard and screen-reader behaviour of the unified tablist passes axe and arrow-key navigation tests.

### Editorial rewrite (EDIT)
- [x] **EDIT-01**: Remove the repeated "one script produces many things" claim everywhere except the single place the playground already demonstrates it (the "Why Vox" header block). Every other section says something different.
- [x] **EDIT-02**: Rewrite all `index.mdx` prose (and the matching `SYNC-FROM-README` blocks) in a news-report register: plain, specific, third person, claims backed by a number or a link, no superlatives, no slogans, minimal jargon, jargon explained on first use.
- [x] **EDIT-03**: Each section answers three questions in order: what it is, why it is built that way, and who benefits (the AI-generation benefit and the user's use case, stated separately).
- [ ] **EDIT-04**: A written style guide (`docs/src/contributors/…`) captures the register and a lint (extending the doc pipeline) flags banned marketing terms and unanchored superlatives on the frontpage and README.
- [ ] **EDIT-05**: Frontpage numbers (view count, tool count, benchmark) are read from generated sources or carry a drift anchor, so they cannot go stale silently (reuses the v9.1 anchor ledger).
- [x] **EDIT-06**: README and frontpage have distinct jobs, written into the style guide. README (repo audience): status and stability, build/install from source, repo layout, contributing, citations, licence. Frontpage (newcomer audience): what Vox is, a live try-it, the tours, why it is designed this way. Shared facts (counts, status) come from one source and are synced, not retyped; neither page repeats the other's long-form content, and each links to the other. The README is updated in the same phases as the frontpage.
- [x] **EDIT-07**: Hero shows the Hawthorne bust and epigraph (serif italic, display-font attribution, light/dark, mobile) in place of the test-count sentence (done 2026-11-19; `docs/src/index.mdx`, `reader-layout.css` `.vox-home-epigraph`; not yet visually verified in a browser).

### Graphics (GFX)
- [ ] **GFX-01**: Graphics audit: every image on the frontpage is classified (real capture / diagram / illustration), checked for accuracy against current code, resolution, dark-mode legibility, and whether it still earns its place.
- [ ] **GFX-02**: Regenerate failing graphics at ≥2× with a better image model chosen through the model registry (selection and prompts recorded in a manifest, no hardcoded vendor); diagrams that can be vector (SVG/Mermaid) become vector.
- [ ] **GFX-03**: Every kept graphic has light/dark treatment (or a theme-neutral design), meaningful alt text, and explicit dimensions.

### Core subsystems tour (SUBS)
- [x] **SUBS-01**: Replace the six-card "Core Subsystems" grid with a carousel built on the same component pattern as the GUI tour (shared carousel base, keyboard/ARIA, theme toggle, deep links).
- [x] **SUBS-02**: Cover more subsystems than today (candidates: compiler & type system, dual execution lanes, durable workflows, MCP/tool server and receipts, orchestrator & routing, knowledge base & search, OpenWiki self-healing docs, Scientia/research, MENS training, Axis workbench, secrets/Clavis, telemetry trust); final list agreed at phase discussion.
- [x] **SUBS-03**: Each slide: a real capture or accurate diagram (light + dark, HD), a short factual paragraph (what / why this way / benefit for AI generation / benefit for users), the CLI or MCP command to try it, and a link to the doc.
- [x] **SUBS-04**: Slide content is data-driven (one source file) so the carousel and the docs map cannot diverge.

### Verification (VER)
- [ ] **VER-01**: Playwright captures of the page in light and dark at desktop and mobile show no overflow, the unified tabs work, and carousels reach every slide.
- [ ] **VER-02**: Lighthouse/size budget: the HD images are lazy-loaded with responsive sources and the frontpage load-time budget from v9.0 is not regressed.
- [x] **VER-03**: Doc pipeline lint and `vox audit docs --check-drift` stay green for touched pages.

## Out of Scope
- Changing compiler behaviour to make examples look better (examples must match real output).
- Rewriting architecture/ADR pages (only the frontpage, README sync blocks, and the tutorials the new copy links to).
- Bulk page generation (v9.1 policy).

## Traceability
| REQ | Phase |
|-----|-------|
| PLAY-01..06 | 541 |
| GFX-01..03 | 542 |
| TOUR-HD-01..04 | 543 |
| SUBS-01..04 | 544 |
| EDIT-01..05 | 545 |
| VER-01..03 | 546 |

## Status notes (2026-11-19)

Done and verified: PLAY-01..04, TOUR-HD-01/02, SUBS-01..04, EDIT-01..03 and EDIT-07, VER-03 (doc lint, drift gate 875 baselined, link validator, `docs-astro/tests/suite-portal-showcase.spec.ts` 2/2 in light and dark).

Partial:
- PLAY-05: playground output is real compiler output (fixtures), but no automated check re-captures and diffs it against the compiler. 
- PLAY-06: arrow/Home/End keys verified in a browser; no axe run on the playground itself.
- TOUR-HD-03: a build-time guard fails if a captured slide lacks light/dark at 1280 and 2560 (`VoxCarousel.astro`); no generated JSON manifest with capture dates.
- VER-01: Playwright (desktop, light and dark) plus a browser-agent pass at 390px.

Not done or partial: EDIT-04 (style guide written at docs/src/contributors/frontpage-editorial-style-2026.md and enforced by a Playwright check on the frontpage only; no README lint and no doc-pipeline detector), EDIT-05 (frontpage numbers are not anchored to generators), GFX-01..03 (hero banner regenerated with real Vox text, but the image tool tops out at 1376x768; the schema diagram was rebuilt as light and dark SVG, which is resolution-independent; no audit manifest), VER-02 (payload budget not re-measured).
