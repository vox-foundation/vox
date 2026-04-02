//! Forward `vox-schola train` to canonical `vox mens train` when a `vox` binary is discoverable.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Policy from `VOX_SCHOLA_FORWARD`: default `auto` forwards when `vox` is found; `never` keeps the
/// standalone trainer; `always` requires `vox` and errors if missing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ForwardPolicy {
    Auto,
    Never,
    Always,
}

fn forward_policy() -> ForwardPolicy {
    match std::env::var("VOX_SCHOLA_FORWARD").as_deref() {
        Ok("never") => ForwardPolicy::Never,
        Ok("always") => ForwardPolicy::Always,
        _ => ForwardPolicy::Auto,
    }
}

pub(crate) fn resolve_vox_executable() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var("VOX_EXE") {
        let p = PathBuf::from(raw.trim());
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(self_exe) = std::env::current_exe() {
        if let Some(dir) = self_exe.parent() {
            for name in ["vox.exe", "vox"] {
                let cand = dir.join(name);
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
    }
    which::which("vox").ok()
}

pub(crate) fn effective_force_restart(
    resume: &Option<PathBuf>,
    resume_checkpoint: bool,
    force_restart: bool,
) -> bool {
    if resume.is_some() {
        force_restart
    } else {
        force_restart || !resume_checkpoint
    }
}

fn push_path(out: &mut Vec<String>, flag: &str, p: &Path) {
    out.push(flag.to_string());
    out.push(p.display().to_string());
}

/// Build argv **after** the `vox` executable: `mens train …`.
#[must_use]
pub(crate) fn build_vox_mens_train_forward_argv(
    model: &Option<String>,
    device: &str,
    data_dir: &Path,
    output_dir: &Path,
    preset: &Option<String>,
    rank: Option<usize>,
    alpha: Option<f32>,
    seq_len: Option<usize>,
    checkpoint_every: Option<usize>,
    batch_size: Option<usize>,
    grad_accum: Option<usize>,
    epochs: Option<usize>,
    lr: Option<f64>,
    warmup: Option<usize>,
    seed: u64,
    min_rating: Option<u8>,
    resume: &Option<PathBuf>,
    resume_checkpoint: bool,
    force_restart: bool,
    adapter_tag: &Option<String>,
    context_filter: &Option<String>,
    vram_limit_fraction: Option<f32>,
    background: bool,
    log_dir: &Option<PathBuf>,
    qlora_no_double_quant: bool,
    qlora_require_full_proxy_stack: bool,
    qlora_allow_partial_proxy_stack: bool,
    qlora_lm_head_only: bool,
    qlora_max_skip_rate: Option<f32>,
    qlora_proxy_max_layers: Option<usize>,
    qlora_ce_last_k: usize,
    base_model_family: &Option<String>,
    upstream_model_id: &Option<String>,
    license_class: &Option<String>,
    attribution_required: bool,
    trajectory_weighting_enabled: bool,
    trajectory_tool_trace_boost: f32,
    trajectory_failure_category_boost: f32,
    trajectory_quality_floor: Option<u8>,
    trajectory_quality_boost: f32,
) -> Vec<String> {
    let mut out = Vec::new();
    out.extend([
        "mens".to_string(),
        "train".to_string(),
        "--backend".to_string(),
        "qlora".to_string(),
        "--tokenizer".to_string(),
        "hf".to_string(),
    ]);
    if let Some(m) = model {
        out.push("--model".into());
        out.push(m.clone());
    }
    out.push("--device".into());
    out.push(device.to_string());
    push_path(&mut out, "--data-dir", data_dir);
    push_path(&mut out, "--output-dir", output_dir);
    if let Some(p) = preset {
        out.push("--preset".into());
        out.push(p.clone());
    }
    if let Some(r) = rank {
        out.push("--rank".into());
        out.push(r.to_string());
    }
    if let Some(a) = alpha {
        out.push("--alpha".into());
        out.push(a.to_string());
    }
    if let Some(s) = seq_len {
        out.push("--seq-len".into());
        out.push(s.to_string());
    }
    if let Some(c) = checkpoint_every {
        out.push("--checkpoint-every".into());
        out.push(c.to_string());
    }
    if let Some(b) = batch_size {
        out.push("--batch-size".into());
        out.push(b.to_string());
    }
    if let Some(g) = grad_accum {
        out.push("--grad-accum".into());
        out.push(g.to_string());
    }
    if let Some(e) = epochs {
        out.push("--epochs".into());
        out.push(e.to_string());
    }
    if let Some(l) = lr {
        out.push("--lr".into());
        out.push(l.to_string());
    }
    if let Some(w) = warmup {
        out.push("--warmup".into());
        out.push(w.to_string());
    }
    out.push("--seed".into());
    out.push(seed.to_string());
    if let Some(m) = min_rating {
        out.push("--min-rating".into());
        out.push(m.to_string());
    }
    if let Some(r) = resume {
        out.push("--resume".into());
        out.push(r.display().to_string());
    }
    if effective_force_restart(resume, resume_checkpoint, force_restart) {
        out.push("--force-restart".into());
    }
    if let Some(t) = adapter_tag {
        out.push("--adapter-tag".into());
        out.push(t.clone());
    }
    if let Some(c) = context_filter {
        out.push("--context-filter".into());
        out.push(c.clone());
    }
    if let Some(v) = vram_limit_fraction {
        out.push("--vram-limit-fraction".into());
        out.push(v.to_string());
    }
    if background {
        out.push("--background".into());
    }
    if let Some(ld) = log_dir {
        out.push("--log-dir".into());
        out.push(ld.display().to_string());
    }
    if qlora_no_double_quant {
        out.push("--qlora-no-double-quant".into());
    }
    if qlora_require_full_proxy_stack {
        out.push("--qlora-require-full-proxy-stack".into());
    }
    if qlora_allow_partial_proxy_stack {
        out.push("--qlora-allow-partial-proxy-stack".into());
    }
    if qlora_lm_head_only {
        out.push("--qlora-lm-head-only".into());
    }
    if let Some(r) = qlora_max_skip_rate {
        out.push("--qlora-max-skip-rate".into());
        out.push(r.to_string());
    }
    if let Some(n) = qlora_proxy_max_layers {
        out.push("--qlora-proxy-max-layers".into());
        out.push(n.to_string());
    }
    out.push("--qlora-ce-last-k".into());
    out.push(qlora_ce_last_k.to_string());
    if let Some(f) = base_model_family {
        out.push("--base-model-family".into());
        out.push(f.clone());
    }
    if let Some(u) = upstream_model_id {
        out.push("--upstream-model-id".into());
        out.push(u.clone());
    }
    if let Some(l) = license_class {
        out.push("--license-class".into());
        out.push(l.clone());
    }
    if attribution_required {
        out.push("--attribution-required".into());
    }
    if trajectory_weighting_enabled {
        out.push("--trajectory-weighting-enabled".into());
    }
    out.push("--trajectory-tool-trace-boost".into());
    out.push(trajectory_tool_trace_boost.to_string());
    out.push("--trajectory-failure-category-boost".into());
    out.push(trajectory_failure_category_boost.to_string());
    if let Some(q) = trajectory_quality_floor {
        out.push("--trajectory-quality-floor".into());
        out.push(q.to_string());
    }
    out.push("--trajectory-quality-boost".into());
    out.push(trajectory_quality_boost.to_string());
    out
}

pub(crate) fn maybe_forward_to_vox(
    model: &Option<String>,
    device: &str,
    data_dir: &Path,
    output_dir: &Path,
    preset: &Option<String>,
    rank: Option<usize>,
    alpha: Option<f32>,
    seq_len: Option<usize>,
    checkpoint_every: Option<usize>,
    batch_size: Option<usize>,
    grad_accum: Option<usize>,
    epochs: Option<usize>,
    lr: Option<f64>,
    warmup: Option<usize>,
    seed: u64,
    min_rating: Option<u8>,
    resume: &Option<PathBuf>,
    resume_checkpoint: bool,
    force_restart: bool,
    adapter_tag: &Option<String>,
    context_filter: &Option<String>,
    vram_limit_fraction: Option<f32>,
    background: bool,
    log_dir: &Option<PathBuf>,
    skip_corpus_mix: bool,
    qlora_no_double_quant: bool,
    qlora_require_full_proxy_stack: bool,
    qlora_allow_partial_proxy_stack: bool,
    qlora_lm_head_only: bool,
    qlora_max_skip_rate: Option<f32>,
    qlora_proxy_max_layers: Option<usize>,
    qlora_ce_last_k: usize,
    base_model_family: &Option<String>,
    upstream_model_id: &Option<String>,
    license_class: &Option<String>,
    attribution_required: bool,
    trajectory_weighting_enabled: bool,
    trajectory_tool_trace_boost: f32,
    trajectory_failure_category_boost: f32,
    trajectory_quality_floor: Option<u8>,
    trajectory_quality_boost: f32,
) -> anyhow::Result<Option<std::process::ExitStatus>> {
    if std::env::var("VOX_SCHOLA_TRAIN_IN_PROCESS")
        .ok()
        .as_deref()
        == Some("1")
    {
        return Ok(None);
    }

    let policy = forward_policy();
    if matches!(policy, ForwardPolicy::Never) {
        return Ok(None);
    }

    let vox_exe = resolve_vox_executable();
    let Some(vox_exe) = vox_exe else {
        if matches!(policy, ForwardPolicy::Always) {
            anyhow::bail!(
                "VOX_SCHOLA_FORWARD=always but no `vox` executable found (set VOX_EXE or install `vox` on PATH)"
            );
        }
        return Ok(None);
    };

    let argv = build_vox_mens_train_forward_argv(
        model,
        device,
        data_dir,
        output_dir,
        preset,
        rank,
        alpha,
        seq_len,
        checkpoint_every,
        batch_size,
        grad_accum,
        epochs,
        lr,
        warmup,
        seed,
        min_rating,
        resume,
        resume_checkpoint,
        force_restart,
        adapter_tag,
        context_filter,
        vram_limit_fraction,
        background,
        log_dir,
        qlora_no_double_quant,
        qlora_require_full_proxy_stack,
        qlora_allow_partial_proxy_stack,
        qlora_lm_head_only,
        qlora_max_skip_rate,
        qlora_proxy_max_layers,
        qlora_ce_last_k,
        base_model_family,
        upstream_model_id,
        license_class,
        attribution_required,
        trajectory_weighting_enabled,
        trajectory_tool_trace_boost,
        trajectory_failure_category_boost,
        trajectory_quality_floor,
        trajectory_quality_boost,
    );

    eprintln!(
        "note: `vox-schola train` forwards to canonical `{}` (`vox mens train`, {} args).",
        vox_exe.display(),
        argv.len()
    );
    eprintln!("      Set VOX_SCHOLA_FORWARD=never to run the standalone schola trainer.");

    let mut cmd = Command::new(&vox_exe);
    cmd.args(&argv);
    if skip_corpus_mix {
        cmd.env("VOX_TRAIN_SKIP_CORPUS_MIX", "1");
    }
    let status = cmd
        .status()
        .map_err(|e| anyhow::anyhow!("failed to spawn `{}`: {e}", vox_exe.display()))?;
    Ok(Some(status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn forward_argv_starts_with_mens_train_qlora_hf() {
        let args = build_vox_mens_train_forward_argv(
            &Some("Qwen/Qwen3.5-4B".into()),
            "cuda",
            Path::new("target/dogfood"),
            Path::new("mens/runs/out"),
            &None,
            None,
            None,
            None,
            Some(500),
            None,
            None,
            None,
            None,
            None,
            42,
            None,
            &None,
            false,
            false,
            &None,
            &None,
            None,
            false,
            &None,
            false,
            false,
            false,
            false,
            None,
            None,
            64,
            &None,
            &None,
            &None,
            false,
            false,
            1.1,
            1.15,
            None,
            1.05,
        );
        assert_eq!(args.first().map(String::as_str), Some("mens"));
        assert_eq!(args.get(1).map(String::as_str), Some("train"));
        assert!(args.windows(2).any(|w| w[0] == "--backend" && w[1] == "qlora"));
    }

    #[test]
    fn effective_force_restart_maps_resume_checkpoint() {
        assert!(
            effective_force_restart(&None, false, false),
            "fresh run without continue => force restart"
        );
        assert!(
            !effective_force_restart(&None, true, false),
            "continue-run without resume path => load checkpoint if present"
        );
        assert!(
            !effective_force_restart(&Some(PathBuf::from("o")), false, false),
            "explicit resume dir: only force_restart flag matters"
        );
        assert!(
            effective_force_restart(&Some(PathBuf::from("o")), false, true),
            "explicit resume + force_restart"
        );
    }
}
