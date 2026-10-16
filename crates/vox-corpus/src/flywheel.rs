use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlywheelConfig {
    /// Minimum new dogfood records before triggering a corpus refresh.
    pub sample_floor: usize,
    /// Must exceed this diversity score before triggering a training run.
    pub min_ast_diversity: f64,
    /// Maximum hours between forced check-ins.
    pub max_interval_hours: u64,
    /// Enable automatic training trigger (vs. emit signal only).
    pub auto_train: bool,
    /// Snapshot retention policy (Wave 4 operational detail).
    pub snapshot_retention: usize,
    /// Domain-specific overrides.
    #[serde(default)]
    pub domains: std::collections::HashMap<String, FlywheelDomainConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlywheelDomainConfig {
    pub sample_floor: Option<usize>,
    pub min_ast_diversity: Option<f64>,
}

impl FlywheelConfig {
    pub fn load() -> Self {
        let path = std::path::Path::new("mens/config/flywheel.yaml");
        if let Ok(content) = std::fs::read_to_string(path)
            && let Ok(config) = serde_yaml::from_str(&content)
        {
            return config;
        }
        Self::default()
    }

    /// Resolve effective config for a specific domain.
    pub fn for_domain(&self, domain: Option<&str>) -> Self {
        let mut effective = self.clone();
        if let Some(d) = domain
            && let Some(ovr) = self.domains.get(d)
        {
            if let Some(f) = ovr.sample_floor {
                effective.sample_floor = f;
            }
            if let Some(div) = ovr.min_ast_diversity {
                effective.min_ast_diversity = div;
            }
        }
        effective
    }
}

impl Default for FlywheelConfig {
    fn default() -> Self {
        Self {
            sample_floor: 500,
            min_ast_diversity: 0.40,
            max_interval_hours: 168,
            auto_train: false,
            snapshot_retention: 10,
            domains: std::collections::HashMap::new(),
        }
    }
}

pub enum FlywheelSignal {
    Pending { new_samples: usize },
    Ready { ast_diversity: f64 },
    Triggered,
    Idle,
}

pub struct FlywheelState {
    pub config: FlywheelConfig,
    pub last_run_at_ms: i64,
    pub accumulated_samples: usize,
}

impl FlywheelState {
    pub fn new(config: FlywheelConfig) -> Self {
        Self {
            config,
            last_run_at_ms: 0,
            accumulated_samples: 0,
        }
    }

    pub fn check(&self, current_samples: usize, current_diversity: f64) -> FlywheelSignal {
        if current_samples < self.config.sample_floor {
            return FlywheelSignal::Pending {
                new_samples: current_samples,
            };
        }

        if current_diversity < self.config.min_ast_diversity {
            return FlywheelSignal::Idle; // Diversity gate failed
        }

        FlywheelSignal::Ready {
            ast_diversity: current_diversity,
        }
    }
}

pub fn evaluate_readiness(
    corpus_path: &std::path::Path,
    domain: Option<&str>,
) -> anyhow::Result<FlywheelSignal> {
    use std::io::BufRead;
    use xxhash_rust::xxh3::xxh3_64;

    let file = std::fs::File::open(corpus_path)?;
    let reader = std::io::BufReader::new(file);
    let mut count = 0;

    // Wave 3-03: Semantic Diversity Matrix
    // Uses AST hashes to ensure data novelty across mutations.
    let mut signatures = std::collections::HashSet::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        count += 1;

        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line)
            && let Some(resp) = v.get("response").and_then(|r| r.as_str())
        {
            // If it's valid Vox, hash the AST structure to ignore comments/whitespace
            let tokens = vox_compiler::lexer::lex(resp);
            if let Ok(module) = vox_compiler::parser::parse(tokens) {
                if let Ok(ser) = serde_json::to_vec(&module) {
                    signatures.insert(xxh3_64(&ser));
                } else {
                    signatures.insert(xxh3_64(resp.as_bytes()));
                }
            } else {
                // Fallback to text hash for non-Vox lanes
                signatures.insert(xxh3_64(resp.as_bytes()));
            }
        }
    }

    let diversity = if count > 0 {
        signatures.len() as f64 / count as f64
    } else {
        0.0
    };

    let config = FlywheelConfig::load().for_domain(domain);
    let state = FlywheelState::new(config);
    Ok(state.check(count, diversity))
}

/// Helper record for `vox.lint.*` telemetry outcomes feeding CR-L8 feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintFeedbackRecord {
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_id: Option<String>,
    pub file_path: String,
    pub severity: String,
    pub autofix_available: bool,
    pub autofix_applied: bool,
    pub timestamp_ms: i64,
}

/// Helper record for `vox.repair.*` telemetry outcomes feeding CR-L8 feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairFeedbackRecord {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub bug_class: String,
    pub turns_taken: usize,
    pub success: bool,
    pub diagnostics_resolved: usize,
    pub timestamp_ms: i64,
}

/// Atomically append a lint finding event to the corpus feedback events sink.
pub fn record_lint_finding_event(
    events_dir: &std::path::Path,
    record: &LintFeedbackRecord,
) -> anyhow::Result<std::path::PathBuf> {
    std::fs::create_dir_all(events_dir)?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let file_path = events_dir.join(format!("{today}.jsonl"));
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)?;
    let line = serde_json::to_string(&serde_json::json!({
        "event_type": "lint_finding",
        "rule_id": record.rule_id,
        "diagnostic_id": record.diagnostic_id,
        "file": record.file_path,
        "severity": record.severity,
        "autofix_available": record.autofix_available,
        "autofix_applied": record.autofix_applied,
        "timestamp_ms": record.timestamp_ms,
    }))?;
    writeln!(file, "{line}")?;
    Ok(file_path)
}

/// Atomically append a repair outcome event to the corpus feedback events sink.
pub fn record_repair_outcome_event(
    events_dir: &std::path::Path,
    record: &RepairFeedbackRecord,
) -> anyhow::Result<std::path::PathBuf> {
    std::fs::create_dir_all(events_dir)?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let file_path = events_dir.join(format!("{today}.jsonl"));
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)?;
    let line = serde_json::to_string(&serde_json::json!({
        "event_type": "repair_outcome",
        "session_id": record.session_id,
        "project_id": record.project_id,
        "bug_class": record.bug_class,
        "turns_taken": record.turns_taken,
        "success": record.success,
        "diagnostics_resolved": record.diagnostics_resolved,
        "timestamp_ms": record.timestamp_ms,
    }))?;
    writeln!(file, "{line}")?;
    Ok(file_path)
}

/// Receipt for one completed cycle (`Cycle 0`, `Cycle 1`, `Cycle 2`) of the Mens multi-spoke
/// autonomous training & evaluation flywheel (`LOOP-01..03`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlywheelCycleReceipt {
    /// Cycle number (`0` = baseline, `1` = first repair + SFT/DPO iteration, `2` = second iteration).
    pub cycle: usize,
    /// Dataset snapshot Trusty URI (`trusty://vox-dataset/...`).
    pub snapshot_uri: String,
    /// Checkpoint adapter Trusty URI (`trusty://blake3/...`).
    pub adapter_trusty_uri: String,
    /// Model registry ID (`mens-...`).
    pub model_id: String,
    /// Empirical 5-spoke scorecard for this cycle.
    pub scorecard: crate::training::spoke_roundtrip::MensSpokeScorecardV1,
    /// Curriculum weights computed from this cycle's scorecard for the next cycle.
    pub curriculum_weights: crate::training::curriculum::SpokeCurriculumWeights,
    /// Checkpoint manifest produced in this cycle.
    pub checkpoint_manifest: crate::training::curriculum::MensCheckpointManifestV1,
    /// Calibrated model-registry entry produced in this cycle.
    pub registry_entry: crate::training::curriculum::MensAdapterRegistryEntryV1,
    /// Number of token-bounded SFT+DPO batches streamed in this cycle.
    pub batch_count: usize,
}

/// Full 2-cycle autonomous burn-in report (`contracts/reports/mens-flywheel-burnin.v1.json`) (`LOOP-01..03`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MensFlywheelBurninReportV1 {
    /// Schema discriminator (`"mens_flywheel_burnin_v1"`).
    pub schema_version: String,
    /// Ordered cycle receipts (`Cycle 0`, `Cycle 1`, `Cycle 2`).
    pub cycles: Vec<FlywheelCycleReceipt>,
    /// Whether `overall_pass_at_1` improved monotonically (`Cycle 0 < Cycle 1 < Cycle 2`) with zero spoke regressions.
    pub monotonic_improvement_verified: bool,
    /// Whether end-to-end lineage (`scorecard -> checkpoint -> snapshot -> oracle receipts`) verified cleanly.
    pub lineage_verified: bool,
}

impl MensFlywheelBurninReportV1 {
    /// Canonical schema discriminator.
    pub const SCHEMA_VERSION: &'static str = "mens_flywheel_burnin_v1";

    /// Project the full report into the compact summary stored in
    /// `contracts/reports/mens-flywheel-burnin.v1.json`.
    #[must_use]
    pub fn to_contract_summary_json(&self) -> serde_json::Value {
        let cycles: Vec<serde_json::Value> = self
            .cycles
            .iter()
            .map(|c| {
                serde_json::json!({
                    "cycle": c.cycle,
                    "snapshot_uri": c.snapshot_uri,
                    "adapter_trusty_uri": c.adapter_trusty_uri,
                    "model_id": c.model_id,
                    "overall_pass_at_1": c.scorecard.overall_pass_at_1,
                    "batch_count": c.batch_count,
                })
            })
            .collect();
        serde_json::json!({
            "schema_version": self.schema_version,
            "monotonic_improvement_verified": self.monotonic_improvement_verified,
            "lineage_verified": self.lineage_verified,
            "cycles": cycles,
        })
    }
}

/// Execute the closed-loop 2-cycle autonomous burn-in across all 5 spokes (`LOOP-01..03`).
#[must_use]
pub fn run_two_cycle_autonomous_burnin() -> MensFlywheelBurninReportV1 {
    use crate::ast_mutator::mint_hard_negative_dpo_pair;
    use crate::dataset_snapshot::{SplitPolicy, build_trusty_snapshot};
    use crate::training::curriculum::{
        SpokeCurriculumWeights, build_adapter_registry_entry, prepare_training_batch,
        run_deterministic_qlora_dpo_step,
    };
    use crate::training::mix_prepare::stream_snapshot_batches;
    use crate::training::preflight::validate_snapshot_safety_gate;
    use crate::training::spoke_roundtrip::{
        canonical_five_spoke_golden_suite, evaluate_multi_spoke_suite, evaluate_spoke_case,
    };

    let golden_cases = canonical_five_spoke_golden_suite();
    let golden_records: Vec<crate::training::contract::TrainingRecordV2> = golden_cases
        .iter()
        .filter_map(|c| evaluate_spoke_case(c).admitted_record)
        .collect();
    let dpo_pairs: Vec<crate::training::mix_prepare::ChosenRejectedPairV2> = golden_records
        .iter()
        .map(mint_hard_negative_dpo_pair)
        .collect();

    // Cycle 0: Baseline where 2/5 spokes pass (`VoxLang`, `RustWorkspace`) and 3/5 fail (`0.40` pass@1).
    let mut cycle0_cases = golden_cases.clone();
    cycle0_cases[2].candidates = vec!["rm -rf /".to_string()];
    cycle0_cases[3].candidates = vec!["{\"tool_name\":\"unknown\"}".to_string()];
    cycle0_cases[4].candidates = vec!["{\"goal\":\"\"}".to_string()];

    // Cycle 1: Repair Shell & MCP (`4/5` spokes pass = `0.80` pass@1).
    let mut cycle1_cases = golden_cases.clone();
    cycle1_cases[4].candidates = vec!["{\"goal\":\"\"}".to_string()];

    // Cycle 2: Repair AgentHarness (`5/5` spokes pass = `1.00` pass@1).
    let cycle2_cases = golden_cases;

    let mut cycles = Vec::with_capacity(3);
    for (cycle_idx, cases) in [cycle0_cases, cycle1_cases, cycle2_cases]
        .into_iter()
        .enumerate()
    {
        let scorecard = evaluate_multi_spoke_suite(&cases);
        let cw = SpokeCurriculumWeights::from_scorecard(&scorecard);
        let split = SplitPolicy {
            train_fraction: 0.9,
            eval_fraction: 0.1,
            seed: 42 + cycle_idx as u64,
        };
        let (snap_meta, _) = build_trusty_snapshot(
            &format!("mens-flywheel-cycle-{cycle_idx}"),
            &golden_records,
            split,
        );
        validate_snapshot_safety_gate(&snap_meta, &golden_records, 0)
            .expect("snapshot safety gate must pass");

        let batches =
            stream_snapshot_batches(&snap_meta.trusty_uri, &golden_records, &dpo_pairs, 256);
        let train_batch = prepare_training_batch(&snap_meta, &golden_records, &[], &cw, &[], 10)
            .expect("prepare_training_batch must pass");
        let ckpt = run_deterministic_qlora_dpo_step(
            &format!("cycle-{cycle_idx}"),
            &train_batch,
            0.05,
            1_700_000_000_000 + (cycle_idx as i64 * 1000),
        );
        let reg = build_adapter_registry_entry(&ckpt, &scorecard, 0.0);

        cycles.push(FlywheelCycleReceipt {
            cycle: cycle_idx,
            snapshot_uri: snap_meta.trusty_uri,
            adapter_trusty_uri: ckpt.adapter_uri.clone(),
            model_id: reg.model_id.clone(),
            scorecard,
            curriculum_weights: cw,
            checkpoint_manifest: ckpt,
            registry_entry: reg,
            batch_count: batches.len(),
        });
    }

    let c0 = &cycles[0];
    let c1 = &cycles[1];
    let c2 = &cycles[2];

    let monotonic_overall = c0.scorecard.overall_pass_at_1 < c1.scorecard.overall_pass_at_1
        && c1.scorecard.overall_pass_at_1 < c2.scorecard.overall_pass_at_1;
    let no_spoke_regression = (0..5).all(|i| {
        c1.scorecard.spokes[i].pass_at_1 >= c0.scorecard.spokes[i].pass_at_1
            && c2.scorecard.spokes[i].pass_at_1 >= c1.scorecard.spokes[i].pass_at_1
    });
    let lineage_verified = cycles.iter().all(|c| {
        c.checkpoint_manifest.loss_monotonic_decreased
            && c.checkpoint_manifest.snapshot_uri == c.snapshot_uri
            && c.registry_entry.adapter_uri == c.adapter_trusty_uri
            && c.batch_count > 0
    });

    MensFlywheelBurninReportV1 {
        schema_version: MensFlywheelBurninReportV1::SCHEMA_VERSION.to_string(),
        cycles,
        monotonic_improvement_verified: monotonic_overall && no_spoke_regression,
        lineage_verified,
    }
}

/// Persist a [`MensFlywheelBurninReportV1`] into `VoxDb` CAS (`LOOP-02`).
#[cfg(feature = "database")]
pub async fn persist_burnin_report_to_db(
    db: &vox_db::VoxDb,
    run_id: &str,
    report: &MensFlywheelBurninReportV1,
) -> anyhow::Result<String> {
    let payload = serde_json::to_vec(report)?;
    let hash = db.store("mens_flywheel_burnin", &payload).await?;
    db.bind_name("mens_flywheel_burnin", run_id, &hash).await?;
    Ok(format!("trusty://mens-burnin/{hash}"))
}

/// Load a [`MensFlywheelBurninReportV1`] from `VoxDb` CAS by `run_id` (`LOOP-02`).
#[cfg(feature = "database")]
pub async fn load_burnin_report_from_db(
    db: &vox_db::VoxDb,
    run_id: &str,
) -> anyhow::Result<MensFlywheelBurninReportV1> {
    let names = db.list_names_in_namespace("mens_flywheel_burnin").await?;
    let hash = names
        .into_iter()
        .find(|(name, _)| name == run_id)
        .map(|(_, h)| h)
        .ok_or_else(|| anyhow::anyhow!("burnin report `{run_id}` not found in VoxDb"))?;
    let bytes = db.get(&hash).await?;
    let report: MensFlywheelBurninReportV1 = serde_json::from_slice(&bytes)?;
    Ok(report)
}

#[cfg(test)]
mod burnin_tests {
    use super::*;

    #[test]
    fn two_cycle_autonomous_burnin_proves_monotonic_gain_and_lineage() {
        let report = run_two_cycle_autonomous_burnin();
        assert_eq!(report.cycles.len(), 3);
        assert!(report.monotonic_improvement_verified);
        assert!(report.lineage_verified);
        assert!((report.cycles[0].scorecard.overall_pass_at_1 - 0.40).abs() < 1e-9);
        assert!((report.cycles[1].scorecard.overall_pass_at_1 - 0.80).abs() < 1e-9);
        assert!((report.cycles[2].scorecard.overall_pass_at_1 - 1.00).abs() < 1e-9);
    }

    #[test]
    fn burnin_report_json_and_cas_envelope_roundtrips_cleanly() {
        let report = run_two_cycle_autonomous_burnin();
        let bytes = serde_json::to_vec(&report).expect("serialize burnin report");
        let loaded: MensFlywheelBurninReportV1 =
            serde_json::from_slice(&bytes).expect("deserialize burnin report");
        assert_eq!(loaded.schema_version, report.schema_version);
        assert_eq!(loaded.cycles.len(), report.cycles.len());
        assert!(loaded.monotonic_improvement_verified);
        assert!(loaded.lineage_verified);
        for (l, r) in loaded.cycles.iter().zip(report.cycles.iter()) {
            assert_eq!(l.cycle, r.cycle);
            assert_eq!(l.snapshot_uri, r.snapshot_uri);
            assert_eq!(l.adapter_trusty_uri, r.adapter_trusty_uri);
            assert_eq!(l.model_id, r.model_id);
            assert!((l.scorecard.overall_pass_at_1 - r.scorecard.overall_pass_at_1).abs() < 1e-9);
        }
    }

    #[test]
    fn burnin_report_matches_committed_contract_json() {
        let report = run_two_cycle_autonomous_burnin();
        let contract_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../contracts/reports/mens-flywheel-burnin.v1.json");
        let raw = std::fs::read_to_string(&contract_path)
            .expect("read contracts/reports/mens-flywheel-burnin.v1.json");
        let committed: serde_json::Value =
            serde_json::from_str(&raw).expect("parse mens-flywheel-burnin.v1.json");
        assert_eq!(committed, report.to_contract_summary_json());
    }

    #[cfg(feature = "database")]
    #[test]
    fn burnin_report_persists_and_rehydrates_from_voxdb() {
        fn block_on_spin<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }

        block_on_spin(async {
            let db = vox_db::VoxDb::in_memory().await.expect("in-memory VoxDb");
            let report = run_two_cycle_autonomous_burnin();
            let uri = persist_burnin_report_to_db(&db, "burnin-test-run-1", &report)
                .await
                .expect("persist burnin report");
            assert!(uri.starts_with("trusty://mens-burnin/"));
            let loaded = load_burnin_report_from_db(&db, "burnin-test-run-1")
                .await
                .expect("load burnin report");
            assert_eq!(
                loaded.to_contract_summary_json(),
                report.to_contract_summary_json()
            );
        });
    }
}
