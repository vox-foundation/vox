DONE_WITH_CONCERNS

Files changed:
- `crates/vox-gui/ui/src/lib/axisDrive.ts` — catalog rows now mirror `provider` and `provider_type`.
- `crates/vox-gui/ui/src/lib/useDriveBus.ts` — `state` loads the provider catalog.
- `crates/vox-gui/ui/src/lib/axisDrive.test.ts` and `useDriveBus.test.ts` — catalog/provider and state-loading coverage.
- `scripts/axis-drive-openrouter-e2e.vox` — live proof using `reply_ok`, nested state checks, and `submit_ok`.
- `docs/src/how-to/how-to-axis-drive-openrouter-e2e.md` — operator instructions and failure semantics.

Tests run:
- `pnpm test src/lib/axisDrive.test.ts src/lib/useDriveBus.test.ts` — PASS, 20 tests.
- `pnpm typecheck` — PASS.
- `pnpm build` — PASS.

Physical e2e result:
- The script typechecked after VoxScript syntax fixes, but did not reach a green physical proof in this environment.
- Initial run failed with `invalid JSON: EOF while parsing a value at line 1 column 0` while the stale GUI host was not returning a Drive response.
- After rebuilding the GUI frontend, `vox-gui`, and GUI-enabled `vox-cli`, live `drive set` remained stalled waiting for the GUI host to become ready. The direct startup path also initially hit `Operation not permitted (os error 1)` under the sandbox.
- No OpenRouter key was read from the environment. The script exits non-zero and names `SecretId::OpenRouterApiKey` / `vox secrets doctor` when no selectable model is available.

Commits:
- `744d971d8` — `feat(scripts): Axis Drive OpenRouter e2e with reply_ok gate`
- Follow-up syntax/runtime fixes: `39e3046ee`, `495dcb3ec`, `ff2abd6ab`, `3a8874bfd`, `b9d75b71b`

Concerns:
- A real GUI-capable, unsandboxed desktop session with OpenRouter credentials is still required to produce the PASS summary JSON.
