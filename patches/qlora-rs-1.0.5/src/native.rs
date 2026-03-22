//! Candle native quantized format.
//!
//! A native Rust serialization format optimized for the Candle framework.
//! This format is designed for efficient loading and inference without conversion.
//!
//! File structure:
//! ```text
//! Header (32 bytes)
//!   - Magic: "QNAT" (4 bytes)
//!   - Version: u32 (4 bytes)
//!   - Format flags: u32 (4 bytes)
//!   - Metadata size: u64 (8 bytes)
//!   - Tensor count: u32 (4 bytes)
//!   - Reserved: u32 (4 bytes)
//!
//! Metadata section (variable)
//!   - Model name (length-prefixed string)
//!   - Model type (length-prefixed string)
//!   - Compute dtype (1 byte)
//!
//! Tensor headers (variable, one per tensor)
//!   - Name (length-prefixed string)
//!   - Shape (length-prefixed u64 array)
//!   - Block size: u32
//!   - Offset: u64
//!   - Num blocks: u32
//!
//! Tensor data (variable)
//!   - Quantized values (2 values per byte)
//!   - Scale factors:
//!       - Either per-block f32 scales (one f32 per block), or
//!       - Double-quantized scales stored as `scales_quantized` and `scales_scales`
//!   - Zero points (optional, f32 per block)
//! ```

use std::io::Write;
use std::path::Path;

use crate::error::{QLoraError, Result};
use crate::quantization::{ComputeDType, QuantizedTensor};

/// Magic bytes for Candle native quantized format.
const MAGIC: &[u8; 4] = b"QNAT";

/// Version of the Candle native format.
const VERSION: u32 = 1;

/// Format flags (reserved for future extensions).
const FORMAT_FLAGS: u32 = 0;

/// Metadata for Candle native format.
#[derive(Debug, Clone)]
pub struct NativeMetadata {
    /// Name of the model.
    pub model_name: String,
    /// Type of the model (e.g., "qlora").
    pub model_type: String,
    /// Compute data type for the model.
    pub compute_dtype: ComputeDType,
}

impl Default for NativeMetadata {
    fn default() -> Self {
        Self {
            model_name: "qlora-model".to_string(),
            model_type: "qlora".to_string(),
            compute_dtype: ComputeDType::F32,
        }
    }
}

/// Export quantized tensors to Candle native format.
///
/// # Arguments
/// * `tensors` - Named quantized tensors to export
/// * `metadata` - Optional metadata for the model
/// * `output_path` - Path to write the native format file
///
/// # Errors
/// Returns error if file cannot be written
pub fn export_native<P: AsRef<Path>>(
    tensors: &[(&str, &QuantizedTensor)],
    metadata: Option<NativeMetadata>,
    output_path: P,
) -> Result<()> {
    let mut file = std::fs::File::create(output_path)
        .map_err(|e| QLoraError::NativeExport(format!("Failed to create output file: {e}")))?;

    let metadata = metadata.unwrap_or_default();

    // Write header
    write_header(&mut file, tensors.len())?;

    // Write metadata
    write_metadata(&mut file, &metadata)?;

    // Write tensor headers and calculate offsets
    let _tensor_offsets = write_tensor_headers(&mut file, tensors)?;

    // Write tensor data sequentially in the same order as the headers
    for (_name, tensor) in tensors {
        write_tensor_data(&mut file, tensor)?;
    }

    Ok(())
}

/// Write file header.
fn write_header<W: Write>(writer: &mut W, tensor_count: usize) -> Result<()> {
    // Magic
    writer
        .write_all(MAGIC)
        .map_err(|e| QLoraError::NativeExport(format!("Failed to write magic: {e}")))?;

    // Version
    writer
        .write_all(&VERSION.to_le_bytes())
        .map_err(|e| QLoraError::NativeExport(format!("Failed to write version: {e}")))?;

    // Format flags
    writer
        .write_all(&FORMAT_FLAGS.to_le_bytes())
        .map_err(|e| QLoraError::NativeExport(format!("Failed to write flags: {e}")))?;

    // Metadata size (placeholder, will be updated later if needed)
    writer
        .write_all(&0u64.to_le_bytes())
        .map_err(|e| QLoraError::NativeExport(format!("Failed to write metadata size: {e}")))?;

    // Tensor count
    let count = u32::try_from(tensor_count)
        .map_err(|_| QLoraError::NativeExport("Too many tensors".into()))?;
    writer
        .write_all(&count.to_le_bytes())
        .map_err(|e| QLoraError::NativeExport(format!("Failed to write tensor count: {e}")))?;

    // Reserved
    writer
        .write_all(&0u32.to_le_bytes())
        .map_err(|e| QLoraError::NativeExport(format!("Failed to write reserved: {e}")))?;

    Ok(())
}

/// Write metadata section.
fn write_metadata<W: Write>(writer: &mut W, metadata: &NativeMetadata) -> Result<()> {
    // Model name
    write_string(writer, &metadata.model_name)?;

    // Model type
    write_string(writer, &metadata.model_type)?;

    // Compute dtype
    let dtype_byte = match metadata.compute_dtype {
        ComputeDType::F32 => 0u8,
        ComputeDType::F16 => 1u8,
        ComputeDType::BF16 => 2u8,
    };
    writer
        .write_all(&[dtype_byte])
        .map_err(|e| QLoraError::NativeExport(format!("Failed to write compute dtype: {e}")))?;

    Ok(())
}

/// Write tensor headers and return offsets.
fn write_tensor_headers<W: Write>(
    writer: &mut W,
    tensors: &[(&str, &QuantizedTensor)],
) -> Result<Vec<u64>> {
    let mut offsets = Vec::new();
    let mut current_offset = calculate_header_size(tensors);

    for (_name, tensor) in tensors {
        offsets.push(current_offset as u64);
        current_offset += calculate_tensor_size(tensor);
    }

    // Write tensor headers
    for ((name, tensor), offset) in tensors.iter().zip(offsets.iter()) {
        write_string(writer, name)?;

        // Shape
        let shape_len = u32::try_from(tensor.shape.len())
            .map_err(|_| QLoraError::NativeExport("Tensor shape too large".into()))?;
        writer
            .write_all(&shape_len.to_le_bytes())
            .map_err(|e| QLoraError::NativeExport(format!("Failed to write shape length: {e}")))?;

        for &dim in &tensor.shape {
            let dim = u64::try_from(dim)
                .map_err(|_| QLoraError::NativeExport("Dimension too large".into()))?;
            writer
                .write_all(&dim.to_le_bytes())
                .map_err(|e| QLoraError::NativeExport(format!("Failed to write dimension: {e}")))?;
        }

        // Block size
        let block_size = u32::try_from(tensor.block_size)
            .map_err(|_| QLoraError::NativeExport("Block size too large".into()))?;
        writer
            .write_all(&block_size.to_le_bytes())
            .map_err(|e| QLoraError::NativeExport(format!("Failed to write block size: {e}")))?;

        // Offset
        writer
            .write_all(&offset.to_le_bytes())
            .map_err(|e| QLoraError::NativeExport(format!("Failed to write offset: {e}")))?;

        // Number of blocks
        let num_blocks = u32::try_from(tensor.scales.len())
            .map_err(|_| QLoraError::NativeExport("Too many blocks".into()))?;
        writer
            .write_all(&num_blocks.to_le_bytes())
            .map_err(|e| QLoraError::NativeExport(format!("Failed to write block count: {e}")))?;
    }

    Ok(offsets)
}

/// Write tensor data (quantized values, scales, optional zero points).
fn write_tensor_data<W: Write>(writer: &mut W, tensor: &QuantizedTensor) -> Result<()> {
    // Quantized values
    writer
        .write_all(&tensor.data)
        .map_err(|e| QLoraError::NativeExport(format!("Failed to write quantized data: {e}")))?;

    // Scale factors
    for &scale in &tensor.scales {
        writer
            .write_all(&scale.to_le_bytes())
            .map_err(|e| QLoraError::NativeExport(format!("Failed to write scale: {e}")))?;
    }

    // Zero points (if present)
    if let Some(ref zp) = tensor.zero_points {
        for &zp_val in zp {
            writer.write_all(&zp_val.to_le_bytes()).map_err(|e| {
                QLoraError::NativeExport(format!("Failed to write zero point: {e}"))
            })?;
        }
    }

    // Double-quantized scale data (if present)
    if let Some(ref scales_q) = tensor.scales_quantized {
        writer.write_all(scales_q).map_err(|e| {
            QLoraError::NativeExport(format!("Failed to write double-quantized scales: {e}"))
        })?;
    }
    if let Some(ref scales_s) = tensor.scales_scales {
        for &scale_s in scales_s {
            writer.write_all(&scale_s.to_le_bytes()).map_err(|e| {
                QLoraError::NativeExport(format!(
                    "Failed to write double-quantized scale factors: {e}"
                ))
            })?;
        }
    }

    Ok(())
}

/// Write a length-prefixed string.
fn write_string<W: Write>(writer: &mut W, s: &str) -> Result<()> {
    let bytes = s.as_bytes();
    let len = u32::try_from(bytes.len())
        .map_err(|_| QLoraError::NativeExport("String too long".into()))?;
    writer
        .write_all(&len.to_le_bytes())
        .map_err(|e| QLoraError::NativeExport(format!("Failed to write string length: {e}")))?;
    writer
        .write_all(bytes)
        .map_err(|e| QLoraError::NativeExport(format!("Failed to write string: {e}")))?;
    Ok(())
}

/// Calculate size of header (before tensor data).
fn calculate_header_size(tensors: &[(&str, &QuantizedTensor)]) -> usize {
    let mut size = 32; // Fixed header size

    // Metadata (estimate: names + compute dtype)
    size += 4 + 11 + 4 + 8 + 1; // "qlora-model" + "qlora" + dtype

    // Tensor headers
    for (name, tensor) in tensors {
        size += 4 + name.len(); // name
        size += 4 + tensor.shape.len() * 8; // shape
        size += 4; // block size
        size += 8; // offset
        size += 4; // num blocks
    }

    size
}

/// Calculate size of tensor data.
fn calculate_tensor_size(tensor: &QuantizedTensor) -> usize {
    let mut size = tensor.data.len(); // quantized values
    size += tensor.scales.len() * 4; // scale factors
    if let Some(ref zp) = tensor.zero_points {
        size += zp.len() * 4; // zero points
    }
    // Double-quantized scale data (if present)
    if let Some(ref scales_q) = tensor.scales_quantized {
        size += std::mem::size_of_val(scales_q.as_slice());
    }
    if let Some(ref scales_s) = tensor.scales_scales {
        size += std::mem::size_of_val(scales_s.as_slice());
    }
    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantization::quantize_nf4;
    use candle_core::{Device, Tensor};
    use std::io::Read;

    #[test]
    fn test_export_native_basic() {
        let device = Device::Cpu;
        let tensor = Tensor::zeros(&[64, 64], candle_core::DType::F32, &device).unwrap();
        let quantized = quantize_nf4(&tensor, 64).unwrap();

        let temp_path = std::env::temp_dir().join("test_native.qnat");
        export_native(&[("test_tensor", &quantized)], None, &temp_path).unwrap();

        // Verify magic bytes
        let mut file = std::fs::File::open(&temp_path).unwrap();
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, MAGIC);

        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_export_native_with_metadata() {
        let device = Device::Cpu;
        let tensor = Tensor::zeros(&[32, 32], candle_core::DType::F32, &device).unwrap();
        let quantized = quantize_nf4(&tensor, 64).unwrap();

        let metadata = NativeMetadata {
            model_name: "test_model".to_string(),
            model_type: "test".to_string(),
            compute_dtype: ComputeDType::F32,
        };

        let temp_path = std::env::temp_dir().join("test_native_meta.qnat");
        export_native(&[("weights", &quantized)], Some(metadata), &temp_path).unwrap();

        // Verify file was created
        let file_meta = std::fs::metadata(&temp_path).unwrap();
        assert!(file_meta.len() > 0);

        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_export_native_multiple_tensors() {
        let device = Device::Cpu;
        let t1 = Tensor::zeros(&[64, 64], candle_core::DType::F32, &device).unwrap();
        let t2 = Tensor::zeros(&[32, 32], candle_core::DType::F32, &device).unwrap();
        let q1 = quantize_nf4(&t1, 64).unwrap();
        let q2 = quantize_nf4(&t2, 64).unwrap();

        let temp_path = std::env::temp_dir().join("test_native_multi.qnat");
        export_native(&[("w1", &q1), ("w2", &q2)], None, &temp_path).unwrap();

        // Verify file structure
        let mut file = std::fs::File::open(&temp_path).unwrap();
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, MAGIC);

        std::fs::remove_file(temp_path).ok();
    }
}
