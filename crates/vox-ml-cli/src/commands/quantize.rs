//! `vox quantize` — quantize a local SafeTensors model with vox-quantize.
use std::path::PathBuf;

use vox_quantize::{DevicePref, QuantMixture, quantize};

#[derive(Debug, clap::Args)]
pub struct QuantizeArgs {
    /// Model directory (must contain config.json + *.safetensors).
    #[arg(long)]
    pub input: PathBuf,
    /// Output directory for the quantized artifact.
    #[arg(long)]
    pub output: PathBuf,
    /// Target mixture: q4_k_m | q5_k_m | q6_k | q8_0
    #[arg(long, default_value = "q4_k_m")]
    pub to: String,
    /// Skip the round-trip verification pass.
    #[arg(long, default_value_t = false)]
    pub no_verify: bool,
    /// Device: auto | cuda | metal | cpu (default auto → GPU when available).
    #[arg(long, default_value = "auto")]
    pub device: String,
    /// Emit the full report as JSON instead of a table.
    #[arg(long, default_value_t = false)]
    pub json: bool,
    /// Pre-flight-reject the request if the mixture's estimated weights-only
    /// footprint won't fit this many GiB of usable VRAM (e.g. a device's
    /// usable-memory figure). Optional: when omitted, no check is done and
    /// behavior is unchanged from before this flag existed.
    #[arg(long)]
    pub target_vram_gib: Option<f64>,
}

/// Estimate a model's parameter count (in billions) from the total byte size
/// of its `*.safetensors` files, assuming a BF16 source (2 bytes/param) —
/// this pipeline's checkpoints are saved as bf16 safetensors (see
/// `vox-populi::mens::hub`). Fast, pre-flight, file-size-only: no tensor
/// data is read.
fn estimate_params_b(input_dir: &std::path::Path) -> anyhow::Result<f64> {
    let mut total_bytes = 0u64;
    for entry in std::fs::read_dir(input_dir)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|e| e == "safetensors") {
            total_bytes += entry.metadata()?.len();
        }
    }
    Ok(total_bytes as f64 / 2.0 / 1e9)
}

pub fn parse_mixture(s: &str) -> anyhow::Result<QuantMixture> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "q4_k_m" => QuantMixture::Q4KM,
        "q5_k_m" => QuantMixture::Q5KM,
        "q6_k" => QuantMixture::Q6K,
        "q8_0" => QuantMixture::Q8_0,
        other => {
            anyhow::bail!("unknown --to mixture `{other}` (expected q4_k_m|q5_k_m|q6_k|q8_0)")
        }
    })
}

pub fn parse_device(s: &str) -> anyhow::Result<DevicePref> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "auto" => DevicePref::Auto,
        "cuda" | "cuda:0" => DevicePref::Cuda(0),
        "metal" => DevicePref::Metal,
        "cpu" => DevicePref::Cpu,
        other => anyhow::bail!("unknown --device `{other}` (expected auto|cuda|metal|cpu)"),
    })
}

pub fn run(args: QuantizeArgs) -> anyhow::Result<()> {
    if !args.input.join("config.json").exists() {
        anyhow::bail!(
            "no config.json in {} — not a model directory",
            args.input.display()
        );
    }
    let mixture = parse_mixture(&args.to)?;
    let device = parse_device(&args.device)?;

    if let Some(target_gib) = args.target_vram_gib {
        let params_b = estimate_params_b(&args.input)?;
        if !vox_quantize::fits_target_tier(params_b, &mixture, target_gib) {
            let bpw = vox_quantize::policy::mixture_bpw(
                &mixture,
                vox_quantize::policy::QWEN3_27B_BOOSTED_ROLE_FRACTION,
            )
            .unwrap_or(f64::NAN);
            let needed = vox_quantize::needed_gib(params_b, bpw);
            anyhow::bail!(
                "{:.1}B params at --to {} needs ~{needed:.1} GiB, target has ~{target_gib:.1} GiB, short by ~{:.1} GiB — try a larger tier or a different --to mixture \
                 (estimate is weights-only and assumes the Qwen3-27B boosted-role split; it is approximate for other architectures)",
                params_b,
                args.to,
                needed - target_gib,
            );
        }
    }

    let req = vox_quantize::QuantizeRequest {
        input_dir: args.input.clone(),
        output_dir: args.output.clone(),
        mixture,
        verify: !args.no_verify,
        device,
    };
    let report = quantize(&req)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!(
        "{:<60} {:>8} {:>12} {:>10}",
        "tensor", "dtype", "params", "mse"
    );
    for s in &report.tensors {
        let note = if s.fallback { " (fallback)" } else { "" };
        println!(
            "{:<60} {:>8} {:>12} {:>10.2e}{}",
            s.name, s.target_dtype, s.params, s.mse, note
        );
    }
    let gib = |b: u64| b as f64 / (1024.0 * 1024.0 * 1024.0);
    println!(
        "\n{:.2} GiB -> {:.2} GiB  ({:.2}x)   worst MSE {:.2e}",
        gib(report.total_src_bytes),
        gib(report.total_quant_bytes),
        report.compression_ratio,
        report.worst_mse,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mixture_maps_known_values() {
        assert!(matches!(
            parse_mixture("q4_k_m").unwrap(),
            QuantMixture::Q4KM
        ));
        assert!(matches!(parse_mixture("Q8_0").unwrap(), QuantMixture::Q8_0));
        assert!(parse_mixture("bogus").is_err());
    }

    #[test]
    fn parse_device_maps_known_values() {
        assert!(matches!(parse_device("auto").unwrap(), DevicePref::Auto));
        assert!(matches!(parse_device("cuda").unwrap(), DevicePref::Cuda(0)));
        assert!(matches!(parse_device("cpu").unwrap(), DevicePref::Cpu));
        assert!(parse_device("gpu").is_err());
    }

    #[test]
    fn estimate_params_b_halves_safetensors_bytes_for_bf16() {
        let dir = tempfile::tempdir().unwrap();
        // 54e9 bytes of bf16 (2 bytes/param) -> 27B params. A sparse file
        // (set_len, no actual writes) is enough since only the declared
        // length is read.
        let f = std::fs::File::create(dir.path().join("model-00001.safetensors")).unwrap();
        f.set_len(54_000_000_000).unwrap();
        // A non-safetensors file must be ignored.
        std::fs::write(dir.path().join("config.json"), b"{}").unwrap();

        let params_b = estimate_params_b(dir.path()).unwrap();
        assert!(
            (params_b - 27.0).abs() < 1e-6,
            "expected ~27.0B params, got {params_b}"
        );
    }

    #[test]
    fn run_rejects_q4_k_m_27b_against_16gb_target() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.json"), b"{}").unwrap();
        let f = std::fs::File::create(dir.path().join("model-00001.safetensors")).unwrap();
        f.set_len(54_000_000_000).unwrap(); // ~27B bf16 params

        let out = tempfile::tempdir().unwrap();
        let err = run(QuantizeArgs {
            input: dir.path().to_path_buf(),
            output: out.path().to_path_buf(),
            to: "q4_k_m".into(),
            no_verify: false,
            device: "auto".into(),
            json: false,
            target_vram_gib: Some(15.1),
        })
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("short by"), "unexpected message: {msg}");
    }

    #[test]
    fn run_without_target_vram_gib_skips_the_fit_check() {
        // Same oversized 27B fixture, but no --target-vram-gib: run() must
        // not bail on the fit check (it proceeds to actual quantization,
        // which fails on this fixture's empty/fake safetensors payload for
        // an unrelated reason — the point here is only that the error is
        // NOT the fit-check's "short by" message, proving the flag's
        // default-off behavior is unchanged).
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.json"), b"{}").unwrap();
        let f = std::fs::File::create(dir.path().join("model-00001.safetensors")).unwrap();
        f.set_len(54_000_000_000).unwrap();

        let out = tempfile::tempdir().unwrap();
        let err = run(QuantizeArgs {
            input: dir.path().to_path_buf(),
            output: out.path().to_path_buf(),
            to: "q4_k_m".into(),
            no_verify: false,
            device: "auto".into(),
            json: false,
            target_vram_gib: None,
        })
        .unwrap_err();
        assert!(!err.to_string().contains("short by"));
    }
}
