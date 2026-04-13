use std::sync::Arc;
use tokio::time::{interval, Duration};
use vox_corpus::flywheel::{FlywheelState, FlywheelConfig, FlywheelSignal};
use crate::Orchestrator;

/// Background service that monitors corpus diversity and triggers training.
pub struct FlywheelMonitor {
    _orch: Arc<Orchestrator>,
    state: FlywheelState,
}

impl FlywheelMonitor {
    pub fn new(orch: Arc<Orchestrator>) -> Self {
        let config = FlywheelConfig {
            sample_floor: 1000,
            min_ast_diversity: 0.40,
            auto_train: true,
            ..Default::default()
        };
        Self {
            _orch: orch,
            state: FlywheelState::new(config),
        }
    }

    pub async fn spawn(self) {
        let mut tick = interval(Duration::from_secs(300)); // Check every 5 minutes
        let state = self.state;
        let orch = self._orch.clone();
        let training_in_progress = Arc::new(tokio::sync::Mutex::new(false));

        tokio::spawn(async move {
            let flywheel = state;
            loop {
                tick.tick().await;
                
                // 1. Gather current metrics from Codex
                let (current_samples, current_diversity) = {
                    if let Some(db) = orch.db() {
                        let snapshot = db.get_latest_corpus_snapshot().await.ok().flatten();
                        let metrics = db.list_research_metrics_by_type("ast_diversity", "corpus_diversity_check", 1).await.ok();
                        
                        let count = snapshot.map(|(_, tp, _)| tp).unwrap_or(0);
                        let diversity = metrics.and_then(|m| m.first().and_then(|(_, val, _)| *val)).unwrap_or(0.0);
                        (count as usize, diversity)
                    } else {
                        (0, 0.0)
                    }
                };
                
                // 2. Evaluate
                match flywheel.check(current_samples, current_diversity) {
                    FlywheelSignal::Ready { ast_diversity } => {
                        tracing::info!(current_samples, ast_diversity, "Flywheel: Training gate PASSED. Ready for wave.");
                        
                        // 3. Trigger training if auto_train is true and not already running
                        if flywheel.config.auto_train {
                            let training_in_progress = training_in_progress.clone();
                            let in_progress = training_in_progress.lock().await;
                            if !*in_progress {
                                tracing::info!("Flywheel: Auto-dispatching autonomous training wave...");
                                drop(in_progress);
                                
                                let training_flag = training_in_progress.clone();
                                tokio::spawn(async move {
                                    {
                                        let mut g = training_flag.lock().await;
                                        *g = true;
                                    }
                                    let res = trigger_autonomous_training().await;
                                    if let Err(e) = res {
                                        tracing::error!(error = %e, "Flywheel: Autonomous training wave FAILED");
                                    } else {
                                        tracing::info!("Flywheel: Autonomous training wave COMPLETED successfully");
                                    }
                                    let mut g = training_flag.lock().await;
                                    *g = false;
                                });
                            } else {
                                tracing::debug!("Flywheel: Training already in progress, skipping trigger.");
                            }
                        }
                    }
                    FlywheelSignal::Pending { new_samples } => {
                        tracing::debug!(new_samples, "Flywheel: Accumulating samples...");
                    }
                    _ => {
                        tracing::debug!("Flywheel: Idle (waiting for diversity signal/samples)");
                    }
                }
            }
        });
    }
}

async fn trigger_autonomous_training() -> anyhow::Result<()> {
    let status = tokio::process::Command::new("cargo")
        .arg("run")
        .arg("--features")
        .arg("gpu")
        .arg("--bin")
        .arg("vox")
        .arg("--")
        .arg("mens")
        .arg("train")
        .arg("--config")
        .arg("mens/config/mix.yaml")
        .arg("--backend")
        .arg("qlora")
        .arg("--tokenizer")
        .arg("hf")
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("Autonomous training subprocess exited with code {:?}", status.code());
    }
    Ok(())
}
