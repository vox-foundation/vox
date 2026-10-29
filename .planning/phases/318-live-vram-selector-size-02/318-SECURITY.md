---
phase: "318"
slug: "live-vram-selector-size-02"
status: verified
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-10-29"
---

# Phase 318 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

ASVS L1. `workflow.security_block_on` is `high`. The plan-time register has six mitigations (T-318-01 through T-318-06) and one accepted supply-chain row (T-318-SC, copied in all three plans). The security auditor checked committed `07f504dfb..74129dc11` (tip `74129dc11`) on 2026-10-29, including review fixes `9a41951bc` through `e3a27eda7`, and closed all seven rows. No implementation files were changed for this audit. `318-01-SUMMARY.md`, `318-02-SUMMARY.md`, `318-03-SUMMARY.md`, and `SUMMARY.md` have no Threat Flags section.

Checked after the review fixes: a non-empty `VOX_MENS_DEFAULT_MODEL` is an explicit pin, skip paths do not probe, step-down uses the same overlay as the pick, `model_needs_accelerator_reading` does not substring-match `Qwen3-`, a missing or unfit reading fails closed, and the range adds no package.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Injected free-memory `u32` → `pick_base` | A caller-supplied reading chooses the size. Production must pass the accelerator reading, not a guessed total. | Free MiB |
| Size spelling → `explicit_qwen3_size_for_free_mb` | An operator name must not be rewritten to a smaller rung when it does not fit. | Size token or Hugging Face id |
| CLI model string → `resolve_training_selection` | Absent means the live selector. A size spelling is explicit. A capability tag stays on `pick_base`. | Model name, env pin |
| Second free-memory reading → schola `run_train` | The reading can shrink an automatic pick or refuse an explicit one. It must not start a weight load after `Err`. | Free MiB |
| Host accelerator reading → `select_optimal_local_model` | A failed or empty read must not become a model id the chat or GUI host will serve. | Free MiB, Hugging Face id |
| `auto-local` string in chat → `LlmConfig.model` | The only automatic id is the selector `hf_id`. A caller-typed model string is not rewritten by this phase. | Model id |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-318-01 | Tampering | `free_mb_from_readings` | high | mitigate | `free_mb_from_readings` (`spoke_base_resolver.rs:614-633`) returns `(free_gb * 1024).round()` when CUDA info is present and otherwise the Metal `u64`. Both absent is `Err`. The function reads no environment variable. `read_accelerator_free_mb` (`spoke_base_resolver.rs:636-645`) calls `query_nvidia_smi_vram` and, only when that is `None`, `monitor_metal().memory_free_mb`. It does not call `get_system_vram_info`, `hw.memsize`, or `vm_stat`. Tests: CUDA wins (`:1490`), Metal only (`:1501`), both absent (`:1507`). | closed |
| T-318-02 | Denial of service | `confirm_before_load` | high | mitigate | `select_qwen3_code_qlora_for_free_mb(1_500)` is `Err` (`spoke_base_resolver.rs:1383`). Automatic `confirm_before_load` at 1500 MB is `Err` and does not name 1.7B or 4B (`:1390-1393`). Explicit over-floor spellings return `Err` and are not replaced (`explicit_qwen3_size_for_free_mb` `:549-568`; test `:1398-1410`; Explicit arm `:595-604`). These functions return a pick or a revision. They do not load weights. | closed |
| T-318-03 | Tampering | `apply_preload_free_mb` | high | mitigate | `automatic == false` builds `SizeChoice::Explicit` (`run_train.rs:210-216`) and never calls `select_qwen3_code_qlora_for_free_mb`. `apply_preload_free_mb_refuses_an_explicit_32b_pin` expects `Err` at 25000 MB and asserts the 14B id is absent (`:800`). A non-empty env pin is applied before the live pick and forces `automatic_qwen3 = false` (`run_train.rs:329-335`). Pipeline and train arm pass automatic only when both the CLI model and the pin are absent (`pipeline.rs:577`, `train_arm.rs:407`). `resolve_training_selection` uses `cli_model.or(pinned)` (`training_selection.rs:61-65`). `env_pin_wins_over_an_injected_free_memory_reading` keeps `org/Pinned-Not-Qwen` at 25000 MB (`:275`). | closed |
| T-318-04 | Denial of service | `resolve_local_spoke_base` / `run_train` | high | mitigate | A missing reading bails in `resolve_training_selection` (`training_selection.rs:94`) before any train call. Pipeline and `resolve_local_spoke_base` call `read_accelerator_free_mb()?` only after `selection_requires_free_reading` (`pipeline.rs:468-477`, `train_arm.rs:772-779`). That predicate is false for RagOnly and PromptOnly (`training_selection.rs:39-42`, `:114-140`; test `:149`). `run_train` applies the env pin, then `apply_preload_free_mb()?` (`run_train.rs:391-400`), and only then `select_mens_backend` (`:456`). `run_train_checks_preload_before_select_mens_backend` (`:829`) asserts that order. `model_needs_accelerator_reading` (`:124-134`) is true only for automatic, `is_qwen3_size_spelling`, or an exact measured QLoRA row. `org/Qwen3-8B-finetune` does not require a reading (`:737`). The source does not contain `repo.contains("Qwen3-")` (`:760-764`). | closed |
| T-318-05 | Spoofing | `select_optimal_local_model` | high | mitigate | `select_tier_for_vram` is absent from `crates/vox-orchestrator/src/models`. `select_from_free_mb` (`auto_select.rs:114-136`) returns the QLoRA `hf_id` or `Err`. `select_from_free_mb(1_500)` is `Err` (`:174`). `select_optimal_local_model` (`:140-152`) reads `read_accelerator_free_mb` and calls `select_from_free_mb`. Its body does not call `detect_hardware_vram_gb` (`:218`). | closed |
| T-318-06 | Denial of service | `resolve_chat_config` / `get_auto_model_recommendation` | medium | mitigate | `resolve_chat_config_with_auto_pick` maps selector `Err` with `?` before it builds `LlmConfig` (`chat.rs:584-592`). The injected `Err("unreadable")` test expects `is_err` (`:710`). `get_auto_model_recommendation` returns `recommendation_from_selection(select_optimal_local_model_for_host())` (`models_host.rs:33-44`). The mapper test expects the string error `"unreadable"` (`:118`). | closed |
| T-318-SC | Tampering | cargo/npm/pip installs | high | accept | Plans 01, 02, and 03 each declare this row. The range adds no crate, npm, or pip package and no `Cargo.toml` dependency. The only manifest edit is an empty `hw-probe-live-test` feature flag on `vox-ml-cli` (`Cargo.toml`, no `dep:`). No install step. Recorded as AR-318-SC. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `high` count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-318-SC | T-318-SC | This phase installs nothing and adds no crate dependency. The empty `hw-probe-live-test` feature keeps two existing live `train_arm` tests out of the default suite. It is not a package. | phase 318 plans 01–03 | 2026-10-29 |

*Accepted risks do not resurface in future audit runs.*

---

## Unregistered Flags

None. The phase summaries have no `## Threat Flags` section.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-29 | 7 | 7 | 0 | gsd-security-auditor, ASVS L1, `block_on: high`, range `07f504dfb..74129dc11` |

The three plan rows named T-318-SC are one accepted supply-chain row here.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-29
