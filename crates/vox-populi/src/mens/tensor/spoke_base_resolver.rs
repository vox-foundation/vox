//! Resolve a spoke capability tag -> concrete fine-tunable base that fits VRAM.
//! Overlay source: `train_bases:` in mens/config/gpu-specs.yaml. Pure core +
//! a thin disk loader; reuses vram_autodetect for the live VRAM number.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Preset names accepted by `--preset` / planner normalization.
///
/// **Contract SSOT:** mirror every entry in `contracts/mens/training-presets.v1.yaml` (enforced by
/// `vox-populi` integration test `training_presets_yaml_contract`).
///
/// Lives here (under the plain `mens` feature) rather than in `preset_schema`
/// (gated behind `mens-train`/`mens-cloud`) so `spoke_validate` — and the
/// lightweight `vox ci spoke-check` gate that runs it — can validate a spoke's
/// `base.preset` without pulling in the full Candle/QLoRA stack.
/// `preset_schema` re-exports this constant so existing callers are unaffected.
pub const KNOWN_PRESETS: &[&str] = &[
    "tiny",
    "safe",
    "4080",
    "4080_safe",
    "qwen_4080_16g",
    "qwen_small_8g",
    "qwen_rtx3090_24g",
    "qwen_a100_80g",
    "a100",
    "default",
    "distributed",
    "mobile_edge",
    // Code-generation fine-tune preset (Vox .box target language).
    "vox-gen",
    // Qwen3 dense ladder presets — additive alongside legacy qwen_* presets.
    "qwen3_dev_cpu", // Qwen3-0.6B r8, CPU smoke — no quality gate
    "qwen3_6g",      // Qwen3-1.7B QLoRA (6 GB class), live selector rung
    "qwen3_12g",     // Qwen3-4B QLoRA (10-12 GB class), live selector rung
    "qwen3_16g",     // Qwen3-8B QLoRA r16 (RTX 4080 Super 16GB)
    "qwen3_24g",     // Qwen3-14B QLoRA r32 (3090/4090 24GB)
    "qwen3_48g",     // Qwen3-14B LoRA r32 un-quantized (48GB)
    "qwen3_96g",     // Qwen3-32B QLoRA r64 (96GB)
];

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TrainBase {
    pub hf_id: String,
    pub floor_mb: u32,
    #[serde(default)]
    pub methods: Vec<String>,
    /// SPDX-ish license identifier for this exact base checkpoint (e.g.
    /// `apache-2.0`, `qwen-research`). `None` when this rung has no known
    /// license mapping yet — `resolve_license_class` treats that as a hard
    /// error unless the caller passes an explicit override, rather than
    /// silently defaulting to any particular license.
    #[serde(default)]
    pub license: Option<String>,
    /// Whether downstream publication of an artifact trained from this base
    /// must carry an attribution notice per the base's license terms.
    #[serde(default)]
    pub attribution_required: bool,
    /// True only after `floor_mb` was measured on hardware; otherwise it is an
    /// estimate. Deliberately has no serde default: a rung without the key
    /// fails to load.
    pub floor_measured: bool,
    /// Host and accelerator of the measurement; present only when `floor_measured`.
    #[serde(default)]
    pub measured_on: Option<String>,
    /// Measured peak memory (MB); present only when `floor_measured`.
    #[serde(default)]
    pub peak_mb: Option<u32>,
}

impl TrainBase {
    /// One-line floor report, e.g. `qlora floor 7400 MB is an estimate (floor unmeasured)`.
    pub fn floor_label(&self) -> String {
        let methods = self.methods.join("+");
        if !self.floor_measured {
            return format!(
                "{methods} floor {} MB is an estimate (floor unmeasured)",
                self.floor_mb
            );
        }
        format!(
            "{methods} floor {} MB measured on {} (peak {} MB)",
            self.floor_mb,
            self.measured_on.as_deref().unwrap_or("?"),
            self.peak_mb
                .map_or_else(|| "?".to_string(), |p| p.to_string())
        )
    }
}

/// Every rung whose `hf_id` equals `hf_id` exactly (a different revision or a
/// bare repo id matches nothing), across all tags, sorted by `(floor_mb,
/// methods)`. Only fully equal rungs are deduped, so an inconsistent copy of a
/// repeated rung stays visible.
pub fn rungs_for_hf_id<'a>(
    overlay: &'a HashMap<String, Vec<TrainBase>>,
    hf_id: &str,
) -> Vec<&'a TrainBase> {
    let mut rungs: Vec<&TrainBase> = overlay
        .values()
        .flatten()
        .filter(|b| b.hf_id == hf_id)
        .collect();
    rungs.sort_by(|a, b| (a.floor_mb, &a.methods).cmp(&(b.floor_mb, &b.methods)));
    rungs.dedup_by(|a, b| a == b);
    rungs
}

/// Train-start report of the base's ladder rungs: one [`TrainBase::floor_label`]
/// per rung of `hf_id`. Errs when the overlay does not parse, so a malformed
/// gpu-specs.yaml is reported rather than read as "no rungs".
pub fn floor_notes(
    workspace_root: Option<&std::path::Path>,
    hf_id: &str,
) -> anyhow::Result<Vec<String>> {
    let overlay = match workspace_root {
        Some(root) => load_overlay(root)?,
        None => load_embedded_overlay()?,
    };
    Ok(rungs_for_hf_id(&overlay, hf_id)
        .iter()
        .map(|b| b.floor_label())
        .collect())
}

/// Largest candidate for `tag` whose `floor_mb <= vram_mb`. Errors if the tag is
/// unknown or nothing fits (fail-closed — never silently pick a too-big base).
pub fn pick_base<'a>(
    overlay: &'a HashMap<String, Vec<TrainBase>>,
    tag: &str,
    vram_mb: u32,
) -> anyhow::Result<&'a TrainBase> {
    let candidates = overlay.get(tag).ok_or_else(|| {
        anyhow::anyhow!("unknown base tag '{tag}' (not in gpu-specs train_bases)")
    })?;
    candidates
        .iter()
        .filter(|b| b.floor_mb <= vram_mb)
        .max_by_key(|b| b.floor_mb)
        .ok_or_else(|| anyhow::anyhow!("no '{tag}' base fits {vram_mb}MB VRAM"))
}

#[derive(Debug, Deserialize)]
struct GpuSpecsTrainBases {
    #[serde(default)]
    train_bases: HashMap<String, Vec<TrainBase>>,
}

fn parse_overlay(raw: &str) -> anyhow::Result<HashMap<String, Vec<TrainBase>>> {
    let parsed: GpuSpecsTrainBases = serde_yaml::from_str(raw)
        .map_err(|e| anyhow::anyhow!("parse train_bases in gpu-specs.yaml: {e}"))?;
    for (tag, rungs) in &parsed.train_bases {
        for b in rungs {
            let has_host = b
                .measured_on
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty());
            let has_peak = b.peak_mb.is_some();
            let ok = if b.floor_measured {
                // pick_base trusts floor_mb, so the measured peak must fit under it.
                has_host && b.peak_mb.is_some_and(|p| p > 0 && p <= b.floor_mb)
            } else {
                b.measured_on.is_none() && !has_peak
            };
            if !ok {
                anyhow::bail!(
                    "train_bases.{tag} rung {}: floor_measured is {} but measured_on/peak_mb \
                     disagree; a measured floor needs measured_on and a peak_mb in \
                     1..=floor_mb, an estimate carries neither",
                    b.hf_id,
                    b.floor_measured
                );
            }
        }
    }
    Ok(parsed.train_bases)
}

/// Compile-time copy so an installed `vox` can resolve `agentic_default`
/// without a Vox Cargo workspace checkout.
pub(crate) fn load_embedded_overlay() -> anyhow::Result<HashMap<String, Vec<TrainBase>>> {
    const EMBEDDED: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../mens/config/gpu-specs.yaml"
    ));
    parse_overlay(EMBEDDED)
}

pub fn load_overlay(root: &std::path::Path) -> anyhow::Result<HashMap<String, Vec<TrainBase>>> {
    let p = root.join("mens/config/gpu-specs.yaml");
    match std::fs::read_to_string(&p) {
        Ok(s) => parse_overlay(&s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => load_embedded_overlay(),
        Err(e) => Err(anyhow::anyhow!("read {}: {e}", p.display())),
    }
}

/// License metadata for a concrete `hf_id` (exact match, including any pinned
/// `@sha` suffix), looked up across every `train_bases` rung regardless of tag.
/// `Ok(None)` when `hf_id` doesn't appear in any known rung (e.g. an ad hoc
/// `--model` that isn't in `gpu-specs.yaml`); `Err` when the overlay does not parse.
fn license_for_hf_id(
    workspace_root: Option<&std::path::Path>,
    hf_id: &str,
) -> anyhow::Result<Option<(Option<String>, bool)>> {
    let overlay = match workspace_root {
        Some(root) => load_overlay(root)?,
        None => load_embedded_overlay()?,
    };
    Ok(overlay
        .values()
        .flatten()
        .find(|b| b.hf_id == hf_id)
        .map(|b| (b.license.clone(), b.attribution_required)))
}

/// Conservative attribution-requirement default for a `--license-class` override whose
/// base has no `gpu-specs.yaml` entry to confirm it against. Only a small explicit
/// allowlist of licenses with no attribution obligation is treated as permissive;
/// everything else (including anything unrecognized) defaults to `true`. Wrongly
/// requiring attribution is a labeling nuisance; wrongly suppressing it for a
/// license that needs it is a compliance violation — this defaults to the safer
/// direction instead of assuming permissive like the bug this replaces did.
fn default_attribution_required(license_class: &str) -> bool {
    const NO_ATTRIBUTION_REQUIRED: &[&str] = &[
        "apache-2.0",
        "mit",
        "bsd-2-clause",
        "bsd-3-clause",
        "unlicense",
        "cc0-1.0",
    ];
    !NO_ATTRIBUTION_REQUIRED.contains(&license_class.trim().to_ascii_lowercase().as_str())
}

/// Resolve the effective `(license_class, attribution_required)` for a training
/// or merge run: an explicit `cli_override` always wins; otherwise the license
/// comes from the resolved base's `license:` entry in `gpu-specs.yaml`
/// (matched by exact `hf_id`). Returns a hard `Err` — never a silent default
/// like `apache-2.0` — when neither source provides a license class, so a
/// missing mapping surfaces immediately instead of mislabeling every trained
/// model's contract identity as license-free.
pub fn resolve_license_class(
    workspace_root: Option<&std::path::Path>,
    hf_id: Option<&str>,
    cli_override: Option<String>,
) -> anyhow::Result<(String, bool)> {
    let resolved = match hf_id {
        Some(id) => license_for_hf_id(workspace_root, id)?,
        None => None,
    };
    if let Some(explicit) = cli_override {
        // If the base IS in gpu-specs.yaml, trust its recorded attribution_required — it was
        // reviewed when the entry was added. If the base is unmapped (`resolved` is `None`:
        // no hf_id, or an hf_id not in `train_bases`), there is nothing to trust, so fall back
        // to `default_attribution_required` rather than silently assuming `false` — the same
        // "never silently default toward permissive" rule this function already enforces for
        // the missing-license-class case below.
        let attribution_required = resolved
            .map(|(_, attr)| attr)
            .unwrap_or_else(|| default_attribution_required(&explicit));
        return Ok((explicit, attribution_required));
    }
    match resolved {
        Some((Some(license), attribution_required)) => Ok((license, attribution_required)),
        _ => anyhow::bail!(
            "no license_class resolved for base model {hf_id:?}: it has no `license:` entry in \
             mens/config/gpu-specs.yaml train_bases (or isn't a known train_bases rung at all), \
             and no --license-class override was given. Pass --license-class explicitly, or add \
             a `license:` entry for this base in gpu-specs.yaml — never assume a default license."
        ),
    }
}

/// Fail-closed placeholder guard for the real train / dispatch path.
///
/// All Qwen3 ladder rungs now carry real pinned HF commit SHAs in
/// `gpu-specs.yaml`. This guard remains in place to reject any future rung added
/// with an `@PLACEHOLDER-*` revision before it is pinned. Training against an
/// unpinned base is unsafe (non-reproducible, may resolve to a moving `main`),
/// so this guard rejects any resolved id/revision whose text contains "PLACEHOLDER"
/// (case-insensitive).
///
/// Call this at the base-resolution boundary on the **actual** train/dispatch
/// path (after resolving the concrete `hf_id`), NOT on a `--dry-run`/plan path —
/// planning may still print a plan containing a placeholder id and exit 0.
pub fn ensure_not_placeholder(resolved_hf_id: &str) -> anyhow::Result<()> {
    if resolved_hf_id.to_ascii_lowercase().contains("placeholder") {
        anyhow::bail!(
            "base revision is a placeholder ('{resolved_hf_id}') — pin a real HF commit SHA \
             in gpu-specs.yaml (hf_id@<sha>) before training"
        );
    }
    Ok(())
}

/// Resolve `base.model` to a concrete HF id.
/// - concrete id (contains '/') -> pass-through (no VRAM needed).
/// - capability tag -> overlay + VRAM fit.
///
/// `vram_mb_override`: Some(v) for tests / known hosts; None -> vram_autodetect.
/// On None VRAM with a tag, returns Err (fail-closed) — callers that must NOT
/// require a GPU (e.g. --skip-train dry-runs) should treat Err as "defer to the
/// existing default-model path" rather than aborting (see Phase 2 / §E).
pub fn resolve_base_model(
    root: &std::path::Path,
    base_model: &str,
    vram_mb_override: Option<u32>,
) -> anyhow::Result<String> {
    if base_model.contains('/') {
        return Ok(base_model.to_string());
    }
    let overlay = load_overlay(root)?;
    let vram_mb = match vram_mb_override {
        Some(v) => v,
        None => {
            let gb =
                crate::mens::tensor::vram_autodetect::get_system_vram_gb().ok_or_else(|| {
                    anyhow::anyhow!("no GPU VRAM detected; cannot size base tag '{base_model}'")
                })?;
            (gb * 1024.0) as u32
        }
    };
    Ok(pick_base(&overlay, base_model, vram_mb)?.hf_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlay() -> HashMap<String, Vec<TrainBase>> {
        let mut m = HashMap::new();
        m.insert(
            "strong_code_default".into(),
            vec![
                TrainBase {
                    hf_id: "small".into(),
                    floor_mb: 6000,
                    methods: vec!["qlora".into()],
                    license: Some("apache-2.0".into()),
                    attribution_required: false,
                    floor_measured: false,
                    measured_on: None,
                    peak_mb: None,
                },
                TrainBase {
                    hf_id: "big".into(),
                    floor_mb: 11000,
                    methods: vec!["qlora".into()],
                    license: None,
                    attribution_required: false,
                    floor_measured: false,
                    measured_on: None,
                    peak_mb: None,
                },
            ],
        );
        m
    }

    #[test]
    fn picks_largest_that_fits() {
        assert_eq!(
            pick_base(&overlay(), "strong_code_default", 16384)
                .unwrap()
                .hf_id,
            "big"
        );
        assert_eq!(
            pick_base(&overlay(), "strong_code_default", 8000)
                .unwrap()
                .hf_id,
            "small"
        );
    }

    #[test]
    fn errors_when_none_fit() {
        assert!(pick_base(&overlay(), "strong_code_default", 4000).is_err());
    }

    #[test]
    fn errors_unknown_tag() {
        assert!(pick_base(&overlay(), "nope", 16384).is_err());
    }

    #[test]
    fn resolves_repo_tag_with_injected_vram() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let id = resolve_base_model(root, "strong_code_default", Some(44_000)).unwrap();
        assert!(id.contains("Qwen3"), "got {id}");
        // Below every Qwen3 rung: fail closed, never a retired Qwen2.5 base.
        let err = resolve_base_model(root, "strong_code_default", Some(16384)).unwrap_err();
        assert!(
            err.to_string()
                .contains("no 'strong_code_default' base fits"),
            "{err}"
        );
    }

    #[test]
    fn concrete_id_passthrough_needs_no_vram() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        assert_eq!(
            resolve_base_model(root, "org/My-Model", None).unwrap(),
            "org/My-Model"
        );
    }

    #[test]
    fn qwen3_code_24g_returns_14b_qlora() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let overlay = load_overlay(root).expect("load overlay");
        let floor = qlora_floor(&overlay, "qwen3_code", "Qwen/Qwen3-14B@");
        let base = pick_base(&overlay, "qwen3_code", floor).expect("14B QLoRA fits");
        assert!(
            base.hf_id.contains("Qwen3-14B") && base.methods.iter().any(|m| m == "qlora"),
            "expected Qwen3-14B QLoRA at its floor {floor} MB, got: {} {:?}",
            base.hf_id,
            base.methods
        );
        assert!(
            base.hf_id.contains('@'),
            "hf_id must be revision-pinned (contains @): {}",
            base.hf_id
        );
    }

    #[test]
    fn qwen3_code_fail_closed_below_floor() {
        // Lowest qwen3_code rung floor follows the measured 0.6B QLoRA peak (Phase 320).
        // Anything below that floor must fail-closed — no base should be returned.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let overlay = load_overlay(root).expect("load overlay");
        let result = pick_base(&overlay, "qwen3_code", 1_000);
        assert!(
            result.is_err(),
            "below 2GB floor must fail-closed, got {:?}",
            result.ok().map(|b| &b.hf_id)
        );
    }

    #[test]
    fn placeholder_id_rejected_on_real_path() {
        // BLOCKER 3: a resolved id whose revision is a placeholder must fail-closed
        // before any download / dispatch on the real train path.
        let resolved = "Qwen/Qwen3-14B@PLACEHOLDER-c4e8f122";
        let err = ensure_not_placeholder(resolved).unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("placeholder"),
            "error must mention placeholder, got: {err}"
        );
        // Case-insensitive match.
        assert!(ensure_not_placeholder("org/Model@placeholder-abc").is_err());
    }

    #[test]
    fn pinned_id_passes_placeholder_guard() {
        // A real pinned revision (no PLACEHOLDER text) must pass.
        assert!(ensure_not_placeholder("Qwen/Qwen3-14B@a1b2c3d4e5f6").is_ok());
        assert!(ensure_not_placeholder("Qwen/Qwen2.5-Coder-7B-Instruct").is_ok());
    }

    /// Floor of the `tag` QLoRA rung whose `hf_id` starts with `prefix`.
    fn qlora_floor(overlay: &HashMap<String, Vec<TrainBase>>, tag: &str, prefix: &str) -> u32 {
        overlay[tag]
            .iter()
            .find(|b| b.hf_id.starts_with(prefix) && b.methods.iter().any(|m| m == "qlora"))
            .unwrap_or_else(|| panic!("{tag} has no QLoRA rung {prefix}"))
            .floor_mb
    }

    #[test]
    fn strong_code_default_16g_resolves_qwen3_8b() {
        // USER DECISION (Qwen3 everywhere): once free memory clears the Qwen3-8B
        // QLoRA floor, strong_code_default resolves Qwen3-8B.
        let overlay = load_overlay(workspace_root()).expect("load overlay");
        let floor = qlora_floor(&overlay, "strong_code_default", "Qwen/Qwen3-8B@");
        let base = pick_base(&overlay, "strong_code_default", floor).expect("a base fits");
        assert!(
            base.hf_id.contains("Qwen3-8B"),
            "{floor} MB strong_code_default must resolve Qwen3-8B (Qwen3 everywhere), got: {}",
            base.hf_id
        );
    }

    #[test]
    fn agentic_default_16g_resolves_qwen3_8b() {
        // USER DECISION (Qwen3 everywhere): the agentic_default ladder must also
        // resolve Qwen3-8B once free memory clears its QLoRA floor.
        let overlay = load_overlay(workspace_root()).expect("load overlay");
        let floor = qlora_floor(&overlay, "agentic_default", "Qwen/Qwen3-8B@");
        let base = pick_base(&overlay, "agentic_default", floor).expect("a base fits");
        assert!(
            base.hf_id.contains("Qwen3-8B"),
            "{floor} MB agentic_default must resolve Qwen3-8B (Qwen3 everywhere), got: {}",
            base.hf_id
        );
    }

    /// Qwen 2.5 is a retired family (AGENTS.md Retired Surfaces: use Qwen 3+),
    /// so no train_bases tag may offer a Qwen2.5 rung for any selector to pick.
    #[test]
    fn no_train_base_tag_offers_a_retired_qwen2_5_rung() {
        let overlay = load_overlay(workspace_root()).expect("load overlay");
        for (tag, rungs) in &overlay {
            for b in rungs {
                assert!(
                    !b.hf_id.contains("Qwen2.5"),
                    "train_bases.{tag} offers retired {}",
                    b.hf_id
                );
            }
        }
    }

    #[test]
    fn small_code_default_caps_at_8b() {
        // small_code_default is documented to cap at 8B — adding the Qwen3-8B rung to
        // the strong/agentic ladders must NOT change small_code_default's cap.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let overlay = load_overlay(root).expect("load overlay");
        // Even on a huge card, small_code_default must not exceed an 8B rung.
        let base = pick_base(&overlay, "small_code_default", 1_000_000).expect("a base fits");
        assert!(
            !base.hf_id.contains("14B")
                && !base.hf_id.contains("32B")
                && !base.hf_id.contains("72B"),
            "small_code_default must cap at 8B, got: {}",
            base.hf_id
        );
    }

    #[test]
    fn qwen3_code_48g_prefers_unquantized_14b() {
        // At 48GB: 14B-LoRA (floor ~44GB) should beat 14B-QLoRA (floor ~20GB)
        // because max_by_key(floor_mb) picks the highest floor that fits.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let overlay = load_overlay(root).expect("load overlay");
        let base = pick_base(&overlay, "qwen3_code", 49_152).expect("14B-LoRA fits at 48GB"); // 48 GB GPU expressed in MB; the LoRA un-quantized rung has floor_mb=44_000
        assert!(
            base.methods.iter().any(|m| m == "lora" || m == "full_lora"),
            "at 48GB should prefer LoRA (un-quantized) over QLoRA, but got methods: {:?}",
            base.methods
        );
        assert!(
            base.hf_id.contains("Qwen3-14B"),
            "should be 14B, got: {}",
            base.hf_id
        );
    }

    fn workspace_root() -> &'static std::path::Path {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
    }

    #[test]
    fn agentic_default_rungs_pin_live_available_mb() {
        // Injected values are live-available MB (vm_stat reclaimable after
        // margin), not nameplate×0.85. 8 GB / 6144 must fail-closed — do not
        // weaken that assertion if a rung appears.
        let overlay = load_overlay(workspace_root()).expect("load overlay");
        let cases: &[(u32, Option<(&str, &str)>)] = &[
            (6144, None),
            // Below the cheapest agentic_default Qwen3 rung (Qwen3.8-27B, 34000)
            // nothing fits: fail closed rather than fall to a retired Qwen2.5 base.
            (11500, None),
            (13926, None),
            (20890, None),
            (27853, None),
            (31334, None),
            // Task 15 (plan ID P1.5): the new Qwen3.8-27B QLoRA rung
            // (floor_mb 34000) correctly displaces Qwen3-14B-LoRA at this
            // real, hardware-measured Mac memory tier — QLoRA's genuinely
            // lower memory need legitimately wins here. This is the
            // disclosed, intended consequence of adding a more capable,
            // cheaper-to-run rung, not a regression; see the scope ruling
            // in .superpowers/sdd/2026-09-10-qwen38-27b-hub/task-15-brief.md.
            (41779, Some(("Qwen3.8-27B", "qlora"))),
            (55706, Some(("Qwen3-14B", "lora"))),
            // Qwen3-8B / 14B QLoRA floors are the Metal estimate (122876 / 195382),
            // so the 14B un-quantized LoRA rung (44000, an older estimate) wins here.
            (83558, Some(("Qwen3-14B", "lora"))),
            (111411, Some(("Qwen3-32B", "lora"))),
        ];
        for &(live_mb, expected) in cases {
            let got = pick_base(&overlay, "agentic_default", live_mb);
            match expected {
                None => {
                    assert!(
                        got.is_err(),
                        "{live_mb} MB must fail-closed on agentic_default, got {:?}",
                        got.ok().map(|b| (&b.hf_id, &b.methods))
                    );
                }
                Some((hf_sub, method)) => {
                    let base = got.unwrap_or_else(|e| {
                        panic!("{live_mb} MB should resolve {hf_sub} {method}, got Err: {e}")
                    });
                    assert!(
                        base.hf_id.contains(hf_sub),
                        "{live_mb} MB: expected hf_id containing {hf_sub}, got {}",
                        base.hf_id
                    );
                    assert!(
                        base.methods.iter().any(|m| m == method),
                        "{live_mb} MB: expected methods to contain {method}, got {:?}",
                        base.methods
                    );
                }
            }
        }
    }

    #[test]
    fn mac_128g_prefers_unquantized_32b_lora() {
        // 128 GiB physical - 12 GiB GUI reserve = 116 GiB usable = 118_784 MB.
        // At that budget the un-quantized 32B rung must outrank the QLoRA one.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let overlay = load_overlay(root).expect("load overlay");
        let base = pick_base(&overlay, "strong_code_default", 118_784).expect("a base fits");
        assert!(
            base.hf_id.contains("Qwen3-32B"),
            "116 GiB should resolve Qwen3-32B, got: {}",
            base.hf_id
        );
        assert!(
            base.methods.iter().any(|m| m == "lora"),
            "at 116 GiB the un-quantized LoRA rung should win, got methods: {:?}",
            base.methods
        );
    }

    #[test]
    fn resolve_base_model_picks_27b_at_measured_big_box_memory() {
        // Task 15 (plan ID P1.5): the new Qwen3.8-27B QLoRA rung (floor_mb
        // 34000) must be resolvable at a real, hardware-measured Mac memory
        // tier, AND must NOT displace Qwen3-32B as the real 128 GB
        // (~118_784 MB) default — that pin flip is Phase 3's job, not this
        // task's. See the scope ruling in
        // .superpowers/sdd/2026-09-10-qwen38-27b-hub/task-15-brief.md.
        let root = workspace_root();

        // Positive case: the real, hardware-measured 41779 MB tier (also
        // pinned in agentic_default_rungs_pin_live_available_mb) — above the
        // new rung's floor (34000) and below the next rung up (Qwen3-14B
        // LoRA at 44000), so only the 27B rung fits. This proves
        // reachability on hardware this file already models, not merely in
        // the abstract.
        let mid = resolve_base_model(root, "agentic_default", Some(41_779))
            .expect("41_779 MB should resolve the Qwen3.8-27B QLoRA rung");
        assert_eq!(
            mid, "Qwen/Qwen3.8-27B@1d4bf0f2ff6012fd82039f2fa52739d0dd7c60c0",
            "41_779 MB agentic_default must resolve Qwen3.8-27B, got {mid}"
        );

        // Negative case (the non-goal): a real 128 GB Mac (116 GiB usable =
        // 118_784 MB) must still resolve Qwen3-32B, not Qwen3.8-27B — no pin
        // flip at the top end.
        let big_box = resolve_base_model(root, "agentic_default", Some(118_784))
            .expect("118_784 MB should resolve a base");
        assert_eq!(
            big_box, "Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137",
            "118_784 MB agentic_default must still resolve Qwen3-32B (no pin flip yet), got {big_box}"
        );
    }

    #[test]
    fn resolve_license_class_uses_gpu_specs_license_when_present() {
        let (license, attribution_required) = resolve_license_class(
            Some(workspace_root()),
            Some("Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218"),
            None,
        )
        .expect("Qwen3-8B has a license: entry in gpu-specs.yaml");
        assert_eq!(license, "apache-2.0");
        assert!(!attribution_required);
    }

    /// Qwen3.8-27B has no `license:` entry in gpu-specs.yaml today — its real
    /// license could not be confirmed with confidence, so it is deliberately
    /// left unmapped rather than guessed. Resolving it with no override must
    /// be a hard error, never a silent fallback (e.g. to `apache-2.0`).
    #[test]
    fn resolve_license_class_hard_errors_when_base_has_no_license_entry() {
        let err = resolve_license_class(
            Some(workspace_root()),
            Some("Qwen/Qwen3.8-27B@1d4bf0f2ff6012fd82039f2fa52739d0dd7c60c0"),
            None,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("no license_class resolved"),
            "got: {err}"
        );
    }

    #[test]
    fn resolve_license_class_cli_override_wins_even_for_unknown_base() {
        let (license, attribution_required) = resolve_license_class(
            Some(workspace_root()),
            Some("some/unmapped-model"),
            Some("mit".into()),
        )
        .expect("explicit override must always resolve");
        assert_eq!(license, "mit");
        assert!(
            !attribution_required,
            "mit is a known no-attribution-required permissive license"
        );
    }

    /// Regression test: an unmapped base with a `--license-class` override that names a
    /// license NOT on the known-permissive allowlist must default `attribution_required`
    /// to `true`, never silently to `false`. Before the fix, any unmapped base silently got
    /// `attribution_required = false` regardless of which license was passed — wrongly
    /// suppressing attribution for exactly the license family that needs it.
    #[test]
    fn resolve_license_class_defaults_attribution_required_true_for_unknown_license_on_unmapped_base()
     {
        let (license, attribution_required) = resolve_license_class(
            Some(workspace_root()),
            Some("some/unmapped-model"),
            Some("cc-by-nc-4.0".into()),
        )
        .expect("explicit override must always resolve");
        assert_eq!(license, "cc-by-nc-4.0");
        assert!(
            attribution_required,
            "an unrecognized, non-permissive license override on an unmapped base must \
             default to attribution_required = true, not silently false"
        );
    }

    // ---- Phase 317 (SIZE-01): floor evidence ----

    const PIN_4B: &str = "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c";

    fn qlora_rung(floor_mb: u32) -> TrainBase {
        TrainBase {
            hf_id: PIN_4B.into(),
            floor_mb,
            methods: vec!["qlora".into()],
            license: Some("apache-2.0".into()),
            attribution_required: false,
            floor_measured: false,
            measured_on: None,
            peak_mb: None,
        }
    }

    #[test]
    fn floor_label_says_floor_unmeasured_for_an_estimate() {
        let mut rung = qlora_rung(7400);
        assert_eq!(
            rung.floor_label(),
            "qlora floor 7400 MB is an estimate (floor unmeasured)"
        );
        rung.floor_measured = true;
        rung.measured_on = Some("m4-max-metal".into());
        rung.peak_mb = Some(6900);
        assert_eq!(
            rung.floor_label(),
            "qlora floor 7400 MB measured on m4-max-metal (peak 6900 MB)"
        );
    }

    #[test]
    fn overlay_rung_without_floor_measured_is_rejected() {
        let raw =
            "train_bases:\n  t:\n    - { hf_id: \"org/M@x\", floor_mb: 1, methods: [qlora] }\n";
        let e = parse_overlay(raw).expect_err("a rung without floor_measured must not load");
        assert!(format!("{e:#}").contains("floor_measured"), "{e:#}");
    }

    #[test]
    fn rungs_for_hf_id_dedupes_repeats_across_tags() {
        let overlay = load_overlay(workspace_root()).expect("load overlay");
        let floors = |id: &str| -> Vec<u32> {
            rungs_for_hf_id(&overlay, id)
                .iter()
                .map(|b| b.floor_mb)
                .collect()
        };
        assert_eq!(
            floors("Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218"),
            [122876]
        );
        assert_eq!(
            floors("Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18"),
            [44000, 195382]
        );
        assert_eq!(
            floors("Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137"),
            [100000, 295822]
        );
        assert!(floors("Qwen/Qwen3-8B@0123456789abcdef0123456789abcdef01234567").is_empty());
        assert!(floors("Qwen/Qwen3-8B").is_empty());
    }

    #[test]
    fn qwen3_code_4b_rung_is_pinned_and_carries_its_measured_floor() {
        let overlay = load_overlay(workspace_root()).expect("load overlay");
        let base = pick_base(&overlay, "qwen3_code", 33_200).expect("4B fits");
        assert_eq!(base.hf_id, PIN_4B);
        assert_eq!(base.floor_mb, 33037);
        assert_eq!(
            pick_base(&overlay, "qwen3_code", 33_036)
                .expect("1.7B fits")
                .hf_id,
            "Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e"
        );
        assert_eq!(base.license.as_deref(), Some("apache-2.0"));
        assert_eq!(
            base.floor_label(),
            "qlora floor 33037 MB measured on silicon-gpu-metal (peak 33037 MB)"
        );
        assert_eq!(
            pick_base(&overlay, "qwen3_code", 122_876)
                .expect("8B fits")
                .hf_id,
            "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218"
        );
    }

    #[test]
    fn measured_floor_needs_evidence() {
        let yaml = |extra: &str| {
            format!(
                "train_bases:\n  t:\n    - {{ hf_id: \"{PIN_4B}\", floor_mb: 7400, methods: [qlora], {extra} }}\n"
            )
        };
        for bad in [
            "floor_measured: true",
            "floor_measured: true, measured_on: m4-max-metal",
            "floor_measured: true, peak_mb: 6900, measured_on: \"  \"",
            "floor_measured: false, peak_mb: 6900",
            "floor_measured: false, measured_on: m4-max-metal",
            // A measured peak must be real and must fit under the floor it backs.
            "floor_measured: true, measured_on: m4-max-metal, peak_mb: 0",
            "floor_measured: true, measured_on: m4-max-metal, peak_mb: 7401",
        ] {
            let msg = format!("{:#}", parse_overlay(&yaml(bad)).expect_err(bad));
            assert!(
                msg.contains("floor_measured") && msg.contains(PIN_4B),
                "{bad}: {msg}"
            );
        }
        let ok = parse_overlay(&yaml(
            "floor_measured: true, measured_on: m4-max-metal, peak_mb: 6900",
        ))
        .expect("a measured rung with evidence loads");
        assert!(
            ok["t"][0]
                .floor_label()
                .contains("measured on m4-max-metal")
        );
        parse_overlay(&yaml(
            "floor_measured: true, measured_on: m4-max-metal, peak_mb: 7400",
        ))
        .expect("a peak equal to the floor loads");
        parse_overlay(&yaml(
            "floor_measured: true, measured_on: m4-max-metal, peak_mb: 1",
        ))
        .expect("the smallest real peak loads");
    }

    #[test]
    fn every_dense_ladder_pin_resolves_its_license() {
        for pin in [
            "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca",
            "Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e",
            PIN_4B,
            "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218",
            "Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18",
            "Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137",
        ] {
            assert_eq!(
                resolve_license_class(Some(workspace_root()), Some(pin), None).expect(pin),
                ("apache-2.0".to_string(), false),
                "{pin}"
            );
        }
    }

    #[test]
    fn floor_notes_name_every_rung_of_a_pin() {
        let notes = floor_notes(
            Some(workspace_root()),
            "Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18",
        )
        .expect("overlay loads");
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(
            notes.iter().all(|n| n.contains("floor unmeasured")),
            "{notes:?}"
        );
        assert!(
            notes.iter().any(|n| n.contains("195382")) && notes.iter().any(|n| n.contains("44000")),
            "{notes:?}"
        );
        assert_eq!(
            floor_notes(None, PIN_4B).expect("embedded overlay loads"),
            ["qlora floor 33037 MB measured on silicon-gpu-metal (peak 33037 MB)"]
        );
        assert!(
            floor_notes(
                Some(workspace_root()),
                "someone/Other-Model@0123456789abcdef0123456789abcdef01234567"
            )
            .expect("overlay loads")
            .is_empty()
        );
    }

    /// WR-01: a gpu-specs.yaml that fails to parse must surface its parse
    /// error, never read as "no license entry" or "no floor notes". A missing
    /// file still falls back to the embedded overlay.
    #[test]
    fn malformed_overlay_surfaces_its_parse_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("mens/config")).expect("mkdir");
        std::fs::write(
            dir.path().join("mens/config/gpu-specs.yaml"),
            format!(
                "train_bases:\n  t:\n    - {{ hf_id: \"{PIN_4B}\", floor_mb: 7400, \
                 methods: [qlora], license: apache-2.0 }}\n"
            ),
        )
        .expect("write overlay");
        let root = Some(dir.path());
        for over in [None, Some("apache-2.0".to_string())] {
            let e = resolve_license_class(root, Some(PIN_4B), over.clone())
                .expect_err("a malformed overlay must not resolve a license");
            assert!(
                format!("{e:#}").contains("parse train_bases"),
                "{over:?}: {e:#}"
            );
        }
        let e = floor_notes(root, PIN_4B).expect_err("a malformed overlay has no floor notes");
        assert!(format!("{e:#}").contains("parse train_bases"), "{e:#}");

        let absent = tempfile::tempdir().expect("tempdir");
        assert_eq!(
            floor_notes(Some(absent.path()), PIN_4B).expect("absent file uses the embedded copy"),
            ["qlora floor 33037 MB measured on silicon-gpu-metal (peak 33037 MB)"]
        );

        // Present but unreadable (here: a directory) is an error, not a silent fallback.
        let unreadable = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(unreadable.path().join("mens/config/gpu-specs.yaml"))
            .expect("mkdir");
        load_overlay(unreadable.path()).expect_err("an unreadable overlay must not fall back");
    }
}
