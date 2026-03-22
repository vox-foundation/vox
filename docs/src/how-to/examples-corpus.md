---
title: "How to: Examples corpus & strict parse"
category: how-to
last_updated: 2026-03-21
---

# Examples corpus & strict parse

## Golden set (15 files, 0 failures)

`crates/vox-parser/tests/parity_test.rs` defines **`MUST_PARSE`**: those paths under `examples/` **must** parse in CI. The list is mirrored in [`examples/README.md`](../../../examples/README.md) and [`examples/PARSE_STATUS.md`](../../../examples/PARSE_STATUS.md).

As of March 2026, the golden set covers 15 files:
- Core app patterns: `chatbot.vox`, `full_stack_minimal.vox`, `hello-vox/src/main.vox`, `multi_route_app.vox`
- Data layer: `data_layer.vox`, `durable_counter.vox`, `server_fn.vox`
- Durable execution: `workflow.vox`
- ADTs & testing: `generics_option.vox`, `pattern_matching.vox`, `testing.vox`
- React hooks: `hooks_demo.vox`
- Islands & v0: `island_demo.vox`, `v0_component.vox`
- MCP/AI-native: `mcp_tool_demo.vox`

## Canonical style

[`examples/STYLE.md`](../../../examples/STYLE.md) is the target shape for new golden files (JSX, `routes:`, imports).

> **Brace-syntax note (v0.3 plan, not yet implemented):** The KI `vox_v0_2_syntax_standard.md` documents a planned migration to `fn f() { }` brace-delimited blocks for lower Kolmogorov complexity. This migration is **not yet live in the parser**. All golden files must use the current colon-indent syntax until the lexer/parser migration lands.

## Known parser gaps (for training data curation)

- `true`/`false` are **not** valid `match` arm patterns — use `if`/`else` or constructor patterns
- Multi-line JSX attributes (attribute on its own line) are **not** reliably supported
- Generic function syntax (`fn foo<T>(...)`) parses tokens but is not supported in the type system

## Strict parse (opt-in)

- **Env:** `VOX_EXAMPLES_STRICT_PARSE=1`
- **Command:** `cargo test -p vox-parser --test parity_test`
- **Meaning:** every `examples/**/*.vox` must parse — **not** the default CI gate while 13 archive files still fail.

Thin delegates: [`scripts/examples_strict_parse.sh`](../../../scripts/examples_strict_parse.sh), [`scripts/examples_strict_parse.ps1`](../../../scripts/examples_strict_parse.ps1).

Runner contract: [CI runner contract](../ci/runner-contract.md) (section **Optional: strict parse for all examples**).

## Training / Populi

Prefer golden root examples for corpus ingest; treat `examples/archive/**` as **non-canonical** unless a pipeline explicitly opts in (see [`examples/archive/README.md`](../../../examples/archive/README.md)).

