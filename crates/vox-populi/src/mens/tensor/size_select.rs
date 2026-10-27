//! Live VRAM size selector (SIZE-02).
//!
//! Reads free accelerator memory (D-01: Metal working set with no extra margin,
//! or nvidia-smi `memory.free` x0.88), picks the largest `train_bases.qwen3_code`
//! QLoRA size whose `floor_mb` fits (D-02), and returns that size's preset
//! (D-03). Making one pick per training invocation and handing it to every spoke
//! (D-06) is the caller's job. Without a live reading, or when nothing fits, an
//! automatic pick is refused: there is no default size.

use std::collections::HashMap;
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

/// Live free accelerator memory, or `None` when nothing can be read. Never total
/// VRAM, an override, or a registry fallback.
pub fn read_free_memory() -> Option<FreeMemory> {
    #[cfg(target_os = "macos")]
    let free = crate::mens::tensor::accel_budget::query_accel_budget()
        .map(|b| FreeMemory::metal(b.working_set_bytes));
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
        bail!("explicit size {req} (force {force}): explicit sizes land in Task 2");
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
        let repo = b.hf_id.split_once('@').map_or(b.hf_id.as_str(), |(r, _)| r);
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
    use std::path::Path;

    const PIN_8B: &str = "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218";

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
}
