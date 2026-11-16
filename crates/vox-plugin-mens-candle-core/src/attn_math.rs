//! Shared attention and Gated DeltaNet mathematical primitives.
//!
//! Keeps Qwen3.5 / Qwen3.8 parity math in one place across the Metal and CUDA
//! backends so neither backend can reintroduce RMS-vs-L2 normalization drift,
//! flat-half `q_proj` gate splits, or overflowing `exp()` softplus calls.

use candle_core::{D, DType, Tensor};

/// True vector L2 normalization over the final axis (`x / sqrt(sum(x^2) + eps)`).
///
/// Unlike RMSNorm (`x / sqrt(mean(x^2) + eps)`), there is NO division by the
/// head dimension `d` — Gated DeltaNet's Householder transition `(I - beta * k * k^T)`
/// assumes unit-norm `k` and becomes unstable if `||k||_2^2 = d`.
pub fn l2norm_last_no_div(x: &Tensor, eps: f64) -> candle_core::Result<Tensor> {
    let dt = x.dtype();
    let xf = if dt != DType::F32 {
        x.to_dtype(DType::F32)?
    } else {
        x.clone()
    };
    let sq = xf.sqr()?.sum_keepdim(D::Minus1)?;
    let inv = sq.affine(1.0, eps)?.sqrt()?.recip()?;
    let normed = xf.broadcast_mul(&inv)?;
    if dt != DType::F32 {
        normed.to_dtype(dt)
    } else {
        Ok(normed)
    }
}

/// Numerically stable softplus: `relu(x) + log(1 + exp(-|x|))` computed in F32.
///
/// Avoids `exp(x) -> inf` overflow for large positive inputs (`x > 88.7`).
pub fn stable_softplus(x: &Tensor) -> candle_core::Result<Tensor> {
    let dt = x.dtype();
    let xf = if dt != DType::F32 {
        x.to_dtype(DType::F32)?
    } else {
        x.clone()
    };
    let out = xf
        .relu()?
        .add(&xf.abs()?.neg()?.exp()?.affine(1.0, 1.0)?.log()?)?;
    if dt != DType::F32 {
        out.to_dtype(dt)
    } else {
        Ok(out)
    }
}

/// Split full-attention `q_proj` output into `(query, optional_output_gate)`.
///
/// Qwen3.5 / Qwen3.8 packs per-head `[query_h, gate_h]` contiguously inside
/// each head's `2 * head_dim` slice, so the split MUST reshape to
/// `(b, seq_len, n_heads, 2 * head_dim)` and narrow along axis 3 before
/// flattening back to `(b, seq_len, n_heads * head_dim)`.
pub fn split_q_and_gate(
    q: Tensor,
    b: usize,
    seq_len: usize,
    n_heads: usize,
    head_dim: usize,
) -> candle_core::Result<(Tensor, Option<Tensor>)> {
    let q_dim = n_heads * head_dim;
    let last = q.dim(D::Minus1)?;
    if last == q_dim {
        return Ok((q, None));
    }
    if last == 2 * q_dim {
        let reshaped = q.reshape((b, seq_len, n_heads, 2 * head_dim))?;
        let actual_q = reshaped
            .narrow(3, 0, head_dim)?
            .contiguous()?
            .reshape((b, seq_len, q_dim))?;
        let gate = reshaped
            .narrow(3, head_dim, head_dim)?
            .contiguous()?
            .reshape((b, seq_len, q_dim))?;
        return Ok((actual_q, Some(gate)));
    }
    candle_core::bail!(
        "split_q_and_gate: expected last dim {q_dim} or {}, got {last}",
        2 * q_dim
    )
}

/// Apply upstream `y * sigmoid(gate)` when an output gate tensor is present.
pub fn apply_attn_output_gate(y: Tensor, gate: Option<&Tensor>) -> candle_core::Result<Tensor> {
    let Some(g) = gate else {
        return Ok(y);
    };
    let dt = y.dtype();
    let gf = if g.dtype() != DType::F32 {
        g.to_dtype(DType::F32)?
    } else {
        g.clone()
    };
    let sig = candle_nn::ops::sigmoid(&gf)?;
    let sig = if dt != DType::F32 {
        sig.to_dtype(dt)?
    } else {
        sig
    };
    y.broadcast_mul(&sig)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    #[test]
    fn l2norm_last_no_div_produces_unit_l2_norm_for_d128() {
        let dev = Device::Cpu;
        let data: Vec<f32> = (0..256).map(|i| ((i as f32) * 0.031) - 1.5).collect();
        let x = Tensor::from_vec(data, (2, 128), &dev).unwrap();
        let n = l2norm_last_no_div(&x, 1e-6).unwrap();
        let norms = n
            .sqr()
            .unwrap()
            .sum(D::Minus1)
            .unwrap()
            .sqrt()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        for l2 in norms {
            assert!(
                (l2 - 1.0).abs() < 1e-4,
                "expected unit L2 norm 1.0 (not sqrt(128)), got {l2}"
            );
        }
    }

    #[test]
    fn stable_softplus_is_finite_and_accurate() {
        let dev = Device::Cpu;
        let x = Tensor::new(&[120.0f32, 0.0, -20.0], &dev).unwrap();
        let out = stable_softplus(&x).unwrap().to_vec1::<f32>().unwrap();
        assert!(out.iter().all(|v| v.is_finite()));
        assert!((out[0] - 120.0).abs() < 1e-5);
        assert!((out[1] - 2.0f32.ln()).abs() < 1e-5);
        assert!((out[2] - (-20.0f32).exp()).abs() < 1e-6);
    }

    #[test]
    fn split_q_and_gate_unpacks_per_head_and_gates_with_sigmoid() {
        let dev = Device::Cpu;
        // b=1, seq=1, n_heads=2, head_dim=2 => 2 * head_dim = 4 per head => total 8.
        // Head 0: q=[1, 2], g=[0, 2]; Head 1: q=[3, 4], g=[-2, 1]
        let raw = Tensor::new(&[1.0f32, 2.0, 0.0, 2.0, 3.0, 4.0, -2.0, 1.0], &dev)
            .unwrap()
            .reshape((1, 1, 8))
            .unwrap();
        let (q, gate) = split_q_and_gate(raw, 1, 1, 2, 2).unwrap();
        assert_eq!(
            q.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            vec![1.0, 2.0, 3.0, 4.0]
        );
        let gate_t = gate.expect("gate must be Some for 2*q_dim");
        assert_eq!(
            gate_t.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            vec![0.0, 2.0, -2.0, 1.0]
        );

        let y = Tensor::new(&[10.0f32, 20.0, 30.0, 40.0], &dev)
            .unwrap()
            .reshape((1, 1, 4))
            .unwrap();
        let gated = apply_attn_output_gate(y, Some(&gate_t))
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let sigmoid = |x: f32| 1.0f32 / (1.0f32 + (-x).exp());
        let expected = [
            10.0 * sigmoid(0.0),
            20.0 * sigmoid(2.0),
            30.0 * sigmoid(-2.0),
            40.0 * sigmoid(1.0),
        ];
        for (g, e) in gated.iter().zip(expected) {
            assert!((g - e).abs() < 1e-5, "got {g}, expected {e}");
        }
    }
}
