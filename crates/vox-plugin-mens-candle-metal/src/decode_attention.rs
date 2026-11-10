//! Preallocated per-sequence KV slot and copy-free GQA decode attention (THRU-02).
//!
//! Full-attention decode steps previously ran `Tensor::cat` on K and V every
//! token, materialized `n_rep` copies of K and V via `repeat_kv`, and made a
//! `.contiguous()` copy of `K^T`. [`KvSlot`] preallocates `[1, n_kv, cap, hd]`
//! once per sequence and writes new tokens in place via `Tensor::slice_set`;
//! [`decode_attention`] folds `n_rep` into the matmul `M` dimension (`[1, n_kv,
//! n_rep, hd]`) and multiplies against the transposed `K` view directly.

use candle_core::{Result, Tensor};

#[derive(Clone)]
struct KvBuffer {
    k: Tensor,
    v: Tensor,
    len: usize,
}

/// Preallocated `(K, V)` cache for one full-attention layer of one sequence.
#[derive(Clone)]
pub struct KvSlot {
    cap: usize,
    buf: Option<KvBuffer>,
}

impl KvSlot {
    pub fn new(cap: usize) -> Self {
        Self { cap, buf: None }
    }

    pub fn is_filled(&self) -> bool {
        self.buf.as_ref().is_some_and(|b| b.len > 0)
    }

    pub fn len(&self) -> usize {
        self.buf.as_ref().map_or(0, |b| b.len)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn bytes(&self) -> usize {
        self.buf.as_ref().map_or(0, |b| {
            (b.k.elem_count() + b.v.elem_count()) * b.k.dtype().size_in_bytes()
        })
    }

    /// Append `k` and `v` (`[1, n_kv, seq_len, hd]`) in place and return
    /// `[1, n_kv, len, hd]` views over the populated prefix.
    pub fn append(&mut self, k: &Tensor, v: &Tensor) -> Result<(Tensor, Tensor)> {
        let (b, n_kv, seq_len, hd) = k.dims4()?;
        if b != 1 {
            return Err(candle_core::Error::Msg(format!(
                "KvSlot::append expects batch=1, got {b}"
            )));
        }
        let k_src = k.contiguous()?;
        let v_src = v.contiguous()?;
        if self.buf.is_none() {
            let k_buf = Tensor::zeros((1, n_kv, self.cap, hd), k.dtype(), k.device())?;
            let v_buf = Tensor::zeros((1, n_kv, self.cap, hd), v.dtype(), v.device())?;
            self.buf = Some(KvBuffer {
                k: k_buf,
                v: v_buf,
                len: 0,
            });
        }
        let buf = self.buf.as_mut().expect("allocated above");
        if buf.len + seq_len > self.cap {
            return Err(candle_core::Error::Msg(format!(
                "KV cache full: len={} + seq_len={seq_len} > cap={}",
                buf.len, self.cap
            )));
        }
        if seq_len > 0 {
            buf.k.slice_set(&k_src, 2, buf.len)?;
            buf.v.slice_set(&v_src, 2, buf.len)?;
            buf.len += seq_len;
        }
        Ok((buf.k.narrow(2, 0, buf.len)?, buf.v.narrow(2, 0, buf.len)?))
    }
}

/// One-token GQA decode attention over `q` (`[1, n_heads, 1, hd]`) and cached
/// views `k`, `v` (`[1, n_kv, cache_len, hd]`), without `repeat_kv` or a
/// contiguous copy of `K^T`.
pub fn decode_attention(
    q: &Tensor,
    k: &Tensor,
    v: &Tensor,
    n_rep: usize,
    scale: f64,
) -> Result<Tensor> {
    let (b, n_heads, seq_len, hd) = q.dims4()?;
    let (_, n_kv, _, _) = k.dims4()?;
    if b != 1 || seq_len != 1 || n_kv * n_rep != n_heads {
        return Err(candle_core::Error::Msg(format!(
            "decode_attention shape mismatch: q={:?} k={:?} n_rep={n_rep}",
            q.dims(),
            k.dims()
        )));
    }
    let q_gqa = q.contiguous()?.reshape((1, n_kv, n_rep, hd))?;
    let att = (q_gqa.matmul(&k.transpose(2, 3)?)? * scale)?;
    let att = candle_nn::ops::softmax(&att, candle_core::D::Minus1)?;
    att.matmul(v)?.reshape((1, n_heads, 1, hd))
}

#[cfg(test)]
pub(crate) fn attend_reference(
    q: &Tensor,
    k_new: &Tensor,
    v_new: &Tensor,
    slot: &mut Option<(Tensor, Tensor)>,
    n_rep: usize,
    scale: f64,
) -> Result<Tensor> {
    let (k, v) = match slot.as_ref() {
        Some((k_prev, v_prev)) => (
            Tensor::cat(&[k_prev, k_new], 2)?,
            Tensor::cat(&[v_prev, v_new], 2)?,
        ),
        None => (k_new.clone(), v_new.clone()),
    };
    *slot = Some((k.clone(), v.clone()));
    let repeat_kv = |x: &Tensor| -> Result<Tensor> {
        if n_rep == 1 {
            return Ok(x.clone());
        }
        let (b, n_kv, seq, hd) = x.dims4()?;
        x.unsqueeze(2)?
            .expand((b, n_kv, n_rep, seq, hd))?
            .reshape((b, n_kv * n_rep, seq, hd))
    };
    let k = repeat_kv(&k)?;
    let v = repeat_kv(&v)?;
    let att = (q.contiguous()?.matmul(&k.transpose(2, 3)?.contiguous()?)? * scale)?;
    candle_nn::ops::softmax(&att, candle_core::D::Minus1)?.matmul(&v.contiguous()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device};

    fn seeded(shape: (usize, usize, usize, usize), salt: f32, dev: &Device) -> Tensor {
        let n = shape.0 * shape.1 * shape.2 * shape.3;
        let v: Vec<f32> = (0..n)
            .map(|i| {
                ((i as f32 * 12.9898 + salt * 78.233).sin() * 43_758.547).rem_euclid(1.0) - 0.5
            })
            .collect();
        Tensor::from_vec(v, shape, dev).unwrap()
    }

    fn bits(t: &Tensor) -> Vec<u32> {
        t.flatten_all()
            .unwrap()
            .to_device(&Device::Cpu)
            .unwrap()
            .to_vec1::<f32>()
            .unwrap()
            .into_iter()
            .map(|v| {
                if v == 0.0 {
                    0.0f32.to_bits()
                } else {
                    v.to_bits()
                }
            })
            .collect()
    }

    fn max_abs_diff(a: &Tensor, b: &Tensor) -> f32 {
        a.flatten_all()
            .unwrap()
            .to_device(&Device::Cpu)
            .unwrap()
            .to_vec1::<f32>()
            .unwrap()
            .into_iter()
            .zip(
                b.flatten_all()
                    .unwrap()
                    .to_device(&Device::Cpu)
                    .unwrap()
                    .to_vec1::<f32>()
                    .unwrap(),
            )
            .map(|(x, y)| (x - y).abs())
            .fold(0.0f32, f32::max)
    }

    fn check_attention_fast_path_on(
        dev: &Device,
        prefill_len: usize,
        decode_steps: usize,
        bitwise_all_n_rep: bool,
    ) {
        let hd = 8usize;
        let scale = 1.0 / (hd as f64).sqrt();
        let cap = prefill_len + decode_steps + 4;
        for (n_heads, n_kv) in [(2, 2), (4, 2), (8, 2), (10, 2)] {
            let n_rep = n_heads / n_kv;
            let mut slot = KvSlot::new(cap);

            let k0 = seeded((1, n_kv, prefill_len, hd), 1.0 + n_heads as f32, dev);
            let v0 = seeded((1, n_kv, prefill_len, hd), 2.0 + n_heads as f32, dev);
            slot.append(&k0, &v0).unwrap();
            let mut ref_slot: Option<(Tensor, Tensor)> = Some((k0.clone(), v0.clone()));
            assert_eq!(
                slot.bytes(),
                2 * n_kv * cap * hd * DType::F32.size_in_bytes()
            );

            for step in 0..decode_steps {
                let s = (step + 10) as f32 + n_heads as f32 * 0.1;
                let q = seeded((1, n_heads, 1, hd), s, dev);
                let k = seeded((1, n_kv, 1, hd), s + 100.0, dev);
                let v = seeded((1, n_kv, 1, hd), s + 200.0, dev);

                let (k_view, v_view) = slot.append(&k, &v).unwrap();
                let got = decode_attention(&q, &k_view, &v_view, n_rep, scale).unwrap();
                let want = attend_reference(&q, &k, &v, &mut ref_slot, n_rep, scale).unwrap();
                if bitwise_all_n_rep || n_rep == 1 {
                    assert_eq!(
                        bits(&got),
                        bits(&want),
                        "mismatch at n_heads={n_heads} n_kv={n_kv} step={step}"
                    );
                } else {
                    // On CPU, gemm may pick a different microkernel at M = n_rep than at M = 1,
                    // producing last-bit differences; Metal (`metal_attention_identity`) is bitwise
                    // for every n_rep.
                    let diff = max_abs_diff(&got, &want);
                    assert!(
                        diff <= 1e-6,
                        "max abs diff {diff} > 1e-6 at n_heads={n_heads} n_kv={n_kv} step={step}"
                    );
                }
            }
        }
    }

    #[test]
    fn attention_fast_path() {
        check_attention_fast_path_on(&Device::Cpu, 5, 7, false);
    }

    #[test]
    fn a_full_slot_refuses_an_append() {
        let dev = Device::Cpu;
        let mut slot = KvSlot::new(4);
        let k3 = Tensor::zeros((1, 2, 3, 8), DType::F32, &dev).unwrap();
        let v3 = Tensor::zeros((1, 2, 3, 8), DType::F32, &dev).unwrap();
        slot.append(&k3, &v3).unwrap();
        assert_eq!(slot.len(), 3);

        let k1 = Tensor::zeros((1, 2, 1, 8), DType::F32, &dev).unwrap();
        let v1 = Tensor::zeros((1, 2, 1, 8), DType::F32, &dev).unwrap();
        slot.append(&k1, &v1).unwrap();
        assert_eq!(slot.len(), 4);

        let err = slot
            .append(&k1, &v1)
            .expect_err("append past cap must fail")
            .to_string();
        assert!(err.contains("KV cache full"), "unexpected error: {err}");
    }

    #[cfg(feature = "metal")]
    #[test]
    #[ignore = "owner:mens sunset:never needs Metal hardware"]
    fn metal_attention_identity() {
        let dev = Device::new_metal(0).expect("metal device");
        check_attention_fast_path_on(&dev, 5, 7, true);
        check_attention_fast_path_on(&dev, 2048, 3, true);
    }
}
