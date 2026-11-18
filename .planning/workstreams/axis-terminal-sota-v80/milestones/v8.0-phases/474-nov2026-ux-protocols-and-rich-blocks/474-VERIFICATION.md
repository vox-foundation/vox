# Phase 474 Verification

- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console`: `23/23` test files passed, `141/141` tests passed.
- `wc -l crates/vox-gui/ui/src/components/surfaces/Console/{sgrToSpans.ts,BlockCard.tsx,oscProtocols.ts,RichBlockPreview.tsx}`: `393`, `392`, `151`, `143` lines (all strictly `< 400` LoC).
- `cargo test -p vox-term`: exit 0 across lib (`1` passed / `1` pre-existing interactive ignored), `dispatch` (`2`), `headless` (`8`), `palette` (`4`), `vt` (`3`).
