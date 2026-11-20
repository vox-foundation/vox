---
gsd_state_version: "1.0"
milestone: v9.3
milestone_name: "Frontpage Tour, Unified Playground & Editorial Rewrite"
current_phase: 541
status: planning
last_updated: "2026-11-19T13:50:00.000Z"
workstream: frontpage-tour-playground-editorial-v93
---

# Project State: frontpage-tour-playground-editorial-v93 (Milestone v9.3)

Phase: 541 in progress (code written; not visually verified)
Resume: `/gsd-discuss-phase 542 --ws frontpage-tour-playground-editorial-v93` after 541 is browser-verified

## Phase 541 progress (2026-11-19)
Done (files written, script syntax-checked with `node --check`; **never rendered**: `astro dev` exits "failed to start within 30s" on the 1,100-page corpus):
- PLAY-01: one tab strip (Run output, SQL, Rust server, TS client, React UI, MCP tools); secondary pane and its CSS removed. `VoxPlayground.astro` rewritten; `index.mdx` now calls `<VoxPlayground />`.
- PLAY-05 (partly): the old playground emitted fabricated output with a regex emitter. It is gone. Output now comes from real `vox build` / `vox run --interp` captured in `docs-astro/src/data/playground/` (README there). Edited code shows a banner saying the browser cannot compile.
- PLAY-02/03: per-tab notes and a request-flow strip, from `presets.json`. `useVoxQuery` does not exist in generated code; the real pieces are the typed fetch function in `vox-client.ts` and optional `useVoxServerQuery` in `vox-tanstack-query.tsx`. The page now says so.
- PLAY-06: arrow/Home/End keys on both tablists, roving tabindex (not yet tested with axe).
Not done: PLAY-04 (VUV demo beyond the `view:` component shown in the React UI tab), automated fixture-vs-compiler check, browser/axe verification.

Findings about the compiler surfaced by the audit (not fixed; outside this milestone):
- The old first preset (`fn main() to Result[str]` using `db`) fails `vox build`: "async main cannot return a value".
- The interpreter and `vox build` disagree on the old preset (`complete_task(id)`: interpreter ran, build rejected it with a type mismatch).
- Generated Rust looks uncompilable in places (`Task::delete(&*db, id).await` inside a non-async fn; `raw < 1024i64` compares a serde `Value`). Not checked with cargo.
- The generated TS client types `Result[T]` queries as `Promise<unknown>`.


Findings from recon:
- Playground (`VoxPlayground.astro`) has `.target-tabs-bar` (runtime/sql/rust/tsx) plus `.secondary-tabs-bar` (rust/tsx): the duplication the user saw.
- Tour (`VoxGuiTourCarousel.astro`): 9 slides, all 1280×800 WebP; only research, memory, doc-reader, theme have dark variants.
- `index.mdx` sections: Start, playground, Axis tour, Why Vox, Interpreter vs Native, Benchmark & Core Subsystems (CardGrid of 6), Ecosystem, Tradeoffs.
- VUV = Vox UI vocabulary (`docs/src/architecture/vuv-naming-policy-2026.md`); not demoed on the frontpage.
