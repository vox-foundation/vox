//! Canonical developer journey rows (machine SSOT) — see `contracts/journeys/`.

pub const SCHEMA_DEVELOPER_JOURNEYS: &str = r#"
CREATE TABLE IF NOT EXISTS developer_journey_definitions (
    journey_id TEXT PRIMARY KEY,
    version INTEGER NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    definition_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS developer_journey_steps (
    journey_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    step_id TEXT NOT NULL,
    step_json TEXT NOT NULL,
    PRIMARY KEY (journey_id, ordinal)
);

CREATE INDEX IF NOT EXISTS idx_developer_journey_steps_journey ON developer_journey_steps(journey_id);

INSERT OR IGNORE INTO developer_journey_definitions (
    journey_id, version, title, description, definition_json
) VALUES (
    'canonical_journey.v1.greenfield_vox_mens_devloop',
    1,
    'Greenfield Vox + MENS dev loop',
    'Bootstrap repo → workspace store → author → plan → assist → research → corpus/train → verify.',
    '{"journey_id":"canonical_journey.v1.greenfield_vox_mens_devloop","version":1,"title":"Greenfield Vox + MENS dev loop","documentation_contract":"contracts/journeys/canonical-journey-definition.v1.schema.json"}'
);

INSERT OR IGNORE INTO developer_journey_steps (journey_id, ordinal, step_id, step_json) VALUES
('canonical_journey.v1.greenfield_vox_mens_devloop', 1, 'bootstrap_repo', '{"step_id":"bootstrap_repo","ordinal":1,"actor":"human","primary_operation_id":"vox","maturity":"stable","limitation_ids":["L-021"]}'),
('canonical_journey.v1.greenfield_vox_mens_devloop', 2, 'open_workspace_store', '{"step_id":"open_workspace_store","ordinal":2,"actor":"runtime","primary_operation_id":"orch.workspace_journey","maturity":"stable","limitation_ids":["L-021","L-023"]}'),
('canonical_journey.v1.greenfield_vox_mens_devloop', 3, 'author_compile', '{"step_id":"author_compile","ordinal":3,"actor":"human","primary_operation_id":"vox.build","maturity":"stable","limitation_ids":["L-001","L-003"]}'),
('canonical_journey.v1.greenfield_vox_mens_devloop', 4, 'structured_plan', '{"step_id":"structured_plan","ordinal":4,"actor":"human","primary_operation_id":"vox.mens.plan","maturity":"beta","limitation_ids":["L-010","L-014","L-015"]}'),
('canonical_journey.v1.greenfield_vox_mens_devloop', 5, 'agentic_assist', '{"step_id":"agentic_assist","ordinal":5,"actor":"mcp","primary_operation_id":"vox_mcp","maturity":"beta","limitation_ids":["L-022","L-025"]}'),
('canonical_journey.v1.greenfield_vox_mens_devloop', 6, 'research_evidence', '{"step_id":"research_evidence","ordinal":6,"actor":"human","primary_operation_id":"vox.research","maturity":"beta","limitation_ids":["L-016","L-017"]}'),
('canonical_journey.v1.greenfield_vox_mens_devloop', 7, 'corpus_train', '{"step_id":"corpus_train","ordinal":7,"actor":"human","primary_operation_id":"vox.mens.train","maturity":"beta","limitation_ids":["L-005","L-006","L-007"]}'),
('canonical_journey.v1.greenfield_vox_mens_devloop', 8, 'verify_ship', '{"step_id":"verify_ship","ordinal":8,"actor":"ci","primary_operation_id":"vox.ci","maturity":"stable","limitation_ids":["L-028","L-029"]}');
"#;
