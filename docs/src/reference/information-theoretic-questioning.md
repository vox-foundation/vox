---
title: "Information-theoretic questioning protocol"
description: "SSOT for when and how Vox asks clarifying questions with maximum diagnostic value per user effort."
category: "reference"
last_updated: 2026-03-27
training_eligible: true
---

# Information-theoretic questioning protocol

This document is the SSOT for clarification strategy across chat, planning, and agent-to-agent handoffs.

## Goals

- Minimize user effort while maximizing uncertainty reduction.
- Prefer high-diagnostic prompts over broad or redundant questions.
- Stop asking as soon as confidence and risk thresholds are met.
- Preserve auditability: each question has reason, expected gain, and stop rationale.

## Question trigger policy

Ask a question only when at least one of these conditions is true:

1. **Ambiguous intent**: multiple plausible actions exist with materially different outcomes.
2. **High consequence uncertainty**: action is costly, irreversible, or policy-sensitive.
3. **Missing hard constraint**: required parameter is absent (`target`, `scope`, `risk tolerance`, `deadline`, etc.).
4. **Socrates medium-risk band**: confidence is in the ask range and contradiction is non-blocking.

Do not ask when:

- the request is unambiguous and low risk,
- additional questions are expected to provide negligible information gain,
- maximum clarification turns or user-time budget is reached.

## Question type selection

Use the smallest interaction that resolves the highest-value uncertainty.

### Multiple-choice (`multiple_choice`)

Prefer when hypothesis space is known and bounded.

- Use 2-5 options (3 default).
- Options must be mutually exclusive when possible.
- Include a deliberate "other / none of the above" only when genuinely needed.
- Design unselected options to remain diagnostically useful (infer constraints/preferences).

### Open-ended (`open_ended`)

Prefer when user intent space is broad or unknown.

- Ask exactly one targeted free-form prompt.
- Include a short frame to reduce interpretation variance.
- Follow with one narrow multiple-choice if remaining ambiguity persists.

### Entry (`entry`)

Prefer for scalar/structured fields (IDs, ranges, dates, file paths, thresholds).

- Validate format immediately.
- Echo parsed value before execution.
- Re-ask only for invalid/unsafe values.

## Information-theoretic scoring

Each candidate question is scored by expected value:

`score = expected_information_gain_bits / expected_user_cost`

Where:

- `expected_information_gain_bits` is entropy reduction over active hypotheses.
- `expected_user_cost` approximates burden (time, complexity, interruption).

Choose the highest-scoring candidate that passes policy constraints:

- `expected_information_gain_bits >= min_information_gain_bits`
- `expected_user_cost <= max_expected_user_cost`
- `clarification_turn_index < max_clarification_turns`

## Stopping rules

Stop clarification when any condition is met:

1. `confidence >= target_confidence`
2. `marginal_information_gain_bits < min_information_gain_bits`
3. `clarification_turn_index >= max_clarification_turns`
4. `expected_user_cost > max_expected_user_cost`
5. contradiction/risk forces abstention or escalation

Persist stop reason explicitly for telemetry and audit.

## Attention and time-respect constraints

Questioning must be cost-aware with attention budget coupling:

- Penalize long clarification loops under high interrupt load.
- Raise gain threshold when attention budget is near exhaustion.
- Prefer concise multiple-choice in high temporal demand contexts.

MCP records estimated wall-time per `session_id` and can mirror those debits into the orchestrator global attention budget. Cap override and mirror toggle: **`VOX_QUESTIONING_MAX_ATTENTION_MS`**, **`VOX_QUESTIONING_MIRROR_GLOBAL_ATTENTION`** — see [Environment variables (SSOT)](env-vars.md#mcp-socrates-questioning).

## A2A clarification contract

For agent-to-agent clarification, persist these payload fields in `a2a_messages.payload`:

- `clarification_intent` (why clarification is needed),
- `hypothesis_set_id`,
- `question_kind`,
- `expected_information_gain_bits`,
- `expected_user_cost`,
- `requested_evidence_dimensions`,
- `urgency`,
- `stop_policy`.

Recommended `msg_type` values:

- `clarification_request`
- `clarification_response`
- `clarification_stop`

## Metrics (minimum set)

- Clarification trigger rate.
- Mean clarification turns per resolved task.
- Mean realized information gain per question.
- Gain-per-cost ratio.
- Multiple-choice option diagnostic power (selected + unselected).
- Clarification abandonment rate.
- Resolution latency after first clarification.
- A2A clarification round-trip latency.

## Persistence requirements

Policy and telemetry must be persisted in dual-write form:

1. Canonical publication artifact (`publication_manifests`).
2. Searchable mirror (`search_documents` + `search_document_chunks`).

Question-level runtime telemetry must be queryable in VoxDB via dedicated questioning tables.

**MCP (clients and agents):** `vox_questioning_pending` returns open sessions, unanswered assistant prompts, and structured multiple-choice options (plus parsed `belief_state_json`). `vox_questioning_submit_answer` persists free-text and optional `selected_option_id` (posteriors in `belief_state_json` and `question_options.posterior_probability` are updated for MC). Env vars for attention caps, global budget mirroring, and task-gate bypass are listed under [MCP / Socrates questioning](env-vars.md#mcp-socrates-questioning) in `env-vars.md`.

## Related SSOTs

- `docs/src/reference/socrates-protocol.md`
- `docs/src/reference/scientia-publication-worthiness-rules.md`
- `docs/src/reference/orchestration-unified.md`
