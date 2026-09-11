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
//! `contracts/mens/memory-model.v1.yaml` seeds `(CandleCuda, true)` from the
//! constants already calibrated in [`super::memory_budget`]
//! (`RESIDENT_GIB_PER_B_PARAMS`, `FIXED_OVERHEAD_GIB`,
//! `ACT_GIB_PER_KTOK_PER_SQRTB`) so the 4080 Super lane keeps working while
//! it awaits a real measurement. `CalSource::Seeded` marks it as inherited,
//! not measured.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};

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

        let (layers, hidden) = match (cfg.num_hidden_layers, cfg.hidden_size) {
            (Some(l), Some(h)) => (l, h),
            _ => {
                let text = cfg.text_config.ok_or_else(|| {
                    anyhow!(
                        "{} has no num_hidden_layers/hidden_size at top level and no text_config",
                        config_path.display()
                    )
                })?;
                match (text.num_hidden_layers, text.hidden_size) {
                    (Some(l), Some(h)) => (l, h),
                    _ => bail!(
                        "{} text_config is missing num_hidden_layers/hidden_size",
                        config_path.display()
                    ),
                }
            }
        };

        let mut artifact_bytes = 0u64;
        for entry in
            std::fs::read_dir(dir).with_context(|| format!("reading dir {}", dir.display()))?
        {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("safetensors") {
                artifact_bytes += entry.metadata()?.len();
            }
        }

        Ok(ModelShape {
            artifact_bytes,
            layers,
            hidden,
        })
    }
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

    /// The shipped contract row for `candle-cuda` must reproduce
    /// `memory_budget::plan_with_resident`'s activation estimate (today's
    /// `ACT_GIB_PER_KTOK_PER_SQRTB = 9.5` formula, `memory_budget.rs:46`)
    /// within 10% on at least two rungs this lane actually serves
    /// (`QWEN3_LADDER`, `memory_budget.rs:309`) — a CUDA row that silently
    /// changes the 4080 Super's answers would be a regression dressed as an
    /// SSOT, not a seed.
    #[test]
    fn seeded_candle_cuda_row_reproduces_the_old_activation_formula_on_real_rungs() {
        const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
        const ACT_GIB_PER_KTOK_PER_SQRTB: f64 = 9.5; // memory_budget.rs:46

        fn old_activation_bytes(params_b: f64, tokens_per_step: u64) -> f64 {
            (tokens_per_step as f64 / 1000.0) * params_b.sqrt() * ACT_GIB_PER_KTOK_PER_SQRTB * GIB
        }

        let models = MemoryModels::load_from_str(SEED_YAML).unwrap();
        let m = models
            .get(&CalKey::new(Lane::CandleCuda, true).unwrap())
            .unwrap();
        let tokens_per_step = 1024u64;

        // Real Qwen3 shapes from QWEN3_LADDER (memory_budget.rs:309-325):
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
