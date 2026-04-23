//! Arca SQL: Automated research discovery, manuscript lifecycle, and scholarly publication.
pub const SCHEMA_SCIENTIA: &str = r#"
-- Canonical record of a discovered insight before manuscript generation.
CREATE TABLE IF NOT EXISTS scientia_discoveries (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    discovery_id      TEXT    NOT NULL UNIQUE,
    session_key       TEXT    NOT NULL,
    repository_id     TEXT    NOT NULL DEFAULT '',
    title             TEXT    NOT NULL,
    summary           TEXT    NOT NULL,
    claims_json       TEXT    NOT NULL,                      -- structured ClaimRecord[]
    evidence_json     TEXT    NOT NULL,                    -- citation/evidence packets
    novelty_score     REAL    NOT NULL DEFAULT 0.0,        -- RAG similarity vs. corpus
    confidence_score  REAL    NOT NULL DEFAULT 0.0,        -- Socrates confidence_at_stop
    human_gate_status TEXT    NOT NULL DEFAULT 'pending',  -- pending|approved|rejected
    human_gate_reason TEXT,
    publication_id    TEXT,                                -- FK to publication_manifests
    created_at_ms     INTEGER NOT NULL,
    updated_at_ms     INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_scientia_discoveries_session 
    ON scientia_discoveries(session_key, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_scientia_discoveries_status 
    ON scientia_discoveries(human_gate_status);

-- Structured citation tracking aligned with discovery claims.
CREATE TABLE IF NOT EXISTS scientia_citations (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    discovery_id      TEXT    NOT NULL,
    citation_key      TEXT    NOT NULL,
    source_type       TEXT    NOT NULL,          -- 'knowledge_node', 'external_url', 'snippet', 'eval_run'
    source_ref        TEXT    NOT NULL,
    title             TEXT,
    authors_json      TEXT,
    year              INTEGER,
    doi               TEXT,
    url               TEXT,
    created_at_ms     INTEGER NOT NULL,
    UNIQUE(discovery_id, citation_key)
);

-- Orchestration queue for the multi-step publication flow.
CREATE TABLE IF NOT EXISTS scientia_publication_queue (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    discovery_id          TEXT    NOT NULL UNIQUE,
    publication_id        TEXT    NOT NULL,
    stage                 TEXT    NOT NULL DEFAULT 'draft',   
    -- stages: draft | doi_reserved | orcid_attributed | approved | submitted | published | failed
    zenodo_deposition_id  TEXT,
    prereserved_doi       TEXT,
    last_error            TEXT,
    attempt_count         INTEGER NOT NULL DEFAULT 0,
    next_retry_at_ms      INTEGER,
    created_at_ms         INTEGER NOT NULL,
    updated_at_ms         INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_scientia_queue_stage 
    ON scientia_publication_queue(stage, next_retry_at_ms);

-- High-performance append-only telemetry projection for dashboards and agent self-awareness.
-- Aggregates execution, cost, a2a, and trust observations into a single sequential table.
CREATE TABLE IF NOT EXISTS agent_telemetry_flat (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id          TEXT    NOT NULL,
    session_id        TEXT    NOT NULL,
    repository_id     TEXT    NOT NULL DEFAULT '',
    event_kind        TEXT    NOT NULL,           -- 'exec', 'cost', 'trust_obs', 'a2a', 'budget_alert'
    tool_name         TEXT,
    model_id          TEXT,
    provider          TEXT,
    duration_ms       INTEGER,
    input_tokens      INTEGER,
    output_tokens     INTEGER,
    cost_usd          REAL,
    trust_score       REAL,
    payload_json      TEXT,                  -- narrow, non-PII subset
    recorded_at_ms    INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agt_tel_flat_agent_ev 
    ON agent_telemetry_flat(agent_id, event_kind, recorded_at_ms);
CREATE INDEX IF NOT EXISTS idx_agt_tel_flat_session 
    ON agent_telemetry_flat(session_id, recorded_at_ms);
CREATE INDEX IF NOT EXISTS idx_agt_tel_flat_repo 
    ON agent_telemetry_flat(repository_id, recorded_at_ms);

-- Consolidated reliability scores for all entities (agents, skills, workflows, repositories).
-- Replaces agent_reliability, skill_reliability, workflow_reliability, repository_reliability.
CREATE TABLE IF NOT EXISTS reliability_scores (
    entity_type      TEXT    NOT NULL,
    entity_id        TEXT    NOT NULL,
    reliability      REAL    NOT NULL DEFAULT 0.5,
    success_count    INTEGER NOT NULL DEFAULT 0,
    failure_count    INTEGER NOT NULL DEFAULT 0,
    updated_at_ms    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_reliability_scores_val 
    ON reliability_scores(reliability);

-- Quantitative evaluation of autonomous research quality (localized vs. Tavily).
CREATE TABLE IF NOT EXISTS research_eval_runs (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id            TEXT    NOT NULL UNIQUE,
    model_id          TEXT    NOT NULL,
    config_json       TEXT    NOT NULL,           -- search policy, depth, backends
    metrics_json      TEXT    NOT NULL,           -- RAGAS rollup (recall, groundedness, quality)
    latency_p50_ms    INTEGER,
    latency_p99_ms    INTEGER,
    tier_distribution_json TEXT,                    -- Tier 1/2/3/4 % breakdown
    created_at_ms     INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS research_eval_samples (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id            TEXT    NOT NULL,
    query             TEXT    NOT NULL,
    gold_answer       TEXT,                         -- optional reference
    model_answer      TEXT    NOT NULL,
    recall_at_5       REAL,
    groundedness      REAL,
    quality_score     REAL,                         -- search_execution quality rollup
    latency_ms        INTEGER,
    evidence_json     TEXT,                         -- array of [url, score, engine, snippet]
    recorded_at_ms    INTEGER NOT NULL,
    FOREIGN KEY(run_id) REFERENCES research_eval_runs(run_id)
);

CREATE INDEX IF NOT EXISTS idx_res_eval_samples_run 
    ON research_eval_samples(run_id);

-- Telemetry: Scoreboard for intelligent routing based on historical execution outcomes.
CREATE TABLE IF NOT EXISTS model_scoreboard (
    model_id              TEXT    NOT NULL,
    task_category         TEXT    NOT NULL,
    strength_tag          TEXT    NOT NULL,
    window_days           INTEGER NOT NULL,
    n_calls               INTEGER NOT NULL DEFAULT 0,
    success_rate          REAL    NOT NULL DEFAULT 0.0,
    p50_latency_ms        INTEGER,
    p99_latency_ms        INTEGER,
    cost_per_success_usd  REAL,
    quality_score         REAL    NOT NULL DEFAULT 1.0,
    updated_at_ms         INTEGER NOT NULL,
    PRIMARY KEY (model_id, task_category, strength_tag, window_days)
);

CREATE INDEX IF NOT EXISTS idx_model_scoreboard_task 
    ON model_scoreboard(task_category, strength_tag, success_rate);

-- Observed pricing SSOT updated from llm_interactions
CREATE TABLE IF NOT EXISTS model_pricing_catalog (
    model_id                   TEXT    NOT NULL,
    provider                   TEXT    NOT NULL,
    observed_blended_per_1k    REAL,
    observed_input_per_1k      REAL,
    observed_output_per_1k     REAL,
    catalog_input_per_1k       REAL    NOT NULL DEFAULT 0.0,
    catalog_output_per_1k      REAL    NOT NULL DEFAULT 0.0,
    n_provider_reported        INTEGER NOT NULL DEFAULT 0,
    n_estimated                INTEGER NOT NULL DEFAULT 0,
    n_free                     INTEGER NOT NULL DEFAULT 0,
    confidence                 TEXT    NOT NULL DEFAULT 'low',
    last_observed_at_ms        INTEGER,
    updated_at_ms              INTEGER NOT NULL,
    PRIMARY KEY (model_id, provider)
);

CREATE INDEX IF NOT EXISTS idx_model_pricing_catalog_model
    ON model_pricing_catalog(model_id, confidence);
"#;
