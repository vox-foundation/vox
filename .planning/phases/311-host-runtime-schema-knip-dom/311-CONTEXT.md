# Phase 311 Context: Host-Runtime Boundary, Schema Hoist, Knip Dual & DOM-Env Discipline

**Requirements:** `T3-HOST-SCHEMA-01..04`

**Intent:** New detector banning direct host platform/arch probes outside allowlisted host crates (`std::env::consts::{OS,ARCH}`, `process.platform`/`process.arch`) — t3code `no-global-process-runtime` analog.

**Sources:** see store `docs/t3-deeper-extraction-v5.5.md`.
