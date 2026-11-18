# Phase 474 Summary 01: Nov 2026 Protocols, Rich Previews & TUI Ghost Text

## Shipped Changes
- **`crates/vox-gui/ui/src/components/surfaces/Console/oscProtocols.ts` (151 LoC) & `sgrToSpans.ts` (393 LoC):** Implemented `OSC 8` clickable hyperlink parsing/rendering, `OSC 52` clipboard extraction, and Kitty `CSI u` key encoding.
- **`crates/vox-gui/ui/src/components/surfaces/Console/RichBlockPreview.tsx` (143 LoC) & `BlockCard.tsx` (392 LoC):** Added `rusage` telemetry chip formatting and inline `std.frame` table + `std.plot` pure React SVG previews.
- **`crates/vox-term/src/ui/input.rs` & `src/app.rs`:** Added inline `ghost_suffix` autosuggestions (`Tab`/`Right` acceptance) and Kitty `CSI u` modified-enter encoding.
- **Tests:** Added coverage in `sgrToSpans.test.tsx`, `terminalCompletions.test.ts`, and `RichBlockPreview.test.tsx`.
