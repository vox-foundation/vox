from pathlib import Path

p = Path("crates/vox-populi/src/tensor/candle_qlora_train.rs")
t = p.read_text(encoding="utf-8")
old = r"""                if global_step.is_multiple_of(20) {
                    let loss_s = train_log::format_loss_for_log(loss);
                    train_log::info(&format!(
                        "candle qlora-rs epoch {epoch} step {global_step} loss={loss_s}"
                    ));
                    telemetry::append(
                        &out,
                        telemetry_schema::events::TRAIN_STEP,
                        serde_json::json!({
                            telemetry_schema::keys::EXECUTION_KERNEL: "candle_qlora",
                            telemetry_schema::keys::EPOCH: epoch,
                            telemetry_schema::keys::STEP: global_step,
                            telemetry_schema::keys::LOSS: loss,
                            "logits_shape": stacked_lm_logits_shape(1, 1, bundle.vocab),
                            "skips_bad_vocab": skips_bad_vocab,
                            "skips_last_hidden": skips_last_hidden,
                            "skips_short_seq": skips_short_seq,
                        }),
                    )?;
                }"""
new = r"""                if global_step.is_multiple_of(20) {
                    let loss_s = train_log::format_loss_for_log(loss);
                    train_log::info(&format!(
                        "candle qlora-rs epoch {epoch} step {global_step} loss={loss_s}"
                    ));
                    let elapsed = training_wall_start.elapsed().as_secs_f64();
                    let (eta_val, frac_val) = if planned_steps_total > 0 {
                        let sps = if elapsed >= 1.0 {
                            global_step as f64 / elapsed
                        } else {
                            0.0
                        };
                        let remaining = planned_steps_total.saturating_sub(global_step as u64);
                        let eta_sec = if sps > 1e-6 {
                            remaining as f64 / sps
                        } else {
                            f64::NAN
                        };
                        let frac = global_step as f64 / planned_steps_total as f64;
                        (
                            serde_json::Value::from(eta_sec),
                            serde_json::Value::from(frac),
                        )
                    } else {
                        (serde_json::Value::Null, serde_json::Value::Null)
                    };
                    telemetry::append(
                        &out,
                        telemetry_schema::events::TRAIN_STEP,
                        serde_json::json!({
                            telemetry_schema::keys::EXECUTION_KERNEL: "candle_qlora",
                            telemetry_schema::keys::EPOCH: epoch,
                            telemetry_schema::keys::STEP: global_step,
                            telemetry_schema::keys::LOSS: loss,
                            "logits_shape": stacked_lm_logits_shape(1, 1, bundle.vocab),
                            "skips_bad_vocab": skips_bad_vocab,
                            "skips_last_hidden": skips_last_hidden,
                            "skips_short_seq": skips_short_seq,
                            telemetry_schema::keys::PLANNED_STEPS_TOTAL: planned_steps_total,
                            telemetry_schema::keys::ETA_SECONDS_REMAINING: eta_val,
                            telemetry_schema::keys::PROGRESS_FRACTION: frac_val,
                        }),
                    )?;
                }"""
if old not in t:
    raise SystemExit("OLD block not found")
p.write_text(t.replace(old, new, 1), encoding="utf-8")
print("ok train_step telemetry")
