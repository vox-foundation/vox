---
phase: 430-hub-promotion-audit
plan: 01
type: execute
wave: 6
depends_on: [429-01]
files_modified:
  - crates/vox-populi/src/mens/hub_repo.rs
  - mens/config/hub-cards.yaml
  - mens/config/eval-gates-ladder.yaml
  - docs/src/reference/mens-models.md
  - docs/src/architecture/mens-model-selection-nov-2026-research-2026-11-13.md
  - .planning/workstreams/mens-nov2026-v73/v7.3-MILESTONE-AUDIT.md
autonomous: true
requirements:
  - MENS-NOV26-PROMOTE-01
  - MENS-NOV26-PROMOTE-02
  - MENS-NOV26-PROMOTE-03
---

# Phase 430-01: Post-Gate Ladder Promotion, Hub Cards v2/v3 & Milestone v7.3 Audit

## Objective
Audit and finalize post-gate ladder promotion across `crates/vox-populi/src/mens/hub_repo.rs`, `mens/config/hub-cards.yaml`, `mens/config/eval-gates-ladder.yaml`, `docs/src/reference/mens-models.md`, and `docs/src/architecture/mens-model-selection-nov-2026-research-2026-11-13.md` using Phase 429 empirical `gate_receipt.json` and `reasoning_compare/` results; run all card, doc, arch, and hardcoded-path audits; and close Milestone `v7.3` with `v7.3-MILESTONE-AUDIT.md`.

## Execution Invariants
- **Zero Metal GPU Workloads:** Phase 430 is 100% CPU/metadata/lint/test work. Independent checks are `[PARALLEL-SAFE]`, while source/doc edits precede final verification.
- **Card Config Rules (`mens/config/hub-cards.yaml`):**
  - Every `sizes.<size>.why` entry must be 2–3 sentences.
  - Zero ASCII digits (`0-9`) inside `why` or `spokes[*].purpose` prose (numbers come dynamically from run records via `hub_card::facts`, except `regate_eta`).
  - Never hand-edit `mens/runs/hub_cards/**`; regenerate via `vox-ml-cli mens hub sync` (dry-run, no `--push`).
- **Scope Guard:** Preserve legacy `Qwen3-*` (`1.7B`, `8B`, `14B`, `32B`) lookup compatibility in `hub_repo.rs` (`LEGACY_SIZES` / `ALL_KNOWN_SIZES`).

---

## Tasks

### Task 430.1 — Post-Gate Promotion Alignment (`hub_repo.rs`, `hub-cards.yaml`, `eval-gates-ladder.yaml`) `[SEQUENTIAL]`
- **Inputs:** Phase 429 `mens/runs/<rung>/gate_receipt.json` (`0.6B`, `2B`, `4B`, `9B`, `27B`), `crates/vox-populi/src/mens/hub_repo.rs`, `mens/config/hub-cards.yaml`, `mens/config/eval-gates-ladder.yaml`.
- **Actions:**
  1. Confirm `hub_repo::SIZES` (`["0.6B", "2B", "4B", "9B", "27B"]`) matches the gate-passing Nov 2026 rungs. If any rung failed Phase 429 gates, document its `ungated` status and exclude it from promoted default tags (never silently relax `eval-gates-ladder.yaml` thresholds).
  2. Verify `mens/config/eval-gates-ladder.yaml` slice hashes (`gsm8k_think`, `math500_think`, `vox_think`) match the `slice_blake3` recorded in every `gate_receipt.json`.
  3. Verify `mens/config/hub-cards.yaml` `why` blurbs and `regate_eta` entries reflect any empirical timing/status updates without introducing ASCII digits into `why` or `purpose`.
- **Verification:** `cargo test -p vox-populi --features mens-train,mens-hf-hub,mens-cloud --lib hub_card` passes with `0` failures.

### Task 430.2 — Sync Reference & Research Documentation `[PARALLEL-SAFE]`
- **Inputs:** Phase 429 `eval_local_report.json`, `collateral_damage_report.json`, `gate_receipt.json`, `mens/runs/reasoning_compare/*.json`, `docs/src/reference/mens-models.md`, `docs/src/architecture/mens-model-selection-nov-2026-research-2026-11-13.md`.
- **Actions:**
  1. Update `docs/src/reference/mens-models.md` with empirical Stage D / Phase 429 gate outcomes for the Nov 2026 ladder (`0.6B`, `2B`, `4B`, `9B`, `27B`), ensuring alignment with `hub_repo.rs` and card anchor contracts (`#hub-and-spoke`, etc.).
  2. Update `docs/src/architecture/mens-model-selection-nov-2026-research-2026-11-13.md` with measured Phase 428/429 results (training loss, peak MiB, pass@1, collateral McNemar verdicts) using relative paths only.
- **Verification:** `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/reference/mens-models.md docs/src/architecture/mens-model-selection-nov-2026-research-2026-11-13.md` exits `0`.

### Task 430.3 — Dry-Run Card Render & Rendered Card Lints `[SEQUENTIAL]`
- **Precondition:** Tasks 430.1 and 430.2 complete; `vox-ml-cli` rebuilt if `hub-cards.yaml` or `hub_repo.rs` changed.
- **Actions:**
  1. Render all Model Cards v3 and index locally (dry run, no `--push`):
     ```sh
     ./target/release/vox-ml-cli mens hub sync --namespace brbrainerd
     ```
  2. Run rendered card lints against `mens/runs/hub_cards/`:
     ```sh
     VOX_HUB_CARDS_DIR=/Users/brbrainerd/dev/vox/mens/runs/hub_cards cargo test -p vox-populi --features mens-train,mens-hf-hub,mens-cloud --lib lints_hold_on_rendered_cards_dir -- --ignored
     ```
- **Expected Output:** `vox-ml-cli mens hub sync` exits `0`; `lints_hold_on_rendered_cards_dir` passes (`1 passed; 0 failed`).

### Task 430.4 — Architectural, Code Audit, Doc Pipeline & Hardcoded-Path Gates `[PARALLEL-SAFE]`
- **Actions (run independently):**
  1. Architecture layer check: `cargo run -p vox-arch-check` (or `./target/debug/vox-arch-check`).
  2. Code audit (LLM boundary + crypto + policy): `cargo test -p vox-code-audit --lib`.
  3. Full doc pipeline lint: `cargo run -p vox-doc-pipeline -- --lint-only`.
  4. Hardcoded host/cache path scan across `mens/runs/hub_cards/` and `docs/src/architecture/measurements/`:
     ```sh
     rg -n "/Users/|/home/|C:\\\\" mens/runs/hub_cards/vox-mens-index mens/runs/hub_cards/vox-mens-playground mens/runs/hub_cards/vox-mens-qwen3-0.6b mens/runs/hub_cards/vox-mens-qwen3.5-2b mens/runs/hub_cards/vox-mens-qwen3.5-4b mens/runs/hub_cards/vox-mens-qwen3.5-9b mens/runs/hub_cards/vox-mens-qwen3.8-27b docs/src/architecture/measurements/
     ```
- **Expected Output:** All checks exit `0`; `rg` hardcoded-path scan exits `1` (`0` matches).

### Task 430.5 — Milestone `v7.3` Audit & Closure `[SEQUENTIAL]`
- **Precondition:** Tasks 430.1–430.4 all green.
- **Actions:**
  1. Write `.planning/workstreams/mens-nov2026-v73/phases/430-hub-promotion-audit/430-SUMMARY.md`.
  2. Write `.planning/workstreams/mens-nov2026-v73/v7.3-MILESTONE-AUDIT.md` recording requirement traceability (`MENS-NOV26-PREFLIGHT-01..04`, `PREP-01..03`, `CAL-01..03`, `TRAIN-01..04`, `EVAL-01..03`, `PROMOTE-01..03`), measured rung metrics, and gate receipts.
  3. Update `.planning/workstreams/mens-nov2026-v73/ROADMAP.md` and `STATE.md` to mark Phases 428–430 complete.

---

## Abort & Rollback Triggers
- **`lints_hold_on_rendered_cards_dir` Failure:** Do not push to Hub; fix source template/config (`mens/config/hub-cards.yaml` or `crates/vox-populi/templates/hub/`) and re-render via `hub sync`.
- **Hardcoded Path Leak (`/Users/...`):** Fix `sanitize_json_paths` or doc source; never ship host-specific paths into `hub_cards/` or `docs/`.
- **`vox-arch-check` or `vox-doc-pipeline` Failure:** Resolve layer/frontmatter violation before closing `v7.3-MILESTONE-AUDIT.md`.
