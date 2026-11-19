---
phase: 330
phase_name: "db-backend-interop-dbi-01-03"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 9
  lessons: 4
  patterns: 4
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 330 Learnings: db-backend-interop-dbi-01-03

## Decisions

### Unmapped SQL types render as str with a // vox
Unmapped SQL types render as str with a // vox: unmapped sql type comment; primary keys are never Option

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 330-01-SUMMARY.md

---

### Rendered tables are table(extern, source, pk) so verify's live name is t...
Rendered tables are table(extern, source, pk) so verify's live name is the SQL table name

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 330-01-SUMMARY.md

---

### OpenAPI for Python clients is dist/openapi.json or vox emit openapi, not...
OpenAPI for Python clients is dist/openapi.json or vox emit openapi, not vox build --target server

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 330-01-SUMMARY.md

---

### schema_owner is checked after the plan is printed and before either appl...
schema_owner is checked after the plan is printed and before either apply path

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 330-01-SUMMARY.md

---

### sql_type_to_vox is the inverse of vox_type_to_sql, and IntrospectedSchem...
`sql_type_to_vox` is the inverse of `vox_type_to_sql`, and `IntrospectedSchema::to_vox_source` emits `table(extern, source, pk)` declarations.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 330-01-SUMMARY.md

---

### A sqlite fixture round-trips through the Vox parser and compare_tables (...
A sqlite fixture round-trips through the Vox parser and `compare_tables` (the same comparison `vox db verify` uses) with no findings.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 330-01-SUMMARY.md

---

### vox db introspect --emit json|vox defaults to JSON
`vox db introspect --emit json|vox` defaults to JSON. Pretty and compact JSON stay byte-identical to the previous `println!` output.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 330-01-SUMMARY.md

---

### [db] schema_owner = "external" makes vox db migrate print the plan and r...
`[db] schema_owner = "external"` makes `vox db migrate` print the plan and refuse to apply it. Absent `[db]` stays `vox`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 330-01-SUMMARY.md

---

### docs/src/how-to/python-client-from-vox-openapi.md tells Python callers t...
`docs/src/how-to/python-client-from-vox-openapi.md` tells Python callers to use `dist/openapi.json` or `vox emit openapi`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 330-01-SUMMARY.md

## Lessons

### cargo test -p vox-cli without --lib launches every integration binary
`cargo test -p vox-cli` without `--lib` launches every integration binary. One of those runs was killed by the output cap (exit 137) after the library round-trip had already passed. Later runs used `--lib`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 330-01-SUMMARY.md

---

### DbCli is a clap subcommand, not a Parser
`DbCli` is a clap subcommand, not a `Parser`. The emit-flag tests parse `VoxCliRoot`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 330-01-SUMMARY.md

---

### Plan frontmatter declares no must_haves.key_links (verify.key-links exit...
Plan frontmatter declares no `must_haves.key_links` (`verify.key-links` exit 66).

**Context:** Verified in phase acceptance checks.
**Source:** 330-VERIFICATION.md

---

### N/A — Infrastructure/foundation phase with no user-facing elements
N/A — Infrastructure/foundation phase with no user-facing elements. All acceptance criteria are verifiable programmatically.

**Context:** Verified in phase acceptance checks.
**Source:** 330-VERIFICATION.md

## Patterns

### Pattern: sql_type_to_vox is the inverse of vox_type_to_sql in the same f...
Pattern: sql_type_to_vox is the inverse of vox_type_to_sql in the same file

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 330-01-SUMMARY.md

---

### Pattern: [db] schema_owner = external blocks migrate apply and still all...
Pattern: [db] schema_owner = external blocks migrate apply and still allows --dry-run

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 330-01-SUMMARY.md

---

### SQL→Vox scalars are derived from sql_for_scalar, the same table vox db v...
SQL→Vox scalars are derived from sql_for_scalar, the same table vox db verify uses

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 330-01-SUMMARY.md

---

### External schema ownership refuses migrate apply after the plan is printe...
External schema ownership refuses migrate apply after the plan is printed

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 330-01-SUMMARY.md

## Surprises

