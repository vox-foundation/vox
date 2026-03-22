/// V9: Codex lineage index (forward-only; Turso `execute_batch` — no bare `SELECT`).
pub const SCHEMA_V9: &str = "
CREATE INDEX IF NOT EXISTS idx_codex_schema_lineage_baseline
  ON codex_schema_lineage(baseline_id);
";
