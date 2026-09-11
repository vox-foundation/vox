//! Resolve a spoke capability tag -> concrete fine-tunable base that fits VRAM.
//! Overlay source: `train_bases:` in mens/config/gpu-specs.yaml. Pure core +
//! a thin disk loader; reuses vram_autodetect for the live VRAM number.
use serde::Deserialize;
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
    "qwen3_16g",     // Qwen3-8B QLoRA r16 (RTX 4080 Super 16GB)
    "qwen3_24g",     // Qwen3-14B QLoRA r32 (3090/4090 24GB)
    "qwen3_48g",     // Qwen3-14B LoRA r32 un-quantized (48GB)
    "qwen3_96g",     // Qwen3-32B QLoRA r64 (96GB)
];

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TrainBase {
    pub hf_id: String,
    pub floor_mb: u32,
    #[serde(default)]
    pub methods: Vec<String>,
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
    Ok(parsed.train_bases)
}

/// Compile-time copy so an installed `vox` can resolve `agentic_default`
/// without a Vox Cargo workspace checkout.
fn load_embedded_overlay() -> anyhow::Result<HashMap<String, Vec<TrainBase>>> {
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
        Err(_) => load_embedded_overlay(),
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

/// Fail-closed Metal default: pick the largest `agentic_default` rung that
/// fits `vram_mb`. Used by `vox mens train` **before** the CandleQlora
/// `DEFAULT_MODEL_ID` fill so a Mac does not silently land on Qwen3-8B.
///
/// `workspace_root` may be `None` (installed binary); the overlay then comes
/// from the compile-time `gpu-specs.yaml` embed.
pub fn resolve_metal_default_base(
    workspace_root: Option<&std::path::Path>,
    vram_mb: u64,
) -> anyhow::Result<String> {
    let overlay = match workspace_root {
        Some(root) => load_overlay(root)?,
        None => load_embedded_overlay()?,
    };
    let base = pick_base(&overlay, "agentic_default", vram_mb as u32).map_err(|e| {
        anyhow::anyhow!(
            "{e} (live-available unified memory: {vram_mb} MB; agentic_default floor is 11000 MB). \
             Pass --model, set VOX_MENS_DEFAULT_MODEL, free memory, or set \
             VOX_MENS_DISABLE_LIVE_MEM=1 to use the static nameplate reserve."
        )
    })?;
    ensure_not_placeholder(&base.hf_id)?;
    Ok(base.hf_id.clone())
}

/// CLI precedence for the Metal default base: `--model` and
/// `VOX_MENS_DEFAULT_MODEL` win; explicit `--device cpu`/`cuda` skip; nvidia
/// never resolves. Fail-closed when the pick itself fails.
pub fn maybe_resolve_metal_default_base(
    model: Option<&str>,
    env_default_model: Option<&str>,
    vendor: &str,
    device_best_or_metal: bool,
    workspace_root: Option<&std::path::Path>,
    vram_mb: u64,
) -> anyhow::Result<Option<String>> {
    let model_set = model.map(str::trim).filter(|s| !s.is_empty()).is_some();
    let env_set = env_default_model
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .is_some();
    if model_set || env_set || !device_best_or_metal {
        return Ok(None);
    }
    if crate::mens::tensor::vram_autodetect::AcceleratorKind::from_vendor(vendor)
        != crate::mens::tensor::vram_autodetect::AcceleratorKind::Metal
    {
        return Ok(None);
    }
    Ok(Some(resolve_metal_default_base(workspace_root, vram_mb)?))
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
                },
                TrainBase {
                    hf_id: "big".into(),
                    floor_mb: 11000,
                    methods: vec!["qlora".into()],
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
        let id = resolve_base_model(root, "strong_code_default", Some(16384)).unwrap();
        assert!(id.contains("Qwen"), "got {id}");
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
        let base = pick_base(&overlay, "qwen3_code", 24_000).expect("14B fits at 24GB");
        assert!(
            base.hf_id.contains("Qwen3-14B"),
            "expected Qwen3-14B at 24GB, got: {}",
            base.hf_id
        );
        assert!(
            base.hf_id.contains('@'),
            "hf_id must be revision-pinned (contains @): {}",
            base.hf_id
        );
    }

    #[test]
    fn qwen3_code_fail_closed_below_floor() {
        // Lowest qwen3_code rung has floor_mb=2000 (CPU/dev tier, ~0.6B QLoRA).
        // Anything below 2000 MB must fail-closed — no base should be returned.
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

    #[test]
    fn strong_code_default_16g_resolves_qwen3_8b() {
        // USER DECISION (Qwen3 everywhere): a 16GB box must resolve to Qwen3-8B,
        // not fall to Qwen2.5-Coder-7B. The Qwen3-8B rung (floor 12000) must outrank
        // the Qwen2.5-Coder-7B rung (floor 11000) at 16384 MB.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let overlay = load_overlay(root).expect("load overlay");
        let base = pick_base(&overlay, "strong_code_default", 16384).expect("a base fits at 16GB");
        assert!(
            base.hf_id.contains("Qwen3-8B"),
            "16GB strong_code_default must resolve Qwen3-8B (Qwen3 everywhere), got: {}",
            base.hf_id
        );
    }

    #[test]
    fn agentic_default_16g_resolves_qwen3_8b() {
        // USER DECISION (Qwen3 everywhere): the agentic_default ladder must also
        // resolve Qwen3-8B at 16GB (the bare no-domain default tier).
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let overlay = load_overlay(root).expect("load overlay");
        let base = pick_base(&overlay, "agentic_default", 16384).expect("a base fits at 16GB");
        assert!(
            base.hf_id.contains("Qwen3-8B"),
            "16GB agentic_default must resolve Qwen3-8B (Qwen3 everywhere), got: {}",
            base.hf_id
        );
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
        let base = pick_base(&overlay, "small_code_default", 96_000).expect("a base fits");
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
    fn resolve_metal_default_base_116g_is_qwen3_32b() {
        let id = resolve_metal_default_base(Some(workspace_root()), 116_000).expect("116k MB fits");
        assert!(
            id.contains("Qwen3-32B"),
            "116_000 MB agentic_default must resolve Qwen3-32B, got {id}"
        );
    }

    #[test]
    fn resolve_metal_default_base_13926_is_qwen3_8b() {
        let id = resolve_metal_default_base(Some(workspace_root()), 13926).expect("13926 MB fits");
        assert!(
            id.contains("Qwen3-8B"),
            "13926 MB agentic_default must resolve Qwen3-8B, got {id}"
        );
    }

    #[test]
    fn resolve_metal_default_base_6144_is_fail_closed() {
        let err = resolve_metal_default_base(Some(workspace_root()), 6144)
            .expect_err("8 GB is below floor");
        let msg = err.to_string();
        assert!(
            msg.contains("agentic_default"),
            "fail-closed error must name the tag, got {msg}"
        );
        assert!(
            msg.contains("6144"),
            "fail-closed error must name the live budget, got {msg}"
        );
        assert!(
            msg.contains("11000") && msg.contains("--model"),
            "fail-closed error must name the floor and a recovery flag, got {msg}"
        );
    }

    #[test]
    fn maybe_resolve_metal_default_base_honors_precedence() {
        let root = Some(workspace_root());
        assert_eq!(
            maybe_resolve_metal_default_base(
                Some("org/Explicit"),
                None,
                "apple",
                true,
                root,
                116_000
            )
            .expect("skip"),
            None
        );
        assert_eq!(
            maybe_resolve_metal_default_base(None, Some("org/Env"), "apple", true, root, 116_000)
                .expect("skip"),
            None
        );
        assert_eq!(
            maybe_resolve_metal_default_base(None, None, "nvidia", true, root, 116_000)
                .expect("skip"),
            None
        );
        assert_eq!(
            maybe_resolve_metal_default_base(None, None, "apple", false, root, 116_000)
                .expect("cpu skip"),
            None
        );
        let got = maybe_resolve_metal_default_base(None, None, "apple", true, root, 116_000)
            .expect("resolve")
            .expect("some");
        assert!(got.contains("Qwen3-32B"), "got {got}");
        assert!(
            maybe_resolve_metal_default_base(None, None, "apple", true, root, 6144).is_err(),
            "6144 must fail-closed before any default id"
        );
        let embedded = resolve_metal_default_base(None, 116_000).expect("embedded overlay");
        assert!(
            embedded.contains("Qwen3-32B"),
            "installed-binary fallback must still resolve 32B, got {embedded}"
        );
    }

    #[test]
    fn agentic_default_rungs_pin_live_available_mb() {
        // Injected values are live-available MB (vm_stat reclaimable after
        // margin), not nameplate×0.85. 8 GB / 6144 must fail-closed — do not
        // weaken that assertion if a rung appears.
        let overlay = load_overlay(workspace_root()).expect("load overlay");
        let cases: &[(u32, Option<(&str, &str)>)] = &[
            (6144, None),
            // 11–12 GiB window: Qwen2.5-Coder-7B still outranks nothing Qwen3.
            (11500, Some(("Qwen2.5-Coder-7B", "qlora"))),
            (13926, Some(("Qwen3-8B", "qlora"))),
            (20890, Some(("Qwen3-14B", "qlora"))),
            (27853, Some(("Qwen3-14B", "qlora"))),
            (31334, Some(("Qwen3-14B", "qlora"))),
            // Task 15 (plan ID P1.5): the new Qwen3.8-27B QLoRA rung
            // (floor_mb 34000) correctly displaces Qwen3-14B-LoRA at this
            // real, hardware-measured Mac memory tier — QLoRA's genuinely
            // lower memory need legitimately wins here. This is the
            // disclosed, intended consequence of adding a more capable,
            // cheaper-to-run rung, not a regression; see the scope ruling
            // in .superpowers/sdd/2026-09-10-qwen38-27b-hub/task-15-brief.md.
            (41779, Some(("Qwen3.8-27B", "qlora"))),
            (55706, Some(("Qwen3-14B", "lora"))),
            (83558, Some(("Qwen3-32B", "qlora"))),
            (111411, Some(("Qwen3-32B", "lora"))),
        ];
        for &(live_mb, expected) in cases {
            let got = pick_base(&overlay, "agentic_default", live_mb);
            match expected {
                None => {
                    assert!(
                        got.is_err(),
                        "8 GB / {live_mb} MB must fail-closed on agentic_default, got {:?}",
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
}
