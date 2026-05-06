//! Write capability routing preferences to the Clavis vault.

use clap::{Parser, Subcommand};
use vox_clavis::SecretId;

#[derive(Parser)]
pub struct PreferencesArgs {
    #[command(subcommand)]
    pub cmd: PreferencesCmd,
}

#[derive(Subcommand)]
pub enum PreferencesCmd {
    /// Set a capability preference flag or pinned model id.
    ///
    /// Keys: `require_tool_use`, `require_reasoning`, `require_web_search`,
    /// `require_image_generation`, `prefer_reasoning`,
    /// `image_model`, `vision_model`, `codegen_model` (values are `true`/`false` or a model id).
    Set {
        key: String,
        value: String,
    },
    /// Clear all `VOX_CAPABILITY_*` vault entries (best-effort).
    Reset,
}

fn secret_for_key(key: &str) -> anyhow::Result<SecretId> {
    match key.trim().to_ascii_lowercase().as_str() {
        "require_image_generation" => Ok(SecretId::VoxCapabilityRequireImageGeneration),
        "require_tool_use" => Ok(SecretId::VoxCapabilityRequireToolUse),
        "require_reasoning" => Ok(SecretId::VoxCapabilityRequireReasoning),
        "require_web_search" => Ok(SecretId::VoxCapabilityRequireWebSearch),
        "prefer_reasoning" => Ok(SecretId::VoxCapabilityPreferReasoning),
        "image_model" | "image_generation_model" => Ok(SecretId::VoxCapabilityImageGenerationModel),
        "vision_model" => Ok(SecretId::VoxCapabilityVisionModel),
        "codegen_model" | "code_model" => Ok(SecretId::VoxCapabilityCodeGenModel),
        other => anyhow::bail!(
            "unknown preference key {other:?}; see `vox model preferences set --help`"
        ),
    }
}

pub async fn run(args: PreferencesArgs) -> anyhow::Result<()> {
    match args.cmd {
        PreferencesCmd::Set { key, value } => {
            let id = secret_for_key(&key)?;
            let spec = id.spec();
            let backend = vox_clavis::backend::vox_vault::VoxCloudBackend::new()
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            backend
                .write_secret(spec.canonical_env, value.trim())
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            println!("wrote {} ({})", spec.canonical_env, spec.scope_description);
        }
        PreferencesCmd::Reset => {
            let backend = vox_clavis::backend::vox_vault::VoxCloudBackend::new()
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            for id in [
                SecretId::VoxCapabilityRequireImageGeneration,
                SecretId::VoxCapabilityRequireToolUse,
                SecretId::VoxCapabilityRequireReasoning,
                SecretId::VoxCapabilityRequireWebSearch,
                SecretId::VoxCapabilityPreferReasoning,
                SecretId::VoxCapabilityImageGenerationModel,
                SecretId::VoxCapabilityVisionModel,
                SecretId::VoxCapabilityCodeGenModel,
            ] {
                let spec = id.spec();
                let _ = backend.write_secret(spec.canonical_env, "");
                println!("cleared {}", spec.canonical_env);
            }
        }
    }
    Ok(())
}
