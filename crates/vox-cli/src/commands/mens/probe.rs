//! `vox mens probe` — GPU capability detection and recommended config.

use anyhow::Result;

pub fn run_probe(verbose: bool) -> Result<()> {
    use owo_colors::OwoColorize;
    use vox_mens::probe_gpu;

    let info = probe_gpu();
    println!("GPU: {} (~{} MB)", info.model_name, info.vram_mb);

    if verbose {
        let (rank, batch_size, seq_len) = recommend_lora_from_vram(info.vram_mb);
        eprintln!();
        eprintln!(
            "  Recommended LoRA config for this hardware (heuristic from ~{} MB VRAM):",
            info.vram_mb
        );
        eprintln!(
            "    --rank {}  --batch-size {}  --seq-len {}",
            rank, batch_size, seq_len
        );
        eprintln!();
        eprintln!("  Full command:");
        eprintln!(
            "    {} mens train --device vulkan --rank {} --batch-size {} --seq-len {}",
            "vox".cyan(),
            rank,
            batch_size,
            seq_len,
        );
    }
    Ok(())
}

/// Best-effort LoRA hyperparameters from reported VRAM (MB).
fn recommend_lora_from_vram(vram_mb: u64) -> (usize, usize, usize) {
    match vram_mb {
        0..=4096 => (8, 1, 256),
        4097..=8192 => (12, 2, 384),
        8193..=16384 => (16, 4, 512),
        _ => (32, 4, 512),
    }
}
