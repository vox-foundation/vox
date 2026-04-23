//! Arca domain: visus (visual intelligence, baselines, and audit logs).
//!
//! Stores "golden" visual snapshots and metadata for regression testing and VLM training records.

pub const SCHEMA_VISUS: &str = r#"
-- Visual "Golden" snapshots gated by target and configuration.
CREATE TABLE IF NOT EXISTS visus_baselines (
    id TEXT PRIMARY KEY,
    target_url TEXT NOT NULL,
    viewport TEXT NOT NULL,
    theme TEXT NOT NULL DEFAULT 'auto',
    
    -- CAS hashes for binary/large JSON evidence.
    screenshot_cas TEXT NOT NULL,
    ax_tree_cas TEXT NOT NULL,
    
    metadata_json TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    UNIQUE(target_url, viewport, theme)
);

-- Historical record of visual audits (both deterministic and semantic).
CREATE TABLE IF NOT EXISTS visus_audit_log (
    id TEXT PRIMARY KEY,
    baseline_id TEXT REFERENCES visus_baselines(id),
    target_url TEXT NOT NULL,
    
    -- 'clean', 'warning', 'fail'
    outcome TEXT NOT NULL,
    
    -- JSON representation of findings (overlaps, VLM rubric responses).
    findings_json TEXT NOT NULL,
    
    -- CAS hash of the screenshot analyzed (if different from baseline).
    screenshot_cas TEXT,
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Index for fast target-specific baseline lookup.
CREATE INDEX IF NOT EXISTS idx_visus_baselines_target ON visus_baselines(target_url);
CREATE INDEX IF NOT EXISTS idx_visus_audit_target ON visus_audit_log(target_url);
"#;
