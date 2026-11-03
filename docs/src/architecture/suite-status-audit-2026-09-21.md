---
title: "Suite status audit and bug handoff (2026-09-21)"
description: "Findings from auditing the README against the whole Vox suite on 2026-09-21, with one self-contained handoff entry per bug found."
category: "Architecture SSOTs"
status: "current"
training_eligible: false
---

# Suite status audit and bug handoff (2026-09-21)

On 2026-09-21 the README was rewritten around a dated status table (Working / Prototype / Research / Not started). Every row was verified by running the installed `vox 0.6.0+build.5357` (about 450 commits behind `main`) and reading current source. This page records the bugs found along the way. Each entry stands alone, so an agent can pick up any one of them as an individual fix.

**Conventions for whoever picks one up:**
- One branch per entry: `fix/<slug>`. Test first. Scoped cargo only.
- Don't push without asking.
- When an entry lands, update the matching README status row and add a README Log line, then mark the entry here `**Fixed <date> (<sha>)**`.
- Before starting, rebuild `vox` from `main`: some behaviour may differ at HEAD.

## High priority (data loss / security / misleading)

1. **`vox fmt` deletes all comments and silently drops unsupported declarations.** `crates/vox-compiler/src/fmt/` has no comment handling, and the printer's catch-all `_ => {}` (`fmt/printer.rs` ~179) discards e.g. `workflow`/`activity` blocks (`golden/checkout_workflow.vox` goes from 29 lines to 1).
   - Repro: `vox fmt` on `examples/golden/hello.vox` strips the frontmatter, ANCHOR and `// vox:skip` lines.
   - Fix: preserve comment trivia. Until that works, fail closed whenever the comment count would drop.
2. **A DB newer than the binary is called "legacy or non-baseline", with advice to export, re-create and re-import it** (doctor separately says "do not delete"). Code: `crates/vox-cli/src/commands/codex.rs:55-60`, `crates/vox-db/src/store/types/error.rs:34`.
   - Seen with DB schema 93 on binary baseline 92.
   - Fix: tell the user to upgrade `vox`, never suggest deleting data, and consider degrading to read-only.
3. **`vox populi serve` writes `mesh.token` into `~/.vox/config.toml` and prints it.**
   - Fix: store the token via Clavis (`vox_secrets`), stop printing it, ask before persisting, and read any legacy config value with a deprecation warning.
   - Then run `vox ci secret-env-guard` and `vox ci secrets-parity`.
4. **`vox rollback` spawns the daemon and rewrites `~/.vox/run/orchestrator-daemon.token`.** That can rotate a live session's token.
   - Its help also credits vox-bounded-fs; the real mechanism is the orchestrator operation log (`orch.undo_operation`).
   - Fix: only the process that binds the socket writes the token, and rollback doesn't spawn a daemon.
5. **The telemetry spool grows without bound and ignores consent.** `init_telemetry_sinks` (`crates/vox-cli/src/lib.rs` ~699) always registers `SpoolSink`.
   - On the audited machine: 8,605 files / 34 MB since Sep 4.
   - Fix per `telemetry-trust-ssot.md` and ADR-023: honour upload-off and `VOX_TELEMETRY=off`, and cap and prune the spool.
6. **The vox-secrets vault panics on a current-thread tokio runtime.** Location: `crates/vox-secrets/src/backend/vox_vault.rs:1459`, `block_in_place`.
   - `vox harness eval` prints the panic 4 times and the secret lookup fails.
   - Fix at the async boundary. Write a `current_thread` test first and confirm it fails.

## Broken core paths

7. **`vox run src/main.vox` fails on the `vox init` scaffold** ("No `fn main()` found"). A fix (route service-shaped files to the app lane) was committed locally on 2026-09-21 but not yet pushed; once it lands the app lane still depends on entry 8.
   - Cause: the auto-mode `@page` heuristic in `crates/vox-cli/src/commands/runtime/run/run.rs`, together with the starter template in `crates/vox-project-scaffold/src/lib.rs`.
8. **The generated Rust server only builds inside a Vox checkout.** (A fix via `VOX_REPO_ROOT` resolution was in progress in another working tree on 2026-09-21.) `crates/vox-codegen/src/codegen_rust/emit/mod.rs` ~698-710 emits relative `path =` dependencies into this repo.
   - It also pulls in heavy crates the app doesn't use.
   - Nothing tests a generated app outside the workspace.
9. **`vox emit client` / `vox build --target client` crash on any `component`.**
   - Cause: the TS write loop in `crates/vox-cli/src/commands/build.rs` ~352 lacks `create_dir_all(parent)`.
10. **`vox test` inherits bug 8.** It fails outside the repo because it goes through the generated Rust project.
11. **The starter template trips its own `id`-column lint** (E0001).
12. **`vox repl` has no session state.** `crates/vox-cli/src/commands/repl.rs` checks each line in isolation, so `let` and `fn` don't persist. Values also print in Debug form.
13. **`vox lsp` needs a `vox-lsp` binary that no installer ships.** Either run the LSP in-process (check crate-edge policy; propose any exception, don't add it) or install it.
14. **`vox term` discards submitted input.** Location: `crates/vox-term/src/app.rs:116`, `let _intent = input.submit();`.
15. **`vox term` fails headless / under `TERM=dumb`**, despite its help text ("Failed to initialize input reader").
16. **`vox memory search` never prints results.** `crates/vox-cli/src/commands/memory_cli/search.rs` binds the results to `_exec`.
    - It also queries DuckDuckGo and Wikipedia without consent.
17. **`vox generate` can't reach a stock Ollama.** It probes `/health` on port 11434, a route Ollama doesn't serve, and its hint references the invalid path `scripts/vox_populi::inference.vox`.
    - `vox chat` has two related problems: its hint names a non-existent `vox ai serve`, and its 30s timeout is too short for a cold model.

## Honesty of CLI surfaces

18. **`vox doctor` exits 0 with 18 checks failing.** Required and optional checks should be distinguished.
19. **`vox doctor` gives wrong advice on macOS:**
    - a WSL Docker fix;
    - `cargo install vox-cli` from crates.io, which isn't published;
    - the retired `vox run --isolation wasm` (ADR-048).
20. **Feature-gated-off commands are still advertised.**
    - `vox speech` needs `oratio`, `vox visus` needs `dei`, `vox mcp` needs `mcp-server`, `vox skill` needs `ars`.
    - `vox stop` exits 0 while printing "not enabled".
    - `vox commands --format json` reports all of them as `compiled_in:true, feature_gate:null`.
    - Decide with evidence whether `mcp-server` and `ars` should be default features.
21. **`vox upgrade` says "up to date" when there are zero releases.** It also still searches for `vox-bootstrap-*` assets (a retired crate).
22. **`vox pm search` turns a registry 404 into "No packages matched".** Code: `crates/vox-package/src/registry.rs` ~139. The default registry URL doesn't exist.
23. **11 of 18 plugin catalog default sources point at nonexistent `vox-foundation/vox-plugin-*` repos.** File: `crates/vox-plugin-catalog/catalog.toml`.
24. **`vox plugin doctor` / `list` accept `mens-candle-cuda` on macOS arm64.** Add platform and host-feature checks.
25. **`vox snapshot orphans` checks nothing.** `crates/vox-cli/src/commands/snapshot.rs:139-140` resolves insta's `source:` relative to the snapshot dir instead of the workspace, then reports 264/264 unresolvable as "0 orphans", exit 0.
26. **`vox term`, `vox shell repl` and `vox init` leave `.vox/store.db` and `clavis_vault.db` in the current directory.**
27. **`vox-orchestrator-d --help` doesn't print help** (it demands a socket env var). The daemon also rejects numeric JSON-RPC ids.
28. **Small polish:**
    - a BOM in `crates/vox-cargo-shim/Cargo.toml`, and a `python -c` hint in `bom-check`;
    - the EBNF header says "Vox 0.4";
    - `vox llm prompt` ignores `NO_COLOR`;
    - the `export-gguf` help points at the gated `merge-qlora`;
    - `vox new web` help overpromises;
    - `vox build -o dist` writes the Rust tree to `src/target/generated`.

## Docs and CI

29. **Dead `cargo vox-cuda-release` alias.** It was removed in e828828a9 but is still referenced in mens-training, cli, how-to-train-mens-4080 and ADR-034. Replace it with the `vox-ml` bundle.
30. **`docs/src/reference/stability.md` overstates maturity.** Re-grade it to the README's four states with dates, fix its tier ordering, and sweep the FAQ and other docs that repeat the old grades.
31. **voxlang.org is stale** (feed stops 2026-05-15) **and `/reference/stability/` returns 404.**
    - `docs-deploy.yml` has no successful run in its last 100; they fail at the Cloudflare Pages API step. The build itself passes.
32. **Main CI is red.**
    - No green main `ci.yml` run was found. "CI Fallback (GitHub-hosted)" failed on 09-19, 09-20 and 09-21.
    - Link checker, Scorecard, CodeQL and setup-e2e are also failing.
    - The self-hosted fleet is down (0/2).
33. **The Mobile iOS E2E workflow can't find `pnpm-lock.yaml` at the repo root.** Set `cache-dependency-path`, and check the Android and EAS lanes too.
34. **ADR-041 is internally stale.**
    - It claims codegen is Stable, and says `emit_main_boot` isn't wired even though `set_current_hir_module` is now emitted.
    - Add a dated status update, and correct the `AGENTS.md` implementation-status paragraph.
35. **Stale crate docs.**
    - The `crates/vox-lsp/README.md` feature list is wrong.
    - The `vox-wasm-engine` docs claim a retired `--backend wasi` CLI path.
36. **The MCP registry and dispatch table drift.**
    - `vox_compiler::ast_inspect` is registered but not dispatched, `vox_visual_rag_query` is a stub, and the dashboard's "mesh kill" isn't wired.
    - Add a registry/dispatch parity test.

## Known elsewhere (not duplicated here)

- **MENS trainer defects:** see the MENS pipeline audit of 2026-09-20 (`mens-fine-tuning-pipeline-audit-2026-09-20.md`, not yet committed on 2026-09-21). The `fix/mens-training-quick-fixes` worktree held 47 uncommitted files and no commits on 2026-09-21.
- **Deep research defects:** see the deep-research audit of 2026-09-20 (`deep-research-system-audit-and-handoff-2026.md`, not yet committed on 2026-09-21). Its fixes were parked, unmerged, on 2026-09-21.
- **`vox-constrained-gen` is declared by 3 crates but imported by none.** That is a design question, not a bug.
