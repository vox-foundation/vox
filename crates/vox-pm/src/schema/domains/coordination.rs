//! Arca SQL: Mesh coordination — distributed locks, persistent op-log, A2A messages, heartbeats.
pub const SCHEMA_COORDINATION: &str = "
CREATE TABLE IF NOT EXISTS distributed_locks (
    lock_key      TEXT    NOT NULL,
    holder_node   TEXT    NOT NULL,
    holder_agent  TEXT    NOT NULL,
    fence_token   INTEGER NOT NULL,
    acquired_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    expires_at    TEXT    NOT NULL,
    repository_id TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (lock_key, repository_id)
);

CREATE INDEX IF NOT EXISTS idx_distributed_locks_expires ON distributed_locks(expires_at);

CREATE TABLE IF NOT EXISTS agent_oplog (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id         TEXT    NOT NULL,
    operation_id     TEXT    NOT NULL UNIQUE,
    kind             TEXT    NOT NULL,
    description      TEXT    NOT NULL,
    predecessor_hash TEXT,
    model_id         TEXT,
    change_id        INTEGER,
    timestamp_ms     INTEGER NOT NULL,
    undone           INTEGER NOT NULL DEFAULT 0,
    repository_id    TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_agent_oplog_agent ON agent_oplog(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_oplog_ts ON agent_oplog(timestamp_ms);
CREATE INDEX IF NOT EXISTS idx_agent_oplog_repo ON agent_oplog(repository_id);

CREATE TABLE IF NOT EXISTS a2a_messages (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    message_uuid   TEXT    NOT NULL UNIQUE,
    sender_agent   TEXT    NOT NULL,
    receiver_agent TEXT    NOT NULL,
    msg_type       TEXT    NOT NULL,
    payload        TEXT    NOT NULL,
    priority       INTEGER NOT NULL DEFAULT 1,
    thread_id      TEXT,
    acknowledged   INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT    NOT NULL DEFAULT (datetime('now')),
    repository_id  TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_a2a_receiver ON a2a_messages(receiver_agent);
CREATE INDEX IF NOT EXISTS idx_a2a_acknowledged ON a2a_messages(acknowledged);
CREATE INDEX IF NOT EXISTS idx_a2a_thread ON a2a_messages(thread_id);

CREATE TABLE IF NOT EXISTS mesh_heartbeats (
    node_id       TEXT    NOT NULL,
    agent_id      TEXT    NOT NULL,
    last_seen_ms  INTEGER NOT NULL,
    activity      TEXT    NOT NULL DEFAULT 'idle',
    repository_id TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (node_id, repository_id)
);

CREATE INDEX IF NOT EXISTS idx_mesh_heartbeats_seen ON mesh_heartbeats(last_seen_ms);
";


