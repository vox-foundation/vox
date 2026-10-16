# Summary 67-02: Partial `db.Table.patch`, `db.bucket` Aggregations & Multi-Tenant Row Scoping

## Status
Completed (`USERDB-02`, `USERDB-03`, `USERDB-04`).

## Key Changes
1. **Partial `patch` Without Column Clobbering (`USERDB-02`):**
   - Emitted `pub async fn patch(db: &Codex, id: <pk>, patch_fields: &serde_json::Map<String, serde_json::Value>) -> Result<usize, turso::Error>` in [`crates/vox-codegen/src/codegen_rust/emit/tables/codegen.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-codegen/src/codegen_rust/emit/tables/codegen.rs).
2. **Declarative `bucket_count` Aggregations (`USERDB-02`):**
   - Emitted `pub async fn bucket_count(db: &Codex, column: &str, limit: usize) -> Result<Vec<(String, i64)>, turso::Error>` with column allowlist validation.
3. **Compile-Time Multi-Tenant Row Scoping (`USERDB-03`):**
   - Added `scoped_by: <field_name>` option parsing to `table(...)` in [`crates/vox-compiler/src/parser/descent/decl/mid.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-compiler/src/parser/descent/decl/mid.rs) and emitted `pub async fn all_for_tenant(db: &Codex, scope_val: &str) -> Result<Vec<Self>, turso::Error>` on tenant-scoped tables.
