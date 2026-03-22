---
title: "Codex, Arca, and Rust import policy"
category: architecture
last_updated: 2026-03-21
---

# Codex, Arca, and Rust import policy

## Names

| Name | Meaning |
|------|---------|
| **Codex** | Product name for the persisted data API. |
| **`VoxDb`** | Stable Rust type for the database facade (`crates/vox-db`). |
| **`Codex`** (Rust) | Type alias for `VoxDb` in `vox_db` — same type. |
| **Arca** | Internal schema / CAS ownership in **`vox-pm`** (`CodeStore`). There is **no** `vox_arca` crate in this workspace. |
| **`vox-codex`** | Compatibility crate: `pub use vox_db::*`. New code should depend on **`vox-db`** directly. |

## Rules

1. Prefer **`vox_db::VoxDb`** (or `vox_db::Codex` alias) in signatures and new modules.
2. Do not introduce new dependencies on the `vox-codex` crate path unless bridging legacy tooling; migrate call sites to `vox-db` when touched.
3. Unwired CLI modules should import **`vox_pm::` / `vox_db::` / `vox_codex`** (shim) only — the historical `vox_arca*` crate names are not used in-tree. Feature-gated or excluded crates (e.g. `vox-dei`) follow the same rule when reattached.

See [ADR 004](../adr/004-codex-arca-turso-ssot.md).
