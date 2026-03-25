/// LoRA-adapted Transformer Block.
///
/// Pre-norm architecture: LayerNorm → Attention → residual → LayerNorm → FFN → residual.
/// Supports GPT-2 (ff_linear1 + GELU) and Qwen2 (ff_gate + ff_up + SwiGLU).
#[cfg(feature = "mens-gpu")]
#[derive(Module, Debug)]
pub struct LoraTransformerBlock<B: Backend> {
    /// LoRA-adapted multi-head attention.
    pub attention: LoraAttention<B>,
    /// First layer normalization (applied before attention).
    pub norm1: nn::LayerNorm<B>,
    /// GPT-2 style: single linear before GELU. None when using SwiGLU.
    pub ff_linear1: Option<nn::Linear<B>>,
    /// Qwen2 SwiGLU: gate projection. None when using GPT-2 MLP.
    pub ff_gate: Option<nn::Linear<B>>,
    /// Qwen2 SwiGLU: up projection. None when using GPT-2 MLP.
    pub ff_up: Option<nn::Linear<B>>,
    /// Second linear layer (projection) of the FFN.
    pub ff_linear2: nn::Linear<B>,
    /// Second layer normalization (applied before FFN).
    pub norm2: nn::LayerNorm<B>,
}

#[cfg(feature = "mens-gpu")]
impl<B: Backend> LoraTransformerBlock<B> {
    /// Create a newly initialized LoRA adapted transformer block.
    pub fn new(device: &B::Device, d_model: usize, n_heads: usize, r: usize, alpha: f32) -> Self {
        Self {
            attention: LoraAttention::new(device, d_model, n_heads, r, alpha),
            norm1: nn::LayerNormConfig::new(d_model).init(device),
            ff_linear1: Some(nn::LinearConfig::new(d_model, d_model * 4).init(device)),
            ff_gate: None,
            ff_up: None,
            ff_linear2: nn::LinearConfig::new(d_model * 4, d_model).init(device),
            norm2: nn::LayerNormConfig::new(d_model).init(device),
        }
    }

    /// Create block with SwiGLU MLP (for Qwen2).
    pub fn new_swiglu(
        device: &B::Device,
        d_model: usize,
        n_heads: usize,
        intermediate_size: usize,
        r: usize,
        alpha: f32,
    ) -> Self {
        Self {
            attention: LoraAttention::new(device, d_model, n_heads, r, alpha),
            norm1: nn::LayerNormConfig::new(d_model).init(device),
            ff_linear1: None,
            ff_gate: Some(nn::LinearConfig::new(d_model, intermediate_size).init(device)),
            ff_up: Some(nn::LinearConfig::new(d_model, intermediate_size).init(device)),
            ff_linear2: nn::LinearConfig::new(intermediate_size, d_model).init(device),
            norm2: nn::LayerNormConfig::new(d_model).init(device),
        }
    }

    /// Forward pass through the transformer block.
    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let x_attn = self.attention.forward(self.norm1.forward(x.clone()));
        let x_mid = x + x_attn;

        let h = self.norm2.forward(x_mid.clone());
        let x_ff = if let (Some(gate), Some(up)) = (&self.ff_gate, &self.ff_up) {
            self.ff_linear2
                .forward(gate.forward(h.clone()) * burn::tensor::activation::silu(up.forward(h)))
        } else if let Some(ref ff1) = self.ff_linear1 {
            self.ff_linear2
                .forward(burn::tensor::activation::gelu(ff1.forward(h)))
        } else {
            unreachable!("block must have either ff_linear1 or (ff_gate, ff_up)");
        };
        x_mid + x_ff
    }

    /// Forward with attention K/V capture for incremental decode.
    pub fn forward_prefill_with_kv(
        &self,
        x: Tensor<B, 3>,
    ) -> (Tensor<B, 3>, LoraAttentionKvCache<B>) {
        let (x_attn, kv) = self
            .attention
            .forward_prefill_with_kv(self.norm1.forward(x.clone()));
        let x_mid = x + x_attn;

        let h = self.norm2.forward(x_mid.clone());
        let x_ff = if let (Some(gate), Some(up)) = (&self.ff_gate, &self.ff_up) {
            self.ff_linear2
                .forward(gate.forward(h.clone()) * burn::tensor::activation::silu(up.forward(h)))
        } else if let Some(ref ff1) = self.ff_linear1 {
            self.ff_linear2
                .forward(burn::tensor::activation::gelu(ff1.forward(h)))
        } else {
            unreachable!("block must have either ff_linear1 or (ff_gate, ff_up)");
        };
        (x_mid + x_ff, kv)
    }

    /// One autoregressive step; updates `kv` for this layer.
    pub fn forward_decode_one(
        &self,
        x: Tensor<B, 3>,
        kv: &mut LoraAttentionKvCache<B>,
        abs_pos: usize,
    ) -> Tensor<B, 3> {
        let attn_out =
            self.attention
                .forward_decode_one(self.norm1.forward(x.clone()), kv, abs_pos);
        let x_mid = x + attn_out;

        let h = self.norm2.forward(x_mid.clone());
        let x_ff = if let (Some(gate), Some(up)) = (&self.ff_gate, &self.ff_up) {
            self.ff_linear2
                .forward(gate.forward(h.clone()) * burn::tensor::activation::silu(up.forward(h)))
        } else if let Some(ref ff1) = self.ff_linear1 {
            self.ff_linear2
                .forward(burn::tensor::activation::gelu(ff1.forward(h)))
        } else {
            unreachable!("block must have either ff_linear1 or (ff_gate, ff_up)");
        };
        x_mid + x_ff
    }

    /// Merge this block into a plain `TransformerBlock`.
    ///
    /// For SwiGLU blocks (Qwen2), merge is not fully supported — returns a placeholder.
    pub fn merge(&self) -> super::burn_stack::TransformerBlock<B> {
        if self.ff_gate.is_some() {
            let device = self.ff_linear2.weight.devices()[0].clone();
            let d_model = self.ff_linear2.weight.dims()[1];
            let intermediate = self.ff_linear2.weight.dims()[0];
            let ff1 = nn::LinearConfig::new(d_model, intermediate).init(&device);
            return super::burn_stack::TransformerBlock {
                attention: self.attention.merge(),
                norm1: self.norm1.clone(),
                ff_linear1: ff1,
                ff_linear2: self.ff_linear2.clone(),
                norm2: self.norm2.clone(),
            };
        }
        let ff1 = self
            .ff_linear1
            .as_ref()
            .expect("GPT-2 block must have ff_linear1");
        super::burn_stack::TransformerBlock {
            attention: self.attention.merge(),
            norm1: self.norm1.clone(),
            ff_linear1: ff1.clone(),
            ff_linear2: self.ff_linear2.clone(),
            norm2: self.norm2.clone(),
        }
    }
}

