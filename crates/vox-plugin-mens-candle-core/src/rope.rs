//! RoPE inverse-frequency synthesis shared by training and inference.
//!
//! This function used to be forked FOUR ways: `inference.rs` and
//! `candle_qlora_train/mod.rs` each carried their own copy, in both
//! `vox-plugin-mens-candle-metal` and `vox-plugin-mens-candle-cuda` — with a
//! comment on the `inference.rs` copies claiming they were "kept byte-for-byte
//! in sync" with the trainer's copy "by hand". A hand-kept invariant across
//! four call sites in two crates is exactly the kind of thing that drifts
//! silently; this is now the single definition all four call sites use, so
//! there is nothing left to keep in sync.
use anyhow::Result;
use candle_core::{Device, Tensor};

/// Synthesize RoPE inverse-frequency table from `rope_theta`.
///
/// Training and inference must apply the *same* rotary frequencies, or a
/// trained adapter's attention pattern silently shifts at serve time.
pub fn synthesize_rope_inv_freq(
    head_dim: usize,
    rope_theta: Option<f64>,
    device: &Device,
) -> Result<Tensor> {
    let half = head_dim / 2;
    if half == 0 {
        anyhow::bail!("invalid head_dim={head_dim} for RoPE synthesis");
    }
    let theta = rope_theta.unwrap_or(10_000.0) as f32;
    let hd = head_dim as f32;
    let mut vals = Vec::with_capacity(half);
    for i in 0..half {
        let exponent = (2.0_f32 * i as f32) / hd;
        vals.push(1.0_f32 / theta.powf(exponent));
    }
    Ok(Tensor::from_vec(vals, (half,), device)?)
}

/// Synthesize RoPE inverse-frequency table with optional Llama-3.1 wavelength scaling.
///
/// When `llama3_scaling` is `None`, delegates directly to [`synthesize_rope_inv_freq`]
/// so Qwen3 and other unscaled checkpoints stay bitwise identical.
pub fn synthesize_rope_inv_freq_with_llama3_scaling(
    head_dim: usize,
    rope_theta: Option<f64>,
    llama3_scaling: Option<vox_hf_layout::Llama3RopeScaling>,
    device: &Device,
) -> Result<Tensor> {
    let Some(sc) = llama3_scaling else {
        return synthesize_rope_inv_freq(head_dim, rope_theta, device);
    };
    let half = head_dim / 2;
    if half == 0 {
        anyhow::bail!("invalid head_dim={head_dim} for RoPE synthesis");
    }
    if !(sc.factor.is_finite()
        && sc.low_freq_factor.is_finite()
        && sc.high_freq_factor.is_finite()
        && sc.factor > 0.0
        && sc.high_freq_factor > sc.low_freq_factor
        && sc.original_max_position_embeddings > 0)
    {
        anyhow::bail!("invalid Llama3 RoPE scaling parameters: {sc:?}");
    }
    let theta = rope_theta.unwrap_or(10_000.0);
    let hd = head_dim as f64;
    let orig = sc.original_max_position_embeddings as f64;
    let low_freq_wavelen = orig / sc.low_freq_factor;
    let high_freq_wavelen = orig / sc.high_freq_factor;
    let mut vals = Vec::with_capacity(half);
    for i in 0..half {
        let exponent = (2.0_f64 * i as f64) / hd;
        let inv_freq = 1.0_f64 / theta.powf(exponent);
        let wavelen = (2.0_f64 * std::f64::consts::PI) / inv_freq;
        let scaled = if wavelen < high_freq_wavelen {
            inv_freq
        } else if wavelen > low_freq_wavelen {
            inv_freq / sc.factor
        } else {
            let smooth =
                (orig / wavelen - sc.low_freq_factor) / (sc.high_freq_factor - sc.low_freq_factor);
            (1.0_f64 - smooth) * (inv_freq / sc.factor) + smooth * inv_freq
        };
        vals.push(scaled as f32);
    }
    Ok(Tensor::from_vec(vals, (half,), device)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_dim_zero_is_rejected() {
        assert!(synthesize_rope_inv_freq(0, None, &Device::Cpu).is_err());
    }

    #[test]
    fn table_length_is_half_head_dim() {
        let t = synthesize_rope_inv_freq(128, Some(1_000_000.0), &Device::Cpu).unwrap();
        assert_eq!(t.dims(), &[64]);
    }

    #[test]
    fn default_theta_matches_explicit_10000() {
        let a = synthesize_rope_inv_freq(64, None, &Device::Cpu).unwrap();
        let b = synthesize_rope_inv_freq(64, Some(10_000.0), &Device::Cpu).unwrap();
        assert_eq!(
            a.to_vec1::<f32>().unwrap(),
            b.to_vec1::<f32>().unwrap(),
            "None must default to theta=10000, matching the explicit value"
        );
    }

    #[test]
    fn llama3_scaling_none_matches_base_and_some_scales_low_freqs() {
        let base = synthesize_rope_inv_freq(128, Some(500_000.0), &Device::Cpu).unwrap();
        let unscaled =
            synthesize_rope_inv_freq_with_llama3_scaling(128, Some(500_000.0), None, &Device::Cpu)
                .unwrap();
        assert_eq!(
            base.to_vec1::<f32>().unwrap(),
            unscaled.to_vec1::<f32>().unwrap()
        );

        let sc = vox_hf_layout::Llama3RopeScaling {
            factor: 8.0,
            low_freq_factor: 1.0,
            high_freq_factor: 4.0,
            original_max_position_embeddings: 8192,
        };
        let scaled = synthesize_rope_inv_freq_with_llama3_scaling(
            128,
            Some(500_000.0),
            Some(sc),
            &Device::Cpu,
        )
        .unwrap()
        .to_vec1::<f32>()
        .unwrap();
        let base_v = base.to_vec1::<f32>().unwrap();
        assert_eq!(scaled[0], base_v[0]);
        let ratio = base_v[63] / scaled[63];
        assert!(
            (ratio - 8.0).abs() < 1e-4,
            "expected 8x low-freq scale, got {ratio}"
        );
    }
}
