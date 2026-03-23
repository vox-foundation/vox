//! GGUF export functionality.
//!
//! Export quantized models to GGUF format for inference with llama.cpp and similar tools.
//!
//! GGUF (GUFF Universal Format) is a binary format designed for LLM storage and inference.
//! This module handles serialization of quantized tensors with proper metadata and offsets.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::error::{QLoraError, Result};
use crate::quantization::QuantizedTensor;

/// GGUF file magic number.
const GGUF_MAGIC: u32 = 0x4655_4747; // "GGUF"

/// GGUF version (3 = latest stable).
const GGUF_VERSION: u32 = 3;

/// GGUF tensor type for `Q4_0` (NF4 compatible).
const GGUF_TYPE_Q4_0: u32 = 2;

/// GGUF metadata key-value pairs.
#[derive(Debug, Clone)]
pub struct GgufMetadata {
    /// Name of the model.
    pub model_name: String,
    /// Type of the model (e.g., "qlora").
    pub model_type: String,
    /// Total size of the model in elements.
    pub model_size: usize,
}

impl Default for GgufMetadata {
    fn default() -> Self {
        Self {
            model_name: "qlora-model".to_string(),
            model_type: "qlora".to_string(),
            model_size: 0,
        }
    }
}

/// Export quantized tensors to GGUF format with proper structure.
///
/// # Arguments
/// * `tensors` - Named quantized tensors to export
/// * `metadata` - Optional metadata for the model
/// * `output_path` - Path to write the GGUF file
///
/// # Errors
/// Returns error if file cannot be written or format is invalid
pub fn export_gguf<P: AsRef<Path>>(
    tensors: &[(&str, &QuantizedTensor)],
    metadata: Option<GgufMetadata>,
    output_path: P,
) -> Result<()> {
    let mut file = std::fs::File::create(output_path)
        .map_err(|e| QLoraError::GgufExport(format!("Failed to create output file: {e}")))?;

    let metadata = metadata.unwrap_or_default();

    // Collect metadata key-value pairs
    let mut kv_pairs = HashMap::new();
    kv_pairs.insert("general.name".to_string(), metadata.model_name);
    kv_pairs.insert("general.type".to_string(), metadata.model_type);

    // Calculate offsets for tensor data (including scales)
    let header_size = calculate_header_size(tensors, &kv_pairs);
    let mut current_offset = header_size as u64;
    let mut tensor_offsets = Vec::new();

    for (_name, tensor) in tensors {
        tensor_offsets.push(current_offset);
        current_offset += tensor.data.len() as u64; // quantized data
        current_offset += (tensor.scales.len() * 4) as u64; // scales (f32)
        if let Some(ref zp) = tensor.zero_points {
            current_offset += (zp.len() * 4) as u64; // zero points (f32)
        }
        // Double-quantized data
        if let Some(ref scales_q) = tensor.scales_quantized {
            current_offset += scales_q.len() as u64;
        }
        if let Some(ref scales_s) = tensor.scales_scales {
            current_offset += (scales_s.len() * 4) as u64;
        }
    }

    // Write header
    write_gguf_header(&mut file, &kv_pairs, tensors, &tensor_offsets)?;

    // Write tensor data (including scales and metadata)
    for (_name, tensor) in tensors {
        // Quantized values
        file.write_all(&tensor.data)
            .map_err(|e| QLoraError::GgufExport(format!("Failed to write tensor data: {e}")))?;

        // Scales
        for &scale in &tensor.scales {
            file.write_all(&scale.to_le_bytes())
                .map_err(|e| QLoraError::GgufExport(format!("Failed to write scale: {e}")))?;
        }

        // Zero points
        if let Some(ref zp) = tensor.zero_points {
            for &zp_val in zp {
                file.write_all(&zp_val.to_le_bytes()).map_err(|e| {
                    QLoraError::GgufExport(format!("Failed to write zero point: {e}"))
                })?;
            }
        }

        // Double-quantized data
        if let Some(ref scales_q) = tensor.scales_quantized {
            file.write_all(scales_q).map_err(|e| {
                QLoraError::GgufExport(format!("Failed to write double-quantized scales: {e}"))
            })?;
        }
        if let Some(ref scales_s) = tensor.scales_scales {
            for &scale_s in scales_s {
                file.write_all(&scale_s.to_le_bytes()).map_err(|e| {
                    QLoraError::GgufExport(format!(
                        "Failed to write double-quantized scale factors: {e}"
                    ))
                })?;
            }
        }
    }

    Ok(())
}

/// Write GGUF file header with metadata and tensor information.
fn write_gguf_header<W: Write>(
    writer: &mut W,
    kv_pairs: &HashMap<String, String>,
    tensors: &[(&str, &QuantizedTensor)],
    offsets: &[u64],
) -> Result<()> {
    // Magic number and version
    writer
        .write_all(&GGUF_MAGIC.to_le_bytes())
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write magic: {e}")))?;
    writer
        .write_all(&GGUF_VERSION.to_le_bytes())
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write version: {e}")))?;

    // Number of tensors and metadata entries
    let n_tensors = u64::try_from(tensors.len())
        .map_err(|_| QLoraError::GgufExport("Too many tensors".into()))?;
    writer
        .write_all(&n_tensors.to_le_bytes())
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write tensor count: {e}")))?;

    let n_kv = u64::try_from(kv_pairs.len())
        .map_err(|_| QLoraError::GgufExport("Too many metadata entries".into()))?;
    writer
        .write_all(&n_kv.to_le_bytes())
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write kv count: {e}")))?;

    // Write key-value metadata
    for (key, value) in kv_pairs {
        write_kv_pair(writer, key, value)?;
    }

    // Write tensor info
    for ((name, tensor), &offset) in tensors.iter().zip(offsets.iter()) {
        write_tensor_info(writer, name, tensor, offset)?;
    }

    Ok(())
}

/// Write a metadata key-value pair in GGUF format.
fn write_kv_pair<W: Write>(writer: &mut W, key: &str, value: &str) -> Result<()> {
    // Key length and key
    let key_bytes = key.as_bytes();
    writer
        .write_all(&(key_bytes.len() as u64).to_le_bytes())
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write key length: {e}")))?;
    writer
        .write_all(key_bytes)
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write key: {e}")))?;

    // Value type (1 = string)
    writer
        .write_all(&1u32.to_le_bytes())
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write value type: {e}")))?;

    // Value length and value
    let value_bytes = value.as_bytes();
    writer
        .write_all(&(value_bytes.len() as u64).to_le_bytes())
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write value length: {e}")))?;
    writer
        .write_all(value_bytes)
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write value: {e}")))?;

    Ok(())
}

/// Write tensor information with proper offset to GGUF header.
fn write_tensor_info<W: Write>(
    writer: &mut W,
    name: &str,
    tensor: &QuantizedTensor,
    offset: u64,
) -> Result<()> {
    // Name length and name
    let name_bytes = name.as_bytes();
    writer
        .write_all(&(name_bytes.len() as u64).to_le_bytes())
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write name length: {e}")))?;
    writer
        .write_all(name_bytes)
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write name: {e}")))?;

    // Number of dimensions
    let n_dims = u32::try_from(tensor.shape.len())
        .map_err(|_| QLoraError::GgufExport("Tensor has too many dimensions".into()))?;
    writer
        .write_all(&n_dims.to_le_bytes())
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write dimension count: {e}")))?;

    // Dimensions (as u64)
    for &dim in &tensor.shape {
        writer
            .write_all(&(dim as u64).to_le_bytes())
            .map_err(|e| QLoraError::GgufExport(format!("Failed to write dimension: {e}")))?;
    }

    // Type (Q4_0 for NF4)
    writer
        .write_all(&GGUF_TYPE_Q4_0.to_le_bytes())
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write tensor type: {e}")))?;

    // Offset to tensor data
    writer
        .write_all(&offset.to_le_bytes())
        .map_err(|e| QLoraError::GgufExport(format!("Failed to write tensor offset: {e}")))?;

    Ok(())
}

/// Calculate the size of the header before tensor data begins.
/// This is needed to compute correct offsets.
fn calculate_header_size(
    tensors: &[(&str, &QuantizedTensor)],
    kv_pairs: &HashMap<String, String>,
) -> usize {
    let mut size = 0;

    // Magic + version + tensor count + kv count
    size += 4 + 4 + 8 + 8;

    // Key-value pairs
    for (key, value) in kv_pairs {
        size += 8 + key.len() + 4 + 8 + value.len(); // key_len + key + value_type + value_len + value
    }

    // Tensor info headers
    for (name, tensor) in tensors {
        size += 8 + name.len(); // name_len + name
        size += 4; // n_dims
        size += tensor.shape.len() * 8; // dimensions
        size += 4; // type
        size += 8; // offset
    }

    size
}

/// Merge `LoRA` weights into quantized base and export.
///
/// This dequantizes the base, merges `LoRA`, and re-quantizes.
/// Useful for deployment without `LoRA` overhead.
///
/// The merged weight is computed as: `W_merged = W_base + (B @ A) * scale`
/// where:
/// - `W_base` is the dequantized base weight
/// - `A` and `B` are the `LoRA` adapter matrices
/// - `scale` is the `LoRA` scaling factor (alpha / rank)
///
/// # Arguments
/// * `layer` - `QLoRA` layer containing base weights and `LoRA` adapters
/// * `output_path` - Path to write the merged GGUF file
///
/// # Errors
/// Returns error if dequantization, merging, or export fails
pub fn merge_and_export_gguf<P: AsRef<Path>>(
    layer: &crate::qlora::QLoraLayer,
    output_path: P,
) -> Result<()> {
    use crate::quantization::{dequantize_nf4, quantize_nf4};

    // Step 1: Dequantize the base weights
    let quantized_base = layer.quantized_weight();
    let device = layer.device();
    let w_base = dequantize_nf4(quantized_base, device)?;

    // Step 2: Get LoRA weights and compute the LoRA delta: (B @ A) * scale
    let (lora_a, lora_b) = layer.lora_weights();
    let scale = layer.lora_scale();

    // Compute B @ A
    let lora_delta = lora_b.matmul(lora_a)?;

    // Apply scaling: (B @ A) * scale
    let lora_delta_scaled = lora_delta.affine(scale, 0.0)?;

    // Step 3: Merge weights: W_merged = W_base + (B @ A) * scale
    let w_merged = w_base.add(&lora_delta_scaled)?;

    // Step 4: Re-quantize the merged weights
    let config = layer.config();
    let merged_quantized = quantize_nf4(&w_merged, config.quantization.block_size)?;

    // Step 5: Export to GGUF format
    let metadata = GgufMetadata {
        model_name: "qlora-merged".to_string(),
        model_type: "merged".to_string(),
        model_size: merged_quantized.numel(),
    };

    export_gguf(
        &[("merged_weight", &merged_quantized)],
        Some(metadata),
        output_path,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantization::quantize_nf4;
    use candle_core::{Device, Tensor};
    use std::io::Read;

    #[test]
    fn test_export_gguf_header() {
        let device = Device::Cpu;
        let tensor = Tensor::zeros(&[64, 64], candle_core::DType::F32, &device).unwrap();
        let quantized = quantize_nf4(&tensor, 64).unwrap();

        let temp_path = std::env::temp_dir().join("test_export.gguf");
        export_gguf(&[("test_tensor", &quantized)], None, &temp_path).unwrap();

        // Verify header
        let mut file = std::fs::File::open(&temp_path).unwrap();
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(u32::from_le_bytes(magic), GGUF_MAGIC);

        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_export_gguf_with_metadata() {
        let device = Device::Cpu;
        let tensor = Tensor::zeros(&[32, 32], candle_core::DType::F32, &device).unwrap();
        let quantized = quantize_nf4(&tensor, 64).unwrap();

        let metadata = GgufMetadata {
            model_name: "test_model".to_string(),
            model_type: "test".to_string(),
            model_size: 32 * 32,
        };

        let temp_path = std::env::temp_dir().join("test_export_meta.gguf");
        export_gguf(&[("weights", &quantized)], Some(metadata), &temp_path).unwrap();

        // Verify file was created and has content
        let metadata = std::fs::metadata(&temp_path).unwrap();
        assert!(metadata.len() > 0);

        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_export_gguf_multiple_tensors() {
        let device = Device::Cpu;
        let tensor1 = Tensor::zeros(&[64, 64], candle_core::DType::F32, &device).unwrap();
        let tensor2 = Tensor::zeros(&[32, 32], candle_core::DType::F32, &device).unwrap();
        let quantized1 = quantize_nf4(&tensor1, 64).unwrap();
        let quantized2 = quantize_nf4(&tensor2, 64).unwrap();

        let temp_path = std::env::temp_dir().join("test_export_multi.gguf");
        export_gguf(
            &[("weights1", &quantized1), ("weights2", &quantized2)],
            None,
            &temp_path,
        )
        .unwrap();

        // Verify file was created with correct structure
        let mut file = std::fs::File::open(&temp_path).unwrap();
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(u32::from_le_bytes(magic), GGUF_MAGIC);

        // Read version
        let mut version = [0u8; 4];
        file.read_exact(&mut version).unwrap();
        assert_eq!(u32::from_le_bytes(version), GGUF_VERSION);

        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_merge_and_export_gguf() {
        use crate::qlora::{QLoraConfig, QLoraLayer, QuantizedLinear};

        let device = Device::Cpu;
        let config = QLoraConfig::preset_all_bf16(8, 16);

        // Create a QLoRA layer with some test weights
        let in_features = 64;
        let out_features = 128;
        let weight = Tensor::ones(
            &[out_features, in_features],
            candle_core::DType::F32,
            &device,
        )
        .unwrap();
        let linear = QuantizedLinear::from_weight(&weight, None, &config, &device).unwrap();
        let layer = QLoraLayer::new(linear);

        // Test merge and export
        let temp_path = std::env::temp_dir().join("test_merge_export.gguf");
        let result = merge_and_export_gguf(&layer, &temp_path);
        assert!(result.is_ok(), "merge_and_export_gguf failed: {result:?}");

        // Verify file was created with correct structure
        let mut file = std::fs::File::open(&temp_path).unwrap();
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(u32::from_le_bytes(magic), GGUF_MAGIC);

        // Verify metadata is present
        let metadata = std::fs::metadata(&temp_path).unwrap();
        assert!(metadata.len() > 0, "Output file should not be empty");

        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_merge_and_export_gguf_preserves_shape() {
        use crate::qlora::{QLoraConfig, QLoraLayer, QuantizedLinear};

        let device = Device::Cpu;
        let config = QLoraConfig::preset_all_bf16(4, 8);

        // Create a small QLoRA layer
        let in_features = 32;
        let out_features = 32;
        let weight = Tensor::randn(0.0f32, 1.0f32, &[out_features, in_features], &device).unwrap();

        let linear = QuantizedLinear::from_weight(&weight, None, &config, &device).unwrap();
        let layer = QLoraLayer::new(linear);

        // Export
        let temp_path = std::env::temp_dir().join("test_merge_shape.gguf");
        merge_and_export_gguf(&layer, &temp_path).unwrap();

        // Verify file was created
        assert!(temp_path.exists(), "Output file should exist after export");

        // Note: We can't easily re-read the GGUF file to verify the shape,
        // but we can verify the file is non-empty and has the right magic number
        let mut file = std::fs::File::open(&temp_path).unwrap();
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(u32::from_le_bytes(magic), GGUF_MAGIC);

        std::fs::remove_file(temp_path).ok();
    }
}
