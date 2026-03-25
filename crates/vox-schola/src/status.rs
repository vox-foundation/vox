//! `vox-schola status` — show training run status from telemetry log.

use std::path::PathBuf;

use anyhow::Result;

pub fn run(run_dir: Option<PathBuf>) -> Result<()> {
    let dir = run_dir.unwrap_or_else(|| PathBuf::from("mens/runs/latest"));
    let log = dir.join("telemetry_log.jsonl");
    if !log.is_file() {
        println!("No telemetry log found at {}", log.display());
        println!("Start training with: vox-schola train --model <HF_REPO>");
        return Ok(());
    }

    let content = std::fs::read_to_string(&log)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", log.display()))?;
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    println!("Run dir:     {}", dir.display());
    println!("Events:      {total}");

    let mut last_step = None::<u64>;
    let mut last_loss = None::<f64>;
    let mut last_epoch = None::<u64>;
    let mut status = "unknown";
    let mut last_lr = None::<f64>;

    for line in &lines {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(e) = v.get("event").and_then(|x| x.as_str()) {
                match e {
                    "train_start" => status = "running",
                    "train_complete" => status = "complete ✓",
                    "train_failed" => status = "FAILED ✗",
                    "train_step" | "checkpoint" => {
                        if let Some(s) = v.get("step").and_then(|x| x.as_u64()) {
                            last_step = Some(s);
                        }
                        if let Some(l) = v.get("loss").and_then(|x| x.as_f64()) {
                            last_loss = Some(l);
                        }
                        if let Some(ep) = v.get("epoch").and_then(|x| x.as_u64()) {
                            last_epoch = Some(ep);
                        }
                        if let Some(lr) = v.get("lr").and_then(|x| x.as_f64()) {
                            last_lr = Some(lr);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    println!("Status:      {status}");
    if let Some(ep) = last_epoch {
        println!("Last epoch:  {ep}");
    }
    if let Some(s) = last_step {
        println!("Last step:   {s}");
    }
    if let Some(l) = last_loss {
        println!("Last loss:   {:.4}", l);
    }
    if let Some(lr) = last_lr {
        println!("Last LR:     {:.2e}", lr);
    }

    let ckpt = dir.join("checkpoint_state.json");
    if ckpt.is_file() {
        println!("\nCheckpoint: {} (resume available)", ckpt.display());
        println!("  Resume with: vox-schola train --resume {}", dir.display());
    }

    let adapter = dir.join("candle_qlora_adapter.safetensors");
    if adapter.is_file() {
        println!("\nFinal adapter: {}", adapter.display());
        println!("  Serve with: vox-schola serve --model {}", dir.display());
    }

    Ok(())
}
