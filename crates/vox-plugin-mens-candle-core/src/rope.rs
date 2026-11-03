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
}
