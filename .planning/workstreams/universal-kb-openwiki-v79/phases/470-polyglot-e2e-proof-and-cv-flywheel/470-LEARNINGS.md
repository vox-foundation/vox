# Phase 470 Learnings

1. **End-to-End Integration Proofs Expose Real Seam Assumptions:** Running onboarding -> Graphify -> 5-layer OpenWiki -> AST shift -> Tier-1 auto-heal -> Tier-2 staged proposal in a single polyglot temp fixture immediately surfaced the real-world interaction between `../../` OpenWiki citations and Tier-2 citation containment validation.
2. **Edition 2024 Clippy & Doc Gates Together:** Running `cargo clippy -- -D warnings`, `vox-doc-pipeline --lint-only`, `vox-arch-check`, `docs_system_parity`, `vox ci operations-verify`, and `toestub` consecutively prevents `collapsible_if`, `raw-git-exec`, and CLI claim drift from compounding across multi-phase milestones.
