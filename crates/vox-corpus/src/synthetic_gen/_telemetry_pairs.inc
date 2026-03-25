// ─── Telemetry interpretation pairs (Gap 10) ──────────────────────────────────

/// Training pairs for interpreting Vox telemetry output (training logs,
/// Codex events, VoxDb records). Teaches the model to read and act on
/// telemetry rather than being blind to training progress.
pub fn generate_telemetry_interpretation_pairs(
    out: &mut impl Write,
    _cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;

    let scenarios: &[(&str, &str, &str)] = &[
        (
            r#"{"event":"train_step","payload":{"step":47,"loss":38.29,"tokens_per_sec":622.9,"eta_sec":240000}}"#,
            "interpret_training_step",
            "Step 47 of the training run. Loss of 38.29 is high but within early-training variance — monitor for trend across 100+ steps. Throughput 622.9 tok/s is healthy for a single-GPU CUDA run. ETA 240000s (~66h) is expected for 283K planned steps at this throughput.",
        ),
        (
            r#"{"event":"train_complete","payload":{"wall_seconds":86400,"mean_steps_per_sec":1.18,"steps_executed":102240}}"#,
            "interpret_training_complete",
            "Training completed in 24h. 102,240 steps at 1.18 step/s. Check the adapter manifest at mens/runs/v1/populi_adapter_manifest_v3.json and the telemetry.jsonl for per-epoch loss trends before promoting the adapter.",
        ),
        (
            r#"{"event":"checkpoint_saved","payload":{"epoch":1,"global_step":31504,"path":"mens/runs/v1/candle_qlora_adapter_epoch1.safetensors"}}"#,
            "interpret_checkpoint_saved",
            "Epoch 1 checkpoint saved at step 31504. The safetensors file contains the LoRA A/B matrices for this epoch. You can inspect it with 'vox schola merge-qlora' or use it as an intermediate checkpoint for early inference testing.",
        ),
        (
            "[Epoch 2/3 Step 500] Loss: 2.14 | Skips: VCB:3 HID:0 SEQ:0 | ETA≈12h30m",
            "interpret_training_log_line",
            "Good progress: loss 2.14 is significantly lower than the typical 13-38 range in early steps. 3 vocabulary skip events (VCB) occurred — tokens not in the model's vocabulary were skipped, which is normal for Vox-specific syntax tokens. No hidden-state or sequence-length skips. ETA 12.5h is on track.",
        ),
        (
            "How do I monitor training progress in real time?",
            "telemetry_monitoring",
            "Tail the training log: Get-Content mens/runs/v1/train_*.log -Wait -Tail 25. For structured telemetry: Get-Content mens/runs/v1/telemetry.jsonl -Wait -Tail 5 | ForEach-Object { $_ | ConvertFrom-Json }. For VoxDb events: vox codex verify checks schema health; the telemetry channel logs train_step events every 20 steps.",
        ),
    ];

    let prompts = [
        "Interpret this telemetry output: {data}",
        "What does this training event mean? {data}",
        "Explain this log line: {data}",
        "Is this telemetry healthy? {data}",
    ];

    for (data, category, interpretation) in scenarios {
        for tmpl in &prompts {
            let prompt = tmpl.replace("{data}", data);
            let response = json!({
                "interpretation": interpretation,
                "event_type": category,
            });
            emit_line(out, &prompt, &response, category, "telemetry_trace")?;
            count += 1;
        }
    }
    Ok(count)
}
