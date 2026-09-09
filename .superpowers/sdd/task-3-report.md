# Task 3 Report: Drive agent-event mirror

Status: Complete

## Implementation

- `AxisDriveHost` now owns an always-on `listenAgentEvents` subscription.
- Agent frames are recorded with `recordAgentFrame`, using `frame.kind.type` and
  `frame.kind.text`, under the synchronous `activeTurnIdRef`.
- The Drive send lifecycle publishes its turn ID before awaiting submit and
  merges frames received during submit back into the final event ring.

## TDD Evidence

1. RED: `pnpm --dir crates/vox-gui/ui test src/components/drive/AxisDriveHost.events.test.tsx`
   failed at `expect(mocks.agentHandler).not.toBeNull()` before the listener existed.
2. GREEN: the same focused test passed after implementing the host subscription
   and synchronous turn-start hook.
3. Mutation: replacing `listenAgentEvents` with a listener that did not register
   the callback made the host test fail at the same assertion; restoring the
   production call returned the suite to green.
4. Required suite: 3 files, 20 tests passed.
5. TypeScript: `pnpm --dir crates/vox-gui/ui typecheck` passed.

## Concerns

- pnpm emits an existing warning that the package-level `pnpm.overrides` field is ignored.
- The test runner emits an existing jsdom `localStorage` experimental warning.

## Fix Evidence: Clear Completed Turn

- RED: `pnpm --dir crates/vox-gui/ui test src/components/drive/AxisDriveHost.events.test.tsx`
  failed because a `token_streamed` frame sent after `send` resolved was still
  present in the subsequent state response.
- GREEN: `pnpm --dir crates/vox-gui/ui test src/components/drive/AxisDriveHost.events.test.tsx src/lib/useDriveBus.test.ts`
  passed — 2 files, 14 tests.
- The Drive request handler now clears `activeTurnIdRef` in a `finally` block
  after `send` handling, preserving frames received while `submit` is awaited
  and ignoring later unrelated frames.
- `pnpm --dir crates/vox-gui/ui typecheck` passed (`tsc --noEmit`).
