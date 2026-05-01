//! Arca SQL: Vox Mesh reputation ledger, compute donation tracking, and A2A durable store.
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

-- ── Populi mesh durable store (schema v1, 2026-05-01) ──────────────────────────

-- A2A inbox: persisted delivery envelopes for the mesh control plane.
CREATE TABLE IF NOT EXISTS mesh_a2a_messages (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    sender_agent_id         TEXT NOT NULL,
    receiver_agent_id       TEXT NOT NULL,
    message_type            TEXT NOT NULL,
    payload                 TEXT NOT NULL,
    idempotency_key         TEXT,
    idempotency_dedupe_key  TEXT,
    privacy_class           TEXT,
    payload_blake3_hex      TEXT,
    worker_ed25519_sig_b64  TEXT,
    jwe_payload             TEXT,
    priority                INTEGER NOT NULL DEFAULT 128,
    task_kind               TEXT,
    model_id                TEXT,
    sender_node_id          TEXT,
    created_at              INTEGER NOT NULL,
    acked_at                INTEGER,
    acknowledged            INTEGER NOT NULL DEFAULT 0,
    lease_holder_node_id    TEXT,
    lease_expires_unix_ms   INTEGER
);

CREATE INDEX IF NOT EXISTS idx_mesh_a2a_receiver
    ON mesh_a2a_messages(receiver_agent_id, acknowledged);
CREATE INDEX IF NOT EXISTS idx_mesh_a2a_idempotency
    ON mesh_a2a_messages(idempotency_key)
    WHERE idempotency_key IS NOT NULL;

-- Remote execution leases: serialised grant/renew/revoke lifecycle.
CREATE TABLE IF NOT EXISTS mesh_exec_leases (
    lease_id        TEXT PRIMARY KEY,
    task_id         TEXT NOT NULL,
    scope_key       TEXT NOT NULL,
    holder_node_id  TEXT NOT NULL,
    granted_at      INTEGER NOT NULL,
    expires_at      INTEGER NOT NULL,
    state           TEXT NOT NULL DEFAULT 'granted'
);

CREATE INDEX IF NOT EXISTS idx_mesh_lease_state
    ON mesh_exec_leases(state, expires_at);

-- Detached dispatch results (Wave 5 async execution).
CREATE TABLE IF NOT EXISTS mesh_dispatch_results (
    key         TEXT PRIMARY KEY,
    value_json  TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);
";
