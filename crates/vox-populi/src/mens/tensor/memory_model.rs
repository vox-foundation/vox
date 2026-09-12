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
}

/// `SAFETY_FRACTION` taken off a device's working set, then `operator_fraction`
/// (if any) layered on top — called from exactly two places: `plan_for` and
/// `budget_gate_usable_bytes`, both of which must consume this value once,
/// never re-scale it and never re-apply `operator_fraction` a second time
/// downstream (e.g. `memory_budget::plan_with_resident`'s own internal
/// safety fraction).
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
    let verdict = match models.get(key) {
        Ok(model) => {
            let predicted = model.predict_bytes(shape, tokens_per_step);
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
    }
}

/// The usable-bytes budget a call site (e.g. a training `budget_gate`) must
/// consume verbatim from `plan_for` — never re-scale by `SAFETY_FRACTION`
/// again. This is the single call site both planners must route through.
pub fn budget_gate_usable_bytes(budget: &DeviceBudget) -> u64 {
    usable_bytes(budget)
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
        // Assert through budget_gate, which is where the second application lived.
        let b = m5max();
        let once = (b.working_set_bytes as f64 * SAFETY_FRACTION).round() as u64;
        assert_eq!(
            budget_gate_usable_bytes(&b),
            once,
            "budget_gate must consume plan_for's usable_bytes, not scale it again"
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
            budget_gate_usable_bytes(&throttled),
            expected,
            "operator_fraction must compose with SAFETY_FRACTION exactly once, not compound \
             with a second independent safety application"
        );
        // Sanity: the throttle must actually shrink the budget relative to uncapped.
        assert!(budget_gate_usable_bytes(&throttled) < budget_gate_usable_bytes(&uncapped));
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
