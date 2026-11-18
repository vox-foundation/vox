# Phase 474 Context: Nov 2026 SOTA Terminal UX: `OSC 8` / `OSC 52` / Kitty `CSI u`, Per-Block Telemetry, `std.frame`/`std.plot` & `vox-term` Ghost Text

**Workstream:** `axis-terminal-sota-v80`
**Phase:** 474
**Requirements:** `TERM-UX26-01`, `TERM-UX26-02`, `TERM-UX26-03`, `TERM-UX26-04`
**SSOT:** [`terminal-surfaces-audit-and-benchmarks-research-2026.md`](../../../../docs/src/architecture/terminal-surfaces-audit-and-benchmarks-research-2026.md)

## Decisions
1. Preserve Console component modularity and `< 400` LoC budget by placing new protocol parsers and rich visual block renderers in dedicated sibling modules (`oscProtocols.ts`, `RichBlockPreview.tsx`).
2. Render `OSC 8` clickable hyperlinks cleanly in `sgrToSpans.ts`, handle bounded `OSC 52` clipboard payloads, and add Kitty Keyboard Protocol `CSI u` encoding for both WebView raw VT (`oscProtocols.ts`) and `vox-term` (`crates/vox-term/src/app.rs`).
3. Surface per-block execution telemetry (`duration_ms`, exit status, optional `rusage` CPU/RSS) and inline `std.frame` tables + `std.plot` pure React SVG charts inside `BlockCard.tsx`.
4. Add dimmed inline ghost-text autosuggestions (`ghost_suffix`, `accept_ghost`, `update_ghost`, dynamic `active_mode`) backed by `HistoryStore` + `CompletionEngine` in `crates/vox-term/src/ui/input.rs`.
