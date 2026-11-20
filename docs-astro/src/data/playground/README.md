# Playground fixtures

Every file here is real compiler output, not hand-written. The playground on the frontpage shows these files and never fabricates output.

Per example (`tasks/`, `result/`, `workflow/`):

| File | How it was produced |
|---|---|
| `source.vox` | the example source |
| `runtime.txt` | `vox run --interp main.vox` |
| `sql.sql` | the `CREATE TABLE` statement from the generated `main.rs` (`vox build main.vox`) |
| `rust.rs` | excerpt of generated `main.rs` / `lib.rs` (route handler, row type, tool and workflow functions) |
| `client.ts` | the per-query functions at the end of generated `vox-client.ts` |
| `ui.tsx` | the generated component file |
| `mcp.rs` | the `tools/list` section of generated `mcp_server.rs` |
| `contract.json` | generated `vox-app-contract.json` |

`presets.json` holds the labels, the one-line summaries, the per-tab explanations and the request-flow steps. Re-capture the fixtures after any change to code generation, and update `capturedWith` in `presets.json`. A file is missing when the compiler emits nothing for it (for example no `client.ts` for a file with no query).

The Rust is shown as emitted. It is not compiled with cargo as part of the docs build.
