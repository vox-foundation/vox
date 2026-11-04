# 321-03 SUMMARY — Collateral / gate / serve (document-close)

**Plan:** 321-03  
**Completed:** 2026-10-29  
**Decision:** User option **1** — document & close; no fake collateral/serve pass.

## Outcomes

| Size | Collateral | failed_on | Gate receipt | Dir serve |
|------|------------|-----------|--------------|-----------|
| 0.6B | `fail` | gsm8k_think (~18.8% rel drop) | not written | refused |
| 1.7B | `fail` | vox_think (~9.5% rel drop) | not written | refused (same policy) |

**Serve smoke** (`evidence/serve-smoke.json`):

- Adapter **file** → refused (`resolve_serve_run_dir` / D-321-10)
- Run **dir** with fail collateral → refused (“cannot be served”)

## Deviation from plan verify

Plan verify required `collateral status == pass` and `dir_serve.passed == true`. Training quality did not clear the 5% bar. Per operator decision, HARN-01 is closed as **gate enforcement verified**, not as live serve success.

## Commits / artifacts

- `mens/runs/qwen3-*/collateral_damage_report.json` (status fail)
- `evidence/collateral-06b.json`, `evidence/collateral-17b.json`
- `evidence/serve-smoke.json`
