from pathlib import Path

p = Path("crates/vox-populi/src/tensor/candle_qlora_train.rs")
t = p.read_text(encoding="utf-8")
old = r"""                if last_progress.elapsed() >= progress_every {
                    last_progress = Instant::now();
                    let loss_s = train_log::format_loss_for_log(loss);
                    train_log::info(&format!(
                        "candle qlora-rs progress epoch {epoch} step {global_step} loss={loss_s} \
                         skips_vocab={skips_bad_vocab} skips_hidden={skips_last_hidden} skips_short_seq={skips_short_seq}"
                    ));
                }"""
new = r"""                if last_progress.elapsed() >= progress_every {
                    last_progress = Instant::now();
                    let loss_s = train_log::format_loss_for_log(loss);
                    let eta_suffix = if planned_steps_total > 0 {
                        let elapsed = training_wall_start.elapsed().as_secs_f64();
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
                        let pct = 100.0 * global_step as f64 / planned_steps_total as f64;
                        let eta_hms = format_eta_hms(eta_sec);
                        format!(" eta_remaining≈{eta_hms} ({pct:.1}% of planned steps)")
                    } else {
                        String::new()
                    };
                    train_log::info(&format!(
                        "candle qlora-rs progress epoch {epoch} step {global_step} loss={loss_s} \
                         skips_vocab={skips_bad_vocab} skips_hidden={skips_last_hidden} skips_short_seq={skips_short_seq}{eta_suffix}"
                    ));
                }"""
if old not in t:
    raise SystemExit("OLD block not found")
p.write_text(t.replace(old, new, 1), encoding="utf-8")
print("ok progress block")
