/// Maximum sequence length for positional embeddings (must match train/serve config).
pub const MAX_SEQ_LEN: usize = 512;

/// Default model architecture (d_model, n_heads, n_layers) — must match training config.
pub const DEFAULT_D_MODEL: usize = 512;
/// Default number of attention heads (must divide `DEFAULT_D_MODEL`).
pub const DEFAULT_N_HEADS: usize = 8;
/// Default number of transformer layers.
pub const DEFAULT_N_LAYERS: usize = 6;

/// LoRA-wrapped Vox Transformer Model.
///
/// A full stack of `LoraTransformerBlock`s with embedding and output head.
/// Used as the trainable model in `run_qlora_training`.
///
/// Architecture:
/// - `embedding`: token → d_model
/// - `pos_embedding`: learned positional embeddings (zero when use_rope)
/// - `blocks`: N × LoraTransformerBlock
/// - `norm`: final LayerNorm
/// - `output`: d_model → vocab_size (output head, not LoRA-adapted)
/// - `use_rope`: when true, skip pos_embedding (Qwen2 uses RoPE in attention)
#[cfg(feature = "mens-gpu")]
#[derive(Module, Debug)]
pub struct LoraVoxTransformer<B: Backend> {
    /// Token embeddings.
    pub embedding: nn::Embedding<B>,
    /// Positional embeddings.
    pub pos_embedding: nn::Embedding<B>,
    /// Stack of LoRA adapted transformer blocks.
    pub blocks: Vec<LoraTransformerBlock<B>>,
    /// Final layer normalization.
    pub norm: nn::LayerNorm<B>,
    /// Output projection layer producing logits.
    pub output: nn::Linear<B>,
    /// When true, do not add pos_embedding (RoPE is used in attention).
    pub use_rope: bool,
}

#[cfg(feature = "mens-gpu")]
impl<B: Backend> LoraVoxTransformer<B> {
    /// Create a newly initialized full LoRA adapted transformer model.
    pub fn new(
        device: &B::Device,
        vocab_size: usize,
        d_model: usize,
        n_heads: usize,
        n_layers: usize,
        r: usize,
        alpha: f32,
    ) -> Self {
        Self {
            embedding: nn::EmbeddingConfig::new(vocab_size, d_model).init(device),
            pos_embedding: nn::EmbeddingConfig::new(MAX_SEQ_LEN, d_model).init(device),
            blocks: (0..n_layers)
                .map(|_| LoraTransformerBlock::new(device, d_model, n_heads, r, alpha))
                .collect(),
            norm: nn::LayerNormConfig::new(d_model).init(device),
            output: nn::LinearConfig::new(d_model, vocab_size).init(device),
            use_rope: false,
        }
    }

    /// Forward pass.
    ///
    /// Input:  `x` — integer token indices, shape `[batch, seq_len]`
    /// Output: logits shape `[batch, seq_len, vocab_size]`
    pub fn forward(&self, x: Tensor<B, 2, burn::tensor::Int>) -> Tensor<B, 3> {
        let [batch, seq_len] = x.dims();
        let device = x.devices()[0].clone();

        // Token embedding → [batch, seq_len, d_model]
        let mut h = self.embedding.forward(x.clone());

        // Learned positional embeddings (skip when use_rope — RoPE is in attention)
        if !self.use_rope {
            let pos_ids_1d =
                burn::tensor::Tensor::<B, 1, burn::tensor::Int>::arange(0..seq_len as i64, &device);
            let pos_ids = pos_ids_1d.reshape([1, seq_len]).repeat_dim(0, batch);
            let pos_emb = self.pos_embedding.forward(pos_ids);
            h = h + pos_emb;
        }

        // Transformer blocks
        for block in &self.blocks {
            h = block.forward(h);
        }

        // Final layer-norm
        h = self.norm.forward(h);

        // Output head: project each token position to vocab logits.
        // Flatten to [batch * seq_len, d_model], apply linear, reshape back.
        // Burn Linear weight is [d_input, d_output] = [in_features, vocab_size]
        let d_model = h.dims()[2];
        let vocab_size = self.output.weight.dims()[1];
        let logits = self.output.forward(h.reshape([batch * seq_len, d_model]));
        logits.reshape([batch, seq_len, vocab_size])
    }

    /// Prompt prefill with KV capture: logits for the **last** prompt token, shape `[batch, vocab_size]`.
    pub fn forward_prefill_last_logits_and_kv(
        &self,
        x: Tensor<B, 2, Int>,
    ) -> (Tensor<B, 2>, LoraTransformerKvCache<B>) {
        let [batch, seq_len] = x.dims();
        let device = x.devices()[0].clone();

        let mut h = self.embedding.forward(x.clone());
        if !self.use_rope {
            let pos_ids_1d = Tensor::<B, 1, Int>::arange(0..seq_len as i64, &device);
            let pos_ids = pos_ids_1d.reshape([1, seq_len]).repeat_dim(0, batch);
            let pos_emb = self.pos_embedding.forward(pos_ids);
            h = h + pos_emb;
        }

        let mut layer_caches: Vec<LoraAttentionKvCache<B>> = Vec::with_capacity(self.blocks.len());
        for block in &self.blocks {
            let (h_next, kv) = block.forward_prefill_with_kv(h);
            layer_caches.push(kv);
            h = h_next;
        }

        h = self.norm.forward(h);
        let d_model = h.dims()[2];
        let last = h.slice([0..batch, (seq_len - 1)..seq_len, 0..d_model]);
        let last = last.reshape([batch, d_model]);
        let logits = self.output.forward(last);
        (
            logits,
            LoraTransformerKvCache {
                layers: layer_caches,
            },
        )
    }

    /// Decode one new token; `abs_pos` is the index of that token in the running sequence.
    pub fn forward_decode_logits(
        &self,
        token: Tensor<B, 2, Int>,
        abs_pos: usize,
        caches: &mut LoraTransformerKvCache<B>,
    ) -> Tensor<B, 2> {
        let [batch, one] = token.dims();
        debug_assert_eq!(one, 1);
        let device = token.devices()[0].clone();

        let mut h = self.embedding.forward(token.clone());
        if !self.use_rope {
            let pos_id =
                Tensor::<B, 1, Int>::from_ints(TensorData::new(vec![abs_pos as i32], [1]), &device);
            let pos_ids = pos_id.reshape([1, 1]).repeat_dim(0, batch);
            let pos_emb = self.pos_embedding.forward(pos_ids);
            h = h + pos_emb;
        }

        for (i, block) in self.blocks.iter().enumerate() {
            h = block.forward_decode_one(h, &mut caches.layers[i], abs_pos);
        }

        h = self.norm.forward(h);
        let d_model = h.dims()[2];
        self.output.forward(h.reshape([batch, d_model]))
    }

    /// Merge all LoRA adapters back into base weights.
    ///
    /// Returns a plain `VoxTransformer` suitable for inference (no LoRA overhead).
    pub fn merge(&self) -> super::burn_stack::VoxTransformer<B> {
        super::burn_stack::VoxTransformer {
            embedding: self.embedding.clone(),
            pos_embedding: self.pos_embedding.clone(),
            blocks: self.blocks.iter().map(|b| b.merge()).collect(),
            norm: self.norm.clone(),
            output: self.output.clone(),
            use_rope: self.use_rope,
        }
    }
}

