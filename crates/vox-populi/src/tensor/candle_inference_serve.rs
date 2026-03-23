//! Native Candle inference server bridging to `vox populi serve` payload

use candle_core::{Device, DType};

/// Stub for loading a native Candle inference model from a directory.
/// In a full implementation, this would instantiate `candle_transformers`
/// or a manual transformer graph.
pub fn load_candle_inference_model(
    _model_dir: &std::path::Path,
    _device: &Device,
    _dtype: DType,
) -> anyhow::Result<()> {
    // Requires candle-transformers or full manual graph implementation
    anyhow::bail!("Candle native inference not yet implemented in this stub. See AI-Native Core Simplification Strategy.")
}
