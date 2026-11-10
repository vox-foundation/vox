# Phase 337 Plan 09 Summary: Cancellable Decode Stream and Verified Batch Width Bound

## Overview
Implemented the host and plugin primitives for continuous, cancellable decode streams and verified batch width bounding:
- Plugin batch cancel: added `batch_cancel` to `InferenceEngine` in `crates/vox-plugin-mens-candle-metal/src/inference/batch.rs` and routed `{"batch_cancel": true}` in `generate_from_json` in `inference.rs`. Drops active batch session and returns `{"cancelled": n}`.
- Host stream control: introduced `StreamControl { Continue, Cancel }` and `decode_stream` in `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs`. On `StreamControl::Cancel`, issues `batch_cancel` to the backend and terminates cleanly. Re-expressed `decode_batched` as a `decode_stream` instance.
- Verified batch width: added `mens/config/decode-identity.yaml` specifying `f32.verified_batch_width: 64` with empirical evidence from `337-08` (CPU and Metal bitwise greedy identity proven across sizes up to width 64; KV allocator power-of-two bound holds). Added `verified_batch_width(path, protocol)` in `reasoning_eval.rs`.
- Request clamping: every f32 batch request automatically carries `max_batch_size` set to the verified batch width, and any explicit `VOX_REASONING_EVAL_BATCH_SIZE` above this bound is clamped with a warning.
- Updated `contracts/config/env-vars.v1.yaml` description for `VOX_REASONING_EVAL_BATCH_SIZE`.

## Verification & Tests
- `batch::tests::test_batch_cancel_drops_the_session`: verifies `batch_cancel` drops session, replies with remaining sequence count, and leaves subsequent `batch_next` returning error.
- `reasoning_eval::tests::a_cancel_stops_consumption`: verifies cancellation stops stream consumption and issues `batch_cancel`.
- `reasoning_eval::tests::a_stream_reports_every_item`: verifies all stream items are received once.
- `reasoning_eval::tests::the_committed_decode_identity_file_loads`: verifies loading committed YAML returns `Some(64)` for F32 and `None` for Bf16.
- `reasoning_eval::tests::decode_identity_refuses_zero_width_or_unknown_key`: ensures zero width or unknown fields are rejected with an informative error.
- `reasoning_eval::tests::a_request_carries_the_verified_width`: verifies requests include `max_batch_size`.
- `reasoning_eval::tests::an_explicit_width_above_the_verified_width_is_clamped`: verifies explicit sizes exceeding verified width are clamped.

## Mutation Proof
- Grep presence check `grep -c 'max_batch_size' crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs`: 4 before, 4 after.
- Mutation: deleted `body["max_batch_size"] = mw.into();` in `decode_stream`.
- Result: `a_request_carries_the_verified_width` FAILED (`left: Null, right: 64`).
- Restored and confirmed test passed green.
