---
phase: "317"
slug: "size-ladder-size-01"
status: verified
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-10-28"
---

# Phase 317 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

ASVS L1. `workflow.security_block_on` is `high`. The plan-time register has four rows (T-317-01, T-317-02, T-317-03, T-317-SC). The security auditor checked the overlay, `resolve_qwen3_size`, `parse_overlay`, and `pick_base` on 2026-10-28 and closed all four. No implementation files were changed for this audit. `317-01-SUMMARY.md` has no Threat Flags section. The Rule 2 deviation names T-317-01 and T-317-03; both checks are in `resolve_qwen3_size_rejects_bad_revision_and_missing_license`.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| checked-in gpu-specs.yaml → pick_base and resolve_qwen3_size | A catalog edit can point training or the harness at a different commit, or can make an unmeasured rung look measured. | Hugging Face pin, revision, license, floor |
| size string from the compare CLI → resolve_base_pin | The operator-supplied name is untrusted as a selector; it must not fetch weights or fall through to a guessed floor. | Size spelling |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-317-01 | Tampering | mens/config/gpu-specs.yaml pins | high | mitigate | Catalog pins are `Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e` and `Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c` (`gpu-specs.yaml:252-253`). `resolve_qwen3_size_1_7b_pin_and_16g_pick_unchanged` and `resolve_qwen3_size_covers_the_dense_ladder` assert those 40-hex revisions (`spoke_base_resolver.rs:919-923`, `spoke_base_resolver.rs:1024-1038`). `is_40_lowercase_hex` (`spoke_base_resolver.rs:133-137`) rejects any other revision inside `resolve_qwen3_size` (`spoke_base_resolver.rs:175-185`); `resolve_qwen3_size_rejects_bad_revision_and_missing_license` covers `not-a-sha` and uppercase hex (`spoke_base_resolver.rs:962-983`). The size path returns `pin.hf_id` from the overlay (`detect.rs:239-242`) and does not call Hugging Face. | closed |
| T-317-02 | Denial | pick_base | medium | mitigate | `parse_overlay` rejects a measured row with no `floor_mb` and an unmeasured row that sets one (`spoke_base_resolver.rs:211-222`). `unmeasured_floor_and_missing_floor_do_not_load` asserts both failures (`spoke_base_resolver.rs:945-958`). `pick_base` keeps only `floor_measured` rows whose `floor_mb` fits (`spoke_base_resolver.rs:85`). `qwen3_code_fail_closed_below_floor` still errors at 1000 MB (`spoke_base_resolver.rs:526-539`). | closed |
| T-317-03 | Information disclosure | Qwen3SizePin.license | medium | mitigate | The new rows record `license: apache-2.0` and `attribution_required: false` (`gpu-specs.yaml:252-253`). `resolve_qwen3_size` returns that license and errors when it is absent (`spoke_base_resolver.rs:186-195`). Tests assert `apache-2.0` (`spoke_base_resolver.rs:924`, `spoke_base_resolver.rs:1036`) and `license: None` (`spoke_base_resolver.rs:984-993`). | closed |
| T-317-SC | Tampering | cargo/npm/pip installs | high | accept | Phase range `931ff26e2..3592ec1e6` touches five files and no `Cargo.toml`, lockfile, `package.json`, or `pyproject.toml`. No install step. Recorded as AR-317-SC. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `high` count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-317-SC | T-317-SC | This plan adds no package. No install step, so no assumed or suspicious package checkpoint. The phase diff adds no dependency manifest. | phase 317 plan | 2026-10-28 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-28 | 4 | 4 | 0 | security auditor (ASVS L1) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-28
