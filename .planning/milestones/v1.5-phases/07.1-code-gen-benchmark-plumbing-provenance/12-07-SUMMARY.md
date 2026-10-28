---
phase: 12-code-gen-benchmark-plumbing-provenance
plan: 07
subsystem: testing
tags: [eval-corpus, provenance, completeness-gate, adapter-card, vox-crypto, scoreboard]
requires:
  - phase: 12-02
    provides: draw_samples / SampleRecord with per-sample provider_model and seed
  - phase: 12-04
    provides: EvalCorpusReport::assemble, RunProvenance, the single completeness gate
  - phase: 12-06
    provides: adapter_card::check_for_eval, grammar_fingerprint()
provides:
  - "eval-corpus writes a schema-2 typed report through EvalCorpusReport::assemble (the JSON literal is gone)"
  - "collect_provenance / finalize_report / ProvenanceInputs / ReportMeta, resolve_runner, tier_label, --runner"
  - "problem_set_hash (SHA-256, framed), prompt_template_hash, context_hash_blake3, ConfigKnobs + config_digest on vox-crypto"
  - "VerifierIdentity, parse_vox_version_line, verifier_identity"
  - "mens_lineage (wraps check_for_eval), should_write_back"
  - "Fixture.spec_path / reference_path"
affects: [12-08, 12-09]
status: complete
actuals:
  tokens: 9100
  tasks: 3
  commits: 3
plan_head_before: 1033175507525b734a612f95e1bf359a8936c63a
tech-stack:
  added: []
  patterns:
    - "provenance fields that cannot be observed stay None and are named by the one gate, never defaulted"
    - "hashing only through vox_crypto::facades: interop_sha256 for externally checkable values, secure_hash (BLAKE3) for internal digests"
key-files:
  created: []
  modified:
    - crates/vox-cli/src/commands/model/eval_corpus.rs
    - crates/vox-corpus/src/humaneval_runner/manifest.rs
key-decisions:
  - "verifier_identity is all-or-nothing: if `--version` or the binary read fails, vox_git_sha, vox_binary_sha256, verifier_version and verifier_matches_harness are all reported missing"
  - "config_digest takes a ConfigKnobs struct instead of eleven positional arguments (transposition-prone strings)"
  - "from-dir rows get sampling n=1, k=1, seed 0 and no provider, so they are honestly incomplete and never write the scoreboard"
requirements-completed: [BENCH-05]
---

# Phase 12 Plan 07: Provenance, typed report and mens refusal in eval-corpus Summary

`vox model eval-corpus` now collects the section 5 provenance record, writes the typed schema-2 report through the 12-04 completeness gate, refuses unlabeled `mens/*` rows through 12-06's card check, and only lets complete generated runs write the routing scoreboard.

## Accomplishments

- **Tracer (d8876169e):** `run()` builds `RunProvenance` with `collect_provenance`, hands it and the `DrawOutcome` to `finalize_report` (the only caller of `EvalCorpusReport::assemble`), prints pass@1/pass@k with CIs, `seed N (enforced)` or `(requested; provider may ignore)`, and each incomplete reason. `seed_enforced` is true only for `voxlocal`. `provider_reported_models` is the sorted, de-duplicated set of per-sample `provider_model` (12-02 handoff). New `--runner` (defaults to `github-hosted` under `GITHUB_ACTIONS=true`). `identity` and `preregistration_id` stay `None`, so every report is honestly incomplete and names both until 12-09.
- **Identity collectors (5c150e8c6):** `problem_set_hash` is SHA-256 via `interop_sha256` over id-sorted fixtures, each field a u64-LE length plus raw bytes of id, spec.toml, reference.vox, tests.vox (order-independent, duplicate id is an error, `("ab","c")` vs `("a","bc")` cannot collide). `config_digest` is BLAKE3 over model id, harness, n, k, condition, context hash, temperature, top_p, max_tokens, seed and problem set hash (12-02 handoff: seed and top_p now included); `sha2` is no longer used in eval_corpus.rs. Verifier identity comes from `<vox_bin> --version` plus SHA-256 of the binary; `verifier_matches_harness = version == VOX_VERSION`. Syntax fingerprint and kind come from `grammar_fingerprint()`. `Fixture` gained `spec_path` and `reference_path`; `files.reference` is now required.
- **Card check and write-back guard (15a70e49a):** `mens_lineage` wraps `check_for_eval`; problems print as a red "mens label refused:" list and leave `MensLineage` unset, so the gate marks the run incomplete (no mens special case in scoring). Syntax drift is printed. `should_write_back(run_complete, measured, no_write_back)` wraps the scoreboard upsert; skipped runs print `model_scoreboard not updated: run incomplete` (or the ingested reason).

## Verification

- `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::)'`: 34 passed (19 pre-existing from 12-02 minus the replaced digest test, plus 5 tracer, 6 identity and 4 card/write-back tests; per-task runs: Task 1 5 passed).
- `cargo nextest run -p vox-corpus --lib -E 'test(manifest::)'`: 5 passed (2 new).
- `cargo check -p vox-cli -p vox-corpus --all-targets`: exit 0.
- Acceptance greps: `EvalCorpusReport::assemble(` 1; non-comment `"schema_version": 1` 0; non-comment `sha2::` 0; `interop_sha256(` 3; `pub reference_path: PathBuf` 1; `check_for_eval(` 1; `pub fn should_write_back` 1.

### Mutation proof (T-12-07-01, write-back guard)

| Step | `grep -c 'run_complete && measured'` | `write_back_requires_complete_measured_runs` |
|---|---|---|
| before | 1 | pass |
| mutated (`run_complete &&` dropped) | 0 | FAIL (`incomplete never steers routing`) |
| restored (byte-identical, shasum matched) | 1 | pass (34/34 in module) |

## Deviations from Plan

**1. [Test-first] Tests were written together with the code and observed green, not red.** The new tests need the new types to compile, so red was a compile error. The write-back mutation proof supplies the failing-test evidence for the guard; the other guards (duplicate id, boundary collision, byte sensitivity, drift) assert both directions in one test.

**2. [Structure] `config_digest(&ConfigKnobs)` instead of positional arguments.** Same inputs as the plan lists; a struct avoids eleven transposable arguments. The old `config_digest_is_stable_and_covers_every_knob` test was replaced by `config_digest_covers_every_knob`.

**3. [Structure] `cargo_version` is `env!("CARGO_PKG_VERSION")` (the vox-cli package version), as the plan specifies**, not the output of `cargo --version`. The design doc's wording for the Language group says "Cargo version"; whether that means the toolchain is unresolved and left for the reviewer.

**4. [Scope] `--from-dir` model ids starting with `mens/` also go through the card check** (the lineage lookup is keyed on the id, not on measured mode). A from-dir row is incomplete regardless (no provider), so this only adds the refusal message.

## Known Stubs

None. `identity` and `preregistration_id` are deliberately `None` (12-09 owns them); the report names both as missing.

## Skipped checks

- No clippy run (lean verification). Pre-commit hooks (fmt-fix, tdd-guard, command-sync, ci-status) ran on all three commits and passed.
- No live end-to-end run: no served MENS checkpoint and no OpenRouter key here. The verifier-identity unit test uses a temp shell script standing in for `vox`.
- BENCH-03 and BENCH-04 deliberately not marked complete (12-08 and 12-09 still feed them).
- `vox-orchestrator-mcp` lib-test build failure noted in the dispatch was not touched.

## Threat Flags

None. T-12-07-01 (mutation-proven), -02 (card check), -03 (binary hash, version match), -04 (`report_json_has_no_credentials`), -05 (no `sha2` import) are mitigated as planned.

## Self-Check: PASSED

- Files: both `files_modified` paths exist and are in the commits.
- Commits present: d8876169e, 5c150e8c6, 15a70e49a.
