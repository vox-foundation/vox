//! Arca SQL: Vox Mesh reputation ledger and compute donation tracking.
pub const SCHEMA_VOX_MESH: &str = "
-- Contributor reputation ledger for decentralized GPU mesh.
-- Unifies compute donation (Kudos) and code contribution rewards.
CREATE TABLE IF NOT EXISTS vox_kudos (
    vox_user_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    primitive TEXT NOT NULL, -- 'gpu_compute', 'code_review', 'bug_fix'
    amount INTEGER NOT NULL,
    task_id TEXT,
    created_unix_ms INTEGER NOT NULL,
    metadata_json TEXT,
    PRIMARY KEY (vox_user_id, node_id, created_unix_ms)
);

CREATE INDEX IF NOT EXISTS idx_vox_kudos_user ON vox_kudos(vox_user_id);
CREATE INDEX IF NOT EXISTS idx_vox_kudos_node ON vox_kudos(node_id);

-- Sybil mitigation and peer routing tracker.
CREATE TABLE IF NOT EXISTS vox_peer_reputation (
    node_id TEXT PRIMARY KEY,
    success_count INTEGER NOT NULL DEFAULT 0,
    fail_count INTEGER NOT NULL DEFAULT 0,
    timeout_count INTEGER NOT NULL DEFAULT 0,
    invalid_output_count INTEGER NOT NULL DEFAULT 0,
    last_updated_unix_ms INTEGER NOT NULL
);
";
