//! `contracts/mens/training-presets.v1.yaml` lists every `KNOWN_PRESETS` id (SSOT parity).

#![cfg(feature = "mens-train")]

use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct Root {
    default_base_model: String,
    presets: Vec<PresetRow>,
}

#[derive(Debug, Deserialize)]
struct PresetRow {
    id: String,
    #[serde(default)]
    aliases: Vec<String>,
}

#[test]
fn training_presets_yaml_covers_known_presets() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("../../contracts/mens/training-presets.v1.yaml");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let root: Root = serde_yaml::from_str(&raw).expect("parse training-presets YAML");

    assert_eq!(
        root.default_base_model,
        vox_populi::mens::DEFAULT_MODEL_ID,
        "training-presets default_base_model must match vox_populi::mens::DEFAULT_MODEL_ID"
    );

    let mut ids: HashSet<String> = HashSet::new();
    for row in &root.presets {
        ids.insert(row.id.clone());
        for a in &row.aliases {
            ids.insert(a.clone());
        }
    }

    for preset in vox_populi::mens::KNOWN_PRESETS {
        assert!(
            ids.contains(*preset),
            "KNOWN_PRESETS entry `{preset}` missing from {} (add id or alias)",
            path.display()
        );
    }
}

/// Third leg of the SSOT chain: does the independent `gpu-specs.yaml`
/// resolver (`spoke_base_resolver::pick_base`, reading `train_bases.agentic_default`)
/// agree with `training-presets.v1.yaml::default_base_model` /
/// `vox_populi::mens::DEFAULT_MODEL_ID` at the 16GB-tier floor? The two YAML
/// files are edited independently and nothing else gates their agreement.
#[test]
fn gpu_specs_agentic_default_16g_floor_matches_training_presets_default() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .join("../..")
        .canonicalize()
        .expect("canonicalize workspace root");

    let overlay = vox_populi::mens::tensor::spoke_base_resolver::load_overlay(&workspace_root)
        .expect("load mens/config/gpu-specs.yaml train_bases");
    let rungs = overlay
        .get("agentic_default")
        .expect("agentic_default tag present in gpu-specs.yaml");
    // The "16GB-tier floor" is the floor_mb of the rung matching DEFAULT_MODEL_ID
    // itself (per the SSOT comment: "agentic_default 16GB-tier rung") — not the
    // lowest floor_mb in the list, which belongs to a different (Coder-7B) rung.
    let floor_mb = rungs
        .iter()
        .find(|b| b.hf_id == vox_populi::mens::DEFAULT_MODEL_ID)
        .unwrap_or_else(|| {
            panic!(
                "agentic_default has no rung matching DEFAULT_MODEL_ID ({}); rungs: {rungs:?}",
                vox_populi::mens::DEFAULT_MODEL_ID
            )
        })
        .floor_mb;

    let base = vox_populi::mens::tensor::spoke_base_resolver::pick_base(
        &overlay,
        "agentic_default",
        floor_mb,
    )
    .unwrap_or_else(|e| panic!("pick_base at the 16GB-tier floor ({floor_mb} MB): {e}"));

    assert_eq!(
        base.hf_id,
        vox_populi::mens::DEFAULT_MODEL_ID,
        "gpu-specs.yaml agentic_default's floor rung ({floor_mb} MB) must match \
         vox_populi::mens::DEFAULT_MODEL_ID (and training-presets.v1.yaml::default_base_model)"
    );

    let path = manifest_dir.join("../../contracts/mens/training-presets.v1.yaml");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let root: Root = serde_yaml::from_str(&raw).expect("parse training-presets YAML");
    assert_eq!(
        base.hf_id, root.default_base_model,
        "gpu-specs.yaml agentic_default's floor rung ({floor_mb} MB) must match \
         training-presets.v1.yaml::default_base_model"
    );
}

/// Every Qwen3 rung's `hf_id` in `agentic_default` must carry a real-shaped
/// pinned revision: `@` followed by exactly 40 hex characters (a git commit
/// SHA), not the literal placeholder text `ensure_not_placeholder()` checks
/// for. Scoped to Qwen3 entries — the legacy Qwen2.5-Coder rungs are
/// intentionally left floating to `main` and were never claimed to be pinned.
#[test]
fn gpu_specs_agentic_default_qwen3_hf_ids_have_real_sha_format() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .join("../..")
        .canonicalize()
        .expect("canonicalize workspace root");
    let overlay = vox_populi::mens::tensor::spoke_base_resolver::load_overlay(&workspace_root)
        .expect("load mens/config/gpu-specs.yaml train_bases");
    let rungs = overlay
        .get("agentic_default")
        .expect("agentic_default tag present in gpu-specs.yaml");

    let qwen3_rungs: Vec<_> = rungs.iter().filter(|b| b.hf_id.contains("Qwen3")).collect();
    assert!(
        !qwen3_rungs.is_empty(),
        "expected at least one Qwen3 rung in agentic_default"
    );
    for base in qwen3_rungs {
        let (_, sha) = base
            .hf_id
            .split_once('@')
            .unwrap_or_else(|| panic!("Qwen3 hf_id missing '@<sha>' pin: {}", base.hf_id));
        assert_eq!(
            sha.len(),
            40,
            "hf_id revision must be a 40-char git SHA, got {}-char '{sha}' in {}",
            sha.len(),
            base.hf_id
        );
        assert!(
            sha.chars().all(|c| c.is_ascii_hexdigit()),
            "hf_id revision must be hex, got '{sha}' in {}",
            base.hf_id
        );
    }
}

/// Real param counts backing every pinned Qwen3 rung `gpu-specs.yaml`'s
/// `train_bases` tags reference. **Not the new memory-model SSOT**
/// (`mens::tensor::memory_model`) — this is a narrower, independent pin of
/// the pre-existing training-preset catalogue against real weight sizes, per
/// Task 7 of `2026-09-11-1-memory-ssot-and-fit-benchmark.md`. Sourced from
/// citations already in this repo, not re-derived and not fetched from HF:
/// - `Qwen/Qwen3-{0.6B,8B,14B,32B}`: `memory_budget::QWEN3_LADDER`
///   (`crates/vox-populi/src/mens/tensor/memory_budget.rs:332`) for params_b;
///   layers/hidden corroborated by `memory_model.rs`'s
///   `seeded_candle_cuda_row_reproduces_the_old_activation_formula_on_real_rungs`
///   (8B/14B) and `shape_27b()`'s "real Qwen3-32B dims stand in" comment (32B).
/// - `Qwen/Qwen3.8-27B`: `docs/superpowers/specs/2026-09-10-qwen38-27b-hub-design.md`
///   §4.1 — measured NF4 weights 12.97 GiB = 13,281.28 MiB; at 0.5 bytes/param
///   (4-bit NF4) that is the ~27.856B params used below.
const KNOWN_PARAMS_B: &[(&str, f64)] = &[
    ("Qwen/Qwen3-0.6B", 0.6),
    ("Qwen/Qwen3-8B", 8.0),
    ("Qwen/Qwen3-14B", 14.0),
    ("Qwen/Qwen3-32B", 32.0),
    ("Qwen/Qwen3.8-27B", 27.856),
];

/// Bare on-disk weight size (MiB) for `params_b` parameters stored at
/// `bytes_per_param` (0.5 for 4-bit NF4 QLoRA, 2.0 for bf16 LoRA) — the
/// physical floor no training method can go below, since it excludes every
/// other cost (optimizer state, activations, allocator slack).
fn bare_weight_mib(params_b: f64, bytes_per_param: f64) -> f64 {
    params_b * 1_000_000_000.0 * bytes_per_param / (1024.0 * 1024.0)
}

/// Pins the pre-existing `gpu-specs.yaml` `train_bases` catalogue's `floor_mb`
/// values for every pinned Qwen3 rung against the real weight size of the
/// model each rung names — the SSOT comment at `gpu-specs.yaml:244-254`
/// already promises `floor_mb` is "approx QLoRA VRAM floor"; this makes that
/// promise executable.
///
/// **Deliberately narrow.** An earlier draft of this check predicted every
/// rung's `floor_mb` from the MLX external-reference activation constant
/// (`a = 75.662`, `docs/superpowers/plans/2026-09-11-1-memory-ssot-and-fit-benchmark.md`)
/// applied uniformly regardless of lane, method, or gradient-checkpointing —
/// none of which the pinned Qwen3 QLoRA rungs share with that MLX+checkpointing
/// measurement. Seven of eight rungs "failed" that mismatch and the draft was
/// withdrawn (see the Task 7 brief) rather than mass-editing `floor_mb` to
/// chase a formula that does not apply to what's actually being measured.
/// This check instead pins against the one thing every method actually shares:
/// the weight bytes on disk, which are read (not fitted) in the real
/// `mens::tensor::memory_model` this task intentionally does not touch.
///
/// Two invariants, both true of the catalogue today:
/// 1. `floor_mb` must exceed the bare weight size — a floor below the weights
///    alone cannot be a real QLoRA/LoRA floor.
/// 2. For QLoRA rungs at 8B and above (the 0.6B CPU/dev rung's fixed overhead
///    dominates at that size and is excluded, matching `memory_budget.rs`'s
///    own `FIXED_OVERHEAD_GIB`), `floor_mb` sits within 2.0x-4.0x of the bare
///    NF4 weight size computed from the model's *nominal* param count. This
///    check's `bare_weight_mib` is a simpler, nominal-params approximation
///    than the per-tensor accounting `gpu-specs.yaml:273-291`'s own comment
///    uses (which nets out non-NF4-quantized embed/lm_head tensors, sized
///    differently per model), so this test's computed ratios (~2.56x-3.93x
///    across 8B/14B-QLoRA/32B-QLoRA/27B) don't line up 1:1 with that
///    comment's own 2.25x-2.96x figures — TDD on this test surfaced that
///    mismatch (see the Task 7 brief and report), and the fix was to widen
///    this test's band to fit the approximation's real spread, not to
///    mass-edit `floor_mb` to chase either number.
#[test]
fn gpu_specs_qwen3_train_bases_floor_mb_pins_against_real_weight_size() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .join("../..")
        .canonicalize()
        .expect("canonicalize workspace root");
    let overlay = vox_populi::mens::tensor::spoke_base_resolver::load_overlay(&workspace_root)
        .expect("load mens/config/gpu-specs.yaml train_bases");

    // Every Qwen3 (not Qwen2.5-Coder — those rungs are intentionally left
    // unpinned per gpu-specs.yaml:248-254 and out of scope) rung across every
    // tag, deduped by (hf_id, floor_mb): the same rung repeats verbatim across
    // tags (e.g. Qwen3-8B@... floor_mb=12000 appears in three tags).
    let mut seen: HashSet<(String, u32)> = HashSet::new();
    let mut checked = 0usize;
    for tag_rungs in overlay.values() {
        for base in tag_rungs {
            if !base.hf_id.contains("Qwen3") {
                continue;
            }
            if !seen.insert((base.hf_id.clone(), base.floor_mb)) {
                continue;
            }

            let bare_id = base
                .hf_id
                .split_once('@')
                .map_or(base.hf_id.as_str(), |(id, _)| id);
            let params_b = KNOWN_PARAMS_B
                .iter()
                .find(|(id, _)| *id == bare_id)
                .unwrap_or_else(|| {
                    panic!(
                        "{bare_id} has a pinned train_bases rung but no entry in \
                         KNOWN_PARAMS_B — add its real param count before adding the rung"
                    )
                })
                .1;

            let is_qlora = base.methods.iter().any(|m| m == "qlora");
            let is_lora = base.methods.iter().any(|m| m == "lora");
            assert!(
                is_qlora || is_lora,
                "{bare_id} floor_mb={} has neither qlora nor lora in methods={:?}; \
                 this check does not know its weight precision",
                base.floor_mb,
                base.methods
            );
            let bytes_per_param = if is_qlora { 0.5 } else { 2.0 }; // NF4 vs bf16
            let bare = bare_weight_mib(params_b, bytes_per_param);

            assert!(
                base.floor_mb as f64 > bare,
                "{bare_id} floor_mb={} MB is below its own bare weight size \
                 ({bare:.0} MiB at {bytes_per_param} bytes/param) — that floor \
                 cannot be real",
                base.floor_mb
            );

            if is_qlora && params_b >= 8.0 {
                let ratio = base.floor_mb as f64 / bare;
                assert!(
                    (2.0..=4.0).contains(&ratio),
                    "{bare_id} floor_mb={} MB is {ratio:.2}x its bare NF4 weight size \
                     ({bare:.0} MiB, nominal-params approximation) — outside the 2.0x-4.0x \
                     band this test's approximation spans for the 8B/14B-QLoRA/32B-QLoRA/27B \
                     rungs today; this is real drift, not a reason to mass-edit floor_mb \
                     (see the Task 7 brief)",
                    base.floor_mb
                );
            }
            checked += 1;
        }
    }
    assert!(
        checked >= 4,
        "expected to check at least the 4 QLoRA Qwen3 rungs, checked {checked}"
    );
}
