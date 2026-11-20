---
phase: 537-llm-nli-claim-verifier-and-tier2-prose-heal
workstream: openwiki-self-healing-docs-ssot-v91
status: in_progress
---

# Phase 537 Progress Note (IN PROGRESS — not complete)

## Done and verified
- `crates/vox-doc-pipeline/src/claims/self_heal_bot.rs`: the always-`Verified`, doc-self-citing stub in `run_advisory_claim_verification` is replaced by `verify_claim_grounded` — deterministic, offline, evidence from code/contracts only:
  - prescriptive mention of a retired symbol → `Drifted`, cited to the matching line of `contracts/documentation/retired-symbols.v1.yaml`;
  - anchors that resolve to real files/crates → `Uncertain` with a `[path:L1-Lk]` citation to the real file (existence is proven; entailment is not, so it never says `Verified`);
  - missing/ambiguous anchors → `Uncertain`, no evidence.
- `symbol_span_hash` inputs are now real `hash_anchor` values (previously the placeholder `unindexed`).
- `llm_verifier.rs`: `is_doc_self_citation` + `enforce_citation_requirement` now downgrade any citation that points at the document being verified.
- Tests: `cargo test -p vox-doc-pipeline` → lib `155 passed; 0 failed`; `openwiki_e2e_integration` 5 passed; other integration suites passed.

## Update: entailment judge seam + CLI implementation
- `llm_verifier.rs`: `ClaimEntailmentJudge` trait, `EvidenceSpan`, `JudgeOutput` (no LLM dependency in `vox-doc-pipeline`).
- `self_heal_bot.rs`: `verify_claim_grounded_with` / `run_advisory_claim_verification_with_judge`; the judge runs only when every anchor resolved and real evidence exists; its verdict still passes `enforce_citation_requirement` (no doc-to-self).
- `vox-cli-research/src/audit_claim_judge.rs`: `LlmEntailmentJudge` over `vox_actor_runtime::llm::llm_chat` (model `CLASSIFIER_FALLBACK` on OpenRouter, 45 s timeout); wired into `vox audit docs --verify-llm`. Any failure → `None` → the deterministic verdict stands.
- Tests: `cargo test -p vox-doc-pipeline --lib claims` → `41 passed; 0 failed` (includes `judge_verdict_is_accepted_only_with_real_non_self_evidence`); `cargo test -p vox-cli-research --lib audit_claim_judge` → `2 passed; 0 failed`.
- NOT done: no live model call was made (no key/cost/data-egress decision taken); the judge's real-world precision is unmeasured; the evidence is only the first 25 lines of a cited file, so it can rarely support `verified`; the model is hardcoded to the OpenRouter classifier default rather than selected through the model registry (`AGENTS.md` expects registry-driven selection) — follow-up.
- `LLM-HEAL-02..04` still open.

## Update 2: benchmark v2, surgical Tier-2 fixer, first real maintenance pass

- **LLM-HEAL-02 (benchmark):** the v1 string-compare set was removed (`evaluate_golden_drift_set`, `verify_claim_against_anchor` deleted). New `claims/golden_mine.rs`: `mine_candidates` (from `git log -p -U0 --format=@@COMMIT%h -- docs/src`), reviewer-set `label`/`reviewed_by`, `evaluate_golden_set_v2` scoring with the real resolver; eligibility counts only `human:`-reviewed cases (>= 50). CLI: `vox audit docs --mine-golden-drift <patch> [--mine-out f]`, `--eval-golden-drift`. `run_self_heal_bot_check` now refuses unless the v2 benchmark licenses the bot.
- Mined 2,524 candidates from real doc-fix commits; I (agent) labelled 69 from a stratified sample into `contracts/reports/docs-freshness/golden-drift-eval-set.v2.json` (65 drift / 4 benign, all `reviewed_by: agent:claude-2026-11-19`). Result: precision 97.9%, recall 73.0%, fp-rate 1.3%, **promotion_eligible=false (0 human-reviewed; 50 required)**. The sample is skewed toward resolver-flagged lines (few benign cases), so these rates are optimistic. A human must review and extend it.
- Extractor hole closed: `is_placeholder_target` skipped any mention containing "fake"/"nonexistent" anywhere; now whole path segments only. Full `vox-doc-pipeline` suite: 159 passed; `vox audit docs --check-drift` still passes.
- **LLM-HEAL-03 (partial):** `synthesize_line_scoped_fix` (one token, one line, gated), `plan_retired_symbol_fixes`, `apply_retired_symbol_fixes`, `vox audit docs --heal-retired [--apply]`. Only exact entries of the crate rename table are auto-fixed (`GroundTruthRegistry::exact_retired_crate_replacement`); regex/prefix/command matches are reported, never rewritten (a dry run had proposed `vox-dei-d` -> `vox-orchestrator` and a CLI command -> a crate name; both would have been wrong). `is_autofix_eligible_path` refuses `adr/`, `history/`, `archive/`, the changelog and ISO-dated files (point-in-time records). Section/paragraph-scoped prose rewrites driven by NLI `Drifted` verdicts are NOT done.
- First real pass: applied 141 fixes, then restored 11 point-in-time files from `HEAD` (each had only my edits) and added the path policy. Remaining living-page edits: actionable deficits 1,076 -> 987, ratcheted 970 -> 875 after `--tighten-baseline`; full-corpus lint clean; `vox-arch-check` exit 0. Ledger refreshed: 1210 pages, 178 verified, 271 drifting, 193 broken (was 180 / 253 / 209): editing pages made some previously attested pages drift, so re-attestation of touched pages is still owed.
- NOT done: model selection for the judge still hardcodes the OpenRouter classifier default (not registry-driven); no real LLM call made; LLM-HEAL-04 gates unchanged and not extended; Phase 538 (CI workflows, PR blast radius, staleness/demotion) not started.

- `LLM-HEAL-01`: no entailment judge through `vox_actor_runtime::llm` (the verifier currently has no `Verified` path); `DocVerifyBudget` is only exercised with 0-token verdicts.
- Retired-symbol `Drifted` path has no dedicated test (the retired crate names in the contract could not be confirmed as unresolvable).
- `LLM-HEAL-02`: `evaluate_golden_drift_set` still compares strings and repeats cases in a loop; a real >= 50-pair on-disk benchmark needs new curated data.
- `LLM-HEAL-03`, `LLM-HEAL-04`: untouched.
- `vox-cli-research/src/audit.rs` (caller of `run_advisory_claim_verification`) was not re-run or re-checked after the behavior change.
