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
- Retried outside the sandbox with full permissions using `vox run scripts/axis-drive-openrouter-e2e.vox`.
- GUI startup and Drive IPC completed far enough to load the catalog, but no selectable OpenRouter model was available. The script exited 1 with:
  `OpenRouter Drive e2e failed: no selectable OpenRouter model; provision SecretId::OpenRouterApiKey and run vox secrets doctor`
- `vox secrets doctor` independently reports `OpenRouterApiKey: MissingRequired via None (missing)`.
- No OpenRouter key was read from the environment. The script fails closed and names `SecretId::OpenRouterApiKey` / `vox secrets doctor` as intended.

Commits:
- `744d971d8` — `feat(scripts): Axis Drive OpenRouter e2e with reply_ok gate`
- Follow-up syntax/runtime fixes: `39e3046ee`, `495dcb3ec`, `ff2abd6ab`, `3a8874bfd`, `b9d75b71b`

Concerns:
- A configured `SecretId::OpenRouterApiKey` and a rerun are still required to produce the PASS summary JSON. The current physical attempt is a parseable, honest failure rather than a green proof.
