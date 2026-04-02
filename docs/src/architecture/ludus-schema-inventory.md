---
title: "Ludus / gamify schema inventory (SSOT pointers)"
description: "Index to vox-db gamification SQL, agents domain, ludus_schema_cutover, plus vox-ludus router/rewards/schema code and key tests—no duplicated schema text."
category: "architecture"
---

# Ludus / gamify schema inventory (SSOT pointers)

## Baseline (vox-db manifest)

- Core tables: [`crates/vox-db/src/schema/domains/sql/gamification.sql`](../../../crates/vox-db/src/schema/domains/sql/gamification.sql) (profiles, companions, quests, battles) plus coordination SQL in the same domain.
- Agents / events: [`crates/vox-db/src/schema/domains/agents.rs`](../../../crates/vox-db/src/schema/domains/agents.rs) (`agent_events`, `cost_records`, …).

## Post-baseline cutover (idempotent)

Extended Ludus tables and column fixes live in [`crates/vox-db/src/ludus_schema_cutover.rs`](../../../crates/vox-db/src/ludus_schema_cutover.rs), invoked from [`crates/vox-db/src/schema_cutover.rs`](../../../crates/vox-db/src/schema_cutover.rs) after baseline migrate.

Covers, among others:

- `gamify_teaching_profiles`, `gamify_policy_snapshots`, `gamify_ai_feedback`, `gamify_periodic_rewards`, `gamify_level_history`
- `gamify_counters` (**column `name`**, not `counter_name`)
- `gamify_collegium` (singular; legacy `gamify_collegiums` renamed when present)
- `gamify_arena_*`, `gamify_daily_counters`, `gamify_event_config`, `gamify_notifications`
- `gamify_hint_telemetry`, `gamify_processed_events` (orchestrator idempotency)
- Profile / quest / companion column alignment (`personality` on companions, streak/lumens on profiles, …)

## Application code

- Router + rewards: [`crates/vox-ludus/src/event_router.rs`](../../../crates/vox-ludus/src/event_router.rs), [`crates/vox-ludus/src/db/process_rewards.rs`](../../../crates/vox-ludus/src/db/process_rewards.rs)
- SQL reference ladder (documentation / partial migrations): [`crates/vox-ludus/src/schema.rs`](../../../crates/vox-ludus/src/schema.rs)

## Tests

- Cutover smoke: [`crates/vox-db/tests/ludus_schema_cutover_test.rs`](../../../crates/vox-db/tests/ludus_schema_cutover_test.rs)
- Policy / router: [`crates/vox-ludus/tests/gamify_integration_test.rs`](../../../crates/vox-ludus/tests/gamify_integration_test.rs)
