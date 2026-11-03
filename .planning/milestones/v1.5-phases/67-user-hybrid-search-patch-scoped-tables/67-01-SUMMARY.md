# Summary 67-01: User-Facing Hybrid Search (`@embed`, `@searchable`, `search_index`, `vector_index`)

## Status
Completed (`USERDB-01`).

## Key Changes
1. **FTS5 & Vector Index DDL Emission (`USERDB-01`):**
   - Added `emit_search_index_ddl` (`CREATE VIRTUAL TABLE IF NOT EXISTS fts_<table>_<idx> USING fts5(...)`) and `emit_vector_index_ddl` (`CREATE INDEX IF NOT EXISTS vidx_<table>_<idx>_<dim>d ...`) in [`crates/vox-codegen/src/codegen_rust/emit/tables/codegen.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-codegen/src/codegen_rust/emit/tables/codegen.rs) and wired both into `emit_db_setup`.
2. **Generated `search_hybrid` Table Method (`USERDB-01`):**
   - Emitted `pub async fn search_hybrid(db: &Codex, query: &str, limit: usize) -> Result<Vec<Self>, turso::Error>` on generated table structs in `emit_table_struct`.
