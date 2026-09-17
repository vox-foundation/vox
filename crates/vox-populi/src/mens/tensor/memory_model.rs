//! Memory model SSOT: one measured (or honestly-seeded) activation constant
//! per `(lane, gradient_checkpointing)` cell, fail closed when uncalibrated.
//!
//! `act_bytes_per_lht` is bytes of activation memory per (layer × hidden-unit
//! × token) — the linear-in-tokens term. The weights term (`artifact_bytes`)
//! is read straight off disk, never fitted (see [`MemoryModel::predict_bytes`]).
//!
//! ## Why `candle-metal` has no calibrated row today
//!
//! As of 2026-09-11 exactly one real `candle-metal` measurement exists (see
//! [`super::calibration`]) and it is a single point — `fit_a_lane` correctly
//! refuses to fit a slope from one point. `MemoryModels::get` therefore
//! returns an error for `candle-metal` naming the lane and the fix
//! (`vox mens probe --measure`), the same fail-closed shape used for any
//! other uncalibrated cell. Nothing here borrows the old MLX-based estimate
//! for Metal — that borrowing is exactly what this SSOT exists to prevent.
//!
//! ## Why `candle-cuda` has a seeded row
//!
//! `contracts/mens/memory-model.v1.yaml` seeds `(CandleCuda, true)` from
//! constants that were calibrated in `memory_budget.rs`
//! (`RESIDENT_GIB_PER_B_PARAMS`, `FIXED_OVERHEAD_GIB`,
//! `ACT_GIB_PER_KTOK_PER_SQRTB`) at the time this row was seeded, so the
//! 4080 Super lane kept working while it awaited a real measurement. Those
//! constants were themselves deleted from `memory_budget.rs` by Task 8 of
//! `2026-09-11-1-memory-ssot-and-fit-benchmark` (this SSOT migration's own
//! ladder-deletion sweep) — see the module doc comment on
//! `memory_budget.rs` and the `seeded_candle_cuda_row_reproduces_the_old_
//! activation_formula_on_real_rungs` test below, which independently
//! reproduces the formula inline rather than importing the deleted
//! constants. `CalSource::Seeded` marks this row as inherited, not measured.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};

use super::accel_budget::AccelBudget;

/// The two real Vox training backends. MLX is intentionally absent — it is
/// an external reference measurement in `docs/src/architecture/measurements/`,
/// never a lane Vox trains on; see [`super::calibration::best_record`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    CandleCuda,
    CandleMetal,
}

impl Lane {
    fn as_str(self) -> &'static str {
        match self {
            Lane::CandleCuda => "candle-cuda",
            Lane::CandleMetal => "candle-metal",
        }
    }

    fn parse(raw: &str) -> Result<Self> {
        match raw {
            "candle-cuda" => Ok(Lane::CandleCuda),
            "candle-metal" => Ok(Lane::CandleMetal),
            other => bail!("unknown lane {other:?}; expected candle-cuda or candle-metal"),
        }
    }
}

/// A calibration cell. Constructing `(CandleMetal, gradient_checkpointing:
/// true)` is a compile-time-adjacent error, not a runtime `Uncalibrated`:
/// candle-metal does not implement gradient checkpointing at all
/// (`candle_qlora_train/mod.rs` warns about exactly this gap today), so that
/// cell should never exist rather than merely be empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CalKey {
    lane: Lane,
    gradient_checkpointing: bool,
}

impl CalKey {
    pub fn new(lane: Lane, gradient_checkpointing: bool) -> Result<Self> {
        if lane == Lane::CandleMetal && gradient_checkpointing {
            bail!(
                "candle-metal does not implement gradient checkpointing; \
                 CalKey(candle-metal, gradient_checkpointing=true) is not a real cell"
            );
        }
        Ok(Self {
            lane,
            gradient_checkpointing,
        })
    }
}

/// Whether an `act_bytes_per_lht` came from a real training run, or is
/// inherited from a pre-measurement estimator pending real calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalSource {
    Measured,
    Seeded,
}

impl CalSource {
    fn parse(raw: &str) -> Result<Self> {
        match raw {
            "measured" => Ok(CalSource::Measured),
            "seeded" => Ok(CalSource::Seeded),
            other => bail!("unknown source {other:?}; expected measured or seeded"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryModel {
    pub act_bytes_per_lht: f64,
    pub source: CalSource,
}

impl MemoryModel {
    /// Predict peak bytes for a model shape at a given `tokens_per_step`
    /// (batch × seq_len). The weights term (`artifact_bytes`) passes through
    /// one-for-one; only the activation term scales with `act_bytes_per_lht`.
    pub fn predict_bytes(&self, shape: &ModelShape, tokens_per_step: u64) -> u64 {
        let activations = self.act_bytes_per_lht
            * shape.layers as f64
            * shape.hidden as f64
            * tokens_per_step as f64;
        shape.artifact_bytes + activations.round() as u64
    }
}

/// Shape facts read from a model directory: never fitted, always read.
#[derive(Debug, Clone)]
pub struct ModelShape {
    pub artifact_bytes: u64,
    pub layers: u32,
    pub hidden: u32,
}

#[derive(serde::Deserialize)]
struct RawTextConfig {
    num_hidden_layers: Option<u32>,
    hidden_size: Option<u32>,
}

#[derive(serde::Deserialize)]
struct RawConfig {
    #[serde(default)]
    num_hidden_layers: Option<u32>,
    #[serde(default)]
    hidden_size: Option<u32>,
    #[serde(default)]
    text_config: Option<RawTextConfig>,
}

impl ModelShape {
    /// Read `(layers, hidden)` from `config.json` — top-level fields first,
    /// falling back to `text_config.*` for multimodal configs where the
    /// top-level fields are `null` (every multimodal Qwen config does this).
    /// `artifact_bytes` is the total size of every `*.safetensors` file in
    /// the directory — read, not estimated.
    pub fn from_model_dir(dir: &Path) -> Result<Self> {
        let config_path = dir.join("config.json");
        let raw = std::fs::read_to_string(&config_path)
            .with_context(|| format!("reading {}", config_path.display()))?;
        let cfg: RawConfig = serde_json::from_str(&raw)
            .with_context(|| format!("parsing {}", config_path.display()))?;
        let (layers, hidden) =
            layers_and_hidden(cfg).with_context(|| format!("{}", config_path.display()))?;

        let mut artifact_bytes = 0u64;
        for entry in
            std::fs::read_dir(dir).with_context(|| format!("reading dir {}", dir.display()))?
        {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("safetensors") {
                // `DirEntry::metadata()` does not follow symlinks, and every
                // weight file in a Hugging Face Hub cache's `snapshots/` dir
                // is a symlink into `blobs/` — `std::fs::metadata` follows
                // the link to the real (multi-GB) target instead of
                // reporting the symlink's own few-dozen-byte size.
                artifact_bytes += std::fs::metadata(entry.path())?.len();
            }
        }

        Ok(ModelShape {
            artifact_bytes,
            layers,
            hidden,
        })
    }

    /// Build shape facts from HF Hub API metadata (`expand=["config","safetensors"]`)
    /// instead of a local directory — no weight bytes downloaded.
    ///
    /// Exists so cloud dispatch (`CloudResolver::dispatch`, `train_arm.rs`'s cloud
    /// path) can size a request the same way `vox mens cloud-estimate` does even
    /// before any local model directory exists — a from-scratch cloud-only run has
    /// nothing on disk to read with [`Self::from_model_dir`]. See
    /// `crate::mens::hub::model_shape_from_hub`, the sole caller.
    ///
    /// `params_by_dtype` is the Hub's `safetensors.parameters` map (dtype name ->
    /// parameter count); `artifact_bytes` approximates on-disk safetensors size as
    /// `sum(count * bytes_per_dtype)`, which omits safetensors' small per-file
    /// header overhead — negligible for VRAM sizing.
    pub fn from_hub_metadata(
        config: &serde_json::Value,
        params_by_dtype: &HashMap<String, u64>,
    ) -> Result<Self> {
        let cfg: RawConfig = serde_json::from_value(config.clone())
            .context("parsing Hub-reported config metadata")?;
        let (layers, hidden) = layers_and_hidden(cfg).context("Hub-reported config metadata")?;
        let artifact_bytes = params_by_dtype
            .iter()
            .map(|(dtype, count)| count * bytes_per_dtype(dtype))
            .sum();
        Ok(ModelShape {
            artifact_bytes,
            layers,
            hidden,
        })
    }
}

/// Extract `(num_hidden_layers, hidden_size)` from a parsed config — top-level
/// fields first, falling back to `text_config.*` for multimodal configs. Shared
/// by [`ModelShape::from_model_dir`] (reads `config.json` off disk) and
/// [`ModelShape::from_hub_metadata`] (reads the Hub API's `config` field).
fn layers_and_hidden(cfg: RawConfig) -> Result<(u32, u32)> {
    match (cfg.num_hidden_layers, cfg.hidden_size) {
        (Some(l), Some(h)) => Ok((l, h)),
        _ => {
            let text = cfg.text_config.ok_or_else(|| {
                anyhow!("has no num_hidden_layers/hidden_size at top level and no text_config")
            })?;
            match (text.num_hidden_layers, text.hidden_size) {
                (Some(l), Some(h)) => Ok((l, h)),
                _ => bail!("text_config is missing num_hidden_layers/hidden_size"),
            }
        }
    }
}

/// Bytes per parameter for a safetensors dtype name, as reported by the Hub's
/// `safetensors.parameters` map. Unknown dtypes fall back to 2 bytes (bf16/fp16
/// is the common training/serving dtype) rather than erroring — this is a
/// sizing approximation, not an exact readout.
fn bytes_per_dtype(dtype: &str) -> u64 {
    match dtype.to_ascii_uppercase().as_str() {
        "F64" | "I64" | "U64" => 8,
        "F32" | "I32" | "U32" => 4,
        "F16" | "BF16" | "I16" | "U16" => 2,
        "I8" | "U8" | "BOOL" => 1,
        _ => 2,
    }
}

/// Predict the CUDA VRAM requirement (MiB) for training `shape` under `request`,
/// via the same measured `plan_for` threshold `vox mens cloud-estimate` uses.
/// Every rented cloud offer is CUDA (see `resolver::dispatch`'s doc comment).
///
/// Exists so the read-only `vox mens cloud-estimate` command and real cloud
/// dispatch (`CloudResolver::dispatch`, `train_arm.rs`) can never disagree about
/// whether an offer is big enough — both call this instead of hardcoding a
/// threshold. `plan_for` is infallible in the `Fits`/budget-exceeded cases;
/// fails closed (naming the lane and the fix) only when the lane itself has no
/// measured calibration (see [`MemoryModels::get`]).
pub fn min_vram_mb_for_cuda(shape: &ModelShape, request: &Request) -> Result<u64> {
    let cuda_key = CalKey::new(Lane::CandleCuda, true)?;
    let models = MemoryModels::load_default()?;
    // A generous ceiling: this call exists to get `predicted_bytes`, not to
    // gate against a particular device's usable memory — the resolver (with
    // real per-offer VRAM) does that gating for the rented rows.
    let sizing_budget = DeviceBudget {
        working_set_bytes: u64::MAX / 2,
        operator_fraction: None,
    };
    let plan = plan_for(&sizing_budget, &models, &cuda_key, shape, request);
    let predicted_bytes = match (&plan.verdict, plan.predicted_bytes) {
        (_, Some(bytes)) => bytes,
        (Verdict::Refused(reason), None) => bail!(
            "cannot size a candle-cuda run: {reason}. The memory model has no \
             measured constant for this lane; borrowing another lane's constant \
             would produce a confident wrong estimate. Measure it first with \
             `vox mens probe --measure` (see the memory-SSOT plan)."
        ),
        (Verdict::Fits, None) => unreachable!("Fits verdict always carries predicted_bytes"),
    };
    // Only a u64 crosses this seam. `GpuOffer.vram_mb` is populated from Vast's
    // `gpu_ram` and RunPod's memory field, both MiB in practice despite the
    // field name — div_ceil(1_048_576) matches that.
    Ok(predicted_bytes.div_ceil(1_048_576))
}

#[derive(serde::Deserialize)]
struct RawDoc {
    schema: String,
    #[serde(default)]
    lanes: Vec<RawLaneRow>,
}

#[derive(serde::Deserialize)]
struct RawLaneRow {
    lane: String,
    gradient_checkpointing: bool,
    act_bytes_per_lht: f64,
    source: String,
}

const SCHEMA: &str = "vox.mens.memory-model.v1";

/// The loaded memory-model contract: one `MemoryModel` per real `CalKey`.
pub struct MemoryModels {
    rows: HashMap<CalKey, MemoryModel>,
}

impl MemoryModels {
    pub fn load_from_str(yaml: &str) -> Result<Self> {
        let doc: RawDoc = serde_yaml::from_str(yaml).context("parsing memory-model YAML")?;
        if doc.schema != SCHEMA {
            bail!("unexpected schema {:?}; expected {SCHEMA:?}", doc.schema);
        }

        let mut rows = HashMap::new();
        for row in doc.lanes {
            let lane = Lane::parse(&row.lane)?;
            let key = CalKey::new(lane, row.gradient_checkpointing).with_context(|| {
                format!(
                    "row for lane {:?} gradient_checkpointing={} is not a constructible cell",
                    row.lane, row.gradient_checkpointing
                )
            })?;
            let source = CalSource::parse(&row.source)?;
            rows.insert(
                key,
                MemoryModel {
                    act_bytes_per_lht: row.act_bytes_per_lht,
                    source,
                },
            );
        }
        Ok(Self { rows })
    }

    /// Load the shipped contract at `contracts/mens/memory-model.v1.yaml`.
    pub fn load_default() -> Result<Self> {
        Self::load_from_str(include_str!(
            "../../../../../contracts/mens/memory-model.v1.yaml"
        ))
    }

    /// Look up the model for a cell. Fails closed: an uncalibrated lane is
    /// an error naming the lane and the fix, never a borrowed constant from
    /// another lane or a silent default.
    pub fn get(&self, key: &CalKey) -> Result<&MemoryModel> {
        self.rows.get(key).ok_or_else(|| {
            anyhow!(
                "no memory-model calibration for lane {} (gradient_checkpointing={}); \
                 measure it with `vox mens probe --measure`",
                key.lane.as_str(),
                key.gradient_checkpointing
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: 100.0 here is a synthetic fixture value chosen only to exercise
    // the predict_bytes math (weights pass-through, linear-in-tokens
    // activations). It is NOT a real candle-metal measurement — as of
    // 2026-09-11 no fitted candle-metal constant exists (see the module doc
    // comment and `super::calibration`). The shipped contract
    // (`contracts/mens/memory-model.v1.yaml`) carries no candle-metal row.
    const SEED_YAML: &str = r#"
schema: vox.mens.memory-model.v1
lanes:
  - lane: candle-metal
    gradient_checkpointing: false
    act_bytes_per_lht: 100.0
    source: measured
  - lane: candle-cuda
    gradient_checkpointing: true
    act_bytes_per_lht: 186.3620764597089
    source: seeded
"#;

    #[test]
    fn the_weights_term_is_read_not_fitted() {
        let m = MemoryModels::load_from_str(SEED_YAML)
            .unwrap()
            .get(&CalKey::new(Lane::CandleMetal, false).unwrap())
            .unwrap()
            .clone();
        let a = ModelShape {
            artifact_bytes: 10_000_000_000,
            layers: 64,
            hidden: 5120,
        };
        let b = ModelShape {
            artifact_bytes: 20_000_000_000,
            layers: 64,
            hidden: 5120,
        };
        assert_eq!(
            m.predict_bytes(&b, 512) - m.predict_bytes(&a, 512),
            10_000_000_000,
            "artifact_bytes must pass through one-for-one"
        );
    }

    #[test]
    fn activations_are_linear_in_tokens() {
        let models = MemoryModels::load_from_str(SEED_YAML).unwrap();
        let m = models
            .get(&CalKey::new(Lane::CandleMetal, false).unwrap())
            .unwrap();
        let s = ModelShape {
            artifact_bytes: 29_501_218_479,
            layers: 64,
            hidden: 5120,
        };
        let a1 = m.predict_bytes(&s, 512) - s.artifact_bytes;
        let a2 = m.predict_bytes(&s, 1024) - s.artifact_bytes;
        assert!(
            (a2 as f64 / a1 as f64 - 2.0).abs() < 0.01,
            "doubling tokens must double activations; a sqrt or table breaks this"
        );
    }

    #[test]
    fn metal_cannot_claim_gradient_checkpointing() {
        assert!(
            CalKey::new(Lane::CandleMetal, true).is_err(),
            "candle-metal does not implement checkpointing; the key must be unconstructible, \
             not merely uncalibrated at runtime"
        );
        assert!(CalKey::new(Lane::CandleCuda, true).is_ok());
    }

    #[test]
    fn an_uncalibrated_lane_is_an_error_not_a_borrowed_constant() {
        let yaml = "schema: vox.mens.memory-model.v1\nlanes: []\n";
        let err = MemoryModels::load_from_str(yaml)
            .unwrap()
            .get(&CalKey::new(Lane::CandleMetal, false).unwrap())
            .expect_err("no rows");
        let msg = err.to_string();
        assert!(
            msg.contains("candle-metal"),
            "the error must name the lane: {msg}"
        );
        assert!(
            msg.contains("probe --measure"),
            "the error must name the fix: {msg}"
        );
    }

    #[test]
    fn model_shape_reads_a_multimodal_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"text_config":{"num_hidden_layers":64,"hidden_size":5120}}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("model.safetensors"), vec![0u8; 1024]).unwrap();
        let s = ModelShape::from_model_dir(dir.path()).unwrap();
        assert_eq!((s.layers, s.hidden, s.artifact_bytes), (64, 5120, 1024));
    }

    #[cfg(unix)]
    #[test]
    fn model_shape_follows_symlinked_weights_like_a_real_hf_cache() {
        // Every weight file in a Hugging Face Hub cache's `snapshots/` dir is
        // a symlink into `blobs/` — `DirEntry::metadata()` reports the
        // symlink's own size (tens of bytes), not the real target's, unless
        // `from_model_dir` resolves the link before reading the length.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"num_hidden_layers":32,"hidden_size":4096}"#,
        )
        .unwrap();
        let blob = dir.path().join("real_blob");
        std::fs::write(&blob, vec![0u8; 5_000_000]).unwrap();
        std::os::unix::fs::symlink(&blob, dir.path().join("model.safetensors")).unwrap();
        let s = ModelShape::from_model_dir(dir.path()).unwrap();
        assert_eq!(
            s.artifact_bytes, 5_000_000,
            "artifact_bytes must be the symlink target's size, not the symlink's own size"
        );
    }

    /// The shipped contract row for `candle-cuda` must reproduce the old
    /// `memory_budget::plan_with_resident`'s activation estimate (an
    /// `ACT_GIB_PER_KTOK_PER_SQRTB = 9.5` formula) within 10% on at least
    /// two rungs this lane actually serves — real Qwen3-14B/8B shapes. Both
    /// the formula and its `QWEN3_LADDER` source rungs were deleted from
    /// `memory_budget.rs` by Task 8 of
    /// `2026-09-11-1-memory-ssot-and-fit-benchmark` (this SSOT migration's
    /// own ladder-deletion sweep) — the constant and shapes are reproduced
    /// standalone below so this test does not depend on the deleted code;
    /// `git log -p -- crates/vox-populi/src/mens/tensor/memory_budget.rs` at
    /// the commit that deleted them has the original line numbers and
    /// hardware provenance. A CUDA row that silently changes the 4080
    /// Super's answers would be a regression dressed as an SSOT, not a seed.
    #[test]
    fn seeded_candle_cuda_row_reproduces_the_old_activation_formula_on_real_rungs() {
        const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
        const ACT_GIB_PER_KTOK_PER_SQRTB: f64 = 9.5; // historical memory_budget.rs constant, see doc comment above

        fn old_activation_bytes(params_b: f64, tokens_per_step: u64) -> f64 {
            (tokens_per_step as f64 / 1000.0) * params_b.sqrt() * ACT_GIB_PER_KTOK_PER_SQRTB * GIB
        }

        let models = MemoryModels::load_from_str(SEED_YAML).unwrap();
        let m = models
            .get(&CalKey::new(Lane::CandleCuda, true).unwrap())
            .unwrap();
        let tokens_per_step = 1024u64;

        // Real Qwen3 shapes from the deleted QWEN3_LADDER, see doc comment above:
        // Qwen3-14B (layers=40, hidden=5120) and Qwen3-8B (layers=36, hidden=4096).
        for (params_b, layers, hidden) in [(14.0, 40u32, 5120u32), (8.0, 36u32, 4096u32)] {
            let shape = ModelShape {
                artifact_bytes: 0,
                layers,
                hidden,
            };
            let new_bytes = m.predict_bytes(&shape, tokens_per_step) as f64;
            let old_bytes = old_activation_bytes(params_b, tokens_per_step);
            let rel_err = (new_bytes - old_bytes).abs() / old_bytes;
            assert!(
                rel_err < 0.10,
                "candle-cuda seed diverges >10% from the old formula at {params_b}B: \
                 new={new_bytes} old={old_bytes} rel_err={rel_err}"
            );
        }
    }

    /// `from_hub_metadata` must read the same `(layers, hidden)` shape as
    /// `from_model_dir` from an equivalent config, and derive `artifact_bytes`
    /// from the Hub's per-dtype parameter counts instead of scanning a
    /// directory — this is what lets cloud dispatch size a repo it has not
    /// downloaded (see `crate::mens::hub::model_shape_from_hub`).
    #[test]
    fn from_hub_metadata_matches_from_model_dir_shape_and_sizes_by_dtype() {
        let config = serde_json::json!({
            "text_config": { "num_hidden_layers": 64u32, "hidden_size": 5120u32 }
        });
        let mut params = HashMap::new();
        params.insert("BF16".to_string(), 1_000_000u64);
        let s = ModelShape::from_hub_metadata(&config, &params).unwrap();
        assert_eq!((s.layers, s.hidden), (64, 5120));
        assert_eq!(
            s.artifact_bytes, 2_000_000,
            "BF16 is 2 bytes/param: 1,000,000 params -> 2,000,000 bytes"
        );
    }

    /// MUTATION CAUGHT: summing dtype byte-widths instead of per-dtype
    /// `count * width` (e.g. treating every param as 2 bytes regardless of
    /// dtype would silently under-size a repo with an F32 head).
    #[test]
    fn from_hub_metadata_sums_bytes_across_mixed_dtypes() {
        let config = serde_json::json!({
            "num_hidden_layers": 1u32, "hidden_size": 1u32
        });
        let mut params = HashMap::new();
        params.insert("BF16".to_string(), 1_000u64); // 2 bytes each
        params.insert("F32".to_string(), 1_000u64); // 4 bytes each
        let s = ModelShape::from_hub_metadata(&config, &params).unwrap();
        assert_eq!(s.artifact_bytes, 2_000 + 4_000);
    }

    /// A config with neither top-level nor `text_config` layer/hidden fields
    /// must fail closed rather than defaulting to zero-sized shape — a silent
    /// zero would under-size every rented offer's VRAM requirement.
    #[test]
    fn from_hub_metadata_rejects_a_config_missing_shape_fields() {
        let config = serde_json::json!({ "some_other_field": true });
        let err = ModelShape::from_hub_metadata(&config, &HashMap::new()).unwrap_err();
        let msg = format!("{err:?}"); // Debug renders the full anyhow context chain
        assert!(
            msg.contains("num_hidden_layers"),
            "error must name the missing fields: {msg}"
        );
    }

    /// `min_vram_mb_for_cuda` must report the seeded candle-cuda lane's
    /// prediction, MiB-rounded up — not bytes, not truncated down (which would
    /// under-size the rental by up to 1 MiB).
    #[test]
    fn min_vram_mb_for_cuda_rounds_bytes_up_to_whole_mebibytes() {
        let shape = ModelShape {
            artifact_bytes: 3 * 1_048_576 + 1, // just over 3 MiB of weights
            layers: 1,
            hidden: 1,
        };
        let request = Request {
            batch_size: 1,
            seq_len: 1,
        };
        let mb = min_vram_mb_for_cuda(&shape, &request).unwrap();
        assert!(
            mb >= 4,
            "3 MiB + 1 byte of weights alone must round up past 3 MiB, got {mb}"
        );
    }
}

/// The one place headroom is taken. Peaks land above the running average, and
/// on a unified-memory machine the window server competes for the same pool.
pub const SAFETY_FRACTION: f64 = 0.88;

/// A device's total addressable memory pool for training — VRAM on a discrete
/// GPU, or unified memory on Apple Silicon.
#[derive(Debug, Clone, Copy)]
pub struct DeviceBudget {
    pub working_set_bytes: u64,
    /// An operator-specified additional throttle (e.g. `--vram-limit-fraction`,
    /// used to share a card with other processes or intentionally under-run
    /// during background training) layered ON TOP of `SAFETY_FRACTION` — a
    /// distinct, user-visible concern from the baked-in OOM margin, not a
    /// second application of it. `None` means no additional throttle.
    pub operator_fraction: Option<f64>,
}

/// What the caller is asking to run: the request `plan_for` reports on, not
/// what it substitutes.
#[derive(Debug, Clone, Copy)]
pub struct Request {
    pub batch_size: u64,
    pub seq_len: u64,
}

/// Whether a request fits the device's usable budget. `Refused` names why,
/// rather than the caller silently retreating to a smaller shape.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Fits,
    Refused(String),
}

/// `plan_for`'s report: it reports, it does not choose. Picking a shape that
/// fits is `sweep`'s job (a separate function) — keeping them separate is
/// what stops a refusal from becoming a silent substitution.
#[derive(Debug, Clone)]
pub struct Plan {
    pub verdict: Verdict,
    pub tokens_per_step: u64,
    pub usable_bytes: u64,
    /// The request `plan_for` was actually asked to evaluate — carried
    /// through so a renderer (see [`render_verdict`]) can name the batch
    /// size and seq_len it is reporting on without a second parameter.
    pub request: Request,
    /// `Some(bytes)` whenever a calibration existed to predict from (both
    /// `Fits` and a budget-exceeded `Refused` carry this); `None` only when
    /// the lane itself was uncalibrated, so there was nothing to predict.
    pub predicted_bytes: Option<u64>,
    /// Whether `request` was picked automatically (e.g. by [`sweep`]) or is
    /// an already-chosen size `plan_for` is merely verifying. `plan_for`
    /// itself cannot know this — it defaults to `false`; a caller building
    /// an auto-sized plan for display sets it explicitly.
    pub auto: bool,
}

/// `SAFETY_FRACTION` taken off a device's working set, then `operator_fraction`
/// (if any) layered on top — `plan_for` is the sole caller, and must consume
/// this value once, never re-scale it and never re-apply `operator_fraction`
/// a second time downstream (e.g. `memory_budget::plan_with_resident`'s own
/// internal safety fraction).
fn usable_bytes(budget: &DeviceBudget) -> u64 {
    let operator = budget.operator_fraction.unwrap_or(1.0);
    (budget.working_set_bytes as f64 * SAFETY_FRACTION * operator).round() as u64
}

/// Report whether `request` fits `shape` on `budget`, using the calibration
/// for `key` in `models`. Refuses (naming why) rather than shrinking the
/// request or substituting a smaller shape — that choice belongs to `sweep`.
pub fn plan_for(
    budget: &DeviceBudget,
    models: &MemoryModels,
    key: &CalKey,
    shape: &ModelShape,
    request: &Request,
) -> Plan {
    let tokens_per_step = request.batch_size * request.seq_len;
    let usable = usable_bytes(budget);
    let mut predicted_bytes = None;
    let verdict = match models.get(key) {
        Ok(model) => {
            let predicted = model.predict_bytes(shape, tokens_per_step);
            predicted_bytes = Some(predicted);
            if predicted <= usable {
                Verdict::Fits
            } else {
                Verdict::Refused(format!(
                    "predicted {predicted} bytes exceeds usable budget {usable} bytes \
                     ({tokens_per_step} tokens/step)"
                ))
            }
        }
        Err(e) => Verdict::Refused(e.to_string()),
    };
    Plan {
        verdict,
        tokens_per_step,
        usable_bytes: usable,
        request: *request,
        predicted_bytes,
        auto: false,
    }
}

/// Pick the largest batch size (at a fixed `seq_len`) that fits — a single
/// `take_while` over [`plan_for`], never a second sizing algorithm. Doubles
/// `batch_size` from 1 and keeps the largest candidate that still `Fits`;
/// returns `None` when even `batch_size=1` is refused. Reused verbatim by
/// both an explicit `vox mens probe --sweep` and auto-default training sizing
/// — one implementation, so a "largest fitting shape" answer never drifts
/// from what `plan_for` itself would say about that shape.
pub fn sweep(
    budget: &DeviceBudget,
    models: &MemoryModels,
    key: &CalKey,
    shape: &ModelShape,
    seq_len: u64,
) -> Result<Request> {
    std::iter::successors(Some(1u64), |b| b.checked_mul(2))
        .map(|batch_size| Request {
            batch_size,
            seq_len,
        })
        .take_while(|req| {
            matches!(
                plan_for(budget, models, key, shape, req).verdict,
                Verdict::Fits
            )
        })
        .last()
        .ok_or_else(|| {
            // Even `batch_size=1` was refused — surface `plan_for`'s own reason
            // (e.g. an uncalibrated lane names itself and the `probe --measure`
            // fix) rather than a generic "nothing fits", which would hide an
            // uncalibrated-lane error behind a misleading budget message.
            let smallest = Request {
                batch_size: 1,
                seq_len,
            };
            match plan_for(budget, models, key, shape, &smallest).verdict {
                Verdict::Refused(reason) => {
                    anyhow!("sweep found nothing that fits at seq_len={seq_len}: {reason}")
                }
                Verdict::Fits => anyhow!(
                    "no batch size fits {shape:?} at seq_len={seq_len} within the usable budget"
                ),
            }
        })
}

#[cfg(test)]
mod plan_for_tests {
    use super::*;

    /// M5 Max-shaped device budget: 128 GiB unified memory as the working set,
    /// no operator throttle.
    fn m5max() -> DeviceBudget {
        DeviceBudget {
            working_set_bytes: 128 * 1024 * 1024 * 1024,
            operator_fraction: None,
        }
    }

    // Synthetic fixture value, not a real candle-metal measurement — see the
    // module doc comment and `SEED_YAML` in `mod tests` above.
    const SEED_YAML: &str = r#"
schema: vox.mens.memory-model.v1
lanes:
  - lane: candle-metal
    gradient_checkpointing: false
    act_bytes_per_lht: 100.0
    source: measured
"#;

    fn models() -> MemoryModels {
        MemoryModels::load_from_str(SEED_YAML).unwrap()
    }

    fn metal_key() -> CalKey {
        CalKey::new(Lane::CandleMetal, false).unwrap()
    }

    /// A 27B-class shape (real Qwen3-32B dims stand in — no 27B rung exists on
    /// the real ladder, see `preset_schema::QwenSizeClass::Other`): large enough
    /// that batch=4×seq=1024 must overflow even a 128 GiB M5 Max after
    /// `SAFETY_FRACTION`.
    fn shape_27b() -> ModelShape {
        ModelShape {
            artifact_bytes: 60_000_000_000,
            layers: 64,
            hidden: 5120,
        }
    }

    #[test]
    fn an_over_large_request_is_refused_with_a_reason_not_shrunk_to_a_smaller_model() {
        let plan = plan_for(
            &m5max(),
            &models(),
            &metal_key(),
            &shape_27b(),
            &Request {
                batch_size: 4,
                seq_len: 1024,
            },
        );
        assert!(!matches!(plan.verdict, Verdict::Fits));
        assert_eq!(
            plan.tokens_per_step, 4096,
            "plan_for must report what was ASKED, not a substitute"
        );
    }

    #[test]
    fn safety_headroom_reaches_the_call_sites_exactly_once() {
        // NOT `plan.usable_bytes == working_set * SAFETY_FRACTION` — that restates
        // plan_for's own line and passes whether or not the call sites were rewired.
        // Assert against `usable_bytes` directly (the old `budget_gate_usable_bytes`
        // public wrapper was deleted for having zero non-test callers).
        let b = m5max();
        let once = (b.working_set_bytes as f64 * SAFETY_FRACTION).round() as u64;
        assert_eq!(
            usable_bytes(&b),
            once,
            "usable_bytes must apply SAFETY_FRACTION exactly once"
        );
    }

    #[test]
    fn sweep_picks_the_largest_shape_that_fits_not_the_first() {
        let picked =
            sweep(&m5max(), &models(), &metal_key(), &shape_27b(), 512).expect("something fits");
        // Compute the expectation from the model, do not hardcode an index:
        // every larger candidate must be refused and this one must fit.
        assert!(matches!(
            plan_for(&m5max(), &models(), &metal_key(), &shape_27b(), &picked).verdict,
            Verdict::Fits
        ));
        let bigger = Request {
            batch_size: picked.batch_size * 2,
            seq_len: picked.seq_len,
        };
        assert!(
            !matches!(
                plan_for(&m5max(), &models(), &metal_key(), &shape_27b(), &bigger).verdict,
                Verdict::Fits
            ),
            "sweep stopped early: {bigger:?} also fits"
        );
    }

    #[test]
    fn an_operator_throttle_layers_on_top_of_safety_fraction_exactly_once() {
        // Pins the composed behavior for Task 5's fix-round gap: `vram_limit_fraction`
        // (an explicit, operator-visible cap — e.g. `--background` defaults it to 0.8)
        // is a DIFFERENT concern from `SAFETY_FRACTION` (the baked-in OOM margin), so
        // it layers ON TOP, applied exactly once by `usable_bytes` itself — never
        // re-applied a second time downstream by a caller (that second application is
        // the actual bug this fix-round closes: train_arm.rs/preset_schema.rs used to
        // pre-multiply by this same fraction *and then* hand the result to
        // `memory_budget::plan_with_resident`, which multiplied by its own internal
        // safety fraction again).
        let uncapped = DeviceBudget {
            working_set_bytes: 128 * 1024 * 1024 * 1024,
            operator_fraction: None,
        };
        let throttled = DeviceBudget {
            operator_fraction: Some(0.8),
            ..uncapped
        };
        let expected = (throttled.working_set_bytes as f64 * SAFETY_FRACTION * 0.8).round() as u64;
        assert_eq!(
            usable_bytes(&throttled),
            expected,
            "operator_fraction must compose with SAFETY_FRACTION exactly once, not compound \
             with a second independent safety application"
        );
        // Sanity: the throttle must actually shrink the budget relative to uncapped.
        assert!(usable_bytes(&throttled) < usable_bytes(&uncapped));
    }
}

#[cfg(test)]
mod load_default_check {
    use super::*;

    #[test]
    fn load_default_parses_the_shipped_contract() {
        let models = MemoryModels::load_default().unwrap();
        let m = models
            .get(&CalKey::new(Lane::CandleCuda, true).unwrap())
            .unwrap();
        assert_eq!(m.source, CalSource::Seeded);
    }
}

/// Real binary GiB (1024^3) — matching `AccelBudget`'s own measured numbers
/// (`accel_budget.rs`'s "107.52 GiB" / "80.64 GiB" doc comments), not the
/// 1e9-based "GiB" that `candle_qlora_train/oom.rs` quotes verbatim from a
/// real driver error string in a different crate.
fn gib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}

/// Render a [`Plan`] into a full report of a training-fit decision: model
/// size, both accelerator caps, the usable budget with its safety fraction
/// named, the chosen shape (tagged `[auto]` or `[you asked for this]`), the
/// predicted peak and headroom, and the verdict — on EVERY path, not only on
/// refusal. A renderer that speaks only on failure would pass a
/// refusal-only test while telling a user nothing when a plan fits, which is
/// the exact defect this function exists to close (see the GPU Probe card's
/// "Detected accelerators + LoRA fit" promise).
///
/// On refusal, names which cap bound (the single-allocation limit or the
/// usable working-set budget) and the largest batch size (at the same
/// `seq_len`) that would fit — derived algebraically from this one plan's
/// own numbers (`predicted_bytes` is linear in `tokens_per_step`), so this
/// needs no second `sweep` call and no `MemoryModels`/`CalKey` parameters.
pub fn render_verdict(plan: &Plan, budget: &AccelBudget, shape: &ModelShape) -> String {
    use std::fmt::Write as _;

    let tag = if plan.auto {
        "[auto]"
    } else {
        "[you asked for this]"
    };
    let mut out = String::new();
    let _ = writeln!(out, "model artifact: {:.1} GiB", gib(shape.artifact_bytes));
    let _ = writeln!(
        out,
        "device: {} (working set {:.1} GiB, max single alloc {:.1} GiB)",
        budget.device_name,
        gib(budget.working_set_bytes),
        gib(budget.max_alloc_bytes)
    );
    let _ = writeln!(
        out,
        "usable budget: {:.1} GiB ({:.0}% safety fraction applied)",
        gib(plan.usable_bytes),
        SAFETY_FRACTION * 100.0
    );
    let _ = writeln!(
        out,
        "chosen shape {tag}: batch {} seq_len {} ({} tokens/step)",
        plan.request.batch_size, plan.request.seq_len, plan.tokens_per_step
    );

    match (&plan.verdict, plan.predicted_bytes) {
        (Verdict::Fits, Some(predicted)) => {
            let headroom = plan.usable_bytes.saturating_sub(predicted);
            let _ = writeln!(
                out,
                "predicted peak: {:.1} GiB, headroom: {:.1} GiB",
                gib(predicted),
                gib(headroom)
            );
            out.push_str("verdict: FITS\n");
        }
        (Verdict::Fits, None) => {
            // Unreachable in practice (plan_for only returns Fits when a
            // model prediction succeeded), but keep the verdict line so a
            // future caller building a Plan by hand never gets a blank
            // report on the happy path.
            out.push_str("verdict: FITS\n");
        }
        (Verdict::Refused(reason), Some(predicted)) => {
            let alloc_cap_broken = !budget.single_alloc_fits(predicted);
            let (cap_bytes, cap_name) = if alloc_cap_broken {
                (budget.max_alloc_bytes, "single-allocation limit")
            } else {
                (plan.usable_bytes, "usable working-set budget")
            };
            let _ = writeln!(
                out,
                "predicted peak: {:.1} GiB exceeds the {cap_name} ({:.1} GiB)",
                gib(predicted),
                gib(cap_bytes)
            );

            // Back out bytes-per-token from this one plan (predicted is
            // linear in tokens_per_step: predicted = artifact_bytes +
            // rate * tokens_per_step) and solve for the largest batch size
            // (at the same seq_len) that would stay under the binding cap.
            let rate = (predicted as f64 - shape.artifact_bytes as f64)
                / plan.tokens_per_step.max(1) as f64;
            let fix = if rate > 0.0 {
                let max_tokens = (cap_bytes as f64 - shape.artifact_bytes as f64) / rate;
                let max_batch = if max_tokens > 0.0 {
                    (max_tokens / plan.request.seq_len.max(1) as f64).floor() as u64
                } else {
                    0
                };
                if max_batch >= 1 {
                    format!(
                        "reduce --batch-size to {max_batch} (or lower --seq-len) to fit at {cap_name}"
                    )
                } else {
                    "even batch 1 does not fit at this --seq-len; lower --seq-len too".to_string()
                }
            } else {
                "reduce --batch-size or --seq-len to shrink the predicted peak".to_string()
            };
            let _ = writeln!(out, "verdict: REFUSED — {reason}\nfix: {fix}");
        }
        (Verdict::Refused(reason), None) => {
            let _ = writeln!(
                out,
                "verdict: REFUSED — {reason}\nfix: run `vox mens probe --measure` to calibrate this lane"
            );
        }
    }

    out
}

#[cfg(test)]
mod render_verdict_tests {
    use super::super::accel_budget::BudgetSource;
    use super::*;

    /// Real measured M5 Max numbers (see `accel_budget.rs`'s own "measured
    /// M5 Max" test fixture): 107.52 GiB working set, 80.64 GiB max single
    /// allocation.
    fn m5max() -> AccelBudget {
        AccelBudget {
            device_name: "Apple M5 Max".to_string(),
            total_bytes: 115_448_725_504,
            working_set_bytes: 115_448_725_504,
            max_alloc_bytes: 86_587_244_544,
            source: BudgetSource::Metal,
        }
    }

    /// A 27B-class shape (mirrors `plan_for_tests::shape_27b`).
    fn shape_27b() -> ModelShape {
        ModelShape {
            artifact_bytes: 60_000_000_000,
            layers: 64,
            hidden: 5120,
        }
    }

    fn fits_plan_auto() -> Plan {
        Plan {
            verdict: Verdict::Fits,
            tokens_per_step: 512,
            usable_bytes: (115_448_725_504f64 * SAFETY_FRACTION).round() as u64,
            request: Request {
                batch_size: 1,
                seq_len: 512,
            },
            predicted_bytes: Some(65_000_000_000),
            auto: true,
        }
    }

    /// The measured b4x1024 OOM: 122.1 GiB needed against an 80.6 GiB
    /// single-allocation cap (see `oom.rs`'s "122.10 GiB" real driver quote
    /// for the same class of event, in a different crate/convention).
    fn single_alloc_too_large_plan() -> Plan {
        let predicted = (122.1f64 * 1024.0 * 1024.0 * 1024.0).round() as u64;
        Plan {
            verdict: Verdict::Refused(format!("predicted {predicted} bytes exceeds usable budget")),
            tokens_per_step: 4096,
            usable_bytes: (115_448_725_504f64 * SAFETY_FRACTION).round() as u64,
            request: Request {
                batch_size: 4,
                seq_len: 1024,
            },
            predicted_bytes: Some(predicted),
            auto: false,
        }
    }

    #[test]
    fn a_fitting_plan_states_what_was_chosen_and_that_it_was_automatic() {
        let s = render_verdict(&fits_plan_auto(), &m5max(), &shape_27b());
        assert!(s.contains("batch 1"), "name the shape it picked: {s}");
        assert!(s.contains("512"), "name the seq len: {s}");
        assert!(
            s.contains("[auto]"),
            "say the user did not choose this: {s}"
        );
        assert!(
            s.contains("GiB"),
            "state predicted AND budget on the happy path too: {s}"
        );
    }

    #[test]
    fn a_refusal_states_the_cap_it_broke_and_the_knob_that_fixes_it() {
        let s = render_verdict(&single_alloc_too_large_plan(), &m5max(), &shape_27b());
        assert!(s.contains("122.1"), "state what it needs: {s}");
        assert!(s.contains("80.6"), "state the cap it broke: {s}");
        assert!(s.contains("batch"), "name the knob that fixes it: {s}");
    }

    #[test]
    fn an_uncalibrated_refusal_still_names_the_fix_without_a_predicted_number() {
        let plan = Plan {
            verdict: Verdict::Refused("no memory-model calibration for lane candle-metal".into()),
            tokens_per_step: 512,
            usable_bytes: 100,
            request: Request {
                batch_size: 1,
                seq_len: 512,
            },
            predicted_bytes: None,
            auto: true,
        };
        let s = render_verdict(&plan, &m5max(), &shape_27b());
        assert!(s.contains("REFUSED"));
        assert!(s.contains("probe --measure"));
    }
}
