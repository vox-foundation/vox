# Roadmap: Workstream `frontpage-tour-playground-editorial-v93` (`v9.3`)

Numbering continues after v9.1 (ends 540). v9.2 has no roadmap yet; renumber if it claims 541+.

- [ ] **Phase 541: Unified Playground & MCP/VUV Explainers** (`PLAY-01..06`)
  1. One tab strip; no second pane duplicating Rust/TSX.
  2. TSX tab explains `useVoxQuery` and shows the MCP descriptor; call-path strip.
  3. VUV example with preview.
  4. Playground emitter checked against compiler output on presets.
- [ ] **Phase 542: Graphics Audit & Regeneration** (`GFX-01..03`)
  1. Inventory + classification manifest.
  2. Regenerate failing images via registry-selected model; vector where possible.
- [ ] **Phase 543: HD Dark/Light Tour Captures** (`TOUR-HD-01..04`)
  1. Re-capture all 9 slides in both themes at ≥2560×1600.
  2. Manifest + CI check; responsive `srcset`.
- [ ] **Phase 544: Core Subsystems Carousel** (`SUBS-01..04`)
  1. Shared carousel base; data-driven slides; ≥10 subsystems with captures/diagrams.
- [ ] **Phase 545: Editorial Rewrite & Style Lint** (`EDIT-01..05`)
  1. De-duplicate the "one script → many things" claim.
  2. Journalism-register rewrite with why/benefit structure; style guide + lint; anchored numbers.
- [ ] **Phase 546: Verification & Budget** (`VER-01..03`)
  1. Playwright light/dark/mobile; load budget; doc lint and drift gate.

Order rationale: 541 and 542 are independent; 543 and 544 depend on 542's asset decisions; 545 runs last so copy describes what actually shipped; 546 closes.
