---
phase: 339-model-cards-v2
plan: "04"
subsystem: mens-hub-cards
tags: [mens, hub-push, cards-only, manifest]
requires: ["339-01", "339-03"]
provides:
  - "vox_populi::mens::hub_cards_push (CARD_FILES, CardPushPlan, plan_card_push, card_only_manifest, stage_card_only, decide_row, sync_cards_only)"
  - "vox mens hub sync --cards-only (dry run without --push)"
  - "adapter_serving_matches_eval_identity parity test (feature gpu)"
affects: [339-08, 339-10]
key-files:
  created:
    - crates/vox-populi/src/mens/hub_cards_push.rs
    - crates/vox-populi/src/mens/hub_cards_push_tests.rs
  modified:
    - crates/vox-populi/src/mens/mod.rs
    - crates/vox-populi/Cargo.toml
    - crates/vox-ml-cli/src/commands/mens/hub.rs
key-decisions:
  - "The rewritten manifest takes gate_status from the local inspection stage (stage_publish_bundle(.., true, ..)), so an otherwise unchanged size classifies InSync after the push (orchestrator decision 6)"
  - "Each size's status is re-classified from a freshly fetched remote manifest instead of trusting the first sync_all_sizes row, and a fetch error skips that size rather than uploading without a manifest rewrite"
requirements-completed: [CARD-12]
status: complete
commits: 4
plan_head_before: c4b689363
plan_head_after: 8fa02026a
actuals:
  tokens: 16000
  tasks: 3
  commits: 4
completed: 2026-11-06
---

# Phase 339 Plan 04: Card-only Hub push and serving parity Summary

`vox mens hub sync --cards-only [--push]` uploads only README.md, MODEL_CARD.md and a rewritten `hf_sync_manifest.json` for each trained size whose gate is not current, then refreshes the index; no weights, no tag, no ungated path. Nothing was pushed or contacted on the Hub: every test is offline.

## Commits

- `7e6b4353f` feat(339-04): add the card-only Hub push (Tasks 1 and 2: both live in `hub_cards_push.rs`)
- `af2a020d4` feat(339-04): add `--cards-only` to `vox mens hub sync` and pin the serving identity (Task 3)
- `2beb50f1b` fix(339-04): stage card-only uploads in random 0700 temp dirs (coordinator security review, see Deviations)
- `8fa02026a` fix(339-04): inline the ACTION header literal (clippy `print_literal` on the moved table header)

`commits: 4` counts this plan's commits. `git rev-list --count c4b689363..2beb50f1b` prints 5 because 339-05 (`799b4e5db`) ran in the same working tree and committed between mine; `plan_head_after` is my last commit and the range up to it contains that one foreign commit.

## A5 evidence (hf-hub folder upload deletes nothing by default)

`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hf-hub-1.0.0/src/repository/upload.rs`, `upload_folder_impl` (lines 261-300). Operations are the files collected under `folder_path`; deletes are pushed only inside

```rust
if let Some(ref delete_patterns) = params.delete_patterns {   // line 276
```

and `upload_model_folder` (`crates/vox-populi/src/mens/hub.rs:407-456`) builds the call with `.folder_path(..).commit_message(..)` only, so `delete_patterns` is `None`. A card-only upload therefore adds or overwrites the staged paths and removes nothing.

## Verification

| Run | Result |
|---|---|
| `cargo test -p vox-populi --features mens-hf-hub --lib hub_cards_push` (final) | 9 passed, 0 failed |
| `cargo test -p vox-ml-cli --features mens-hf-hub --lib hub::tests` | 6 passed, 0 failed (includes `cards_only_flag_parses`, `cards_only_refuses_pull_and_force_ungated`) |
| `cargo test -p vox-ml-cli --features gpu --lib hub::tests` (final, after mutation restore) | 7 passed, 0 failed (adds `adapter_serving_matches_eval_identity`) |

Required tests present by name: `card_only_push_leaves_size_in_sync`, `card_only_manifest_matches_staged_files`, `card_only_stage_dir_holds_only_card_files`, `plan_card_push_cases`, `decide_rows_dry_run`, `gated_current_is_left_to_the_weights_path`, `not_trained_row_is_never_uploaded`. Extra: `staged_card_that_disagrees_with_the_manifest_is_refused`, `scratch_dirs_are_private_distinct_and_leave_predictable_paths_alone`.

Acceptance greps: `pub mod hub_cards_push` once in `mens/mod.rs`; `hub_sync.rs` unchanged (`git diff HEAD` empty); `hash_fast_hex` 1, `blake3::` 0; no `publish_adapter_run_dir(`, `create_repo_tag(`, `pull_adapter_if_missing(` or `force_ungated` outside comments (0 each); `require_clavis_hf_token` and `refresh_ladder_index` used; `hub_cards_push.rs` is 294 lines; `cards_only` 15 times and `sync_cards_only` once in `hub.rs`; no 339 commit touches `item_checkpoint.rs`.

### Clippy

`cargo clippy -p vox-ml-cli --features mens-hf-hub -- -D warnings` exits 101 on three errors, none in files this plan touched: `hub_sync.rs:60` (`match_like_matches_macro`), `hub_sync.rs:245` (`collapsible_if`), and `hub_card/legacy_prose.rs:850` (`needless_lifetimes`). All three are in vox-populi, which stops the run before vox-ml-cli is linted; see the follow-up line below for the unflagged run. Left for their owners (legacy_prose.rs is deleted by 339-08).

Unflagged `cargo clippy -p vox-ml-cli --features mens-hf-hub` (warnings only) reports nothing in `hub_cards_push.rs`. It flagged one warning in `hub.rs` that was mine (`print_literal` on the moved header `println!`), fixed in `fix(339-04): inline the ACTION header literal`. The remaining `hub.rs` warnings (the Download arm's nested `if`s at lines ~194-207) are pre-existing from `4fce136c4`; the other warnings are in files outside this plan.

### Crate edges and lock

`git diff Cargo.lock` is empty (tempfile is already in vox-populi's dependency list as a dev-dependency). `vox ci crate-edges` (`VOX_SKIP_FRESHNESS_CHECK=1`) reports only the pre-existing `NEW EDGE not in baseline: vox-plugin-mens-candle-core -> vox-eval` (phase 313, reported in 339-01) and the stale baseline edge `vox-gui -> vox-db-types`. This plan adds no edge. Needs the user or the phase 313 owner.

## Mutation proofs

| # | File | Grep | Before / mutated / after | Mutation | Test | Mutated | Restored |
|---|---|---|---|---|---|---|---|
| 10 | hub_cards_push.rs | `CARD_FILES` | 4 / 4 / 4 | `card_only_manifest` skips the README.md replacement (added `if path == "README.md" { continue; }`, count 1 then 0) | `card_only_manifest_matches_staged_files`, `card_only_push_leaves_size_in_sync` | both FAILED (7 passed, 2 failed) | file copied back byte for byte (`cmp` identical); 9 passed |
| 14 | hub_card/facts.rs | `CURRENT_ADAPTER_SERVING` in hub.rs | 2 / 2 / 2 | `"nf4_base_plus_lora_arm v2"` changed to `v3` | `adapter_serving_matches_eval_identity` | FAILED: `left: "nf4_base_plus_lora_arm v3"`, `right: "nf4_base_plus_lora_arm v2"` | restored; `git diff` on facts.rs empty; 7 passed |

Proof 14 edits `facts.rs`, a 339-01 file that the 339-05 executor shares. I did it only after 339-05 had committed its facts.rs change (`799b4e5db`) and the file was clean against HEAD. The mutation was live for roughly 25 minutes (the gpu build was queued behind other agents), so a concurrent 339-05/06 test run in that window could have seen a failing serving fixture test.

## Deviations from Plan

**1. [Security review] Temp dirs.** The first commit staged into `std::env::temp_dir()/vox-mens-cards-<tag>-<size>-<pid>` and ignored `remove_dir_all` errors (predictable name on a shared temp dir). The coordinator asked for `tempfile::Builder`; `2beb50f1b` does that, makes `tempfile` an optional normal dependency of `vox-populi` behind `mens-hf-hub` (`dep:tempfile`), and adds a test that two stagings get distinct dirs, that a pre-existing directory at the old predictable path is neither reused nor deleted, and that the dir is mode 0700. tempfile 3.27.0's default mode follows the umask (the test first failed with mode 0755), so 0700 is requested explicitly with `Builder::permissions` on unix. Cleanup is `TempDir`'s drop (no `let _ =` in this file).

**2. [Rule 2] Per-size robustness.** `sync_cards_only` re-fetches the remote manifest per size with errors propagated per row (`cards_failed: ...` in `action_taken`) and re-classifies status from it, rather than reusing the first `sync_all_sizes` row, whose fetch swallows errors. `stage_card_only` also verifies each card file against the local manifest entry (hash and size) before copying. Neither is in the plan text.

**3. `decide_row` has no `push` parameter.** The plan's test line shows `decide_row(row, gate_current, remote, local, push=false)`; the pre-upload text is the same either way and `sync_cards_only` overwrites it with the commit id after an upload, so the parameter had no effect. `decide_row(row, gate_current, remote, local)` returns the plan.

**4. Tests in `hub_cards_push_tests.rs`** (via `#[path]`, as 339-03 did) to keep `hub_cards_push.rs` under 400 lines.

**5. Task 1 and 2 committed together** (same file, one build).

**Known ceiling (marked `ponytail:` in the code).** `fetch_remote_sync_manifest` returns `Ok(None)` when the repo `info()` call fails, so a transient failure reads as "no manifest". The upload that follows would fail the same way in practice, but if it did succeed it would push cards without rewriting the manifest. Fixing it means giving that function an error path in `hub_sync.rs`, which belongs to 338.

**Remote files outside the cards.** `card_only_manifest` keeps every non-card entry from the remote manifest, even when local evidence files have changed since publish. The card then describes local records the Hub bundle does not carry. That follows the plan (T-339-08) and keeps `vox mens hub pull` hashes valid; a size whose weights or evidence changed locally stays `Diverged` after the push.

## Needs the user

- The three clippy findings above (two pre-existing in `hub_sync.rs`, one in `legacy_prose.rs`) and the phase 313 `crate-edges` violation.
- Generated CLI reference docs may drift from the new `--cards-only` flag; `vox ci ssot-drift` was not run (installed `vox` is stale against the tree). The `ssot-autoregen` job handles it.
- 339-10 must still run a push only as `--cards-only --push`, and `--push --size <gated-current size>` for weights; this plan never contacted the Hub.

## Known Stubs

None.

## Threat Flags

None beyond the register: the new upload surface is exactly T-339-08..11, mitigated as listed (T-339-12 accepted: the label is the local inspection label).

## Self-Check: PASSED

- `hub_cards_push.rs`, `hub_cards_push_tests.rs` exist; commits `7e6b4353f`, `af2a020d4`, `2beb50f1b` are ancestors of HEAD.
