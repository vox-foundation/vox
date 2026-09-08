# Full branch Important-findings fix report

## Status and commits

All five Important findings from the `0be37f694..e1a35e422` review were fixed
with test-first coverage. The implementation commit is
`7c47826964f7698fa1351140cc70c09dbccd26cf`
(`fix: close interpreter and mesh resource gaps`). This report is committed
separately so it can record that immutable implementation hash.

No push or merge was performed. The implementation commit contains only the
eleven explicitly staged source/test files listed below; pre-existing worktree
changes were not staged.

## Finding 1: interpreter filesystem quotas

### RED

`cargo test -q -p vox-compiler --test caps_enforcement_test quota`

```text
io_save_over_disk_quota_is_denied_before_writing --- FAILED
Ok(Result(Ok(Null)))
test result: FAILED. 2 passed; 1 failed
```

`cargo test -q -p vox-compiler --test caps_enforcement_test recursive_mkdir_charges_every_missing_component_before_creation`

```text
recursive_mkdir_charges_every_missing_component_before_creation --- FAILED
Ok(Result(Ok(Bool(true))))
test result: FAILED. 0 passed; 1 failed
```

### GREEN and implementation

- `io.save` now serializes before mutation, checks the exact serialized byte
  count and new-file count against `FsQuota`, writes only after admission, and
  charges only after a successful write.
- Recursive `fs.mkdir` counts every missing component before
  `create_dir_all`, so a three-directory creation requires three file slots.
- Missing-path resolution now retains the full suffix below the nearest
  canonical existing ancestor; the previous resolver collapsed `a/b/c` to
  `c`, which the nested quota test exposed.
- Scoped path checks and fatal `fs.quota` denial remain unchanged.

```text
cargo test -q -p vox-compiler --test caps_enforcement_test
25 passed; 0 failed
```

## Finding 2: mailbox receiver/outbox bounds

### RED

The first capacity test failed to compile because `MailboxLimits` had no inbox
entry/byte or outbox-byte fields. After adding per-handle locking, the stronger
independent-handle concurrency test demonstrated the race:

```text
cargo test -q -p vox-mesh-transport --lib concurrent_inbox_writers_cannot_overbook_capacity
assertion failed: left == right
left: 8
right: 1
```

### GREEN and implementation

- Added receiver-imposed `max_inbox_entries`, `max_inbox_bytes`, and
  `max_outbox_bytes`; retained the per-message and outbox-depth limits.
- Inbox and outbox handles for the same absolute queue directory share a
  process-wide reservation mutex. Under that lock they check duplicate slots,
  serialize, recompute entry/byte usage from disk, reserve by admission, and
  atomically write.
- Duplicate inbox/outbox slots succeed even when capacity is full.
- Removals and successful flushes release capacity naturally because every
  admission recomputes usage from current disk metadata.
- Tests cover inbox entry/byte refusal, outbox depth/byte refusal, duplicate
  behavior, and eight concurrent independent handles.

```text
cargo test -q -p vox-mesh-transport --lib mailbox::tests
15 passed; 0 failed
```

## Finding 3: endpoint lifecycle

### RED

`cargo test -q -p vox-mesh-transport --test security live_registration_is_removed_after_connection_handler_exits`

```text
error[E0599]: no method named `registered_connections` found for struct `Arc<MeshTrust>`
```

The companion live-endpoint regression opens 64 idle trusted connections and
then requires a probe on connection 65. With the reviewed implementation, the
first 64 connection tasks retained all 64 handshake permits for their complete
lifetimes, so the 65th request could not be served.

### GREEN and implementation

- The handshake permit is dropped immediately after the authenticated QUIC
  handshake, before the trusted connection enters its protocol handler.
- A per-peer live-connection cap is atomically enforced during registration.
- `LiveRegistration` removes its exact token from `MeshTrust` on every normal,
  error, or timeout return; `untrust` still drains and closes all registered
  connections.
- `accept_bi`, protocol frame reads, and final close waits are bounded.
  Executor execution itself is not wrapped in the protocol timeout, preserving
  the permitted job wall clock.
- Mailbox stream acceptance/frame reads/final wait are bounded as well.

```text
cargo test -q -p vox-mesh-transport --test security
21 passed; 0 failed
```

This includes both
`idle_trusted_connections_release_handshake_permits` and
`live_registration_is_removed_after_connection_handler_exits`.

## Finding 4: Populi activity timeout/cancellation

### RED

`cargo test -q -p vox-workflow-runtime --lib stalled_dispatch_times_out_and_cancels_the_same_job_id`

```text
error[E0425]: cannot find function `run_on_peer_with_timeout`
```

### GREEN and implementation

- The caller now assigns `JobId` before dispatch and passes it into
  `run_on_peer`.
- `PopuliActivity.timeout_ms` bounds the complete connect/open/write/read
  round trip; the existing 300-second behavior is the default when unset.
- Deadline expiry opens a bounded best-effort cancellation round trip and sends
  `JobRequest::Cancel` with the same `JobId`, then returns an explicit timeout
  error.
- The no-source Dispatch failure is unchanged.
- A live stalled test peer records both requests and proves Run/Cancel IDs
  match.

```text
cargo test -q -p vox-workflow-runtime --lib workflow::populi::tests
5 passed; 0 failed; 1 ignored
```

## Finding 5: memory accounting through rendering

### RED

`cargo test -q -p vox-cli --test run_interp_limits composite_result_rendering_cannot_escape_memory_accounting`

```text
status=Some(0), stdout=33554449 bytes
expected exit Some(79)
```

### GREEN and implementation

`mem_limit::disarm()` now runs only after result display, printing, and
exit-command flushing complete. The near-limit composite result therefore
exits 79 during rendering instead of allocating roughly 32 MiB outside the
ceiling.

```text
cargo test -q -p vox-cli --test run_interp_limits
7 passed; 0 failed
```

## Final verification

```text
cargo clippy -q -p vox-compiler -p vox-mesh-transport \
  -p vox-workflow-runtime -p vox-cli --all-targets -- -D warnings
exit 0

cargo build -q -p vox-cli --bin vox
exit 0

cargo test -q -p vox-mesh-transport --test interp_executor -- --include-ignored
14 passed; 0 failed

git diff --check -- <eleven scoped implementation files>
exit 0
```

Formatting used serial `cargo fmt -p` invocations, never `cargo fmt --all`.
The enabled pre-commit hooks ran `fmt-fix` and `tdd-guard`; both passed.

## Implementation files

- `crates/vox-compiler/src/eval/builtins.rs`
- `crates/vox-compiler/src/eval/shell_stdlib.rs`
- `crates/vox-compiler/tests/caps_enforcement_test.rs`
- `crates/vox-mesh-transport/src/endpoint.rs`
- `crates/vox-mesh-transport/src/lib.rs`
- `crates/vox-mesh-transport/src/mailbox.rs`
- `crates/vox-mesh-transport/src/trust.rs`
- `crates/vox-mesh-transport/tests/security.rs`
- `crates/vox-workflow-runtime/src/workflow/populi.rs`
- `crates/vox-cli/src/commands/run.rs`
- `crates/vox-cli/tests/run_interp_limits.rs`

## Concerns

- Queue reservation is concurrent-handle-safe within the receiver process.
  This matches the single-owner mailbox architecture; it is not an
  inter-process filesystem lock.
- The worktree still contains extensive unrelated pre-existing modifications
  and GUI test artifacts. They were not included in either scoped commit.
