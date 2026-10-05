# Deferred items (phase 02)

- 02-05: `cargo check -p vox-search --no-default-features` fails at HEAD: `policy.rs` (~line 493) uses `crate::tavily_research` without a `tavily` cfg guard. Pre-existing, unrelated to the Tantivy removal.
