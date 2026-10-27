---
phase: "317"
slug: "size-ladder-size-01"
status: verified
verdict: SECURED
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-10-27"
---

# Phase 317 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

ASVS L1. `block_on` is `high`, as the plan's threat model states. The plan-time register in `317-01-PLAN.md` has seven rows. Two are high (T-317-01, T-317-02), two are medium (T-317-03, T-317-04) and three are low (T-317-05, T-317-06, T-317-SC). On 2026-10-27 the security auditor checked every row against the code at `7d3377a9e`. The audit range was `git diff 2824de851..HEAD -- . ':(exclude).planning'`, which includes the review fixes WR-01 `d33009b3f`, WR-02 `a6889c0bd`, WR-03 `db1cb7497`, WR-04 `ffd4a5840`, IN-01 `ae274590f` and IN-02 `d710f6dae`. All seven rows are closed. No implementation file was changed for this audit.

The auditor ran these tests at `7d3377a9e`:

- `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(spoke_base_resolver) | test(preset_schema)'`: 64 of 64 passed.
- `cargo nextest run -p vox-populi --features mens-train --test training_presets_yaml_contract`: 5 of 5 passed.
- `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(reasoning_compare) | test(train_arm)'`: 43 of 43 passed.

The mutation evidence comes from the executor (317-01-SUMMARY, M1 to M7) and the fixer (317-REVIEW-FIX: WR-01, WR-02, WR-04, IN-01, IN-02). The auditor did not re-run those mutations.

On the rung lines of `train_bases` (comment lines excluded), the auditor counted 23 `hf_id:`, 23 `floor_measured: false` and 0 `floor_measured: true`. `git diff --stat 2824de851..HEAD -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml' contracts/` is empty.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| repo YAML → overlay loader | `mens/config/gpu-specs.yaml` on disk, or its `include_str!` embedded copy, decides which revision is trained and compared and which license is stamped on artifacts | `train_bases` rungs (hf_id@sha, floor, license, floor evidence) |
| CLI args → resolver | `--base` (compare) and `--model` / `VOX_BASE_MODEL` (train) name a size, a pin or a path | Size name, pin text, path hint |
| Hugging Face Hub → pins | Pins come from the Hub at planning time. At runtime a download fetches exactly the pinned commit | 40-hex commit SHA |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation (evidence) | Status |
|-----------|----------|-----------|----------|-------------|------------------------|--------|
| T-317-01 | Spoofing | a size name or rung resolving to the wrong or a moving revision | high | mitigate | **Pins.** The six dense rungs carry 40-lowercase-hex pins in `qwen3_code` (`gpu-specs.yaml:264`, `:284`, `:291-295`). The ls-remote evidence and its date are at `:251-254` and `:266-278`. **Contract test.** `DENSE_LADDER` holds the literal ls-remote SHAs (`training_presets_yaml_contract.rs:309-322`). It checks the evidence SHAs with `is_lower_hex_40` (`:324`, `:346`), and checks that each `<repo>@<sha>` is a `qwen3_code` rung (`:348-351`). It enforces one pin per size across every tag (`:352-356`), and requires a 40-lowercase-hex revision on every Qwen3 rung in every tag (`:385-387`). **Runtime resolution.** `resolve_base_pin` (`detect.rs:89-123`) passes input through only when `BaseLabel::from_pin` yields an exact 40-lowercase-hex revision (`reasoning_bench.rs:180-186`). Otherwise it considers only pinned overlay rungs (`detect.rs:99-104`), matches `repo_id == input` or `ends_with("/<input>")` (`:105-110`), and returns either exactly one distinct pin or `Err` (`:111-122`). There is no floating fallback. **Tests.** `base_resolution_resolves_every_ladder_size` (`detect.rs:524`) uses literal SHAs, so YAML drift fails it. `base_resolution_unknown_or_ambiguous_is_an_error` (`:549`) rejects `@main` and uppercase hex. **Mutation.** SUMMARY M7 (one hex character of the 4B pin) turned both tests red. | closed |
| T-317-02 | Repudiation | a floor presented as measured without a measurement | high | mitigate | **Required key.** `pub floor_measured: bool` has no serde default (`spoke_base_resolver.rs:57-60`). `measured_on` and `peak_mb` are optional (`:61-66`). **Load-time check.** `parse_overlay` (`:147-175`) rejects `true` unless `measured_on` is non-blank after `trim` and `peak_mb` is in `1..=floor_mb` (`:157-159`, WR-04). It rejects `false` when either is present (`:160-161`). The error names the tag and hf_id (`:164-170`). Both loaders route through it: disk at `:187-193` and embedded at `:179-185`. **Labeling.** `floor_label` returns `... is an estimate (floor unmeasured)` for every estimate (`:71-87`). It is reported on stderr and in `base_rungs` by the compare harness (`reasoning_compare.rs:341-345`, `:188`, `:233`), and at train start (`gpu.rs:430-434`). The YAML has no rung marked `true`, and the contract test asserts that (`training_presets_yaml_contract.rs:380-384`). **WR-01.** A parse error is no longer swallowed: `license_for_hf_id` (`spoke_base_resolver.rs:199-212`), `floor_notes` (`:110-122`, `gpu.rs:431` `?`) and `derive_quantization_for_base` (`train_arm.rs:819`) all propagate it. **Tests:** `floor_label_says_floor_unmeasured_for_an_estimate` (`:881`), `overlay_rung_without_floor_measured_is_rejected` (`:897`), `measured_floor_needs_evidence` (`:952`, seven bad cases including `peak_mb: 0`, `peak_mb > floor_mb` and `false` with `measured_on`), `malformed_overlay_surfaces_its_parse_error` (`:1045`), `quantization_surfaces_a_malformed_overlay` (`train_arm.rs:1070`). **Mutations:** M1, M2 and M3, plus the WR-04 and IN-01 mutations in REVIEW-FIX. | closed |
| T-317-03 | Tampering | artifacts mislabeled with no license or the wrong one | medium | mitigate | **YAML.** Every rung of the six dense repos has `license: apache-2.0, attribution_required: false` (`gpu-specs.yaml:264-295` and the alias-tag copies). **Contract test.** It asserts that license and attribution value for every rung of each dense repo in every tag (`training_presets_yaml_contract.rs:357-358`). `every_dense_ladder_pin_resolves_its_license` (`spoke_base_resolver.rs:994`) resolves all six pins with no override. **Hard error.** `resolve_license_class` (`:240-270`) still fails for an unmapped base with no override (`:261-268`). An override on an unmapped base defaults attribution conservatively (`:256-258`). Since WR-01, an overlay parse error fails even when an override is given (`:245-248`). **Callers.** The only production `LoraTrainingConfig` is `gpu.rs:436`, and it takes its license from `resolve_license_class` (`gpu.rs:423-429`). `merge_qlora.rs:251-262` resolves the same way. The unlicensed `Qwen3.8-27B` rung therefore hard-errors without `--license-class`, and it is deferred in `deferred-items.md`. | closed |
| T-317-04 | Denial of service | a 1.7B or 4B run getting 8B preset clamps because the revision hex contains `8b` | medium | mitigate | `detect_qwen_size_class` (`preset_schema.rs:87-110`) strips the text after the last `@` (`:93`). It then classifies only the last `/` or `\` segment that contains `qwen` (`:94-96`), using the delimited size-token parser `params_b_from_model_hint`. **Test.** `size_class_ignores_the_pinned_revision_text` (`:879`) covers both real pins, a synthetic `32b` revision, HF-cache snapshot paths at both real pins, an 8B snapshot whose hex is `14b32b…`, a Windows 14B snapshot whose hex contains `8b`, and a `checkpoint-32b` path. **Mutations:** M6 in the SUMMARY and the WR-02 mutation in REVIEW-FIX. See residual note 1. | closed |
| T-317-05 | Denial of service | a rung missing the new key breaking every overlay consumer | low | accept | Accepted: AR-317-05. The acceptance's premises hold. All 23 rung lines carry `floor_measured: false`. Every consumer reads the same file through `parse_overlay`. The resolver and contract tests load the real file and pass. After WR-01, the failure is loud: the parse error surfaces in the license, floor-note and quantization paths and is not swallowed. | closed |
| T-317-06 | Spoofing | dedupe hiding an inconsistent copy of a repeated rung | low | mitigate | `rungs_for_hf_id` (`spoke_base_resolver.rs:93-105`) filters on exact `b.hf_id == hf_id` (`:99`). It dedupes only rungs that are equal in every field, because `TrainBase` derives `PartialEq` (`:40`) and the call is `dedup_by(\|a, b\| a == b)` (`:103`). The contract test requires rungs repeated under the same `(hf_id, floor_mb, methods)` key to be equal in every field (`training_presets_yaml_contract.rs:389-392`). `rungs_for_hf_id_dedupes_repeats_across_tags` (`:905`) shows that both another revision and the bare repo id match nothing. SUMMARY mutation M4 covers this. | closed |
| T-317-SC | Tampering | npm/pip/cargo installs | low | accept | Accepted: AR-317-SC. `git diff --stat 2824de851..HEAD -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml' contracts/` is empty, so no package, dependency or crate edge was added. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `high` count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Residual Notes (non-blocking)

These are outside the register or beyond L1 depth. None reopens a row. They are listed so later phases can pick them up.

1. **T-317-04: an edge-case regression from the WR-02 rewrite.** `rsplit_once('@')` (`preset_schema.rs:93`) splits on the last `@` anywhere in the hint, including an `@` in a parent directory. For example, `/home/a@b/hf/models--Qwen--Qwen3-14B/snapshots/<rev>` reduces to `/home/a`. No segment of that names `qwen`, so the function returns `None`, and `preset_schema.rs:558` then skips every size-ladder clamp. The old code returned `S14` for that path. The impact is a possible OOM on a small card, but only for an `@` in a local path. **Fix:** strip `@…` only when no `/` or `\` follows it, and add that path to the test.
2. **Pre-existing, unchanged in this phase.** `load_overlay` (`spoke_base_resolver.rs:187-193`) falls back to the embedded copy on any read error, not only `NotFound`. A gpu-specs.yaml that is not valid UTF-8, or not readable, is therefore silently replaced by the build-time copy instead of surfacing an error. It falls back to the reviewed embedded copy, never to an unvetted default, so it does not weaken T-317-02 or T-317-03. **Fix:** fall back only on `ErrorKind::NotFound`.
3. `TrainBase` has no `deny_unknown_fields`. A misspelled evidence key on an estimate rung, such as `peak_mbb:`, therefore loads silently. On a measured rung, the required-evidence check still rejects it.
4. **Pre-existing; outside T-317-01's Qwen3 scope.**
   - The Qwen2.5-Coder rungs (`gpu-specs.yaml:306`, `:309`, `:348`) are unpinned and float to `main`. This is documented at `:5`.
   - `vox mens train --model Qwen/Qwen3-4B` (a bare repo id) with an explicit `--license-class` reaches the downloader with no revision (`hub.rs:49-51`). Without the override, the exact-match license lookup fails closed.
   - Bare size names on `--model` are SIZE-02 (Phase 318).
5. **Review items not folded in, all low.**
   - IN-05: the order of inconsistent duplicates is nondeterministic. It can over-report rungs but never hides one.
   - IN-03: the 1.7B and 4B floors have no ratio guard. Both floors are labeled as estimates.
   - IN-06: `base_rungs` is passed as an untyped value.
6. `measured_on` is free text from repo YAML and is echoed to stderr and to the record JSON. The repo is the trust boundary, so the risk is low.

## Unregistered Flags

None. 317-01-SUMMARY `## Threat Flags` reports none. The auditor found no new network endpoint, auth path or file-access pattern in the range:

- The only new write is the `base_rungs` key inside the record already guarded by `check_out_path`.
- The only new read is a second `load_overlay` of the same file in `gpu.rs`.

The review fixes change failure behavior to fail closed (WR-01) and change the size-class parser (WR-02). Both map to T-317-02/05 and T-317-04.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-317-05 | T-317-05 | Failing closed is intended: a rung without `floor_measured` must not load. Every rung carries the key (23 of 23). Every consumer (trainer, compare harness, spoke-check, embedded copy) reads the same file through `parse_overlay`. The resolver and contract tests load the real file, so a missing key fails CI before it reaches an operator. | phase 317 plan | 2026-10-27 |
| AR-317-SC | T-317-SC | This phase installs nothing and adds no crate dependency or edge. `Cargo.lock`, every `Cargo.toml` and `contracts/` are unchanged over the phase range. | phase 317 plan | 2026-10-27 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-27 | 7 | 7 | 0 | security auditor (ASVS L1) at `7d3377a9e` |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-27

## Security Audit 2026-10-27

| Metric | Count |
|---|---|
| Threats found | 7 |
| Closed | 7 |
| Open | 0 |
| Unregistered flags | 0 |

## Post-audit fixes (orchestrator, 2026-10-27)

Notes 1 and 2 fixed in 50efac204: `detect_qwen_size_class` strips `@rev` from the chosen model segment only (a `/home/a@b/...` test case failed before, passes after); `load_overlay` falls back to the embedded copy only on `NotFound` and errors otherwise (a directory-as-file test failed before, passes after). vox-populi resolver+preset 64/64, contract 5/5.
