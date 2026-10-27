//! Live VRAM size selector (SIZE-02).
//!
//! Reads free accelerator memory (D-01: Metal working set with no extra margin,
//! or nvidia-smi `memory.free` x0.88), picks the largest `train_bases.qwen3_code`
//! QLoRA size whose `floor_mb` fits (D-02), and returns that size's preset
//! (D-03). Making one pick per training invocation and handing it to every spoke
//! (D-06) is the caller's job. Without a live reading, or when nothing fits, an
//! automatic pick is refused: there is no default size.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use anyhow::bail;

use crate::mens::tensor::memory_model::SAFETY_FRACTION;
use crate::mens::tensor::spoke_base_resolver::{self, TrainBase};

const NO_READING: &str = "no live free-memory reading (Metal working set or nvidia-smi memory.free); refusing to guess a size. Name one explicitly, for example --model Qwen3-8B";

/// Size (repo id without the `Qwen3-` prefix) to training preset (D-03).
const SIZE_PRESETS: [(&str, &str); 6] = [
    ("0.6B", "qwen3_dev_cpu"),
    ("1.7B", "qwen3_6g"),
    ("4B", "qwen3_12g"),
    ("8B", "qwen3_16g"),
    ("14B", "qwen3_24g"),
    ("32B", "qwen3_96g"),
];

/// A live free-memory reading in MB, with where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreeMemory {
    pub mb: u64,
    pub source: &'static str,
}

impl FreeMemory {
    /// CUDA free VRAM, less the `memory_model::SAFETY_FRACTION` margin.
    pub fn cuda(free_mb: u64) -> Self {
        Self {
            mb: (free_mb as f64 * SAFETY_FRACTION) as u64,
            source: "CUDA free memory x0.88",
        }
    }

    /// Metal working set; `query_accel_budget` already takes its margin off.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    fn metal(working_set_bytes: u64) -> Self {
        Self {
            mb: working_set_bytes / 1_048_576,
            source: "Metal working set",
        }
    }
}

/// The Metal working set as a reading. Without `vm_stat` it is only the static
/// `recommendedMaxWorkingSetSize` ceiling, not a live reading, so there is none
/// (D-01).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn metal_reading(working_set_bytes: u64, live: bool) -> Option<FreeMemory> {
    if !live {
        return None;
    }
    Some(FreeMemory::metal(working_set_bytes))
}

/// Live free accelerator memory, or `None` when nothing can be read. Never total
/// VRAM, an override, or a registry fallback.
pub fn read_free_memory() -> Option<FreeMemory> {
    #[cfg(target_os = "macos")]
    let free = crate::mens::tensor::accel_budget::query_accel_budget()
        .and_then(|b| metal_reading(b.working_set_bytes, b.working_set_is_live));
    // Not `query_accel_budget` here: off macOS it reads `get_system_vram_info`,
    // which includes the override and a total-as-free fallback.
    #[cfg(not(target_os = "macos"))]
    let free = crate::mens::tensor::vram_autodetect::query_nvidia_smi_vram()
        .map(|v| FreeMemory::cuda((v.free_gb as f64 * 1024.0) as u64));
    free.filter(|f| f.mb > 0)
}

/// One size of the Qwen3 ladder, judged by its QLoRA rung.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SizeRung {
    /// `8B`, `0.6B`, ...: the repo id's last segment without `Qwen3-`.
    pub size: String,
    pub hf_id: String,
    pub floor_mb: u32,
    pub preset: &'static str,
}

/// A chosen size and why.
#[derive(Debug, Clone)]
pub struct SizePick {
    pub rung: SizeRung,
    /// The operator named it; it is never shrunk.
    pub explicit: bool,
    pub note: String,
}

#[derive(Debug, Clone)]
pub enum Selection {
    Size(SizePick),
    /// A concrete id or path off the ladder: no floor to check.
    PassThrough(String),
}

/// Pick a size. `explicit` is `--model`, `--base` or `VOX_MENS_DEFAULT_MODEL`.
pub fn select_size(
    overlay: &HashMap<String, Vec<TrainBase>>,
    free: Option<&FreeMemory>,
    explicit: Option<&str>,
    force: bool,
) -> anyhow::Result<Selection> {
    let ladder = size_ladder(overlay)?;
    if let Some(req) = explicit {
        return match resolve_base_pin(Some(req), overlay) {
            Ok(pin) => match ladder.iter().find(|r| r.hf_id == pin) {
                Some(rung) => Ok(Selection::Size(explicit_pick(rung, free, force)?)),
                None => Ok(Selection::PassThrough(pin)),
            },
            // Not a size name (`gpt2`, a local dir, another repo): no floor to
            // check (D-04). A size-shaped name that does not resolve is a typo.
            Err(_) if !names_a_ladder_size(req) || Path::new(req).exists() => {
                Ok(Selection::PassThrough(req.to_string()))
            }
            Err(e) => Err(e),
        };
    }
    let Some(free) = free else {
        bail!("{NO_READING}")
    };
    let rung = largest_fit(&ladder, free)?;
    let note = format!(
        "picked {}: largest size whose floor {} MB fits {} MB free ({})",
        rung.size, rung.floor_mb, free.mb, free.source
    );
    Ok(Selection::Size(SizePick {
        rung: rung.clone(),
        explicit: false,
        note,
    }))
}

/// Re-check a pick right before download or load (D-05). An automatic pick that
/// still fits is kept (never stepped up); one that no longer fits steps down to
/// the largest size that does, with a note saying why. An explicit size is
/// re-judged as explicit: refused (never stepped down) when it no longer fits.
pub fn recheck_before_load(
    pick: SizePick,
    overlay: &HashMap<String, Vec<TrainBase>>,
    free_now: Option<&FreeMemory>,
    force: bool,
) -> anyhow::Result<SizePick> {
    if pick.explicit {
        return explicit_pick(&pick.rung, free_now, force);
    }
    let Some(free) = free_now else {
        bail!("{NO_READING}")
    };
    if fits(&pick.rung, free) {
        return Ok(pick);
    }
    let ladder = size_ladder(overlay)?;
    let rung = largest_fit(&ladder, free)?;
    let old = &pick.rung;
    let note = format!(
        "stepped down from {} to {}: free memory fell to {} MB ({}); {} needs {} MB",
        old.size, rung.size, free.mb, free.source, old.size, old.floor_mb
    );
    Ok(SizePick {
        rung: rung.clone(),
        explicit: false,
        note,
    })
}

/// The pin for a requested base. `None` is [`crate::mens::DEFAULT_MODEL_ID`]; a
/// `repo@<40 hex>` pin passes through; a size name (`Qwen3-8B`) or bare repo id
/// (`Qwen/Qwen3-8B`) resolves to the single distinct pinned `hf_id` across all
/// `train_bases` tags. Unknown or ambiguous input is an error that lists the
/// known sizes: it is an operator typo, and there is no fallback revision.
pub fn resolve_base_pin(
    requested: Option<&str>,
    overlay: &HashMap<String, Vec<TrainBase>>,
) -> anyhow::Result<String> {
    let Some(requested) = requested else {
        return Ok(crate::mens::DEFAULT_MODEL_ID.to_string());
    };
    if split_pin(requested).1.is_some() {
        return Ok(requested.to_string());
    }
    // (repo id, full pin) of every pinned rung.
    let pinned: Vec<(&str, &str)> = overlay
        .values()
        .flatten()
        .filter_map(|b| match split_pin(&b.hf_id) {
            (repo, Some(_)) => Some((repo, b.hf_id.as_str())),
            _ => None,
        })
        .collect();
    let suffix = format!("/{requested}");
    let pins: BTreeSet<&str> = pinned
        .iter()
        .filter(|(repo, _)| *repo == requested || repo.ends_with(&suffix))
        .map(|(_, pin)| *pin)
        .collect();
    if let [only] = pins.into_iter().collect::<Vec<_>>().as_slice() {
        return Ok((*only).to_string());
    }
    let sizes: BTreeSet<&str> = pinned
        .iter()
        .filter_map(|(repo, _)| repo.rsplit('/').next())
        .collect();
    bail!(
        "size {requested} does not resolve to exactly one pinned base; known sizes: {}; \
         or pass owner/name@<40-hex sha>",
        sizes.into_iter().collect::<Vec<_>>().join(", ")
    )
}

/// `Some(root)` reads `<root>/mens/config/gpu-specs.yaml`; `None` the embedded copy.
pub fn load_ladder_overlay(
    workspace_root: Option<&Path>,
) -> anyhow::Result<HashMap<String, Vec<TrainBase>>> {
    match workspace_root {
        Some(root) => spoke_base_resolver::load_overlay(root),
        None => spoke_base_resolver::load_embedded_overlay(),
    }
}

/// The `qwen3_code` QLoRA rungs in floor order, each with its preset. A size
/// without a preset fails closed.
fn size_ladder(overlay: &HashMap<String, Vec<TrainBase>>) -> anyhow::Result<Vec<SizeRung>> {
    let Some(rungs) = overlay.get("qwen3_code") else {
        bail!("gpu-specs.yaml train_bases has no qwen3_code ladder");
    };
    let mut ladder = Vec::new();
    for b in rungs
        .iter()
        .filter(|b| b.methods.iter().any(|m| m == "qlora"))
    {
        let repo = split_pin(&b.hf_id).0;
        let name = repo.rsplit('/').next().unwrap_or(repo);
        let size = name.strip_prefix("Qwen3-").unwrap_or(name);
        let Some(&(_, preset)) = SIZE_PRESETS.iter().find(|(s, _)| *s == size) else {
            bail!("qwen3_code rung {} has no preset for size {size}", b.hf_id);
        };
        ladder.push(SizeRung {
            size: size.to_string(),
            hf_id: b.hf_id.clone(),
            floor_mb: b.floor_mb,
            preset,
        });
    }
    ladder.sort_by_key(|r| r.floor_mb);
    Ok(ladder)
}

fn fits(rung: &SizeRung, free: &FreeMemory) -> bool {
    u64::from(rung.floor_mb) <= free.mb
}

/// A size the operator named: kept when it fits or cannot be checked, refused
/// (never shrunk) when it does not fit unless `force`.
fn explicit_pick(
    rung: &SizeRung,
    free: Option<&FreeMemory>,
    force: bool,
) -> anyhow::Result<SizePick> {
    let (size, floor) = (&rung.size, rung.floor_mb);
    let note = match free {
        None => format!("{size} (explicit): no live free-memory reading; fit not checked"),
        Some(f) if fits(rung, f) => format!(
            "{size} (explicit): floor {floor} MB fits {} MB free ({})",
            f.mb, f.source
        ),
        Some(f) => {
            if !force {
                bail!(
                    "explicit size {size} needs {floor} MB but {} MB is free ({}); refusing \
                     rather than shrinking. Choose a smaller size (training: \
                     VOX_MENS_FORCE_TRAIN=1 overrides)",
                    f.mb,
                    f.source
                );
            }
            format!(
                "{size} (explicit): floor {floor} MB does not fit {} MB free ({}); forced",
                f.mb, f.source
            )
        }
    };
    Ok(SizePick {
        rung: rung.clone(),
        explicit: true,
        note,
    })
}

// vox:defactored-from vox-eval 2026-10-27
/// `(repo id, Some(revision))` when the part after the last `@` is exactly 40
/// lowercase hex characters, else `(repo id, None)`; same rule as vox-eval's
/// `BaseLabel::from_pin`, which this `mens`-gated module cannot depend on.
fn split_pin(pin: &str) -> (&str, Option<&str>) {
    let (repo, rev) = match pin.rsplit_once('@') {
        Some((repo, rev)) => (repo, Some(rev)),
        None => (pin, None),
    };
    let hex = |r: &&str| {
        r.len() == 40
            && r.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    (repo, rev.filter(hex))
}

/// Shaped like a ladder size: `Qwen3-<n>B`, optionally `owner/` and `@rev`.
fn names_a_ladder_size(req: &str) -> bool {
    let repo = req.split('@').next().unwrap_or(req);
    let name = repo.rsplit(['/', '\\']).next().unwrap_or(repo);
    name.strip_prefix("Qwen3-")
        .and_then(|s| s.strip_suffix(['B', 'b']))
        .is_some_and(|n| n.parse::<f64>().is_ok())
}

/// The largest rung that fits, or an error naming the smallest floor.
fn largest_fit<'a>(ladder: &'a [SizeRung], free: &FreeMemory) -> anyhow::Result<&'a SizeRung> {
    if let Some(rung) = ladder.iter().rev().find(|r| fits(r, free)) {
        return Ok(rung);
    }
    let Some(smallest) = ladder.first() else {
        bail!("gpu-specs.yaml qwen3_code has no QLoRA rung");
    };
    bail!(
        "no Qwen3 size fits: the smallest ({}) needs {} MB, {} MB free ({})",
        smallest.size,
        smallest.floor_mb,
        free.mb,
        free.source
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN_8B: &str = crate::mens::DEFAULT_MODEL_ID;
    const PIN_0_6B: &str = "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca";
    const PIN_4B: &str = "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c";

    fn repo_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn repo_overlay() -> HashMap<String, Vec<TrainBase>> {
        load_ladder_overlay(Some(&repo_root())).expect("overlay")
    }

    fn free(mb: u64) -> FreeMemory {
        FreeMemory { mb, source: "test" }
    }

    fn rung(hf_id: &str, floor_mb: u32) -> TrainBase {
        TrainBase {
            hf_id: hf_id.to_string(),
            floor_mb,
            methods: vec!["qlora".into()],
            license: None,
            attribution_required: false,
            floor_measured: false,
            measured_on: None,
            peak_mb: None,
        }
    }

    fn sized(sel: Selection) -> SizePick {
        match sel {
            Selection::Size(p) => p,
            other => panic!("expected a size, got {other:?}"),
        }
    }

    fn auto(mb: u64) -> SizePick {
        sized(select_size(&repo_overlay(), Some(&free(mb)), None, false).expect("auto pick"))
    }

    #[test]
    fn cuda_reading_keeps_the_safety_margin() {
        let f = FreeMemory::cuda(10_000);
        assert_eq!(f.mb, 8_800);
        assert!(f.source.contains("x0.88"), "{}", f.source);
    }

    #[test]
    fn metal_reading_has_no_extra_margin() {
        assert_eq!(FreeMemory::metal(13_926 * 1_048_576).mb, 13_926);
    }

    #[test]
    fn metal_working_set_without_vm_stat_is_no_reading() {
        // CR-01: no vm_stat means the static driver ceiling alone; D-01 refuses it.
        assert_eq!(metal_reading(98_000 * 1_048_576, false), None);
        assert_eq!(
            metal_reading(13_926 * 1_048_576, true).map(|f| f.mb),
            Some(13_926)
        );
    }

    #[test]
    fn read_free_memory_is_live_or_absent() {
        if let Some(f) = read_free_memory() {
            assert!(f.mb > 0, "{f:?}");
            assert!(
                f.source == "Metal working set" || f.source == "CUDA free memory x0.88",
                "{f:?}"
            );
        }
    }

    #[test]
    fn ladder_is_the_six_qlora_sizes_in_floor_order() {
        let ladder = size_ladder(&repo_overlay()).expect("ladder");
        let sizes: Vec<&str> = ladder.iter().map(|r| r.size.as_str()).collect();
        assert_eq!(sizes, ["0.6B", "1.7B", "4B", "8B", "14B", "32B"]);
        let floors: Vec<u32> = ladder.iter().map(|r| r.floor_mb).collect();
        assert_eq!(floors, [2000, 5000, 9800, 12000, 20000, 60000]);
        let presets: Vec<&str> = ladder.iter().map(|r| r.preset).collect();
        assert_eq!(
            presets,
            [
                "qwen3_dev_cpu",
                "qwen3_6g",
                "qwen3_12g",
                "qwen3_16g",
                "qwen3_24g",
                "qwen3_96g"
            ]
        );
        let pins: Vec<&str> = ladder.iter().map(|r| r.hf_id.as_str()).collect();
        assert_eq!(
            pins,
            [
                "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca",
                "Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e",
                "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c",
                PIN_8B,
                "Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18",
                "Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137",
            ]
        );
        for preset in presets {
            assert!(
                crate::mens::tensor::spoke_base_resolver::KNOWN_PRESETS.contains(&preset),
                "{preset}"
            );
        }
    }

    #[test]
    fn ladder_fails_closed_on_a_size_without_a_preset() {
        let mut overlay = HashMap::new();
        overlay.insert(
            "qwen3_code".to_string(),
            vec![
                rung(PIN_8B, 12000),
                rung(
                    "Qwen/Qwen3-2B@0123456789abcdef0123456789abcdef01234567",
                    4000,
                ),
            ],
        );
        let msg = format!("{:#}", size_ladder(&overlay).expect_err("2B has no preset"));
        assert!(msg.contains("2B"), "{msg}");
    }

    #[test]
    fn auto_pick_is_the_largest_size_that_fits() {
        let p = auto(13_000);
        assert_eq!(p.rung.size, "8B");
        assert!(!p.explicit);
        assert!(
            p.note.contains("8B") && p.note.contains("13000"),
            "{}",
            p.note
        );
        for (mb, size) in [
            (12_000, "8B"),
            (11_999, "4B"),
            (200_000, "32B"),
            (2_000, "0.6B"),
        ] {
            assert_eq!(auto(mb).rung.size, size, "{mb} MB");
        }
    }

    #[test]
    fn auto_pick_refuses_without_a_reading() {
        let msg = format!(
            "{:#}",
            select_size(&repo_overlay(), None, None, false).expect_err("no reading")
        );
        assert!(msg.contains("no live free-memory reading"), "{msg}");
    }

    #[test]
    fn auto_pick_refuses_when_nothing_fits() {
        let msg = format!(
            "{:#}",
            select_size(&repo_overlay(), Some(&free(1_999)), None, false).expect_err("too small")
        );
        assert!(msg.contains("0.6B") && msg.contains("2000"), "{msg}");
    }

    #[test]
    fn embedded_ladder_matches_the_repo_file() {
        let embedded = load_ladder_overlay(None).expect("embedded overlay");
        assert_eq!(
            size_ladder(&embedded).expect("embedded ladder"),
            size_ladder(&repo_overlay()).expect("repo ladder")
        );
    }

    // ---- explicit sizes (D-04); the base_resolution_* tests moved here from
    // vox-ml-cli reasoning_compare/detect.rs with resolve_base_pin ----

    fn base(hf_id: &str) -> TrainBase {
        TrainBase {
            hf_id: hf_id.to_string(),
            floor_mb: 1,
            methods: Vec::new(),
            license: None,
            attribution_required: false,
            floor_measured: false,
            measured_on: None,
            peak_mb: None,
        }
    }

    fn ok_pin(requested: Option<&str>, overlay: &HashMap<String, Vec<TrainBase>>) -> String {
        resolve_base_pin(requested, overlay).expect("resolves")
    }

    #[test]
    fn base_resolution_default_is_the_pinned_8b() {
        assert_eq!(ok_pin(None, &repo_overlay()), PIN_8B);
        assert_eq!(
            PIN_8B,
            "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218"
        );
    }

    #[test]
    fn base_resolution_full_pin_passes_through() {
        let overlay = repo_overlay();
        for pin in [
            PIN_0_6B,
            "someone/Other-Model@0123456789abcdef0123456789abcdef01234567",
        ] {
            assert_eq!(ok_pin(Some(pin), &overlay), pin);
        }
    }

    #[test]
    fn base_resolution_size_name_uses_train_bases() {
        let overlay = repo_overlay();
        assert_eq!(ok_pin(Some("Qwen3-8B"), &overlay), PIN_8B);
        assert_eq!(ok_pin(Some("Qwen/Qwen3-8B"), &overlay), PIN_8B);
        assert_eq!(ok_pin(Some("Qwen3-0.6B"), &overlay), PIN_0_6B);
        assert_eq!(ok_pin(Some("Qwen3-4B"), &overlay), PIN_4B);
        assert_eq!(ok_pin(Some("Qwen/Qwen3-4B"), &overlay), PIN_4B);
    }

    #[test]
    fn base_resolution_resolves_every_ladder_size() {
        let overlay = repo_overlay();
        let ladder = &overlay["qwen3_code"];
        // The ls-remote pins (2026-10-27), so a changed pin in the YAML fails here.
        for (name, sha) in [
            ("Qwen3-0.6B", "c1899de289a04d12100db370d81485cdf75e47ca"),
            ("Qwen3-1.7B", "70d244cc86ccca08cf5af4e1e306ecf908b1ad5e"),
            ("Qwen3-4B", "1cfa9a7208912126459214e8b04321603b3df60c"),
            ("Qwen3-8B", "b968826d9c46dd6066d109eabc6255188de91218"),
            ("Qwen3-14B", "40c069824f4251a91eefaf281ebe4c544efd3e18"),
            ("Qwen3-32B", "9216db5781bf21249d130ec9da846c4624c16137"),
        ] {
            let pin = ok_pin(Some(name), &overlay);
            assert_eq!(pin, ok_pin(Some(&format!("Qwen/{name}")), &overlay));
            assert_eq!(pin, format!("Qwen/{name}@{sha}"), "{name}");
            assert!(split_pin(&pin).1.is_some(), "{pin}");
            assert!(ladder.iter().any(|b| b.hf_id == pin), "{pin}");
        }
    }

    #[test]
    fn base_resolution_unknown_or_ambiguous_is_an_error() {
        let overlay = repo_overlay();
        for requested in [
            "Qwen3-1234B",
            "Qwen3-8b",
            "Qwen/Qwen3-8B@main",
            "Qwen/Qwen3-8B@B968826D9C46DD6066D109EABC6255188DE91218",
        ] {
            let msg = format!(
                "{:#}",
                resolve_base_pin(Some(requested), &overlay).expect_err(requested)
            );
            assert!(
                msg.contains(requested) && msg.contains("Qwen3-8B") && msg.contains("Qwen3-0.6B"),
                "{msg}"
            );
        }
        let mut ambiguous = HashMap::new();
        ambiguous.insert(
            "a".to_string(),
            vec![base(
                "Qwen/Qwen3-8B@0123456789abcdef0123456789abcdef01234567",
            )],
        );
        ambiguous.insert("b".to_string(), vec![base(PIN_8B)]);
        assert!(resolve_base_pin(Some("Qwen3-8B"), &ambiguous).is_err());
    }

    fn explicit(req: &str, free_mb: Option<u64>, force: bool) -> anyhow::Result<Selection> {
        let f = free_mb.map(free);
        select_size(&repo_overlay(), f.as_ref(), Some(req), force)
    }

    #[test]
    fn explicit_size_that_fits_is_kept() {
        for req in ["Qwen3-8B", "Qwen/Qwen3-8B", PIN_8B] {
            let p = sized(explicit(req, Some(13_000), false).expect(req));
            assert!(p.explicit, "{req}");
            assert_eq!(p.rung.hf_id, PIN_8B, "{req}");
            assert_eq!(p.rung.size, "8B", "{req}");
        }
    }

    #[test]
    fn explicit_size_that_does_not_fit_is_refused_not_shrunk() {
        let msg = format!(
            "{:#}",
            explicit("Qwen3-14B", Some(13_000), false).expect_err("14B does not fit")
        );
        for part in ["14B", "20000", "13000", "refusing"] {
            assert!(msg.contains(part), "{part}: {msg}");
        }
        let p = sized(explicit("Qwen3-14B", Some(13_000), true).expect("forced"));
        assert_eq!(p.rung.size, "14B");
        assert!(p.note.contains("forced"), "{}", p.note);
    }

    #[test]
    fn explicit_size_without_a_reading_is_kept_unchecked() {
        let p = sized(explicit("Qwen3-32B", None, false).expect("kept"));
        assert_eq!(p.rung.size, "32B");
        assert!(p.explicit);
        assert!(p.note.contains("fit not checked"), "{}", p.note);
    }

    #[test]
    fn concrete_ids_off_the_ladder_pass_through() {
        for free_mb in [Some(13_000), None] {
            for id in [
                "org/Manual",
                "Qwen/Qwen2.5-Coder-0.5B-Instruct",
                "someone/Other-Model@0123456789abcdef0123456789abcdef01234567",
                "/models/qwen3-0.6b-finetune",
                "Qwen/Qwen3-Coder-30B-A3B-Instruct",
                // WR-03: bare ids without a slash pass through too (D-04).
                "gpt2",
                "my-local-dir",
                // An existing relative dir (tests run in the crate dir).
                "src",
            ] {
                match explicit(id, free_mb, false).expect(id) {
                    Selection::PassThrough(got) => assert_eq!(got, id),
                    other => panic!("{id}: expected pass-through, got {other:?}"),
                }
            }
        }
    }

    #[test]
    fn unknown_bare_size_name_is_an_error() {
        let msg = format!(
            "{:#}",
            explicit("Qwen3-1234B", Some(13_000), false).expect_err("unknown size")
        );
        for part in ["Qwen3-1234B", "Qwen3-8B", "Qwen3-0.6B"] {
            assert!(msg.contains(part), "{part}: {msg}");
        }
        // Size-shaped names stay typos, with or without an owner or revision.
        for typo in ["Qwen3-8b", "Qwen3-8B@main", "Qwen/Qwen3-8B@main"] {
            assert!(explicit(typo, Some(13_000), false).is_err(), "{typo}");
        }
    }

    // ---- re-check before load (D-05) ----

    fn recheck(pick: SizePick, free_mb: Option<u64>, force: bool) -> anyhow::Result<SizePick> {
        let f = free_mb.map(free);
        recheck_before_load(pick, &repo_overlay(), f.as_ref(), force)
    }

    #[test]
    fn recheck_keeps_an_auto_pick_that_still_fits() {
        let pick = auto(13_000);
        let again = recheck(pick.clone(), Some(200_000), false).expect("still fits");
        assert_eq!(again.rung.size, "8B", "never steps up");
        assert!(!again.explicit);
        assert_eq!(again.note, pick.note);
    }

    #[test]
    fn recheck_steps_an_auto_pick_down_and_says_why() {
        let pick = auto(21_000);
        assert_eq!(pick.rung.size, "14B");
        let down = recheck(pick, Some(13_000), false).expect("8B still fits");
        assert_eq!(down.rung.size, "8B");
        assert!(!down.explicit);
        for part in ["stepped down", "14B", "8B", "13000", "20000"] {
            assert!(down.note.contains(part), "{part}: {}", down.note);
        }
    }

    #[test]
    fn recheck_refuses_an_auto_pick_without_a_reading() {
        let msg = format!(
            "{:#}",
            recheck(auto(13_000), None, false).expect_err("no reading")
        );
        assert!(msg.contains("no live free-memory reading"), "{msg}");
    }

    #[test]
    fn recheck_refuses_when_nothing_fits_any_more() {
        let msg = format!(
            "{:#}",
            recheck(auto(13_000), Some(1_000), false).expect_err("nothing fits")
        );
        assert!(msg.contains("0.6B"), "{msg}");
    }

    #[test]
    fn recheck_never_steps_an_explicit_size_down() {
        let pick = sized(explicit("Qwen3-14B", None, false).expect("kept unchecked"));
        let msg = format!(
            "{:#}",
            recheck(pick.clone(), Some(13_000), false).expect_err("refused, not shrunk")
        );
        assert!(msg.contains("refusing"), "{msg}");
        let forced = recheck(pick, Some(13_000), true).expect("forced");
        assert_eq!(forced.rung.size, "14B");
        assert!(forced.explicit);
    }
}
