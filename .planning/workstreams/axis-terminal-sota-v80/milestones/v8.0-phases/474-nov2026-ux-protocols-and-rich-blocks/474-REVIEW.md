# Phase 474 Code Review

**Scope:** `crates/vox-gui/ui/src/components/surfaces/Console/{oscProtocols.ts,sgrToSpans.ts,RichBlockPreview.tsx,BlockCard.tsx}`, `crates/vox-term/src/{app.rs,ui/input.rs}`

## Findings
- **Warning (`skeleton/untested-pub-api`) — `crates/vox-term/src/ui/input.rs`:** Added public methods (`update_ghost`, `accept_ghost`, `active_mode`) lacked an inline `#[cfg(test)] mod tests` module inside `ui/input.rs`.
- **Confirmed clean:** Console `surfaceHonesty.guard.test.ts` (32/32 surfaces clean), file size budget (`< 400` LoC), and `Console` Vitest (`23/23` files, `141/141` tests).
