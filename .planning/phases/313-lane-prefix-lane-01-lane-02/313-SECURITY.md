---
phase: "313"
slug: "lane-prefix-lane-01-lane-02"
status: verified
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-10-26"
---

# Phase 313 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

ASVS L1. `workflow.security_block_on` is `high`. The plan-time register has no high or critical threats. Mitigations below were checked against the tests re-run on 2026-10-26 (all lib filters green). No implementation files were changed for this audit.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| HTTP client -> `do_generate` | Untrusted JSON `lane` string enters the serve process | Lane name, low sensitivity |
| host `inference_payload` -> plugin `PromptRequest` | Process-internal JSON; a dropped key would silently serve hub or agent as codegen | Lane name |
| HTTP client -> `do_chat_completions` | Untrusted `lane` and message text | Lane name and prompt text |
| training corpus rows -> fusion builders | Row content decides prefix shape; no network boundary | Training text |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-313-01 | Tampering | `Lane::parse` / `do_generate` | medium | mitigate | Closed-set exact match. Unknown lane returns JSON 400 before any worker send. Locked by `unknown_lane_on_generate_is_400_before_the_worker`. | closed |
| T-313-02 | Information disclosure / DoS | 400 error text | low | mitigate | Offending value is quoted and cut to 32 chars. Locked by `lane_parse_error_truncates_the_offending_value_on_a_char_boundary`. | closed |
| T-313-03 | Tampering (silent downgrade) | `inference_payload` | medium | mitigate | `"lane"` is always emitted from `Lane::as_str`. Locked by `the_payload_carries_the_lane` and the hub generate tracer. | closed |
| T-313-04 | Tampering | plugin `resolve_lane` | low | mitigate | Both plugins call `Lane::parse` and reject an unknown value before `assemble_prompt`. Locked by `resolve_lane_rejects_unknown_values` on CUDA. | closed |
| T-313-05 | Tampering | `assemble_prompt` pre-wrapped branch | low | mitigate | A prompt that already starts with `<|im_start|>` is returned unchanged for every lane. Locked by `assemble_prompt_keeps_prewrapped_chatml_for_every_lane` on CUDA and Metal. | closed |
| T-313-06 | Tampering | `flatten_chat_messages` last user turn | low | mitigate | Hub and agent strip `/no_think` from the last user turn only. Locked by `flatten_chat_messages_hub_strips_no_think_from_the_last_user_turn_only`. | closed |
| T-313-07 | Denial of service | `do_chat_completions` | low | mitigate | Chat lane is parsed before flatten and before the worker. Unknown lane returns JSON 400 with a zero receive count. `do_generate` also re-validates the forwarded lane. | closed |
| T-313-08 | Tampering (training data integrity) | `fuse_turns` / `chatml_prefix_open_assistant` | low | mitigate | Tests only. Empty think stays outside the loss; a real think body is not rewritten. Production builders were not edited by plan 313-04. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `high` count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

No accepted risks.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-26 | 8 | 8 | 0 | orchestrator (ASVS L1 short-circuit; tests re-run) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-26
