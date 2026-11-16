---
phase: 344-mens-debt-sweep
status: passed
verified: 2026-11-13
---

# Phase 344 Verification: MENS Debt Sweep

## Automated Checks Run
- `timeout 900s cargo test -p vox-populi --features mens-hf-hub --lib -- hub_repo hub_sync domain_router domain_profiles`
  - Result: `70 passed; 0 failed; 1 ignored; 0 measured; 360 filtered out`
  - Passing checks include `spoke_base_tests::all_domain_profiles_are_valid`, `spoke_base_tests::v1_finetuned_spoke_set_is_exactly_four`, `spoke_base_tests::harness_profile_exists_without_base`, `spoke_base_tests::retired_spokes_have_no_base`, and `spoke_base_tests::hub_embedder_is_present_and_revision_pinned`.
