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
        let plan = vox_quantize::plan_quantize(&args.input, &mixture)?;
        let gib = |b: u64| b as f64 / (1024.0 * 1024.0 * 1024.0);
        let output_gib = gib(plan.output_bytes);
        if output_gib > target_gib {
            anyhow::bail!(
                "--to {} needs {output_gib:.2} GiB of weights (peak {:.2} GiB), target has ~{target_gib:.1} GiB, short by ~{:.2} GiB — try a larger tier or a different --to mixture",
                args.to,
                gib(plan.peak_bytes),
                output_gib - target_gib,
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
    fn run_rejects_a_model_whose_planned_output_exceeds_the_target() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.json"), b"{}").unwrap();
        // A real, small safetensors fixture (plan_quantize reads real
        // headers, not file sizes) with a matrix tensor sized big enough
        // that its exact Q4_K_M output easily exceeds a near-zero target.
        let mut map: std::collections::HashMap<String, candle_core::Tensor> =
            std::collections::HashMap::new();
        map.insert(
            "model.layers.0.self_attn.q_proj.weight".into(),
            candle_core::Tensor::zeros(
                (4096, 4096),
                candle_core::DType::F32,
                &candle_core::Device::Cpu,
            )
            .unwrap(),
        );
        candle_core::safetensors::save(&map, dir.path().join("model.safetensors")).unwrap();

        let out = tempfile::tempdir().unwrap();
        let err = run(QuantizeArgs {
            input: dir.path().to_path_buf(),
            output: out.path().to_path_buf(),
            to: "q4_k_m".into(),
            no_verify: false,
            device: "auto".into(),
            json: false,
            target_vram_gib: Some(0.001),
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
