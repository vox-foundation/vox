//! Plain transformer for merged LoRA weights and shared loss helpers.
//!
//! Named `burn_stack` (not `nn`) so `use burn::nn` does not collide with a local `nn` module.

#[cfg(feature = "gpu")]
use burn::module::Module;
#[cfg(feature = "gpu")]
use burn::nn;
#[cfg(feature = "gpu")]
use burn::nn::attention::{MhaInput, generate_autoregressive_mask};
#[cfg(feature = "gpu")]
use burn::tensor::backend::Backend;
#[cfg(feature = "gpu")]
use burn::tensor::{Int, Tensor};

#[cfg(feature = "gpu")]
pub use vox_tensor::vox_nn::{Sequential, cross_entropy_loss};

/// Label value ignored in CE (matches Hugging Face / HF trainers).
pub const IGNORE_INDEX: i64 = -100;

#[cfg(feature = "gpu")]
#[derive(Module, Debug)]
pub struct TransformerBlock<B: Backend> {
    pub attention: nn::attention::MultiHeadAttention<B>,
    pub norm1: nn::LayerNorm<B>,
    pub ff_linear1: nn::Linear<B>,
    pub ff_linear2: nn::Linear<B>,
    pub norm2: nn::LayerNorm<B>,
}

#[cfg(feature = "gpu")]
impl<B: Backend> TransformerBlock<B> {
    pub fn new(device: &B::Device, d_model: usize, n_heads: usize) -> Self {
        let attention = nn::attention::MultiHeadAttentionConfig::new(d_model, n_heads)
            .with_dropout(0.0)
            .with_min_float(-1e9f64)
            .init(device);
        let norm1 = nn::LayerNormConfig::new(d_model).init(device);
        let norm2 = nn::LayerNormConfig::new(d_model).init(device);
        let ff_linear1 = nn::LinearConfig::new(d_model, d_model * 4).init(device);
        let ff_linear2 = nn::LinearConfig::new(d_model * 4, d_model).init(device);
        Self {
            attention,
            norm1,
            ff_linear1,
            ff_linear2,
            norm2,
        }
    }

    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let [batch, seq_len, _d] = x.dims();
        let device = x.devices()[0].clone();
        let h = self.norm1.forward(x.clone());
        let mask = generate_autoregressive_mask(batch, seq_len, &device);
        let attn_input = MhaInput::self_attn(h).mask_attn(mask);
        let attn_out = self.attention.forward(attn_input);
        let x = x + attn_out.context;

        let h = self.norm2.forward(x.clone());
        let h = self.ff_linear1.forward(h);
        let h = burn::tensor::activation::gelu(h);
        let h = self.ff_linear2.forward(h);
        x + h
    }
}

#[cfg(feature = "gpu")]
#[derive(Module, Debug)]
pub struct VoxTransformer<B: Backend> {
    pub embedding: nn::Embedding<B>,
    pub pos_embedding: nn::Embedding<B>,
    pub blocks: Vec<TransformerBlock<B>>,
    pub norm: nn::LayerNorm<B>,
    pub output: nn::Linear<B>,
    pub use_rope: bool,
}

#[cfg(feature = "gpu")]
impl<B: Backend> VoxTransformer<B> {
    /// Build a randomly initialized transformer matching **`LoraVoxTransformer::merge()`** topology
    /// (used before `Checkpoint::load` for merged `model_merged.bin` weights).
    pub fn new(
        device: &B::Device,
        vocab_size: usize,
        d_model: usize,
        n_heads: usize,
        n_layers: usize,
    ) -> Self {
        // Keep in sync with `lora::LoraVoxTransformer::new` positional table size.
        const POS_EMBED_LEN: usize = 512;
        Self {
            embedding: nn::EmbeddingConfig::new(vocab_size, d_model).init(device),
            pos_embedding: nn::EmbeddingConfig::new(POS_EMBED_LEN, d_model).init(device),
            blocks: (0..n_layers)
                .map(|_| TransformerBlock::new(device, d_model, n_heads))
                .collect(),
            norm: nn::LayerNormConfig::new(d_model).init(device),
            output: nn::LinearConfig::new(d_model, vocab_size).init(device),
            use_rope: false,
        }
    }

    pub fn forward(&self, x: Tensor<B, 2, Int>) -> Tensor<B, 3> {
        let [batch, seq_len] = x.dims();
        let device = x.devices()[0].clone();

        let mut h = self.embedding.forward(x.clone());

        if !self.use_rope {
            let pos_ids_1d = Tensor::<B, 1, Int>::arange(0..seq_len as i64, &device);
            let pos_ids = pos_ids_1d.reshape([1, seq_len]).repeat_dim(0, batch);
            let pos_emb = self.pos_embedding.forward(pos_ids);
            h = h + pos_emb;
        }

        for block in &self.blocks {
            h = block.forward(h);
        }

        h = self.norm.forward(h);
        let d_model = h.dims()[2];
        let vocab_size = self.output.weight.dims()[1];
        let logits = self.output.forward(h.reshape([batch * seq_len, d_model]));
        logits.reshape([batch, seq_len, vocab_size])
    }
}
