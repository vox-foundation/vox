use crate::op::{BackpropOp, Op};
use crate::tensor::from_storage;
use crate::{CpuStorage, CudaStorage, Layout, MetalStorage, Result, Shape, Tensor};
use std::sync::Arc;

/// Unary ops that can be defined in user-land.
pub trait CustomOp1 {
    // Box<dyn> does not support const yet, so use a function to get the name.
    fn name(&self) -> &'static str;

    /// The forward pass, as run on a cpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cpu_fwd(&self, storage: &CpuStorage, layout: &Layout) -> Result<(CpuStorage, Shape)>;

    /// The forward pass, as run on a gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cuda_fwd(&self, _storage: &CudaStorage, _layout: &Layout) -> Result<(CudaStorage, Shape)> {
        Err(crate::Error::Cuda(
            format!("no cuda implementation for {}", self.name()).into(),
        ))
    }

    /// The forward pass, as run on a metal gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn metal_fwd(
        &self,
        _storage: &MetalStorage,
        _layout: &Layout,
    ) -> Result<(MetalStorage, Shape)> {
        Err(crate::Error::Metal(
            format!("no metal implementation for {}", self.name()).into(),
        ))
    }

    /// This function takes as argument the argument `arg` used in the forward pass, the result
    /// produced by the forward operation `res` and the gradient of the result `grad_res`.
    /// The function should return the gradient of the argument.
    fn bwd(&self, _arg: &Tensor, _res: &Tensor, _grad_res: &Tensor) -> Result<Option<Tensor>> {
        Err(crate::Error::BackwardNotSupported { op: self.name() })
    }
}

pub trait CustomOp2 {
    fn name(&self) -> &'static str;

    /// The forward pass, as run on a cpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cpu_fwd(
        &self,
        s1: &CpuStorage,
        l1: &Layout,
        s2: &CpuStorage,
        l2: &Layout,
    ) -> Result<(CpuStorage, Shape)>;

    /// The forward pass, as run on a gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cuda_fwd(
        &self,
        _: &CudaStorage,
        _: &Layout,
        _: &CudaStorage,
        _: &Layout,
    ) -> Result<(CudaStorage, Shape)> {
        Err(crate::Error::Cuda(
            format!("no cuda implementation for {}", self.name()).into(),
        ))
    }

    /// The forward pass, as run on a metal gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn metal_fwd(
        &self,
        _: &MetalStorage,
        _: &Layout,
        _: &MetalStorage,
        _: &Layout,
    ) -> Result<(MetalStorage, Shape)> {
        Err(crate::Error::Metal(
            format!("no metal implementation for {}", self.name()).into(),
        ))
    }

    fn bwd(
        &self,
        _arg1: &Tensor,
        _arg2: &Tensor,
        _res: &Tensor,
        _grad_res: &Tensor,
    ) -> Result<(Option<Tensor>, Option<Tensor>)> {
        Err(crate::Error::BackwardNotSupported { op: self.name() })
    }
}

pub trait CustomOp3 {
    fn name(&self) -> &'static str;

    /// The forward pass, as run on a cpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cpu_fwd(
        &self,
        s1: &CpuStorage,
        l1: &Layout,
        s2: &CpuStorage,
        l2: &Layout,
        s3: &CpuStorage,
        l3: &Layout,
    ) -> Result<(CpuStorage, Shape)>;

    /// The forward pass, as run on a gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cuda_fwd(
        &self,
        _: &CudaStorage,
        _: &Layout,
        _: &CudaStorage,
        _: &Layout,
        _: &CudaStorage,
        _: &Layout,
    ) -> Result<(CudaStorage, Shape)> {
        Err(crate::Error::Cuda(
            format!("no cuda implementation for {}", self.name()).into(),
        ))
    }

    /// The forward pass, as run on a metal gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn metal_fwd(
        &self,
        _: &MetalStorage,
        _: &Layout,
        _: &MetalStorage,
        _: &Layout,
        _: &MetalStorage,
        _: &Layout,
    ) -> Result<(MetalStorage, Shape)> {
        Err(crate::Error::Metal(
            format!("no metal implementation for {}", self.name()).into(),
        ))
    }

    fn bwd(
        &self,
        _arg1: &Tensor,
        _arg2: &Tensor,
        _arg3: &Tensor,
        _res: &Tensor,
        _grad_res: &Tensor,
    ) -> Result<(Option<Tensor>, Option<Tensor>, Option<Tensor>)> {
        Err(crate::Error::BackwardNotSupported { op: self.name() })
    }
}

impl Tensor {
    /// Applies a unary custom op without backward support
    pub fn apply_op1_no_bwd<C: CustomOp1>(&self, c: &C) -> Result<Self> {
        let (storage, shape) = self.storage().apply_op1(self.layout(), c)?;
        Ok(from_storage(storage, shape, BackpropOp::none(), false))
    }

    /// Applies a binary custom op without backward support
    pub fn apply_op2_no_bwd<C: CustomOp2>(&self, rhs: &Self, c: &C) -> Result<Self> {
        let (storage, shape) =
            self.storage()
                .apply_op2(self.layout(), &rhs.storage(), rhs.layout(), c)?;
        Ok(from_storage(storage, shape, BackpropOp::none(), false))
    }

    /// Applies a ternary custom op without backward support
    pub fn apply_op3_no_bwd<C: CustomOp3>(&self, t2: &Self, t3: &Self, c: &C) -> Result<Self> {
        let (storage, shape) = self.storage().apply_op3(
            self.layout(),
            &t2.storage(),
            t2.layout(),
            &t3.storage(),
            t3.layout(),
            c,
        )?;
        Ok(from_storage(storage, shape, BackpropOp::none(), false))
    }

    /// Applies a unary custom op.
    pub fn apply_op1_arc(&self, c: Arc<Box<dyn CustomOp1 + Send + Sync>>) -> Result<Self> {
        let (storage, shape) = self
            .storage()
            .apply_op1(self.layout(), c.as_ref().as_ref())?;
        let op = BackpropOp::new1(self, |s| Op::CustomOp1(s, c.clone()));
        Ok(from_storage(storage, shape, op, false))
    }

    pub fn apply_op1<C: 'static + CustomOp1 + Send + Sync>(&self, c: C) -> Result<Self> {
        self.apply_op1_arc(Arc::new(Box::new(c)))
    }

    /// Applies a binary custom op.
    pub fn apply_op2_arc(
        &self,
        rhs: &Self,
        c: Arc<Box<dyn CustomOp2 + Send + Sync>>,
    ) -> Result<Self> {
        let (storage, shape) = self.storage().apply_op2(
            self.layout(),
            &rhs.storage(),
            rhs.layout(),
            c.as_ref().as_ref(),
        )?;
        let op = BackpropOp::new2(self, rhs, |t1, t2| Op::CustomOp2(t1, t2, c.clone()));
        Ok(from_storage(storage, shape, op, false))
    }

    pub fn apply_op2<C: 'static + CustomOp2 + Send + Sync>(&self, r: &Self, c: C) -> Result<Self> {
        self.apply_op2_arc(r, Arc::new(Box::new(c)))
    }

    /// Applies a ternary custom op.
    pub fn apply_op3_arc(
        &self,
        t2: &Self,
        t3: &Self,
        c: Arc<Box<dyn CustomOp3 + Send + Sync>>,
    ) -> Result<Self> {
        let (storage, shape) = self.storage().apply_op3(
            self.layout(),
            &t2.storage(),
            t2.layout(),
            &t3.storage(),
            t3.layout(),
            c.as_ref().as_ref(),
        )?;
        let op = BackpropOp::new3(self, t2, t3, |t1, t2, t3| {
            Op::CustomOp3(t1, t2, t3, c.clone())
        });
        Ok(from_storage(storage, shape, op, false))
    }

    pub fn apply_op3<C: 'static + CustomOp3 + Send + Sync>(
        &self,
        t2: &Self,
        t3: &Self,
        c: C,
    ) -> Result<Self> {
        self.apply_op3_arc(t2, t3, Arc::new(Box::new(c)))
    }
}

// In place ops.

/// Unary ops that can be defined in user-land.
/// These ops work in place and as such back-prop is unsupported.
pub trait InplaceOp1 {
    // Box<dyn> does not support const yet, so use a function to get the name.
    fn name(&self) -> &'static str;

    /// The forward pass, as run on a cpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cpu_fwd(&self, storage: &mut CpuStorage, layout: &Layout) -> Result<()>;

    /// The forward pass, as run on a gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cuda_fwd(&self, _storage: &mut CudaStorage, _layout: &Layout) -> Result<()> {
        Err(crate::Error::Cuda(
            format!("no cuda implementation for {}", self.name()).into(),
        ))
    }

    /// The forward pass, as run on a metal gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn metal_fwd(&self, _storage: &mut MetalStorage, _layout: &Layout) -> Result<()> {
        Err(crate::Error::Metal(
            format!("no metal implementation for {}", self.name()).into(),
        ))
    }
}

pub trait InplaceOp2 {
    fn name(&self) -> &'static str;

    /// The forward pass, as run on a cpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cpu_fwd(&self, s1: &mut CpuStorage, l1: &Layout, s2: &CpuStorage, l2: &Layout)
    -> Result<()>;

    /// The forward pass, as run on a gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cuda_fwd(&self, _: &mut CudaStorage, _: &Layout, _: &CudaStorage, _: &Layout) -> Result<()> {
        Err(crate::Error::Cuda(
            format!("no cuda implementation for {}", self.name()).into(),
        ))
    }

    /// The forward pass, as run on a metal gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn metal_fwd(
        &self,
        _: &mut MetalStorage,
        _: &Layout,
        _: &MetalStorage,
        _: &Layout,
    ) -> Result<()> {
        Err(crate::Error::Metal(
            format!("no metal implementation for {}", self.name()).into(),
        ))
    }
}

pub trait InplaceOp3 {
    fn name(&self) -> &'static str;

    /// The forward pass, as run on a cpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cpu_fwd(
        &self,
        s1: &mut CpuStorage,
        l1: &Layout,
        s2: &CpuStorage,
        l2: &Layout,
        s3: &CpuStorage,
        l3: &Layout,
    ) -> Result<()>;

    /// The forward pass, as run on a gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn cuda_fwd(
        &self,
        _: &mut CudaStorage,
        _: &Layout,
        _: &CudaStorage,
        _: &Layout,
        _: &CudaStorage,
        _: &Layout,
    ) -> Result<()> {
        Err(crate::Error::Cuda(
            format!("no cuda implementation for {}", self.name()).into(),
        ))
    }

    /// The forward pass, as run on a metal gpu device. Note that the storage can use arbitrary strides,
    /// offsets etc so the associated layout should be used to access it.
    fn metal_fwd(
        &self,
        _: &mut MetalStorage,
        _: &Layout,
        _: &MetalStorage,
        _: &Layout,
        _: &MetalStorage,
        _: &Layout,
    ) -> Result<()> {
        Err(crate::Error::Metal(
            format!("no metal implementation for {}", self.name()).into(),
        ))
    }
}

impl Tensor {
    /// Applies a unary custom op in place.
    pub fn inplace_op1<C: InplaceOp1>(&self, c: &C) -> Result<()> {
        self.storage_mut().inplace_op1(self.layout(), c)
    }

    /// Applies a unary custom op in place (for the first tensor).
    pub fn inplace_op2<C: InplaceOp2>(&self, rhs: &Self, c: &C) -> Result<()> {
        self.storage_mut()
            .inplace_op2(self.layout(), &rhs.storage(), rhs.layout(), c)
    }

    /// Applies a ternary custom op in place (for the first tensor).
    pub fn inplace_op3<C: InplaceOp3>(&self, t2: &Self, t3: &Self, c: &C) -> Result<()> {
        self.storage_mut().inplace_op3(
            self.layout(),
            &t2.storage(),
            t2.layout(),
            &t3.storage(),
            t3.layout(),
            c,
        )
    }
}

#[cfg(feature = "ug")]
pub struct UgIOp1 {
    name: &'static str,
    #[cfg(feature = "cuda")]
    func: cudarc::driver::CudaFunction,
    #[cfg(feature = "metal")]
    func: candle_metal_kernels::metal::ComputePipeline,
}

#[cfg(feature = "ug")]
impl UgIOp1 {
    #[allow(unused)]
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "ios")))]
    pub fn new(
        name: &'static str,
        kernel: candle_ug::lang::ssa::Kernel,
        device: &crate::Device,
    ) -> Result<Self> {
        #[cfg(feature = "cuda")]
        {
            let device = device.as_cuda_device()?;
            let func = device.compile(name, kernel)?;
            Ok(Self {
                name,
                func: func.into_cuda_function(),
            })
        }
        #[cfg(feature = "metal")]
        {
            let device = device.as_metal_device()?;
            let func = device.compile(name, kernel)?;
            Ok(Self { name, func })
        }
        #[cfg(not(any(feature = "cuda", feature = "metal")))]
        {
            Ok(Self { name })
        }
    }
}

#[cfg(feature = "ug")]
impl InplaceOp1 for UgIOp1 {
    fn name(&self) -> &'static str {
        self.name
    }

    fn cpu_fwd(&self, _: &mut CpuStorage, _: &Layout) -> Result<()> {
        crate::bail!("ug ops are only supported on metal/cuda at the moment")
    }

    #[cfg(feature = "metal")]
    fn metal_fwd(&self, sto: &mut MetalStorage, layout: &Layout) -> Result<()> {
        use crate::backend::BackendStorage;
        use objc2_metal;

        let elem_count = layout.shape().elem_count();
        if sto.dtype() != crate::DType::F32 {
            // TODO: support more dtypes.
            crate::bail!("input is not a f32 tensor")
        }
        let device = sto.device();
        let encoder = device.command_encoder()?;
        encoder.set_compute_pipeline_state(&self.func);
        let (g, b) = if elem_count.is_multiple_of(32) {
            (elem_count / 32, 32)
        } else {
            (elem_count, 1)
        };
        let grid_dims = objc2_metal::MTLSize {
            width: g,
            height: 1,
            depth: 1,
        };
        let group_dims = candle_metal_kernels::utils::get_block_dims(b, 1, 1);
        candle_metal_kernels::utils::set_param(&encoder, 0, (sto.buffer(), 0usize));

        encoder.use_resource(sto.buffer(), objc2_metal::MTLResourceUsage::Write);
        encoder.dispatch_threads(grid_dims, group_dims);

        Ok(())
    }

    #[cfg(feature = "cuda")]
    fn cuda_fwd(&self, sto: &mut CudaStorage, layout: &Layout) -> Result<()> {
        use crate::cuda_backend::WrapErr;
        use cudarc::driver::PushKernelArg;

        let elem_count = layout.shape().elem_count();
        let stream = sto.device.cuda_stream();
        // TODO: support more dtypes.
        let sto = sto.as_cuda_slice::<f32>()?;
        let sto = match layout.contiguous_offsets() {
            None => crate::bail!("input has to be contiguous"),
            Some((o1, o2)) => sto.slice(o1..o2),
        };
        let (g, b) = if elem_count % 32 == 0 {
            (elem_count / 32, 32)
        } else {
            (elem_count, 1)
        };
        let cfg = cudarc::driver::LaunchConfig {
            grid_dim: (g as u32, 1, 1),
            block_dim: (b as u32, 1, 1),
            shared_mem_bytes: 0,
        };
        let mut builder = stream.launch_builder(&self.func);
        builder.arg(&sto);
        unsafe { builder.launch(cfg) }.w()?;
        Ok(())
    }
}

struct FusedSoftmaxLastDim;

impl CustomOp1 for FusedSoftmaxLastDim {
    fn name(&self) -> &'static str {
        "fused-softmax-last-dim"
    }

    fn cpu_fwd(&self, storage: &CpuStorage, layout: &Layout) -> Result<(CpuStorage, Shape)> {
        let (o1, o2) = layout.contiguous_offsets().ok_or_else(|| {
            crate::Error::Msg("fused-softmax-last-dim input must be contiguous".into())
        })?;
        let dims = layout.shape().dims();
        if dims.is_empty() {
            crate::bail!("fused-softmax-last-dim requires rank >= 1");
        }
        let last_dim = dims[dims.len() - 1];
        let elem_count = layout.shape().elem_count();
        let out_storage = match storage {
            CpuStorage::F32(src) => {
                let mut dst = vec![0f32; elem_count];
                for (s, d) in src[o1..o2]
                    .chunks_exact(last_dim)
                    .zip(dst.chunks_exact_mut(last_dim))
                {
                    let max = s.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                    let mut sum = 0f32;
                    for (&sv, dv) in s.iter().zip(d.iter_mut()) {
                        let e = (sv - max).exp();
                        *dv = e;
                        sum += e;
                    }
                    for dv in d.iter_mut() {
                        *dv /= sum;
                    }
                }
                CpuStorage::F32(dst)
            }
            CpuStorage::F64(src) => {
                let mut dst = vec![0f64; elem_count];
                for (s, d) in src[o1..o2]
                    .chunks_exact(last_dim)
                    .zip(dst.chunks_exact_mut(last_dim))
                {
                    let max = s.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                    let mut sum = 0f64;
                    for (&sv, dv) in s.iter().zip(d.iter_mut()) {
                        let e = (sv - max).exp();
                        *dv = e;
                        sum += e;
                    }
                    for dv in d.iter_mut() {
                        *dv /= sum;
                    }
                }
                CpuStorage::F64(dst)
            }
            CpuStorage::BF16(src) => {
                let mut dst = vec![half::bf16::ZERO; elem_count];
                for (s, d) in src[o1..o2]
                    .chunks_exact(last_dim)
                    .zip(dst.chunks_exact_mut(last_dim))
                {
                    let max = s
                        .iter()
                        .map(|v| v.to_f32())
                        .fold(f32::NEG_INFINITY, f32::max);
                    let mut sum = 0f32;
                    let mut tmp = vec![0f32; last_dim];
                    for (sv, tv) in s.iter().zip(tmp.iter_mut()) {
                        let e = (sv.to_f32() - max).exp();
                        *tv = e;
                        sum += e;
                    }
                    for (tv, dv) in tmp.iter().zip(d.iter_mut()) {
                        *dv = half::bf16::from_f32(*tv / sum);
                    }
                }
                CpuStorage::BF16(dst)
            }
            CpuStorage::F16(src) => {
                let mut dst = vec![half::f16::ZERO; elem_count];
                for (s, d) in src[o1..o2]
                    .chunks_exact(last_dim)
                    .zip(dst.chunks_exact_mut(last_dim))
                {
                    let max = s
                        .iter()
                        .map(|v| v.to_f32())
                        .fold(f32::NEG_INFINITY, f32::max);
                    let mut sum = 0f32;
                    let mut tmp = vec![0f32; last_dim];
                    for (sv, tv) in s.iter().zip(tmp.iter_mut()) {
                        let e = (sv.to_f32() - max).exp();
                        *tv = e;
                        sum += e;
                    }
                    for (tv, dv) in tmp.iter().zip(d.iter_mut()) {
                        *dv = half::f16::from_f32(*tv / sum);
                    }
                }
                CpuStorage::F16(dst)
            }
            _ => crate::bail!("unsupported dtype for fused-softmax-last-dim"),
        };
        Ok((out_storage, Shape::from_dims(dims)))
    }

    #[cfg(feature = "metal")]
    fn metal_fwd(&self, storage: &MetalStorage, layout: &Layout) -> Result<(MetalStorage, Shape)> {
        use crate::backend::BackendStorage;
        let device = storage.device();
        let encoder = device.command_encoder()?;
        encoder.set_label("softmax");
        let name = match storage.dtype() {
            crate::DType::F32 => "softmax_f32",
            crate::DType::F16 => "softmax_f16",
            crate::DType::BF16 => "softmax_bf16",
            dtype => crate::bail!("fused-softmax-last-dim is not implemented for {dtype:?}"),
        };
        let n = layout.stride().len();
        if !(layout.is_contiguous() && layout.stride()[n - 1] == 1) {
            crate::bail!("Non contiguous fused-softmax-last-dim is not implemented");
        }
        let last_dim = layout.dims()[layout.shape().rank() - 1];
        let elem_count = layout.shape().elem_count();
        let output = device.new_buffer(elem_count, storage.dtype(), "softmax")?;
        candle_metal_kernels::call_last_softmax(
            device.metal_device(),
            &encoder,
            device.kernels(),
            name,
            elem_count,
            last_dim,
            storage.buffer(),
            layout.start_offset() * storage.dtype().size_in_bytes(),
            &output,
        )
        .map_err(crate::Error::wrap)?;
        let newstorage = MetalStorage::new(output, device.clone(), elem_count, storage.dtype());
        Ok((newstorage, layout.shape().clone()))
    }

    fn bwd(&self, _arg: &Tensor, res: &Tensor, grad_res: &Tensor) -> Result<Option<Tensor>> {
        let dot = (res * grad_res)?.sum_keepdim(crate::D::Minus1)?;
        Ok(Some((res * grad_res.broadcast_sub(&dot)?)?))
    }
}

impl Tensor {
    /// Differentiable fused last-dimension softmax. Uses a single Metal reduction kernel
    /// (`call_last_softmax`) instead of 5 decomposed ops and retains only the output tensor
    /// for backward instead of 4 intermediate tensors.
    pub fn softmax_last_dim(&self) -> Result<Self> {
        if self.device().is_metal() || self.device().is_cpu() {
            self.contiguous()?.apply_op1(FusedSoftmaxLastDim)
        } else {
            let max = self.max_keepdim(crate::D::Minus1)?;
            let diff = self.broadcast_sub(&max)?;
            let num = diff.exp()?;
            let den = num.sum_keepdim(crate::D::Minus1)?;
            num.broadcast_div(&den)
        }
    }

    /// Fused Metal NF4 matrix-vector multiply `[1, in_features] @ W_nf4^T -> [1, out_features]`
    /// with block_size=64 F32 scales and packed U8 NF4 nibbles.
    pub fn nf4_gemv(
        &self,
        q_data: &Self,
        scales: &Self,
        out_features: usize,
        in_features: usize,
    ) -> Result<Self> {
        self.contiguous()?.apply_op3_no_bwd(
            q_data,
            scales,
            &Nf4GemvOp {
                out_features,
                in_features,
            },
        )
    }

    /// Fused Metal NF4 dequantize `[out_features * in_features / 2]` U8 + F32 scales
    /// -> `[out_features, in_features]` in `out_dtype`.
    pub fn nf4_dequant(
        q_data: &Self,
        scales: &Self,
        out_features: usize,
        in_features: usize,
        out_dtype: crate::DType,
    ) -> Result<Self> {
        q_data.apply_op2_no_bwd(
            scales,
            &Nf4DequantOp {
                out_features,
                in_features,
                out_dtype,
            },
        )
    }
}

struct Nf4GemvOp {
    out_features: usize,
    in_features: usize,
}

impl CustomOp3 for Nf4GemvOp {
    fn name(&self) -> &'static str {
        "nf4-gemv"
    }

    fn cpu_fwd(
        &self,
        _s1: &CpuStorage,
        _l1: &Layout,
        _s2: &CpuStorage,
        _l2: &Layout,
        _s3: &CpuStorage,
        _l3: &Layout,
    ) -> Result<(CpuStorage, Shape)> {
        crate::bail!("nf4-gemv is Metal-only")
    }

    #[cfg(feature = "metal")]
    fn metal_fwd(
        &self,
        s_x: &MetalStorage,
        l_x: &Layout,
        s_q: &MetalStorage,
        l_q: &Layout,
        s_sc: &MetalStorage,
        l_sc: &Layout,
    ) -> Result<(MetalStorage, Shape)> {
        use crate::backend::BackendStorage;
        if !l_x.is_contiguous() || !l_q.is_contiguous() || !l_sc.is_contiguous() {
            crate::bail!("nf4-gemv requires contiguous inputs");
        }
        if s_q.dtype() != crate::DType::U8 || s_sc.dtype() != crate::DType::F32 {
            crate::bail!(
                "nf4-gemv expects U8 q_data and F32 scales, got {:?} and {:?}",
                s_q.dtype(),
                s_sc.dtype()
            );
        }
        if self.in_features < 64 || !self.in_features.is_multiple_of(64) {
            crate::bail!(
                "nf4-gemv requires in_features ({}) to be a multiple of 64",
                self.in_features
            );
        }
        let (m, k_in) = l_x.shape().dims2()?;
        if !(1..=16).contains(&m) || k_in != self.in_features {
            crate::bail!(
                "nf4-gemv expects lhs shape [1..=16, {}], got [{m}, {k_in}]",
                self.in_features
            );
        }
        let gemm_dtype = match s_x.dtype() {
            crate::DType::F32 => candle_metal_kernels::GemmDType::F32,
            crate::DType::F16 => candle_metal_kernels::GemmDType::F16,
            crate::DType::BF16 => candle_metal_kernels::GemmDType::BF16,
            dt => crate::bail!("nf4-gemv unsupported activation dtype {dt:?}"),
        };
        let out_elems = m * self.out_features;
        let device = s_x.device();
        let encoder = device.command_encoder()?;
        encoder.set_label("nf4_gemv");
        let output = device.new_buffer(out_elems, s_x.dtype(), "nf4_gemv")?;
        candle_metal_kernels::call_nf4_gemv(
            device.metal_device(),
            &encoder,
            device.kernels(),
            gemm_dtype,
            (m, self.out_features, self.in_features),
            l_x.start_offset() * s_x.dtype().size_in_bytes(),
            s_x.buffer(),
            l_q.start_offset(),
            s_q.buffer(),
            l_sc.start_offset() * crate::DType::F32.size_in_bytes(),
            s_sc.buffer(),
            &output,
        )
        .map_err(crate::Error::wrap)?;
        let out_storage = MetalStorage::new(output, device.clone(), out_elems, s_x.dtype());
        Ok((out_storage, Shape::from_dims(&[m, self.out_features])))
    }
}

struct Nf4DequantOp {
    out_features: usize,
    in_features: usize,
    out_dtype: crate::DType,
}

impl CustomOp2 for Nf4DequantOp {
    fn name(&self) -> &'static str {
        "nf4-dequant"
    }

    fn cpu_fwd(
        &self,
        _s1: &CpuStorage,
        _l1: &Layout,
        _s2: &CpuStorage,
        _l2: &Layout,
    ) -> Result<(CpuStorage, Shape)> {
        crate::bail!("nf4-dequant is Metal-only")
    }

    #[cfg(feature = "metal")]
    fn metal_fwd(
        &self,
        s_q: &MetalStorage,
        l_q: &Layout,
        s_sc: &MetalStorage,
        l_sc: &Layout,
    ) -> Result<(MetalStorage, Shape)> {
        use crate::backend::BackendStorage;
        if !l_q.is_contiguous() || !l_sc.is_contiguous() {
            crate::bail!("nf4-dequant requires contiguous inputs");
        }
        if s_q.dtype() != crate::DType::U8 || s_sc.dtype() != crate::DType::F32 {
            crate::bail!(
                "nf4-dequant expects U8 q_data and F32 scales, got {:?} and {:?}",
                s_q.dtype(),
                s_sc.dtype()
            );
        }
        let numel = self.out_features * self.in_features;
        let num_bytes = numel / 2;
        let gemm_dtype = match self.out_dtype {
            crate::DType::F32 => candle_metal_kernels::GemmDType::F32,
            crate::DType::F16 => candle_metal_kernels::GemmDType::F16,
            crate::DType::BF16 => candle_metal_kernels::GemmDType::BF16,
            dt => crate::bail!("nf4-dequant unsupported out_dtype {dt:?}"),
        };
        let device = s_q.device();
        let encoder = device.command_encoder()?;
        encoder.set_label("nf4_dequant");
        let output = device.new_buffer(numel, self.out_dtype, "nf4_dequant")?;
        candle_metal_kernels::call_nf4_dequant(
            device.metal_device(),
            &encoder,
            device.kernels(),
            gemm_dtype,
            num_bytes,
            l_q.start_offset(),
            s_q.buffer(),
            l_sc.start_offset() * crate::DType::F32.size_in_bytes(),
            s_sc.buffer(),
            &output,
        )
        .map_err(crate::Error::wrap)?;
        let out_storage = MetalStorage::new(output, device.clone(), numel, self.out_dtype);
        Ok((
            out_storage,
            Shape::from_dims(&[self.out_features, self.in_features]),
        ))
    }
}
