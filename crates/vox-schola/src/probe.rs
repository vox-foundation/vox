//! `vox-schola probe` — GPU capability report and recommended training profile.

use anyhow::Result;

pub fn run() -> Result<()> {
    let info = vox_populi::mens::probe_gpu();
    println!("GPU vendor:    {}", info.vendor);
    println!("GPU model:     {}", info.model_name);
    println!("VRAM:          {} MB", info.vram_mb);
    println!();

    // Resolve recommended preset from current GPU
    let device_profile =
        vox_populi::mens::DeviceProfile::from_gpu_info(&info.model_name, info.vram_mb);
    let overrides = vox_populi::mens::CliOverrides::default();
    let profile =
        vox_populi::mens::resolve_effective_profile(None, device_profile, None, overrides);

    println!("Recommended profile:");
    println!("  rank:       {}", profile.rank);
    println!("  alpha:      {}", profile.alpha);
    println!("  seq_len:    {}", profile.seq_len);
    println!("  batch_size: {}", profile.batch_size);
    println!("  grad_accum: {}", profile.grad_accum);
    println!("  epochs:     {}", profile.epochs);
    println!("  lr:         {:.2e}", profile.lr);
    println!("  warmup:     {}", profile.warmup);
    println!();
    println!("Quick start:");
    println!("  vox-schola train --model Qwen/Qwen2.5-Coder-1.5B-Instruct");
    println!("  vox-schola train --model Qwen/Qwen2.5-Coder-1.5B-Instruct --preset 4080");
    Ok(())
}
