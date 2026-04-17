# GitHub Copilot Instructions for Vox

Vox uses `AGENTS.md` in the repository root as its single source of truth for cross-tool policy. Ensure you review it.

## Critical Invariants

1. **Retired Surfaces (LLM Guard):**
   - Use `vox-orchestrator`, NOT `vox-dei`.
   - Use `vox-skills`, NOT `vox-ars`.
   - Use `vox-ludus`, NOT `vox-gamify`.
   - Use `vox-compiler`, NOT `vox-lexer`, `vox-parser`, `vox-hir`, `vox-typeck`.
   - Use `component Name() {}`, NOT `@component fn Name()`.
   - Use `VOX_DB_URL` / `VOX_DB_TOKEN`, NOT `TURSO_URL` / `VOX_TURSO_URL` / `VOX_TURSO_TOKEN`.
   - Use `recall_async()`, NOT `recall()`.
   - Use `sync_to_db()`, NOT `persist_fact()`.

2. **Secret Management:**
   - NEVER read secrets from environment variables directly (e.g., `std::env::var`).
   - ALWAYS use `vox_clavis::resolve_secret(...)`.

3. **TOESTUB & Governance:**
   - Skeleton code (`stub/todo`, `unimplemented!()`, empty bodies) is blocked by CI.
   - Do NOT modify `contracts/` without extreme care.
   - Do NOT write to `archive/` or `docs/src/archive/`.
   - Do NOT create `.py` files in `scripts/`; prefer Rust tooling.
