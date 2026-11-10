---
phase: "339"
slug: "model-cards-v2"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-06"
---

# Phase 339 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: 339-RESEARCH.md §Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`; nextest in hosted CI with the same filters) |
| **Config file** | none; goldens regenerate with `VOX_UPDATE_GOLDEN=1` |
| **Quick run command** | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card` |
| **Full suite command** | `cargo test -p vox-populi --features mens-hf-hub --lib` (per-crate only, never workspace-wide) |
| **Docs lint** | `cargo run -p vox-doc-pipeline -- --lint-only --paths reference/mens-models.md` (path relative to `docs/src`) |
| **Diagram check** | `cargo run -p vox-cli -- --json diagram check -` (the installed 0.6.0 `vox` has no `diagram` subcommand) |
| **Edge check** | `vox ci crate-edges` |
| **Estimated runtime** | ~60 s per filtered vox-populi run after a warm build |

Run cargo in the background, one crate at a time (a GPU eval may be running; never block the foreground on cargo).

---

## Sampling Rate

- **After every task commit:** the new module's filter, e.g. `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::examples`
- **After every plan wave:** `cargo test -p vox-populi --features mens-hf-hub --lib hub_card`, plus `cargo check -p vox-ml-cli --features mens-hf-hub` when `hub_sync` or a CLI flag changed
- **Before `/gsd-verify-work`:** full vox-populi `--lib` for `mens-hf-hub`, `vox ci crate-edges`, docs lint, diagram check, all green
- **Max feedback latency:** ~120 s

---

## Per-Task Verification Map

Reconciled 2026-11-06 with the final 10 plans' `<automated>` blocks. Requirement → test map:

| Requirement | Behavior | Test Type | Automated Command | File Exists | Status |
|-------------|----------|-----------|-------------------|-------------|--------|
| CARD-01 | gate state per fixture: passed / needs re-gate / failed / not evaluated / not trained; stand-in refusals | unit | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::facts` and `... hub_card::collateral_standin` | ❌ W0 | ⬜ pending |
| CARD-02 | `RunRecords::load` behaviour and `render_*` signatures preserved; hub_sync compiles unchanged | unit + compile | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card` and `cargo check -p vox-populi --features mens-hf-hub` | port | ⬜ pending |
| CARD-03 | D2 pick, preview join, ambiguous → "no recorded output", verdicts, no-loss case | unit | `... --lib hub_card::examples` | ❌ W0 | ⬜ pending |
| CARD-04 | askama templates render unescaped, deterministic, host-free | unit | `... --lib vox_symbols_are_not_escaped` (with the `hub_card` sweep) | ❌ W0 | ⬜ pending |
| CARD-05 | hub-cards.yaml validated on load | unit | `... --lib hub_card::config` | ❌ W0 | ⬜ pending |
| CARD-06/11 | goldens: 5 states + with/without examples + index | golden | `... --lib hub_card::tests::model_card` | ❌ W0 | ⬜ pending |
| CARD-07 | every `vox` block on every card passes the compiler front end | lint | `... --lib hub_card::lints::vox_blocks_compile` | ❌ W0 | ⬜ pending |
| CARD-08 | mens-models.md lint clean; diagram valid | lint | docs lint + diagram check above | ❌ W0 | ⬜ pending |
| CARD-09 | index grid, which-model rule, what's-next from records | unit/golden | `... --lib hub_card::tests::index_card` | ❌ W0 | ⬜ pending |
| CARD-10 | duplication, anchors, HF metadata (local fixture) | lint | `... --lib hub_card::lints` | ❌ W0 | ⬜ pending |
| CARD-12 | card-only staging rewrites manifest entries, skips not-trained/32B; never force-ungated | unit + checkpoint | `... --lib hub_cards_push` and `cargo test -p vox-ml-cli --features gpu --lib hub::tests::adapter_serving_matches_eval_identity` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Mutation proofs (presence grep before, break, named test FAILS, restore, grep + test green again)

1. Stand-in returns `Ok` for a pass without pins → `stale_pass_is_needs_re_gate` fails (spec §7).
2. Loss example always `None` → `loss_example_present_or_stated_none` fails (spec §7).
3. Gate label rendered twice → `gate_state_stated_once` and the duplication lint fail (spec §7).
4. D2 sort reversed → `wins_are_the_two_shortest_by_length_then_id` fails.
5. Ambiguous preview join accepted → `ambiguous_join_renders_no_recorded_output` fails.
6. Template escaping switched to html → `vox_symbols_are_not_escaped` fails.
7. Unknown-size check removed → `unknown_size_is_an_error` fails.
8. Docs heading renamed in fixture → `every_card_anchor_exists_in_mens_models` fails.
9. Unparseable body in a `vox` fence → `vox_blocks_compile` fails.
10. Card-only staging skips the README hash rewrite → `card_only_manifest_matches_staged_files` fails.
11. Collateral numbers shown on a `needs re-gate` card → `no_stale_numbers_without_current_rule` fails.
13. A checkpoint header without `adapter_serving` accepted as current → `adapter_evidence_requires_current_serving` fails (339-03).
14. Card-side serving id drifts from vox-ml-cli `ADAPTER_SERVING` (committed 8589be99c) → `adapter_serving_matches_eval_identity` fails (339-04).

---

## Wave 0 Requirements

- [ ] `crates/vox-populi/tests/fixtures/hub_cards/` fixture run dirs (state matrix + real-schema ckpt/report sample) — `mens/runs/` is gitignored
- [ ] `crates/vox-populi/tests/fixtures/hub_cards/hf-metadata-rules.v1.yaml`
- [ ] askama tracer: one template + one test proves the dependency compiles on the pinned toolchain
- [ ] `verify_vox_admission` probe over the fixtures' adapter wins
- [ ] failing tests written first for each CARD-ID

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Side-by-side review of every rendered card + index | CARD-12 | human judgement of prose and honesty | open `mens/runs/hub_cards/<repo>/README.md` next to each other; approve or list fixes |
| Docs page deployed before any Hub push | CARD-08/12 | external deploy | confirm `voxlang.org/reference/mens-models/` resolves with every anchor |
| Hub push result | CARD-12 | network, credentials | after approval, run the sync; confirm each repo's README updated, weights untouched except the gated-current size |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120 s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
