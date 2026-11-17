---
phase: 453-deep-research-reasoning-budget-separation-citation-scoped-au
review_scope: plans 453-01 & 453-02 (LIVE-09..12) --fix
reviewed_at: 2026-11-16
reviewer: cr-fix-453
status: clean
---

# Phase 453 Code & Architecture Review (`--fix`)

## Scope Reviewed
- [`crates/vox-llm-egress/src/{lib.rs,wire.rs}`](crates/vox-llm-egress/src/wire.rs)
- [`crates/vox-actor-runtime/src/{model_resolution.rs,llm/types.rs,llm/chat.rs,llm/stream.rs,llm/cascade.rs}`](crates/vox-actor-runtime/src/llm/cascade.rs)
- [`crates/vox-research-events/src/schema_types.rs`](crates/vox-research-events/src/schema_types.rs)
- [`crates/vox-research-shim/src/research/{discovery_bridge.rs,verifier.rs,orchestrator/{config.rs,pipeline.rs,pipeline_cache.rs,stages.rs}}`](crates/vox-research-shim/src/research/orchestrator/pipeline.rs)
- [`crates/vox-scientia/src/{review_flow.rs,claim_extractor/{atomic.rs,pipeline.rs},class_routing/defaults.rs,dashboard/cost.rs}`](crates/vox-scientia/src/claim_extractor/pipeline.rs)
- [`crates/vox-cli-research/src/{lib.rs,chat.rs,scientia.rs,scientia_phase_handlers.rs,daemon_db_proxy.rs,daemon_db_proxy/formatters.rs,daemon_db_proxy/scientia_ops.rs,db/publication/discovery.rs}`](crates/vox-cli-research/src/daemon_db_proxy.rs)
- [`crates/vox-search/src/embedding_env.rs`](crates/vox-search/src/embedding_env.rs)
- [`crates/vox-effort-audit/src/judge/mod.rs`](crates/vox-effort-audit/src/judge/mod.rs)
- [`crates/vox-effort-route/src/{embed.rs,route/mod.rs}`](crates/vox-effort-route/src/route/mod.rs)

## Fixes Applied (`--fix`)
1. **Citation Conflict Guard ([`pipeline.rs`](crates/vox-research-shim/src/research/orchestrator/pipeline.rs)):** Updated `audit_citations` to check `!has_conflict && score >= 0.55` from `lexical_containment_score` instead of `.0 >= 0.55` alone, preventing contradicting/negated/numerically mismatched snippets from crediting `Verdict::Supported` claims; added unit test `audit_citations_rejects_conflicting_lexical_overlap_snippet`.
2. **Panic-Free Frontmatter/Markdown Masking ([`claim_extractor/pipeline.rs`](crates/vox-scientia/src/claim_extractor/pipeline.rs)):** Replaced both `.expect(...)` calls in `mask_non_prose_regions` with `let Some(first_nl)` and `let Ok(masked) = String::from_utf8(bytes) else { return source_text.to_string(); }`, eliminating both `toestub` `rust/expect-call` warnings.
3. **Bare-Filename Output & Daemon Proxy Hardening ([`daemon_db_proxy.rs`](crates/vox-cli-research/src/daemon_db_proxy.rs), [`formatters.rs`](crates/vox-cli-research/src/daemon_db_proxy/formatters.rs), [`lib.rs`](crates/vox-cli-research/src/lib.rs)):** Extracted `formatters::write_output_file` filtering `!p.as_os_str().is_empty()` before `std::fs::create_dir_all` (fixing `ENOENT` on bare `--output report.md` paths where `Path::parent()` returns `Some("")`); kept `daemon_db_proxy.rs` at `188` LoC (`<= 12` entities), `formatters.rs` at `219` LoC, and `scientia_ops.rs` at `77` LoC (`< 400` LoC each); added `write_output_file_handles_bare_filename_and_nested_directory`, `format_retrieval_status_proxy_renders_lane_timeouts_and_providers`, and JSON/error payload assertions.
4. **Percent-Encoded Daemon Path & Tests ([`scientia_ops.rs`](crates/vox-cli-research/src/daemon_db_proxy/scientia_ops.rs)):** Added `encode_uri_component` / `scientia_claims_api_path` so reserved characters (`?`, `#`, `/`, `..`, spaces) in `publication_id` cannot fail `validate_api_v2_path` or misroute Axum, switched `println!` to `writeln!(std::io::stdout(), ...)?`, and added `scientia_claims_api_path_percent_encodes_reserved_segments` (eliminating `skeleton/untested-pub-api`).
5. **Discovery Explain Parity & `.expect` Removal ([`discovery.rs`](crates/vox-cli-research/src/db/publication/discovery.rs)):** Removed `.expect("root normalized to object")` in `merge_novelty_bundle_into_metadata_json_str` via `if let Some(root_obj)` and reused `rank_discovery_row` in `publication_discovery_explain` to match `publication_discovery_scan` overlap fallback semantics.

## Verification Commands & Results
- **Formatting:** `rustfmt --edition 2024 crates/vox-research-shim/src/research/orchestrator/pipeline.rs crates/vox-scientia/src/claim_extractor/pipeline.rs crates/vox-cli-research/src/daemon_db_proxy.rs crates/vox-cli-research/src/daemon_db_proxy/scientia_ops.rs crates/vox-cli-research/src/daemon_db_proxy/formatters.rs crates/vox-cli-research/src/lib.rs crates/vox-cli-research/src/db/publication/discovery.rs` — exit `0`.
- **Clippy (`-D warnings`):**
  - `VOX_BROKER_COALESCE=0 VOX_BROKER_RUN_TIMEOUT_SECS=2400 timeout 1800s cargo clippy -p vox-llm-egress -p vox-actor-runtime -p vox-research-events -p vox-research-shim -p vox-scientia --lib -- -D warnings` — exit `0` (`0` warnings).
  - `VOX_BROKER_COALESCE=0 VOX_BROKER_RUN_TIMEOUT_SECS=2400 timeout 1800s cargo clippy -p vox-cli-research --lib -- -D warnings` — exit `0` (`0` warnings; initial combined/shorter 600s/900s runs hit timeout waiting on shared Cargo build-broker lock).
- **`toestub --mode legacy`:** `toestub --mode legacy crates/vox-cli-research/src/daemon_db_proxy.rs crates/vox-cli-research/src/daemon_db_proxy/formatters.rs crates/vox-cli-research/src/daemon_db_proxy/scientia_ops.rs crates/vox-scientia/src/claim_extractor/pipeline.rs crates/vox-research-shim/src/research/orchestrator/pipeline.rs` — `0` findings in `daemon_db_proxy.rs`, `formatters.rs`, and `scientia_ops.rs`; `0` Phase 453 findings in `claim_extractor/pipeline.rs` and `orchestrator/pipeline.rs` (only pre-existing legacy `arch/god_object` file/entity budget errors on those two legacy files and 5 pre-existing legacy warnings in `orchestrator/pipeline.rs` lines 947, 1130, 1192, 1234, 1857).
- **Unit Tests (`--lib` after final edits):**
  - `VOX_BROKER_COALESCE=0 VOX_BROKER_RUN_TIMEOUT_SECS=2400 RUST_MIN_STACK=8388608 timeout 1800s cargo test -p vox-scientia -p vox-research-shim -p vox-cli-research --lib`:
    - `vox-cli-research`: `673 passed; 0 failed; 2 ignored`
    - `vox-research-shim`: `183 passed; 0 failed; 2 ignored`
    - `vox-scientia`: `380 passed; 0 failed; 1 ignored`

## Non-Owned Follow-Up Notes
- `contracts/scientia/finding-candidate.v1.schema.json` and `contracts/scientia/finding-class-defaults.v1.yaml` (outside Phase 453 review ownership) do not yet enumerate `"empirical_science"` alongside the new `EmpiricalScience` Rust enum variant.

## Verdict
**CLEAN** — Phase 453 satisfies all `LIVE-09..12` correctness, clippy, `toestub`, formatting, module-budget, and unit-test requirements.
