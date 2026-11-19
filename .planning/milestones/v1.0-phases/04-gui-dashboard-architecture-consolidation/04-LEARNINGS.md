---
phase: 4
phase_name: "gui-dashboard-architecture-consolidation"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 12
  lessons: 5
  patterns: 2
  surprises: 3
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 4 Learnings: gui-dashboard-architecture-consolidation

## Decisions

### Deliberately did NOT follow CONTEXT.md D-01's literal instruction to set...
Deliberately did NOT follow CONTEXT.md D-01's literal instruction to set ADR-045 frontmatter status to 'accepted' -- that value isn't in vox-doc-pipeline's VALID_STATUS enum and would be a hard lint failure. Used the body Status-line convention every other locked ADR in this repo (037, 041, 047) actually uses instead. Documented as a visible correction, not applied silently.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-01-SUMMARY.md

---

### Added ADR-045's body Status
Added ADR-045's body `**Status**: Accepted (2026-09-22)` line, matching ADR-037/041/047's existing convention, while leaving its frontmatter `status: "current"` untouched (required — see Decisions Made)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 04-01-SUMMARY.md

---

### Synced .planning/intel/classifications/045-*.json to locked
Synced `.planning/intel/classifications/045-*.json` to `locked: true`, `confidence: "high"`, removing the now-superseded `notes` key

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 04-01-SUMMARY.md

---

### Synced .planning/intel/decisions.md's ADR-045 block to status
Synced `.planning/intel/decisions.md`'s ADR-045 block to `status: locked` with a ratification note

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 04-01-SUMMARY.md

---

### Verified cargo run -p vox-doc-pipeline -- --lint-only passes on ADR-045 ...
Verified `cargo run -p vox-doc-pipeline -- --lint-only` passes on ADR-045 with the frontmatter unchanged

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 04-01-SUMMARY.md

---

### Appended one ## Consequences bullet to ADR-037 citing generate_tauri_wor...
Appended one `## Consequences` bullet to ADR-037 citing `generate_tauri_workspace`, `build_single_binary`/`build_tauri_app`, and layers.toml's three forbidden-pattern rules — all independently confirmed present in the tree before writing the citation

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 04-01-SUMMARY.md

---

### Added the 'crates/' prefix to one previously-bare 'vox-dashboard' refere...
Added the 'crates/' prefix to one previously-bare 'vox-dashboard' reference (Worked Examples section) when repointing it to vox-gui, for consistency with the other 3 references and to meet the acceptance criterion's literal 4-occurrence count -- a cosmetic normalization, not a prose-meaning change

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 04-02-SUMMARY.md

---

### Enumerated all 4 crates/vox-gui/src files referencing build_catalog() (c...
Enumerated all 4 `crates/vox-gui/src` files referencing `build_catalog()` (`catalog.rs`, `action_manifest.rs`, `discovery.rs`, `coderabbit.rs`) — one more than RESEARCH.md's stale 3-consumer count

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 04-02-SUMMARY.md

---

### Confirmed crates/vox-gui/ui/src's sole command-surface consumer (Catalog...
Confirmed `crates/vox-gui/ui/src`'s sole command-surface consumer (`Catalog.tsx`) has zero hardcoded fallback, sourcing entries entirely from the `get_command_catalog` Tauri invoke

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 04-02-SUMMARY.md

---

### Grounded the navigation.ts PARENT_CHILD_MAP non-violation verdict in Com...
Grounded the `navigation.ts` `PARENT_CHILD_MAP` non-violation verdict in `CommandCatalogEntry`'s actual field list (no section/parent/group field exists, so it couldn't be catalog-derived)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 04-02-SUMMARY.md

---

### Recorded the audit finding and the coderabbit.rs exemption as durable no...
Recorded the audit finding and the `coderabbit.rs` exemption as durable notes on `contracts/frontend/surface-ownership.v1.yaml`'s `vox-gui` entry — declined a permanent CI check per D-03 (would duplicate `gui_catalog_parity.rs` for a zero-instance violation class)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 04-02-SUMMARY.md

---

### Added authoring_track field (react-hand-authored / vox-native-interop-re...
Added `authoring_track` field (`react-hand-authored` / `vox-native-interop-reference`) to the `vox-gui` and `marquee-app` surface entries, plus a matching prose bullet in `external-frontend-interop-plan-2026.md`

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 04-02-SUMMARY.md

## Lessons

### None beyond the documented D-01 correction, which the plan itself specif...
None beyond the documented D-01 correction, which the plan itself specified in advance (see `<planner_correction_to_d01>` in 04-01-PLAN.md) — not an in-flight deviation.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-01-SUMMARY.md

---

### None of substance
None of substance. One verify command (`git diff --quiet -- crates/vox-gui`) initially reported "modified" due to three pre-existing, unrelated local changes in the shared working tree (`research.rs`, `search_probe.rs`, `research_doc_publish_test.rs` — present before this session started, part of separate in-progress work, not touched by this task). Confirmed via targeted `git diff --stat` on the 7 audit-relevant files specifically, which was empty — the task's actual constraint (no GUI source touched) held; the blanket check was a false positive from the shared dirty tree, not a real violation.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-02-SUMMARY.md

---

### None beyond the verify false-positive above.
None beyond the verify false-positive above.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 04-02-SUMMARY.md

---

### The ## Canonical Frontend section now states crates/vox-gui is the canon...
The `## Canonical Frontend` section now states `crates/vox-gui` is the canonical primary GUI and orchestration surface (ADR 045, ratified). The only remaining `vox-dashboard` mentions (lines 33, 43) are explicit retirement statements. File is clean vs commit e4afa966d (no uncommitted diff).

**Context:** Verified in phase acceptance checks.
**Source:** 04-VERIFICATION.md

---

### Every cited path exists
Every cited path exists: `crates/vox-gui/src/main.rs`, `crates/vox-gui/ui/src/App.tsx`, `crates/vox-gui/src/commands/`, `contracts/frontend/surface-ownership.v1.yaml`, and the three state_machine compiler files (`hir/nodes/state_machine.rs`, `typeck/state_machine_check.rs`, `vox-codegen-ts/src/state_machine_emit.rs`). `crates/vox-dashboard` and `scripts/check_dashboard_ssot.vox` do not exist, matching the page's statement that they are gone.

**Context:** Verified in phase acceptance checks.
**Source:** 04-VERIFICATION.md

## Patterns

### ADR ratification lives in a body Status
ADR ratification lives in a body `**Status**: Accepted (date)` line, never the frontmatter `status:` key -- vox-doc-pipeline's VALID_STATUS enum has no 'accepted' value

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 04-01-SUMMARY.md

---

### Per-surface authoring_track field in contracts/frontend/surface-ownershi...
Per-surface authoring_track field in contracts/frontend/surface-ownership.v1.yaml, paired with a prose bullet naming it -- two agreeing halves of one rule, not a new enforcement mechanism

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 04-02-SUMMARY.md

## Surprises

### Estimate variance on 04-01
Estimate variance on 04-01: planned 34000 tokens vs 3800 actual (0.11x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 04-01-PLAN.md, 04-01-SUMMARY.md

---

### None beyond the documented D-01 correction, which the plan itself specif...
None beyond the documented D-01 correction, which the plan itself specified in advance (see `<planner_correction_to_d01>` in 04-01-PLAN.md) — not an in-flight deviation.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 04-01-SUMMARY.md

---

### Estimate variance on 04-02
Estimate variance on 04-02: planned 46000 tokens vs 5200 actual (0.11x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 04-02-PLAN.md, 04-02-SUMMARY.md

