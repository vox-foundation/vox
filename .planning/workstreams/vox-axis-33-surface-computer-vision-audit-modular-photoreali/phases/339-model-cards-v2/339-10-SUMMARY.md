---
phase: 339-model-cards-v2
plan: "10"
subsystem: mens-hub-cards
tags: [hub-cards, lints, review-packet, dry-run]
requires:
  - phase: 339-01..09
    provides: "Model card templates, facts loader, SVG assets, and 4-lint verifier"
provides:
  - "Local verification of all rendered v2 Hub cards and SVGs in mens/runs/hub_cards/"
  - "Local review packet at mens/runs/hub_cards/REVIEW.md"
  - "Verification of lints_hold_on_rendered_cards_dir passing on real rendered output"
affects: [mens/runs/hub_cards]
tech-stack:
  added: []
  patterns: [local-first card review packet before Hub push]
key-files:
  created:
    - mens/runs/hub_cards/REVIEW.md
key-decisions:
  - "Defer remote HF Hub push (--push / VOX_MENS_HF_SYNC) for explicit operator sign-off after REVIEW.md inspection"
requirements-completed: [CARD-12]
completed: 2026-11-13
---

# Phase 339 Plan 10: Real-Cards Lint Gate & Review Packet Summary

**Verified all locally rendered Hub v2 cards in `mens/runs/hub_cards/` against the 4 card lints and produced `REVIEW.md` without pushing to Hugging Face.**

## Key Results
- Verified `mens/runs/hub_cards/` contains rendered `README.md` + `assets/scorecard.svg` for `vox-mens-qwen3-{0.6b,1.7b,4b,8b,14b,32b}`, `vox-mens-index/{README.md,ladder_index.json,assets/overview.svg}`, and `vox-mens-playground/{README.md,index.html}`.
- Ran `VOX_HUB_CARDS_DIR=/Users/brbrainerd/dev/vox/mens/runs/hub_cards cargo test -p vox-populi --features mens-hf-hub --lib lints_hold_on_rendered_cards_dir -- --ignored` -> `1 passed; 0 failed`.
- Authored `mens/runs/hub_cards/REVIEW.md`; remote Hub upload intentionally held for operator approval.
