# Summary 68-02: `@webhook` Verification, `@rate_limit` & `@metered` Transactional Credit Ledger

## Status
Completed (`CLOUD-02`, `CLOUD-03`, `CLOUD-04`).

## Key Changes
1. **`@webhook` Signature & Replay-Window Middleware (`CLOUD-02`):**
   - Added `emit_webhook_guard_prelude` in [`crates/vox-codegen/src/codegen_rust/emit/http.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-codegen/src/codegen_rust/emit/http.rs) supporting `Stripe`, `Github`, `Slack`, and `Custom` providers with replay-window timestamp enforcement (`x-vox-webhook-timestamp`).
2. **Transactional `@metered` Credit Ledger (`CLOUD-03`):**
   - Added `CreditLedger` (`reserve`, `commit`, `refund`, `run_metered`) in [`crates/vox-actor-runtime/src/builtins/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-actor-runtime/src/builtins/mod.rs) guaranteeing automatic credit refund on failure.
