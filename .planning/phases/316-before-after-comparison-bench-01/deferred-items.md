# Deferred items (phase 316)

- `cargo run -p vox-cli -- ci command-compliance` cannot run on branch `gsd/mens-compare-316`:
  `vox-cli` fails to compile at `vox-orchestrator-mcp` (19 errors, e.g. E0609 "no field
  `id_allocator` / `thread_run_queue` / `steered_prompts` on `ServerState`" in
  `crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs`). This is the same class as
  4bf115518 / 35dd5d679 (code present only in the untracked `~/dev/vox` checkout). The installed
  `vox` is stale (built at 252cfe7af) and refuses to run guards. Out of scope for 316-01; re-run
  `command-compliance` once `vox-cli` builds on this branch.
