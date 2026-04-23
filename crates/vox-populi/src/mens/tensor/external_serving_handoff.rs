//! Machine-readable serving handoff (`contracts/eval/external-serving-handoff.schema.json`).

use std::path::Path;

use serde::Serialize;

/// Payload for `schema` = `vox_external_serving_handoff_v1`.
#[derive(Debug, Clone, Serialize)]
pub struct ExternalServingHandoffV1 {
    pub schema: &'static str,
    pub schema_version: i32,
    pub backend_family: &'static str,
    pub base_model: String,
    pub tokenizer_source: String,
    pub artifact_dir: String,
    pub prompt_format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl ExternalServingHandoffV1 {
    const SCHEMA: &'static str = "vox_external_serving_handoff_v1";

    /// After Candle QLoRA training: serve locally with `vox-schola serve --model <artifact_dir>`.
    #[must_use]
    pub fn schola_training_run(run_dir: &Path, base_model: &str, adapter_filename: &str) -> Self {
        let tok = run_dir.join("tokenizer.json");
        Self {
            schema: Self::SCHEMA,
            schema_version: 1,
            backend_family: "openai-compatible",
            base_model: base_model.to_string(),
            tokenizer_source: tok.display().to_string(),
            artifact_dir: run_dir.display().to_string(),
            prompt_format: "qwen_chatml_im_start".to_string(),
            adapter_path: Some(adapter_filename.to_string()),
            openai_base_url: None,
            notes: Some(
                "Local: vox-schola serve (OpenAI /v1/chat/completions + Ollama-shaped /api/generate, /api/chat). Set POPULI_URL to http://HOST:PORT and POPULI_MODEL to match --model-name or run directory name."
                    .to_string(),
            ),
        }
    }

    /// After merge-qlora: merged safetensors subset for external HF / vLLM / Ollama import workflows.
    #[must_use]
    pub fn merged_qlora_subset(
        merged_shard_path: &Path,
        base_model: &str,
        tokenizer_hint: Option<&str>,
    ) -> Self {
        let parent = merged_shard_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| Path::new(".").to_path_buf());
        let tok = tokenizer_hint
            .map(String::from)
            .unwrap_or_else(|| "tokenizer.json from original training run directory".to_string());
        Self {
            schema: Self::SCHEMA,
            schema_version: 1,
            backend_family: "other",
            base_model: base_model.to_string(),
            tokenizer_source: tok,
            artifact_dir: parent.display().to_string(),
            prompt_format: "qwen_chatml_im_start".to_string(),
            adapter_path: Some(merged_shard_path.display().to_string()),
            openai_base_url: None,
            notes: Some(
                "Merged f32 subset shard(s); not loaded by vox-schola serve. Use vLLM, HF Transformers, or import paths documented in mens-serving-ssot.md."
                    .to_string(),
            ),
        }
    }
}

/// Writes `external_serving_handoff_v1.json` into `artifact_dir`.
pub fn write_handoff(
    artifact_dir: &Path,
    handoff: &ExternalServingHandoffV1,
) -> anyhow::Result<()> {
    let p = artifact_dir.join("external_serving_handoff_v1.json");
    let s = serde_json::to_string_pretty(handoff)?;
    std::fs::write(&p, s)?;
    Ok(())
}
