# Deferred items (phase 04)

- `cargo run -p vox-arch-check` exits 1 on a pre-existing, unrelated violation: `[no-hardcoded-abs-path] crates/vox-orchestrator-mcp/src/browser_tools.rs:965 "/etc`. Found during 04-01 Task 3; no vox-kb-facets violation. Out of scope.
- `vox-orchestrator-mcp chat_tools::chat::message::kb_leak_tests::kb_leak_chat_turn_writes_no_kb_entry_or_finding` fails (`a chat turn must not write a finding`, left 2, right 0) when run in 04-06. The chat-turn path does not call `vox_search::library` (grep: only `kb_tools/library.rs` and `kb_resources.rs` use it), so the failure is not from the 04-06 change; not bisected. Out of scope.
