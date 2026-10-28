//! Query-chunked causal attention for training on Metal.
//!
//! Eager attention keeps the whole `[b, h, s, s]` score/softmax chain on the
//! autograd tape and allocates several more of them in its backward; on Metal that
//! was ~295 B per head per token pair, the dominant term of a checkpointed QLoRA
//! step (320-S2-DEBUG.md). Here the forward runs block by block over query rows
//! without a tape, and the backward recomputes each block's probabilities from q
//! and k (the flash-attention recurrence, written in candle ops), so no tensor
//! larger than `[b, h, qb, s]` exists and the tape keeps only q, k, v and the output.

use candle_core::{CpuStorage, CustomOp3, Layout, MetalStorage, Result, Shape, Tensor};

/// Query rows per block in training attention. Peak attention memory per layer
/// is a few `[b, h, QUERY_BLOCK, s]` F32 tensors in the backward.
pub(crate) const QUERY_BLOCK: usize = 512;

/// Causal `softmax(q kᵀ · scale) v` for `[b, h, s, hd]` q, k, v (heads already
/// repeated for GQA), computed in blocks of `qb` query rows. Row block
/// `r0..r0+len` only reads keys `0..r0+len`.
pub(crate) fn chunked_causal_attention(
    q: &Tensor,
    k: &Tensor,
    v: &Tensor,
    scale: f64,
    qb: usize,
) -> Result<Tensor> {
    let (q, k, v) = (q.contiguous()?, k.contiguous()?, v.contiguous()?);
    let out = ChunkedAttention::forward(&q.detach(), &k.detach(), &v.detach(), scale, qb)?;
    q.apply_op3(&k, &v, ChunkedAttention { scale, qb, out })
}

/// The custom op only attaches the backward: its forward hands back `out`,
/// computed tape-free before the op is applied.
struct ChunkedAttention {
    scale: f64,
    qb: usize,
    out: Tensor,
}

/// Additive causal mask for query rows `r0..r0+len` over keys `0..r0+len`.
fn block_mask(r0: usize, len: usize, like: &Tensor) -> Result<Tensor> {
    let kv = r0 + len;
    let data: Vec<f32> = (0..len * kv)
        .map(|i| {
            if i % kv > r0 + i / kv {
                f32::NEG_INFINITY
            } else {
                0.0
            }
        })
        .collect();
    Tensor::from_vec(data, (len, kv), like.device())?.to_dtype(like.dtype())
}

/// One block's (q rows, keys, values, probabilities).
fn block(
    q: &Tensor,
    k: &Tensor,
    v: &Tensor,
    r0: usize,
    len: usize,
    scale: f64,
) -> Result<(Tensor, Tensor, Tensor, Tensor)> {
    let kv = r0 + len;
    let qb = q.narrow(2, r0, len)?.contiguous()?;
    let kb = k.narrow(2, 0, kv)?.contiguous()?;
    let vb = v.narrow(2, 0, kv)?.contiguous()?;
    let scores = (qb.matmul(&kb.t()?)? * scale)?.broadcast_add(&block_mask(r0, len, q)?)?;
    let p = candle_nn::ops::softmax_last_dim(&scores)?;
    Ok((qb, kb, vb, p))
}

fn blocks(s: usize, qb: usize) -> impl Iterator<Item = (usize, usize)> {
    let qb = qb.max(1);
    (0..s).step_by(qb).map(move |r0| (r0, qb.min(s - r0)))
}

impl ChunkedAttention {
    fn forward(q: &Tensor, k: &Tensor, v: &Tensor, scale: f64, qb: usize) -> Result<Tensor> {
        let mut outs = Vec::new();
        for (r0, len) in blocks(q.dim(2)?, qb) {
            let (_, _, vb, p) = block(q, k, v, r0, len, scale)?;
            outs.push(p.matmul(&vb)?);
        }
        Tensor::cat(&outs, 2)
    }

    fn checked_out(&self, shape: &Shape) -> Result<()> {
        let l = self.out.layout();
        if !l.is_contiguous() || l.start_offset() != 0 || self.out.shape() != shape {
            candle_core::bail!("chunked attention: precomputed output has an unexpected layout");
        }
        Ok(())
    }
}

impl CustomOp3 for ChunkedAttention {
    fn name(&self) -> &'static str {
        "chunked-causal-attention"
    }

    fn cpu_fwd(
        &self,
        _: &CpuStorage,
        l1: &Layout,
        _: &CpuStorage,
        _: &Layout,
        _: &CpuStorage,
        _: &Layout,
    ) -> Result<(CpuStorage, Shape)> {
        self.checked_out(l1.shape())?;
        match &*self.out.storage_and_layout().0 {
            candle_core::Storage::Cpu(s) => Ok((s.clone(), l1.shape().clone())),
            _ => candle_core::bail!("chunked attention: output is not on the cpu"),
        }
    }

    fn metal_fwd(
        &self,
        _: &MetalStorage,
        l1: &Layout,
        _: &MetalStorage,
        _: &Layout,
        _: &MetalStorage,
        _: &Layout,
    ) -> Result<(MetalStorage, Shape)> {
        self.checked_out(l1.shape())?;
        match &*self.out.storage_and_layout().0 {
            // Shares the buffer; no copy.
            candle_core::Storage::Metal(s) => Ok((s.clone(), l1.shape().clone())),
            _ => candle_core::bail!("chunked attention: output is not on metal"),
        }
    }

    fn bwd(
        &self,
        q: &Tensor,
        k: &Tensor,
        v: &Tensor,
        out: &Tensor,
        grad: &Tensor,
    ) -> Result<(Option<Tensor>, Option<Tensor>, Option<Tensor>)> {
        let (q, k, v, out, grad) = (
            q.detach(),
            k.detach(),
            v.detach(),
            out.detach(),
            grad.detach(),
        );
        let s = q.dim(2)?;
        let mut dq = Vec::new();
        let mut dk = k.zeros_like()?;
        let mut dv = v.zeros_like()?;
        for (r0, len) in blocks(s, self.qb) {
            let kv = r0 + len;
            let (qb, kb, vb, p) = block(&q, &k, &v, r0, len, self.scale)?;
            let d_out = grad.narrow(2, r0, len)?.contiguous()?;
            let o = out.narrow(2, r0, len)?;
            // rowsum(dP ∘ P) == rowsum(dO ∘ O), the softmax backward's correction term.
            let delta = (&d_out * &o)?.sum_keepdim(3)?;
            dv = (dv + p.t()?.matmul(&d_out)?.pad_with_zeros(2, 0, s - kv)?)?;
            let d_scores = ((d_out.matmul(&vb.t()?)?.broadcast_sub(&delta)? * p)? * self.scale)?;
            dq.push(d_scores.matmul(&kb)?);
            dk = (dk + d_scores.t()?.matmul(&qb)?.pad_with_zeros(2, 0, s - kv)?)?;
        }
        Ok((Some(Tensor::cat(&dq, 2)?), Some(dk), Some(dv)))
    }
}

#[cfg(test)]
mod tests {
    //! The chunked op must be numerically the eager attention it replaces, in the
    //! forward output and in the gradients it hands back to q, k and v. The eager
    //! reference builds the full `[h, s, s]` masked softmax with candle autograd.

    use super::*;
    use candle_core::{DType, Device, Tensor, Var};

    fn eager(q: &Tensor, k: &Tensor, v: &Tensor, scale: f64) -> Tensor {
        let s = q.dim(2).unwrap();
        let mask: Vec<f32> = (0..s * s)
            .map(|i| {
                if i % s > i / s {
                    f32::NEG_INFINITY
                } else {
                    0.0
                }
            })
            .collect();
        let mask = Tensor::from_vec(mask, (s, s), q.device()).unwrap();
        let att = (q.matmul(&k.t().unwrap()).unwrap() * scale)
            .unwrap()
            .broadcast_add(&mask)
            .unwrap();
        candle_nn::ops::softmax(&att, candle_core::D::Minus1)
            .unwrap()
            .matmul(v)
            .unwrap()
    }

    fn flat(t: &Tensor) -> Vec<f32> {
        t.to_device(&Device::Cpu)
            .unwrap()
            .to_dtype(DType::F32)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap()
    }

    fn max_diff(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y).abs())
            .fold(0f32, f32::max)
    }

    /// Output and d(sum(out * w))/d{q,k,v}, computed on `dev`.
    fn run(dev: &Device, x: &[Tensor; 4], chunk: Option<usize>) -> (Vec<f32>, [Vec<f32>; 3]) {
        let vars: Vec<Var> = x[..3]
            .iter()
            .map(|t| Var::from_tensor(&t.to_device(dev).unwrap()).unwrap())
            .collect();
        let (q, k, v) = (
            vars[0].as_tensor(),
            vars[1].as_tensor(),
            vars[2].as_tensor(),
        );
        let scale = 1.0 / (q.dim(3).unwrap() as f64).sqrt();
        let out = match chunk {
            Some(qb) => chunked_causal_attention(q, k, v, scale, qb).unwrap(),
            None => eager(q, k, v, scale),
        };
        let w = x[3].to_device(dev).unwrap();
        let grads = (&out * &w).unwrap().sum_all().unwrap().backward().unwrap();
        let g = |t: &Tensor| flat(grads.get(t).expect("grad must reach q, k and v"));
        (flat(&out), [g(q), g(k), g(v)])
    }

    fn inputs(b: usize, h: usize, s: usize, hd: usize) -> [Tensor; 4] {
        let r = || Tensor::randn(0f32, 1.0, (b, h, s, hd), &Device::Cpu).unwrap();
        [r(), r(), r(), r()]
    }

    fn assert_close(
        want: &(Vec<f32>, [Vec<f32>; 3]),
        got: &(Vec<f32>, [Vec<f32>; 3]),
        tol: f32,
        what: &str,
    ) {
        let d = max_diff(&want.0, &got.0);
        assert!(d < tol, "{what}: output differs (max |diff| = {d})");
        for (name, (w, g)) in ["dq", "dk", "dv"].iter().zip(want.1.iter().zip(&got.1)) {
            let d = max_diff(w, g);
            assert!(d < tol, "{what}: {name} differs (max |diff| = {d})");
        }
    }

    #[test]
    fn chunked_attention_matches_eager_output_and_grads() {
        // 7 rows over blocks of 3 leaves a 1-row tail; 1 is the degenerate block;
        // 7 and 16 are one block (exact and oversized).
        for qb in [3, 1, 7, 16] {
            let x = inputs(2, 2, 7, 4);
            let want = run(&Device::Cpu, &x, None);
            let got = run(&Device::Cpu, &x, Some(qb));
            assert_close(&want, &got, 1e-5, &format!("cpu qb={qb}"));
        }
    }

    #[test]
    fn chunked_attention_on_metal_matches_eager_on_cpu() {
        let Ok(metal) = Device::new_metal(0) else {
            return; // no Metal device (e.g. a macOS CI VM): nothing to compare
        };
        let x = inputs(1, 4, 37, 16);
        let want = run(&Device::Cpu, &x, None);
        let got = run(&metal, &x, Some(16));
        assert_close(&want, &got, 1e-4, "metal qb=16");
    }
}
