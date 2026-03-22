//! Export format selection and unified interface.
//!
//! Provides a unified API for exporting quantized models in different formats
//! with user-selectable backends.

use std::path::Path;

use crate::error::Result;
use crate::quantization::QuantizedTensor;
use crate::{export, native};

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportFormat {
    /// GGUF format (compatible with llama.cpp ecosystem).
    #[default]
    Gguf,
    /// Candle native format (optimized for Candle framework).
    Native,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gguf => write!(f, "GGUF"),
            Self::Native => write!(f, "Candle Native"),
        }
    }
}

/// Export configuration for quantized models.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Target export format.
    pub format: ExportFormat,
    /// Model name for metadata.
    pub model_name: String,
    /// Model type for metadata.
    pub model_type: String,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Gguf,
            model_name: "qlora-model".to_string(),
            model_type: "qlora".to_string(),
        }
    }
}

impl ExportConfig {
    /// Create a new export configuration with GGUF format.
    #[must_use]
    pub fn new_gguf() -> Self {
        Self {
            format: ExportFormat::Gguf,
            ..Default::default()
        }
    }

    /// Create a new export configuration with native format.
    #[must_use]
    pub fn new_native() -> Self {
        Self {
            format: ExportFormat::Native,
            ..Default::default()
        }
    }

    /// Set the format for this export configuration.
    #[must_use]
    pub fn with_format(mut self, format: ExportFormat) -> Self {
        self.format = format;
        self
    }

    /// Set the model name for metadata.
    #[must_use]
    pub fn with_model_name(mut self, name: String) -> Self {
        self.model_name = name;
        self
    }

    /// Set the model type for metadata.
    #[must_use]
    pub fn with_model_type(mut self, model_type: String) -> Self {
        self.model_type = model_type;
        self
    }
}

/// Export quantized tensors using the specified format.
///
/// # Arguments
/// * `tensors` - Named quantized tensors to export
/// * `config` - Export configuration with format selection
/// * `output_path` - Path to write the exported file
///
/// # Errors
/// Returns error if export fails
pub fn export_model<P: AsRef<Path>>(
    tensors: &[(&str, &QuantizedTensor)],
    config: ExportConfig,
    output_path: P,
) -> Result<()> {
    match config.format {
        ExportFormat::Gguf => {
            let metadata = export::GgufMetadata {
                model_name: config.model_name,
                model_type: config.model_type,
                model_size: tensors.iter().map(|(_, t)| t.numel()).sum(),
            };
            export::export_gguf(tensors, Some(metadata), output_path)
        }
        ExportFormat::Native => {
            let metadata = native::NativeMetadata {
                model_name: config.model_name,
                model_type: config.model_type,
                compute_dtype: crate::quantization::ComputeDType::F32,
            };
            native::export_native(tensors, Some(metadata), output_path)
        }
    }
}

/// Export quantized tensors with default GGUF format.
///
/// # Arguments
/// * `tensors` - Named quantized tensors to export
/// * `output_path` - Path to write the GGUF file
///
/// # Errors
/// Returns error if export fails
pub fn export_gguf<P: AsRef<Path>>(
    tensors: &[(&str, &QuantizedTensor)],
    output_path: P,
) -> Result<()> {
    export_model(tensors, ExportConfig::new_gguf(), output_path)
}

/// Export quantized tensors to native Candle format.
///
/// # Arguments
/// * `tensors` - Named quantized tensors to export
/// * `output_path` - Path to write the native format file
///
/// # Errors
/// Returns error if export fails
pub fn export_native_format<P: AsRef<Path>>(
    tensors: &[(&str, &QuantizedTensor)],
    output_path: P,
) -> Result<()> {
    export_model(tensors, ExportConfig::new_native(), output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantization::quantize_nf4;
    use candle_core::{Device, Tensor};

    #[test]
    fn test_export_config_builder() {
        let config = ExportConfig::default()
            .with_format(ExportFormat::Native)
            .with_model_name("my_model".to_string());

        assert_eq!(config.format, ExportFormat::Native);
        assert_eq!(config.model_name, "my_model");
    }

    #[test]
    fn test_export_gguf_via_unified_api() {
        let device = Device::Cpu;
        let tensor = Tensor::zeros(&[32, 32], candle_core::DType::F32, &device).unwrap();
        let quantized = quantize_nf4(&tensor, 64).unwrap();

        let temp_path = std::env::temp_dir().join("test_unified_gguf.gguf");
        export_gguf(&[("weights", &quantized)], &temp_path).unwrap();

        assert!(std::fs::metadata(&temp_path).is_ok());
        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_export_native_via_unified_api() {
        let device = Device::Cpu;
        let tensor = Tensor::zeros(&[32, 32], candle_core::DType::F32, &device).unwrap();
        let quantized = quantize_nf4(&tensor, 64).unwrap();

        let temp_path = std::env::temp_dir().join("test_unified_native.qnat");
        export_native_format(&[("weights", &quantized)], &temp_path).unwrap();

        assert!(std::fs::metadata(&temp_path).is_ok());
        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_export_model_with_config() {
        let device = Device::Cpu;
        let tensor = Tensor::zeros(&[32, 32], candle_core::DType::F32, &device).unwrap();
        let quantized = quantize_nf4(&tensor, 64).unwrap();

        let config = ExportConfig::default()
            .with_format(ExportFormat::Native)
            .with_model_name("test_model".to_string())
            .with_model_type("test".to_string());

        let temp_path = std::env::temp_dir().join("test_config_export.qnat");
        export_model(&[("weights", &quantized)], config, &temp_path).unwrap();

        assert!(std::fs::metadata(&temp_path).is_ok());
        std::fs::remove_file(temp_path).ok();
    }
}
