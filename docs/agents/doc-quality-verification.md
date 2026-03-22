# Documentation quality: verification and progress tracking

## After each documentation batch

### Rust crates

| Check | Command |
|-------|---------|
| No missing-docs regressions | `cargo rustc -p <crate> --lib -- --deny=missing-docs` |
| Clippy clean | `cargo clippy -p <crate> --all-targets -- -D warnings` |
| Format (if many edits) | `cargo fmt -p <crate>` |

### Workspace (periodic)

| Check | Command |
|-------|---------|
| Full clippy | `cargo clippy --workspace --all-targets -- -D warnings` |

### Inventory refresh

| Check | Command |
|-------|---------|
| Regenerate baseline | `cargo run -p vox-cli -- ci doc-inventory generate` |
| CI parity (no drift) | `cargo run -p vox-cli -- ci doc-inventory verify` (runs in `.github/workflows/ci.yml`) |

Output uses `schema_version` **3**: includes `symbol_hints` (hotspot Rust files: `///` → next item line + `quality_tag`, plus `containing_symbol` / `doc_preview`). Commit `docs/agents/doc-inventory.json` when you want a named baseline for diffs; otherwise regenerate locally before each wave.

## Progress metrics (lightweight)

Track manually or in CI notes:

1. **Hotspot tier-1 files** touched from [doc-inventory.json](doc-inventory.json) (plan-listed paths).
2. **Line deltas** (optional): `lines_triple_slash + lines_inner_doc` before/after for the file from inventory snapshots.
3. **Qualitative**: fewer restate-the-field docs; more invariants and SSOT links.

### Hotspot rewrite wave (Phase 6)

Use **`symbol_hints`** + **`quality_tag`** in [doc-inventory.json](doc-inventory.json) to pick the next file: prefer **`ssot_sensitive`** / **`operational`** on plan-listed paths (see `.cursor/plans/native_qlora_ssot_dea968e4.plan.md` § Phase 6 and hotspot lists in `crates/vox-doc-inventory/src/lib.rs`). One file per PR keeps review small; re-run **`vox ci doc-inventory generate`** after substantive `///` edits.

## Success criteria (per the rubric)

- Mechanical docs replaced with **intent, contracts, failure modes**, or **links**.
- No new SSOT contradictions vs [AGENTS.md](../../AGENTS.md) and ADRs.
- Lints unchanged or stricter. **Exceptions:** `vox-tensor` uses `#![cfg_attr(feature = "gpu", allow(missing_docs))]` (Burn wrappers). `vox-populi` uses `#![allow(missing_docs)]` on the crate root (CLI/training wiring). Do not copy these to compiler/runtime/core crates.

## Snapshot template (paste into PR / issue)

| Metric | Before | After |
|--------|--------|-------|
| Inventory `hotspot_tier` for touched paths | | |
| `cargo clippy --workspace -- -D warnings` | pass/fail | pass/fail |
| Notes (SSOT links added, files deleted) | | |

## Related

- [documentation-rubric.md](documentation-rubric.md)
- [llm-documentation-playbook.md](llm-documentation-playbook.md)
- [.agents/cargo-safety.md](../../.agents/cargo-safety.md) (Windows / cargo lock hygiene)
