# Failure modes & mitigations (snapshot)

Evidence-backed themes driving UX + data-shape choices:

| Risk | Mitigation in app |
|------|-------------------|
| High dropout / friction | One-tap quick-add; sub-minute flows (Daylio-style evidence). |
| Gamification increases attrition (trial evidence) | No streaks/badges; informational summaries only. |
| Recall bias vs EMA | Prefer immediate logging; flag backfills (`recorded_at` vs `event_at` delta > 1h in future views). |
| Sleep duration mis-estimation vs timing accuracy | Derive duration from start/end events; never ask “hours slept” as free text. |
| Diet underreporting | No calories in v1 — capture description + meal timing. |
| Physicians lack time to parse raw PGHD | PDF summary dashboard + stable CSV/JSON (`contracts/export/`). |

See app-owned **`docs/how-to/clinical-export.md`** for export philosophy (monorepo pointer: `docs/src/how-to/clinical-export-from-vox-apps.md`).
